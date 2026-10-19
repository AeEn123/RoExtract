//! Message boxes and file pickers.
//!
//! Native dialogs (native_dialog) are tried first. When they fail, e.g. on Linux
//! without zenity/kdialog, an egui dialog is shown instead:
//! - inside the main window when it is open (callers on other threads block until answered),
//! - in a small standalone window on the main thread before the main window opens,
//! - otherwise as a terminal prompt (CLI mode, headless).
//!
//! Blocking dialogs must not be called from the GUI thread while the egui fallback
//! may be needed, as the GUI thread is the one that draws it. Run them in a thread.

use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{BufRead, IsTerminal, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, LazyLock, Mutex,
    },
    thread::{self, ThreadId},
};

use egui_file_dialog::{DialogState, FileDialog};
use native_dialog::{DialogBuilder, MessageLevel};

use crate::locale;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Alert,
    Confirm,
    PickFolder,
    PickFile,
    SaveFile(String),
}

#[derive(Debug, PartialEq)]
enum Answer {
    Bool(bool),
    Path(Option<PathBuf>),
}

struct Request {
    level: Level,
    title: String,
    text: String,
    kind: Kind,
    /// None for alerts, which never wait for the user.
    reply: Option<mpsc::Sender<Answer>>,
}

/// The main window, when it is open.
struct Host {
    ctx: egui::Context,
    thread: ThreadId,
}

static HOST: LazyLock<Mutex<Option<Host>>> = LazyLock::new(|| Mutex::new(None));
static QUEUE: LazyLock<Mutex<VecDeque<Request>>> = LazyLock::new(|| Mutex::new(VecDeque::new()));
const STANDALONE_FILES_SIZE: egui::Vec2 = egui::vec2(820.0, 560.0);
/// Standalone fallback windows are only opened in GUI mode, never from the CLI.
static WINDOWS_ALLOWED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The fallback dialog currently drawn in the main window (GUI thread only).
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}

impl Kind {
    fn default_answer(&self) -> Answer {
        match self {
            Kind::Alert => Answer::Bool(true),
            Kind::Confirm => Answer::Bool(false),
            _ => Answer::Path(None),
        }
    }
}

// Public API

pub fn alert(level: Level, title: &str, text: &str) {
    run(level, title, text, Kind::Alert);
}

pub fn confirm(level: Level, title: &str, text: &str) -> bool {
    matches!(run(level, title, text, Kind::Confirm), Answer::Bool(true))
}

/// [`alert`] with the title and text looked up from locale keys.
pub fn alert_localised(level: Level, title_key: &str, text_key: &str) {
    let locale = locale::get_locale(None);
    alert(
        level,
        &locale::get_message(&locale, title_key, None),
        &locale::get_message(&locale, text_key, None),
    );
}

/// [`confirm`] with the title and text looked up from locale keys.
pub fn confirm_localised(level: Level, title_key: &str, text_key: &str) -> bool {
    let locale = locale::get_locale(None);
    confirm(
        level,
        &locale::get_message(&locale, title_key, None),
        &locale::get_message(&locale, text_key, None),
    )
}

pub fn pick_folder() -> Option<PathBuf> {
    path_answer(run(Level::Info, "", "", Kind::PickFolder))
}

pub fn pick_file() -> Option<PathBuf> {
    path_answer(run(Level::Info, "", "", Kind::PickFile))
}

pub fn save_file(file_name: &str) -> Option<PathBuf> {
    path_answer(run(
        Level::Info,
        "",
        "",
        Kind::SaveFile(file_name.to_owned()),
    ))
}

/// Run `f` on a new thread so it may block on dialogs without freezing the GUI.
pub fn spawn<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Pending<T> {
    let result = Arc::new(Mutex::new(None));
    let slot = result.clone();
    thread::spawn(move || {
        let value = f();
        *slot.lock().unwrap() = Some(value);
        if let Some(host) = HOST.lock().unwrap().as_ref() {
            host.ctx.request_repaint();
        }
    });
    Pending(result)
}

/// The result of [`spawn`], polled from the GUI each frame.
pub struct Pending<T>(Arc<Mutex<Option<T>>>);

impl<T> Pending<T> {
    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }
}

/// Allow standalone fallback windows (GUI mode).
pub fn allow_windows() {
    WINDOWS_ALLOWED.store(true, Ordering::Relaxed);
}

/// Route fallback dialogs into the main window. Call from the GUI thread.
pub fn attach(ctx: &egui::Context) {
    *HOST.lock().unwrap() = Some(Host {
        ctx: ctx.clone(),
        thread: thread::current().id(),
    });
}

/// The main window closed; unblock anything still waiting with default answers.
pub fn detach() {
    *HOST.lock().unwrap() = None;
    QUEUE.lock().unwrap().clear(); // Drops the reply senders
    let _ = ACTIVE.try_with(|active| active.borrow_mut().take());
}

/// Draw the queued fallback dialog, if any. Call every frame from the main window.
pub fn show(ctx: &egui::Context) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if active.is_none() {
            *active = QUEUE
                .lock()
                .unwrap()
                .pop_front()
                .map(|request| Active::new(request, false));
        }
        if let Some(current) = active.as_mut() {
            if let Some(answer) = current.ui(ctx, false) {
                if let Some(reply) = &current.request.reply {
                    let _ = reply.send(answer);
                }
                *active = None;
                ctx.request_repaint(); // Show the next one
            }
        }
    });
}

// Internals

fn path_answer(answer: Answer) -> Option<PathBuf> {
    match answer {
        Answer::Path(path) => path,
        Answer::Bool(_) => None,
    }
}

fn run(level: Level, title: &str, text: &str, kind: Kind) -> Answer {
    let request = Request {
        level,
        title: title.to_owned(),
        text: text.to_owned(),
        kind,
        reply: None,
    };
    match native(&request) {
        Ok(answer) => answer,
        Err(e) => {
            log_warn!("Native dialog unavailable, using fallback: {}", e);
            fallback(request)
        }
    }
}

fn native(request: &Request) -> Result<Answer, native_dialog::Error> {
    let level = match request.level {
        Level::Info => MessageLevel::Info,
        Level::Warning => MessageLevel::Warning,
        Level::Error => MessageLevel::Error,
    };
    let message = || {
        DialogBuilder::message()
            .set_level(level)
            .set_title(&request.title)
            .set_text(&request.text)
    };
    match &request.kind {
        Kind::Alert => message().alert().show().map(|_| Answer::Bool(true)),
        Kind::Confirm => message().confirm().show().map(Answer::Bool),
        Kind::PickFolder => DialogBuilder::file()
            .open_single_dir()
            .show()
            .map(Answer::Path),
        Kind::PickFile => DialogBuilder::file()
            .open_single_file()
            .show()
            .map(Answer::Path),
        Kind::SaveFile(file_name) => DialogBuilder::file()
            .set_filename(file_name)
            .save_single_file()
            .show()
            .map(Answer::Path),
    }
}

fn fallback(mut request: Request) -> Answer {
    let host = HOST
        .lock()
        .unwrap()
        .as_ref()
        .map(|host| (host.ctx.clone(), host.thread));

    if let Some((ctx, gui_thread)) = host {
        let default = request.kind.default_answer();
        if request.kind == Kind::Alert {
            QUEUE.lock().unwrap().push_back(request);
            ctx.request_repaint();
            return default;
        }
        if thread::current().id() == gui_thread {
            // Waiting here would stop the GUI from ever drawing the dialog.
            log_error!("A blocking dialog was requested on the GUI thread");
            return default;
        }
        let (sender, receiver) = mpsc::channel();
        request.reply = Some(sender);
        QUEUE.lock().unwrap().push_back(request);
        ctx.request_repaint();
        return receiver.recv().unwrap_or(default); // Err: the window closed
    }

    if WINDOWS_ALLOWED.load(Ordering::Relaxed) && thread::current().name() == Some("main") {
        if let Some(answer) = standalone(&request) {
            return answer;
        }
    }
    terminal(&request)
}

/// Show the dialog in its own window. None if no window could be opened.
fn standalone(request: &Request) -> Option<Answer> {
    let answer = Arc::new(Mutex::new(None));
    let app = StandaloneApp {
        active: Active::new(
            Request {
                level: request.level,
                title: request.title.clone(),
                text: request.text.clone(),
                kind: request.kind.clone(),
                reply: None,
            },
            true,
        ),
        answer: answer.clone(),
    };
    let size = if app.active.files.is_some() {
        STANDALONE_FILES_SIZE
    } else {
        egui::vec2(460.0, 200.0)
    };
    let title = if request.title.is_empty() {
        "RoExtract".to_owned()
    } else {
        request.title.clone()
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(size),
        ..Default::default()
    };

    match eframe::run_native(&title, options, Box::new(|_| Ok(Box::new(app)))) {
        Ok(()) => Some(
            answer
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| request.kind.default_answer()), // Window closed
        ),
        Err(e) => {
            log_warn!("Failed to open a dialog window: {}", e);
            None
        }
    }
}

fn terminal(request: &Request) -> Answer {
    let locale = locale::get_locale(None);
    if !request.title.is_empty() || !request.text.is_empty() {
        eprintln!("{}\n{}", request.title, request.text);
    }
    let interactive = std::io::stdin().is_terminal();
    let read_line = |prompt: &str| -> String {
        eprint!("{prompt} ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        line.trim().to_owned()
    };

    match &request.kind {
        Kind::Alert => Answer::Bool(true),
        Kind::Confirm => {
            if !interactive {
                return Answer::Bool(false);
            }
            let answer = read_line("[y/N]");
            Answer::Bool(answer.to_lowercase().starts_with('y'))
        }
        _ => {
            if !interactive {
                return Answer::Path(None);
            }
            let answer = read_line(&locale::get_message(&locale, "dialog-enter-path", None));
            Answer::Path((!answer.is_empty()).then(|| PathBuf::from(answer)))
        }
    }
}

/// A fallback dialog being drawn.
struct Active {
    request: Request,
    files: Option<FileDialog>,
    ok: String,
    yes: String,
    no: String,
}

impl Active {
    fn new(request: Request, standalone: bool) -> Self {
        let new_file_dialog = || {
            if standalone {
                // Fill the window it has to itself
                FileDialog::new()
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .default_size(STANDALONE_FILES_SIZE - egui::vec2(16.0, 16.0))
                    .movable(false)
                    .resizable(false)
            } else {
                FileDialog::new()
            }
        };
        let files = match &request.kind {
            Kind::PickFolder => {
                let mut dialog = new_file_dialog();
                dialog.pick_directory();
                Some(dialog)
            }
            Kind::PickFile => {
                let mut dialog = new_file_dialog();
                dialog.pick_file();
                Some(dialog)
            }
            Kind::SaveFile(file_name) => {
                let mut dialog = new_file_dialog().default_file_name(file_name);
                dialog.save_file();
                Some(dialog)
            }
            Kind::Alert | Kind::Confirm => None,
        };
        let locale = locale::get_locale(None);
        Self {
            request,
            files,
            ok: locale::get_message(&locale, "button-ok", None),
            yes: locale::get_message(&locale, "button-yes", None),
            no: locale::get_message(&locale, "button-no", None),
        }
    }

    /// Draw one frame. Returns the answer once the user has given one.
    fn ui(&mut self, ctx: &egui::Context, standalone: bool) -> Option<Answer> {
        if let Some(files) = &mut self.files {
            files.update(ctx);
            return match files.state() {
                DialogState::Picked(path) => Some(Answer::Path(Some(path))),
                DialogState::Open => None,
                _ => Some(Answer::Path(None)), // Cancelled or closed
            };
        }

        let content = |ui: &mut egui::Ui| -> Option<Answer> {
            let colour = match self.request.level {
                Level::Info => ui.visuals().strong_text_color(),
                Level::Warning => ui.visuals().warn_fg_color,
                Level::Error => ui.visuals().error_fg_color,
            };
            ui.set_max_width(420.0);
            if !self.request.title.is_empty() {
                ui.heading(egui::RichText::new(&self.request.title).color(colour));
            }
            ui.label(&self.request.text);
            ui.add_space(8.0);

            let mut answer = None;
            ui.horizontal(|ui| match self.request.kind {
                Kind::Confirm => {
                    if ui.button(&self.yes).clicked() {
                        answer = Some(Answer::Bool(true));
                    }
                    if ui.button(&self.no).clicked() {
                        answer = Some(Answer::Bool(false));
                    }
                }
                _ => {
                    if ui.button(&self.ok).clicked() {
                        answer = Some(Answer::Bool(true));
                    }
                }
            });
            answer
        };

        if standalone {
            egui::CentralPanel::default().show(ctx, content).inner
        } else {
            let modal =
                egui::Modal::new(egui::Id::new("roextract_fallback_dialog")).show(ctx, content);
            if modal.inner.is_none() && modal.should_close() {
                return Some(self.request.kind.default_answer()); // Escape or clicked outside
            }
            modal.inner
        }
    }
}

struct StandaloneApp {
    active: Active,
    answer: Arc<Mutex<Option<Answer>>>,
}

impl eframe::App for StandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.answer.lock().unwrap().is_some() {
            return; // Waiting for the window to close
        }
        if let Some(answer) = self.active.ui(ctx, true) {
            *self.answer.lock().unwrap() = Some(answer);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_answers() {
        assert_eq!(Kind::Alert.default_answer(), Answer::Bool(true));
        assert_eq!(Kind::Confirm.default_answer(), Answer::Bool(false));
        assert_eq!(Kind::PickFolder.default_answer(), Answer::Path(None));
        assert_eq!(
            Kind::SaveFile("a".to_owned()).default_answer(),
            Answer::Path(None)
        );
    }

    #[test]
    fn test_spawn_pending_returns_value() {
        let pending = spawn(|| 42);
        let mut value = None;
        for _ in 0..200 {
            value = pending.take();
            if value.is_some() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(value, Some(42));
    }
}

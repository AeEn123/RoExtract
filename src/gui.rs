// Used for gui
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use fluent_bundle::{FluentBundle, FluentResource};
use native_dialog::{DialogBuilder};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use std::sync::Arc;

use std::collections::HashMap;
// Used for input
use crate::{config, locale, log, logic, updater}; // Used for functionality
use eframe::egui::TextureHandle;

mod file_list;
mod settings;
mod welcome;

const VERSION: &str = env!("CARGO_PKG_VERSION"); // Get version for use in the title bar
const COMPILE_DATE: &str = env!("COMPILE_DATE");
const ICON: &[u8; 11400] = include_bytes!("../assets/icon.png");
const CONTRIBUTORS: [&str; 6] = [
    "AeEn123",
    "Vonercent",
    "MarcelDev",
    "JustKanade",
    "IDDQD1337",
    "yuk1n0w",
];
const DEPENDENCIES: [[&str; 2]; 14] = [
    ["https://github.com/emilk/egui", ""],
    ["https://github.com/Adanos020/egui_dock", ""],
    ["https://github.com/lampsitter/egui_commonmark", ""],
    ["https://github.com/native-dialog-rs/native-dialog-rs", ""],
    ["https://github.com/projectfluent/fluent-rs", ""],
    ["https://github.com/1Password/sys-locale", ""],
    ["https://github.com/zbraniecki/unic-locale", ""],
    ["https://github.com/clap-rs/clap", ""],
    ["https://github.com/ardaku/whoami", ""],
    ["https://github.com/seanmonstar/reqwest", ""],
    ["https://github.com/serde-rs/json", ""],
    ["https://github.com/Peternator7/strum", ""],
    ["https://github.com/chronotope/chrono", ""],
    ["https://github.com/image-rs/image", ""],
];

pub static IMAGES: LazyLock<Mutex<HashMap<String, TextureHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct TabViewer<'a> {
    locale: &'a mut FluentBundle<Arc<FluentResource>>,
    file_list_ui: &'a mut file_list::FileListUi
}

pub fn load_image(
    id: &str,
    data: &[u8],
    ctx: egui::Context,
) -> Result<TextureHandle, image::ImageError> {
    let images = { IMAGES.lock().unwrap().clone() };
    if let Some(texture) = images.get(id) {
        Ok(texture.clone())
    } else {
        let icon_image = image::load_from_memory(data)?;
        let icon_rgba = icon_image.to_rgba8();
        let icon_size = [icon_rgba.width() as usize, icon_rgba.height() as usize];
        let texture = ctx.load_texture(
            id,
            egui::ColorImage::from_rgba_unmultiplied(
                icon_size,
                icon_rgba.as_flat_samples().as_slice(),
            ),
            Default::default(),
        );
        let mut images = IMAGES.lock().unwrap();
        images.insert(id.to_string(), texture.clone());
        Ok(texture)
    }
}

fn add_dependency_credit(dependency: [&str; 2], ui: &mut egui::Ui, sponsor_message: &str) {
    if !dependency[1].is_empty() {
        ui.horizontal(|ui| {
            ui.hyperlink_to(
                dependency[0].replace("https://github.com/", ""),
                dependency[0],
            );
            ui.label("|");
            ui.hyperlink_to(sponsor_message, dependency[1]);
        });
    } else {
        ui.hyperlink_to(
            dependency[0].replace("https://github.com/", ""),
            dependency[0],
        );
    }
}

impl TabViewer<'_> {}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        locale::get_message(self.locale, &*tab, None).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if tab != "settings" && tab != "about" && tab != "logs" {
            // This is only shown on tabs other than settings (Extracting assets)
            self.file_list_ui.ui(tab.to_string(), ui);
        } else if tab == "settings" {
            // This is only shown in the settings tab

            settings::actions(ui, self.locale);
            settings::cache_dir_management(ui, self.locale);
            settings::sql_db_management(ui, self.locale);
            settings::behavior(ui, self.locale);
            settings::updates(ui, self.locale);

            if settings::language(ui, self.locale) {
                // This returns true if the locales need to be refreshed
                *self.locale = locale::get_locale(None);
                self.file_list_ui.locale = locale::get_locale(None);
            }
        } else if tab == "logs" {
            ui.heading(locale::get_message(self.locale, "logs", None));
            ui.label(locale::get_message(self.locale, "logs-description", None));

            let mut hide_username_from_logs =
                config::get_config_bool("hide_username_from_logs").unwrap_or(true);

            let logs = if hide_username_from_logs {
                log::get_anonymous_logs()
            } else {
                log::get_logs()
            };
            let lines = logs.lines();

            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut hide_username_from_logs,
                    locale::get_message(self.locale, "checkbox-hide-user-logs", None),
                );
                config::set_config_value("hide_username_from_logs", hide_username_from_logs.into());

                if ui
                    .button(locale::get_message(self.locale, "button-copy-logs", None))
                    .clicked()
                {
                    ui.ctx().copy_text(logs.clone());
                }
                if ui
                    .button(locale::get_message(self.locale, "button-export-logs", None))
                    .clicked()
                {
                    if let Some(path) = DialogBuilder::file().save_single_file().show().unwrap() {
                        if let Err(e) = std::fs::write(path, logs.clone()) {
                            log_critical!("Failed to save logs: {}", e);
                        }
                    }
                }
            });

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    for line in lines {
                        let colour = if line.contains("WARN") {
                            egui::Color32::from_rgb(150, 150, 0)
                        } else if line.contains("ERROR") | line.contains("CRITICAL") {
                            egui::Color32::RED
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.colored_label(colour, line);
                    }
                });
        } else {
            // This is only shown in the about tab

            // Display logo and name side by side
            ui.horizontal(|ui| {
                if let Ok(texture) = load_image("ICON", ICON, ui.ctx().clone()) {
                    ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(40.0, 40.0)));
                }
                ui.vertical(|ui| {
                    ui.heading("RoExtract");

                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("version", VERSION);
                    args.set("date", COMPILE_DATE);

                    ui.horizontal(|ui| {
                        ui.label(locale::get_message(self.locale, "version", Some(&args)));
                        ui.label("|");
                        ui.hyperlink_to("Discord", "https://discord.gg/xqNA5jt6DN");
                    });
                })
            });

            ui.separator();

            ui.heading(locale::get_message(
                self.locale,
                "support-project-donate",
                None,
            ));

            ui.horizontal(|ui| {
                ui.hyperlink_to(
                    locale::get_message(self.locale, "support-sponsor", None),
                    "https://github.com/sponsors/AeEn123",
                );
                ui.label("|");
                ui.hyperlink_to(
                    "Roblox",
                    "https://www.roblox.com/communities/10808976/Alfie-Likes-Computers#!/store",
                )
            });

            ui.separator();

            ui.heading(locale::get_message(self.locale, "contributors", None));
            for contributor in CONTRIBUTORS {
                ui.hyperlink_to(
                    format!("@{contributor}"),
                    format!("https://github.com/{contributor}"),
                );
            }

            ui.separator();

            ui.heading(locale::get_message(self.locale, "dependencies", None));

            let sponsor_message = locale::get_message(self.locale, "support-sponsor", None);
            for dependency in DEPENDENCIES {
                add_dependency_credit(dependency, ui, &sponsor_message);
            }
        }
    }
}

struct MyApp {
    tree: DockState<String>,
    tab_map: HashMap<u32, (SurfaceIndex, NodeIndex, usize)>, // Tab map for keyboard navigation
    locale: FluentBundle<Arc<FluentResource>>,
    file_list_ui: file_list::FileListUi
}

impl Default for MyApp {
    fn default() -> Self {
        let tree = DockState::new(vec![
            "music".to_owned(),
            "sounds".to_owned(),
            "images".to_owned(),
            "rbxm-files".to_owned(),
            "ktx-files".to_owned(),
            "settings".to_owned(),
            "logs".to_owned(),
            "about".to_owned(),
        ]);

        // Tab map for keyboard navigation
        let mut tab_map = HashMap::new();

        let surface = SurfaceIndex(0);
        let node = NodeIndex(0);
        for (i, _) in tree.iter_all_tabs().enumerate() {
            tab_map.insert((i as u32) + 1, (surface, node, i));
        }

        Self {
            tree,
            tab_map,
            locale: locale::get_locale(None),
            file_list_ui: file_list::FileListUi::default()
        }
    }
}

fn detect_japanese_font() -> Option<std::path::PathBuf> {
    let font_dirs = [
        "C:\\Windows\\Fonts\\msgothic.ttc",
        "/usr/share/fonts/noto-cjk/NotoSerifCJK-Regular.ttc",
        "~/.local/share/fonts/noto-cjk/NotoSerifCJK-Regular.ttc",
        "~/.fonts/noto-cjk/NotoSerifCJK-Regular.ttc",
    ];

    for font in font_dirs {
        let resolved_font = PathBuf::from(logic::resolve_path(font));
        match std::fs::metadata(&resolved_font) {
            Ok(metadata) => {
                if metadata.is_file() {
                    log_info!("{}: valid", resolved_font.display());
                    return Some(resolved_font);
                }
            }
            Err(e) => {
                log_warn!("{}: invalid - {}", resolved_font.display(), e);
            }
        }
    }
    None
}

// Some code in the function below is taken from this URL
// https://users.rust-lang.org/t/is-posible-egui-change-fonts-to-japanese-how/59662/5
fn init_japanese_font(cc: &eframe::CreationContext<'_>) {
    //Custom font install
    // 1. Create a `FontDefinitions` object.
    let mut font = egui::FontDefinitions::default();
    // Install my own font (maybe supporting non-latin characters):
    // 2. register the font content with a name.
    match detect_japanese_font() {
        Some(font_path) => {
            match std::fs::read(font_path) {
                Ok(bytes) => {
                    font.font_data.insert(
                        "japanese".to_owned(),
                        egui::FontData::from_owned(bytes).into(),
                    );
                    font.families
                        .get_mut(&egui::FontFamily::Monospace)
                        .unwrap()
                        .push("japanese".to_owned());
                    font.families
                        .get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .push("japanese".to_owned());
                    // 3. Configure context with modified `FontDefinitions`.
                    cc.egui_ctx.set_fonts(font);
                }
                Err(e) => {
                    log_error!("Error loading Japanese fonts: {e}");
                }
            }
        }
        None => {
            log_warn!("No Japanese fonts detected, Japanese characters will not render.")
        }
    }
}

pub fn gui_setup(cc: &eframe::CreationContext<'_>) {
    init_japanese_font(cc);

    // Get theme from config
    match config::get_config_string("theme")
        .unwrap_or("system".to_owned())
        .as_str()
    {
        "dark" => cc.egui_ctx.set_theme(egui::Theme::Dark),
        "light" => cc.egui_ctx.set_theme(egui::Theme::Light),
        _ => (),
    }
}


impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        gui_setup(cc);

        Default::default()
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Display the status bar at the bottom
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add(egui::ProgressBar::new(logic::get_progress()).text(logic::get_status()));
        });

        // Switch tabs with keyboard input (num keys)
        if ctx.input(|input| input.modifiers.ctrl || input.modifiers.alt) {
            for i in 1..=self.tab_map.len() as u32 {
                if ctx.input(|input| {
                    input.key_pressed(egui::Key::from_name(&i.to_string()).expect("Invalid key"))
                }) {
                    if let Some(&(surface, node, tab)) = self.tab_map.get(&i) {
                        self.tree
                            .set_active_tab((surface, node, egui_dock::TabIndex(tab)));
                    }
                }
            }
        }

        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show_close_buttons(false)
            .draggable_tabs(false)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .show(
                ctx,
                &mut TabViewer {
                    locale: &mut self.locale,
                    file_list_ui: &mut self.file_list_ui,
                }
            );

        {
            // Allow for different threads to request refresh
            if logic::get_request_repaint() {
                ctx.request_repaint_after(Duration::from_millis(250)); // Delay added here to prevent refreshes from stopping
            }
        }
    }
}

pub fn run_gui() {
    // If the user is not welcomed before, welcome them
    if !config::get_config_bool("welcomed").unwrap_or(false) {
        let _ = welcome::run_gui();
    }

    // Only run GUI after user has been welcomed
    if config::get_config_bool("welcomed").unwrap_or(true) {
        // Check for updates when running GUI
        if config::get_config_bool("check_for_updates").unwrap_or(false) {
            updater::check_for_updates(
                true,
                config::get_config_bool("automatically_install_updates").unwrap_or(false),
            );
        }

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_icon(
                eframe::icon_data::from_png_bytes(&ICON[..]).expect("Failed to load icon"),
            ),
            ..Default::default()
        };

        let result = eframe::run_native(
            &format!("RoExtract v{VERSION}").to_owned(),
            options,
            Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
        );

        if result.is_err() {
            log_critical!("GUI failed: {}", result.unwrap_err());
        }
    }
}

// Used for gui
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use fluent_bundle::{FluentBundle, FluentResource};
use native_dialog::DialogBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use std::collections::{HashMap, HashSet};
// Used for input
use crate::{config, locale, log, logic, updater}; // Used for functionality
use eframe::egui::TextureHandle;

mod file_list;
mod ktx_preview;
mod settings;
mod welcome;

const VERSION: &str = env!("CARGO_PKG_VERSION"); // Get version for use in the title bar
const COMPILE_DATE: &str = env!("COMPILE_DATE");
const ICON: &[u8; 11400] = include_bytes!("../assets/icon.png");
const CONTRIBUTORS: [&str; 7] = [
    "AeEn123",
    "Vonercent",
    "BlankHtmlPage",
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

/// VRAM budget for cached previews (~64 MB).
const TEXTURE_VRAM_BUDGET: usize = 64 * 1024 * 1024;

/// Preview size used before the first insert.
const DEFAULT_PREVIEW_SIZE: u32 = 128;

/// Preview texture max edge: 2x size, floored at 256.
pub fn preview_max_dimension(preview_size: u32) -> u32 {
    (preview_size * 2).max(256)
}

/// Preview size bounds, shared by the settings slider and ctrl+scroll zoom.
pub const PREVIEW_SIZE_MIN: u64 = 16;
pub const PREVIEW_SIZE_MAX: u64 = 512;

/// Cells that fit in `available_width` with `spacing` between them. `ui.horizontal`
/// advances `tile + spacing` per cell but only spaces *between* cells, so n cells
/// need `n*tile + (n-1)*spacing`. Always at least one.
pub fn tiles_per_row(available_width: f32, tile: f32, spacing: f32) -> usize {
    (((available_width + spacing) / (tile + spacing))
        .floor()
        .max(1.0)) as usize
}

/// Scale the preview size by a signed wheel `notches` amount (positive = zoom in).
pub fn preview_size_after_zoom(current: u64, notches: f32) -> u64 {
    if notches == 0.0 {
        return current;
    }
    let scaled = current as f32 * (notches * 0.25).exp();
    (scaled.round() as u64).clamp(PREVIEW_SIZE_MIN, PREVIEW_SIZE_MAX)
}

/// Ctrl+scroll scales the preview size. egui's own zoom gesture goes unused here
/// and its smoothed `zoom_delta()` is too small to change an integer size, so
/// read the raw wheel events.
pub fn preview_size_from_scroll(ctx: &egui::Context, current: u64) -> u64 {
    let zoom_modifier = ctx.options(|o| o.input_options.zoom_modifier);

    let notches = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|event| match event {
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                } if modifiers.matches_any(zoom_modifier) => Some(match unit {
                    egui::MouseWheelUnit::Line => delta.x + delta.y,
                    egui::MouseWheelUnit::Point => (delta.x + delta.y) / 40.0,
                    egui::MouseWheelUnit::Page => (delta.x + delta.y) * 10.0,
                }),
                _ => None,
            })
            .sum::<f32>()
    });

    preview_size_after_zoom(current, notches)
}

/// RGBA8 bytes for a texture of the given edge.
fn texture_bytes(max_dim: u32) -> usize {
    (max_dim as usize) * (max_dim as usize) * 4
}

/// Max cached previews for a preview size, floored at 16.
pub fn max_textures_for_preview(preview_size: u32) -> usize {
    let count = TEXTURE_VRAM_BUDGET / texture_bytes(preview_max_dimension(preview_size)).max(1);
    count.max(16)
}

/// Max cached GPU-compressed KTX previews. ETC2/BC formats cost at most 1 byte per
/// texel, so the same budget holds 4x as many as RGBA previews.
pub fn max_ktx_textures_for_preview(preview_size: u32) -> usize {
    let max_dim = preview_max_dimension(preview_size) as usize;
    (TEXTURE_VRAM_BUDGET / (max_dim * max_dim).max(1)).max(16)
}

/// A cached preview: an egui-managed texture, or a GPU-compressed KTX texture.
#[derive(Clone)]
pub enum PreviewTexture {
    Egui(TextureHandle),
    Ktx(Arc<ktx_preview::KtxTexture>),
}

impl PreviewTexture {
    /// Paint fitted inside `rect`, keeping the aspect ratio.
    pub fn paint(&self, ui: &egui::Ui, rect: egui::Rect) {
        match self {
            Self::Egui(texture) => {
                egui::Image::new(texture).paint_at(ui, fit_rect(rect, texture.size_vec2()))
            }
            Self::Ktx(texture) => {
                ktx_preview::paint(ui, fit_rect(rect, texture.size()), texture.clone())
            }
        }
    }
}

/// Largest rect with `size`'s aspect ratio that fits centred in `rect`.
pub fn fit_rect(rect: egui::Rect, size: egui::Vec2) -> egui::Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return rect;
    }
    let scale = (rect.width() / size.x).min(rect.height() / size.y);
    egui::Rect::from_center_size(rect.center(), size * scale)
}

struct ImageCache<T> {
    /// Texture plus the stamp of its last use, for LRU eviction.
    textures: HashMap<String, (T, u64)>,
    clock: u64,
    max_count: usize,
    /// Ids painted this frame.
    pinned: HashSet<String>,
    /// Ids painted last frame; keeps textures unevictable across the frame boundary.
    pinned_prev: HashSet<String>,
}

impl<T: Clone> ImageCache<T> {
    fn new(max_count: usize) -> Self {
        Self {
            textures: HashMap::new(),
            clock: 0,
            max_count,
            pinned: HashSet::new(),
            pinned_prev: HashSet::new(),
        }
    }

    /// Look up a texture and bump LRU. O(1), as this runs per visible tile per frame.
    fn get(&mut self, id: &str) -> Option<T> {
        self.clock += 1;
        let (texture, last_used) = self.textures.get_mut(id)?;
        *last_used = self.clock;
        Some(texture.clone())
    }

    /// Look up a texture that is about to be painted, pinning it for this frame.
    fn get_pinned(&mut self, id: &str) -> Option<T> {
        let texture = self.get(id)?;
        if !self.pinned.contains(id) {
            self.pinned.insert(id.to_owned());
        }
        Some(texture)
    }

    /// True if `id` was painted this frame or last.
    fn is_pinned(&self, id: &str) -> bool {
        self.pinned.contains(id) || self.pinned_prev.contains(id)
    }

    /// Insert a texture, evicting least recently used non-displayed entries past
    /// `max_count`. Never drops painted textures; stops when only those remain.
    fn insert(&mut self, id: String, texture: T, max_count: usize) {
        self.max_count = max_count;
        self.clock += 1;
        self.textures.insert(id.clone(), (texture, self.clock));
        // Protect until first paint.
        self.pinned.insert(id);
        while self.textures.len() > self.max_count {
            let oldest = self
                .textures
                .iter()
                .filter(|(k, _)| !self.is_pinned(k))
                .min_by_key(|(_, (_, last_used))| *last_used)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(old_id) => {
                    self.textures.remove(&old_id);
                }
                None => break, // only displayed textures left
            }
        }
    }

    /// Start a frame: current pins become previous pins.
    fn begin_frame(&mut self) {
        self.pinned_prev = std::mem::take(&mut self.pinned);
    }

    fn clear(&mut self) {
        self.textures.clear();
        self.pinned.clear();
        self.pinned_prev.clear();
    }
}

static IMAGE_CACHE: LazyLock<Mutex<ImageCache<TextureHandle>>> = LazyLock::new(|| {
    Mutex::new(ImageCache::new(max_textures_for_preview(
        DEFAULT_PREVIEW_SIZE,
    )))
});

/// GPU-compressed KTX previews, budgeted separately from RGBA textures.
static KTX_CACHE: LazyLock<Mutex<ImageCache<Arc<ktx_preview::KtxTexture>>>> = LazyLock::new(|| {
    Mutex::new(ImageCache::new(max_ktx_textures_for_preview(
        DEFAULT_PREVIEW_SIZE,
    )))
});

struct TabViewer<'a> {
    locale: &'a mut FluentBundle<Arc<FluentResource>>,
    file_list_ui: &'a mut file_list::FileListUi,
}

/// Returns a cached texture without cloning the entire cache.
pub fn get_cached_texture(id: &str) -> Option<TextureHandle> {
    IMAGE_CACHE.lock().unwrap().get(id)
}

/// Look up a preview in either cache and pin it for this frame.
pub fn get_cached_preview(id: &str) -> Option<PreviewTexture> {
    // Bind first: an `if let` scrutinee guard would stay locked through the body.
    let ktx_texture = KTX_CACHE.lock().unwrap().get_pinned(id);
    if let Some(texture) = ktx_texture {
        return Some(PreviewTexture::Ktx(texture));
    }
    let texture = IMAGE_CACHE.lock().unwrap().get_pinned(id)?;
    Some(PreviewTexture::Egui(texture))
}

/// Evict all cached textures, freeing GPU memory.
pub fn clear_image_cache() {
    IMAGE_CACHE.lock().unwrap().clear();
    KTX_CACHE.lock().unwrap().clear();
}

/// Rotate the pin sets for a new frame. Call once per frame before rendering.
pub fn begin_frame_textures() {
    IMAGE_CACHE.lock().unwrap().begin_frame();
    KTX_CACHE.lock().unwrap().begin_frame();
}

/// Load a KTX preview. Uses GPU decoding when the context supports the format,
/// otherwise decodes on the CPU into a regular texture.
pub fn load_ktx(
    id: &str,
    data: &[u8],
    ctx: egui::Context,
    preview_size: u32,
) -> Result<(), String> {
    let max_dimension = preview_max_dimension(preview_size);
    let image = logic::ktx::read_preview_level(data, max_dimension)?;

    if ktx_preview::gpu_supports(image.format) {
        let texture = Arc::new(ktx_preview::KtxTexture::new(image));
        KTX_CACHE.lock().unwrap().insert(
            id.to_owned(),
            texture,
            max_ktx_textures_for_preview(preview_size),
        );
        return Ok(());
    }

    let rgba = logic::ktx::decode_to_rgba(&image)?;
    let mut decoded = image::RgbaImage::from_raw(image.width, image.height, rgba)
        .ok_or("Decoded KTX image has the wrong size")?;
    if decoded.width() > max_dimension || decoded.height() > max_dimension {
        decoded = image::DynamicImage::ImageRgba8(decoded)
            .thumbnail(max_dimension, max_dimension)
            .to_rgba8();
    }
    let _ = insert_rgba_texture(id, &decoded, ctx, max_textures_for_preview(preview_size));
    Ok(())
}

fn insert_rgba_texture(
    id: &str,
    rgba: &image::RgbaImage,
    ctx: egui::Context,
    max_textures: usize,
) -> TextureHandle {
    let size = [rgba.width() as usize, rgba.height() as usize];
    let texture = ctx.load_texture(
        id,
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice()),
        Default::default(),
    );
    IMAGE_CACHE
        .lock()
        .unwrap()
        .insert(id.to_string(), texture.clone(), max_textures);
    texture
}

/// Decode, downscale to `max_dimension`, cache up to `max_textures`.
pub fn load_image(
    id: &str,
    data: &[u8],
    ctx: egui::Context,
    max_dimension: Option<u32>,
    max_textures: usize,
) -> Result<TextureHandle, image::ImageError> {
    if let Some(texture) = get_cached_texture(id) {
        return Ok(texture);
    }

    let mut icon_image = image::load_from_memory(data)?;

    // Downscale before GPU upload.
    if let Some(max_dim) = max_dimension {
        if icon_image.width() > max_dim || icon_image.height() > max_dim {
            icon_image = icon_image.thumbnail(max_dim, max_dim);
        }
    }

    Ok(insert_rgba_texture(
        id,
        &icon_image.to_rgba8(),
        ctx,
        max_textures,
    ))
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
            settings::rbx_storage_dir_management(ui, self.locale);
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

            let old_hide = config::get_config_bool("hide_username_from_logs").unwrap_or(true);
            let mut hide_username_from_logs = old_hide;

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
                if hide_username_from_logs != old_hide {
                    config::set_config_value(
                        "hide_username_from_logs",
                        hide_username_from_logs.into(),
                    );
                }

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
                let preview_size =
                    config::get_config_u64("image_preview_size").unwrap_or(128) as u32;
                if let Ok(texture) = load_image(
                    "ICON",
                    ICON,
                    ui.ctx().clone(),
                    None,
                    max_textures_for_preview(preview_size),
                ) {
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
    file_list_ui: file_list::FileListUi,
    update_check_started: bool, // Ensures the background update check only runs once
    update_prompt: Option<updater::gui::UpdatePrompt>, // In-app "update available" prompt
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
            file_list_ui: file_list::FileListUi::default(),
            update_check_started: false,
            update_prompt: None,
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
        ktx_preview::init(cc.gl.as_deref());

        Default::default()
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Kick off the background update check once, on the first frame.
        if !self.update_check_started {
            self.update_check_started = true;
            if config::get_config_bool("check_for_updates").unwrap_or(false) {
                updater::check_for_updates_background(
                    ctx.clone(),
                    config::get_config_bool("automatically_install_updates").unwrap_or(false),
                );
            }
        }

        // Surface any update found by the background check.
        if self.update_prompt.is_none() {
            if let Some((release, url)) = updater::take_available_update() {
                self.update_prompt = Some(updater::gui::UpdatePrompt::new(release, url));
            }
        }
        if let Some(prompt) = &mut self.update_prompt {
            if prompt.show(ctx) {
                self.update_prompt = None;
            }
        }

        // Display the status bar at the bottom
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add(egui::ProgressBar::new(logic::get_progress()).text(logic::get_status()));
        });

        // Switch tabs with keyboard input (num keys)
        if ctx.input(|input| input.modifiers.ctrl || input.modifiers.alt) {
            for i in 1..=self.tab_map.len() as u32 {
                // from_name returns None for key names "0" and beyond "9"; skip.
                let Some(key) = egui::Key::from_name(&i.to_string()) else {
                    continue;
                };
                if ctx.input(|input| input.key_pressed(key)) {
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
                },
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
        // Update check now runs in MyApp::update (background thread, non-blocking).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_cache_evicts_least_recently_used() {
        let mut cache = ImageCache::new(2);
        cache.insert("a".to_owned(), 1, 2);
        cache.insert("b".to_owned(), 2, 2);
        cache.begin_frame();
        cache.begin_frame(); // nothing painted for two frames: all unpinned
        assert_eq!(cache.get("a"), Some(1)); // "b" is now least recently used
        cache.insert("c".to_owned(), 3, 2);
        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.get("a"), Some(1));
        assert_eq!(cache.get("c"), Some(3));
    }

    #[test]
    fn test_image_cache_never_evicts_painted_textures() {
        let mut cache = ImageCache::new(1);
        cache.insert("a".to_owned(), 1, 1);
        cache.begin_frame();
        assert_eq!(cache.get_pinned("a"), Some(1)); // painted this frame
        cache.insert("b".to_owned(), 2, 1);
        // Over budget, but both are on screen.
        assert_eq!(cache.get("a"), Some(1));
        assert_eq!(cache.get("b"), Some(2));
        cache.begin_frame();
        cache.begin_frame();
        cache.insert("c".to_owned(), 3, 1);
        assert_eq!(cache.textures.len(), 1);
        assert_eq!(cache.get("c"), Some(3));
    }

    #[test]
    fn test_tiles_per_row_accounts_for_spacing() {
        // 33 tiles fit (33*16 + 32*8 = 784); a 34th would need 808.
        assert_eq!(tiles_per_row(800.0, 16.0, 8.0), 33);
    }

    #[test]
    fn test_tiles_per_row_never_zero() {
        // Keep one per row even when a tile can't fit.
        assert_eq!(tiles_per_row(10.0, 512.0, 8.0), 1);
        assert_eq!(tiles_per_row(0.0, 16.0, 8.0), 1);
    }

    #[test]
    fn test_tiles_per_row_large_tiles() {
        // 5 fit (5*128 + 4*8 = 672), 6 would need 808.
        assert_eq!(tiles_per_row(800.0, 128.0, 8.0), 5);
    }

    #[test]
    fn test_preview_size_after_zoom_no_scroll_is_noop() {
        assert_eq!(preview_size_after_zoom(128, 0.0), 128);
    }

    #[test]
    fn test_preview_size_after_zoom_increases_and_decreases() {
        assert!(preview_size_after_zoom(128, 1.0) > 128);
        assert!(preview_size_after_zoom(128, -1.0) < 128);
    }

    #[test]
    fn test_preview_size_after_zoom_clamps() {
        assert_eq!(preview_size_after_zoom(512, 100.0), PREVIEW_SIZE_MAX);
        assert_eq!(preview_size_after_zoom(16, -100.0), PREVIEW_SIZE_MIN);
    }

    #[test]
    fn test_preview_size_from_scroll_reads_ctrl_wheel() {
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::Vec2::new(0.0, 1.0),
            modifiers: egui::Modifiers::COMMAND,
        });
        let mut zoomed = 0;
        ctx.run(raw, |ctx| zoomed = preview_size_from_scroll(ctx, 128));
        assert!(zoomed > 128, "ctrl+wheel up should grow the preview");

        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::Vec2::new(0.0, -1.0),
            modifiers: egui::Modifiers::COMMAND,
        });
        let mut shrunk = 0;
        ctx.run(raw, |ctx| shrunk = preview_size_from_scroll(ctx, 128));
        assert!(shrunk < 128, "ctrl+wheel down should shrink the preview");
    }

    #[test]
    fn test_preview_size_from_scroll_ignores_plain_wheel() {
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::Vec2::new(0.0, 1.0),
            modifiers: egui::Modifiers::NONE,
        });
        let mut size = 0;
        ctx.run(raw, |ctx| size = preview_size_from_scroll(ctx, 128));
        assert_eq!(size, 128, "plain wheel must not change the preview size");
    }
}

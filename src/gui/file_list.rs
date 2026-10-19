use super::player;
use crate::{
    config, dialog, gui, locale,
    logic::{self, AssetInfo},
};
use egui::Color32;
// Used for functionality
use fluent_bundle::{FluentBundle, FluentResource};
use std::num::NonZero;

mod table;
use std::{
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};

static ASSETS_LOADING: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn double_click(
    asset: logic::AssetInfo,
    swapping: &mut bool,
    copying: &mut bool,
    swapping_asset: &mut Option<logic::AssetInfo>,
) {
    if *copying {
        if swapping_asset.is_none() {
            *swapping_asset = Some(asset);
        } else {
            logic::copy_assets(swapping_asset.clone().unwrap(), asset);
        }
    } else if *swapping {
        if swapping_asset.is_none() {
            *swapping_asset = Some(asset);
        } else {
            logic::swap_assets(swapping_asset.clone().unwrap(), asset);
            *swapping_asset = None;
            *swapping = false
        }
    } else {
        let temp_dir = logic::get_temp_dir();
        let alias = config::get_asset_alias(&asset.name);
        let destination = temp_dir.join(alias);
        match logic::extract_to_file(asset, destination.clone(), true) {
            Ok(new_destination) => match open::that(new_destination) {
                Ok(()) => (),
                Err(err) => {
                    logic::update_status(locale::get_message(
                        &locale::get_locale(None),
                        "failed-opening-file",
                        None,
                    ));
                    log_error!("Failed opening file: {}", err)
                }
            },
            Err(e) => {
                logic::update_status(locale::get_message(
                    &locale::get_locale(None),
                    "failed-opening-file",
                    None,
                ));
                log_error!("Failed opening file: {}", e)
            }
        }
    }
}

fn extract_all_of_type(category: logic::Category) {
    // Runs on its own thread so the dialogs don't block the GUI.
    thread::spawn(move || {
        // Confirmation dialog, the program is still listing files
        if logic::get_list_task_running()
            && !dialog::confirm_localised(
                dialog::Level::Info,
                "confirmation-filter-confirmation-title",
                "confirmation-filter-confirmation-description",
            )
        {
            return;
        }

        // If the user provides a directory, the program will extract the assets to that directory
        if let Some(path) = dialog::pick_folder() {
            logic::extract_dir(
                path,
                category,
                false,
                config::get_config_bool("use_alias").unwrap_or(false),
            );
        }
    });
}

fn extract_file_button(asset: logic::AssetInfo) {
    let target = logic::convert::target_on_extract(&asset);
    extract_file_as(asset, target);
}

/// Ask where to save `asset`, then extract it there, converted to `target` if given.
fn extract_file_as(asset: logic::AssetInfo, target: Option<logic::convert::Target>) {
    thread::spawn(move || {
        let mut file_name = config::get_asset_alias(&asset.name);
        if let Some(target) = target {
            file_name = format!("{file_name}.{}", target.extension());
        }
        if let Some(destination) = dialog::save_file(&file_name) {
            if let Err(e) = logic::extract_to_file_as(asset, destination, false, target) {
                log_critical!("{}", e)
            }
        }
    });
}

/// Decode a preview into the texture cache. KTX goes through the GPU path.
fn load_preview(
    asset: &AssetInfo,
    bytes: &[u8],
    ctx: egui::Context,
    preview_size: u32,
) -> Result<(), String> {
    if asset.category == logic::Category::Ktx {
        return gui::load_ktx(&asset.name, bytes, ctx, preview_size);
    }
    // Downscale capped to 2x preview; bounds VRAM.
    gui::load_image(
        &asset.name,
        bytes,
        ctx,
        Some(gui::preview_max_dimension(preview_size)),
        gui::max_textures_for_preview(preview_size),
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn load_asset_image(asset: AssetInfo, ctx: egui::Context) -> Option<gui::PreviewTexture> {
    if let Some(texture) = gui::get_cached_preview(&asset.name) {
        return Some(texture);
    }

    {
        let assets_loading = ASSETS_LOADING.lock().unwrap();
        if assets_loading.contains(&asset.name)
            || assets_loading.len()
                >= thread::available_parallelism()
                    .unwrap_or(NonZero::new(2).unwrap())
                    .into()
        {
            return None; // Don't load multiple at a time or more than CPU threads
        }
    }

    let preview_size = config::get_config_u64("image_preview_size").unwrap_or(128) as u32;

    thread::spawn(move || {
        {
            let mut assets_loading = ASSETS_LOADING.lock().unwrap();
            assets_loading.push(asset.name.clone()); // Add the asset to the loading set
        }

        match logic::extract_asset_to_bytes(asset.clone()) {
            Ok(bytes) => match load_preview(&asset, &bytes, ctx.clone(), preview_size) {
                Ok(_) => {
                    ctx.request_repaint(); // paint loaded thumbnail promptly
                    let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                    assets_loading.retain(|x| x != &asset.name);
                }
                Err(e) => {
                    log_warn!(
                        "Failed to load {} as image, cooldown for 1000 ms ({})",
                        asset.name,
                        e
                    );
                    thread::sleep(Duration::from_millis(1000));
                    let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                    assets_loading.retain(|x| x != &asset.name);
                }
            },
            Err(e) => {
                log_error!("Unable read file, 1000 ms cooldown: {}", e);
                thread::sleep(Duration::from_millis(1000));
                let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                assets_loading.retain(|x| x != &asset.name);
            }
        }
    });
    None
}

fn clear_cache() {
    thread::spawn(|| {
        if dialog::confirm_localised(
            dialog::Level::Info,
            "confirmation-clear-cache-title",
            "confirmation-clear-cache-description",
        ) {
            gui::clear_image_cache();
            logic::clear_cache();
        }
    });
}

#[derive(Clone, Copy)]
enum EditMode {
    Swap,
    Copy,
}

fn is_audio(asset: &AssetInfo) -> bool {
    matches!(
        asset.category,
        logic::Category::Music | logic::Category::Sounds
    ) || matches!(asset.format, Some("ogg" | "mp3"))
}

/// Assets whose name or alias contains `query` (lowercase), case-insensitively.
fn filter_assets(assets: &[AssetInfo], query: &str) -> Vec<AssetInfo> {
    assets
        .iter()
        .filter(|asset| {
            asset.name.to_lowercase().contains(query)
                || config::get_asset_alias(&asset.name)
                    .to_lowercase()
                    .contains(query)
        })
        .cloned()
        .collect()
}

/// The rows shown: the category's list, filtered by the search query and sorted.
/// Rebuilt only when the list, the query or the sort changes.
#[derive(Default)]
struct ListView {
    category: Option<logic::Category>,
    generation: u64,
    query: String,
    sort: Option<table::Sort>,
    /// Audio lengths known when sorted by length.
    lengths_generation: u64,
    assets: Arc<Vec<AssetInfo>>,
}

pub struct FileListUi {
    selected: Option<usize>, // Used for storing selected state to retain keyboard navigation as seen in the tkinter version
    current_tab: Option<String>, // Allows for detecting when the user changes tabs to refresh automatically
    renaming: bool,
    searching: bool,
    search_query: String,
    swapping: bool,
    swapping_asset: Option<logic::AssetInfo>,
    asset_context_menu_open: Option<usize>,
    copying: bool,
    view: ListView,
    table: table::TableLayout,
    player: player::Player,
    pending_edit_mode: Option<(dialog::Pending<bool>, EditMode, Option<AssetInfo>)>, // Waiting on the ban warning
    pub locale: FluentBundle<Arc<FluentResource>>,
}
impl FileListUi {
    /// Double click/Enter/Open: swap or copy when in those modes, otherwise
    /// play sounds in the built-in player or open the file externally.
    fn open_asset(&mut self, asset: AssetInfo) {
        if !self.swapping
            && !self.copying
            && is_audio(&asset)
            && config::get_config_bool("use_builtin_player").unwrap_or(true)
        {
            self.player.play(asset);
        } else {
            double_click(
                asset,
                &mut self.swapping,
                &mut self.copying,
                &mut self.swapping_asset,
            );
        }
    }

    /// While something is playing, play whatever gets selected instead.
    fn preview_selection(&mut self, file_list: &[AssetInfo]) {
        let Some(asset) = self.selected.and_then(|i| file_list.get(i)) else {
            return;
        };
        let changed = self
            .player
            .current()
            .is_some_and(|current| current.name != asset.name);
        if changed && self.player.is_playing() && is_audio(asset) && !self.swapping && !self.copying
        {
            self.player.play(asset.clone());
        }
    }

    /// Toggle swapping/copying, showing the ban warning first if it hasn't been acknowledged.
    fn toggle_edit_mode(&mut self, mode: EditMode, asset: Option<AssetInfo>) {
        if config::get_config_bool("ban-warning-ack").unwrap_or(false) {
            self.apply_edit_mode(mode, asset);
        } else if self.pending_edit_mode.is_none() {
            let pending = dialog::spawn(|| {
                dialog::confirm_localised(
                    dialog::Level::Warning,
                    "confirmation-ban-warning-title",
                    "confirmation-ban-warning-description",
                )
            });
            self.pending_edit_mode = Some((pending, mode, asset));
        }
    }

    fn apply_edit_mode(&mut self, mode: EditMode, asset: Option<AssetInfo>) {
        let enabled = match mode {
            EditMode::Swap => !self.swapping,
            EditMode::Copy => !self.copying,
        };
        // Only one of the two at a time
        self.swapping = enabled && matches!(mode, EditMode::Swap);
        self.copying = enabled && matches!(mode, EditMode::Copy);
        self.swapping_asset = if enabled { asset } else { None };
    }

    /// The rows to show for `category`, filtered when searching and sorted.
    fn update_view(&mut self, category: logic::Category) -> Arc<Vec<AssetInfo>> {
        let generation = logic::get_file_list_generation(category);
        let query = if self.searching {
            self.search_query.to_lowercase()
        } else {
            String::new()
        };
        // Only audio has a length to sort by
        let sort = self.table.sort.filter(|sort| {
            sort.column != table::Column::Length
                || matches!(category, logic::Category::Music | logic::Category::Sounds)
        });
        // Sorting by length re-sorts as lengths come in
        let sort_by_length = sort.is_some_and(|sort| sort.column == table::Column::Length);
        let lengths_generation = if sort_by_length {
            logic::audio::generation()
        } else {
            0
        };

        if self.view.category != Some(category)
            || self.view.generation != generation
            || self.view.query != query
            || self.view.sort != sort
            || self.view.lengths_generation != lengths_generation
        {
            let assets = logic::get_file_list(category);
            let assets = if query.is_empty() && sort.is_none() {
                assets // Unchanged, share the snapshot
            } else {
                let mut assets = if query.is_empty() {
                    assets.to_vec()
                } else {
                    filter_assets(&assets, &query)
                };
                if sort_by_length {
                    for asset in &assets {
                        logic::audio::length(asset); // Queue the ones not known yet
                    }
                }
                if let Some(sort) = sort {
                    table::sort_assets(&mut assets, sort);
                }
                Arc::new(assets)
            };

            // Keep the same asset selected when rows move around
            if self.view.category == Some(category) {
                let selected_name = self
                    .selected
                    .and_then(|i| self.view.assets.get(i))
                    .map(|asset| asset.name.clone());
                self.selected = selected_name
                    .and_then(|name| assets.iter().position(|asset| asset.name == name));
            }

            self.view = ListView {
                category: Some(category),
                generation,
                query,
                sort,
                lengths_generation,
                assets,
            };
        }
        self.view.assets.clone()
    }

    fn poll_pending_edit_mode(&mut self) {
        let answer = match &self.pending_edit_mode {
            Some((pending, _, _)) => pending.take(),
            None => return,
        };
        if let Some(acknowledged) = answer {
            let (_, mode, asset) = self.pending_edit_mode.take().unwrap();
            if acknowledged {
                config::set_config_value("ban-warning-ack", true.into());
                self.apply_edit_mode(mode, asset);
            }
        }
    }

    /// Text box for renaming, laid out normally or placed in `rect`.
    fn handle_text_edit(
        &mut self,
        ui: &mut egui::Ui,
        alias: &str,
        file_name: &str,
        rect: Option<egui::Rect>,
    ) {
        let mut mutable_name = alias.to_string();
        let edit = egui::TextEdit::singleline(&mut mutable_name).hint_text(file_name);
        let response = match rect {
            Some(rect) => ui.put(rect, edit),
            None => edit.show(ui).response,
        };

        if mutable_name != alias {
            config::set_asset_alias(file_name, &mutable_name);
        }

        if response.lost_focus() {
            self.renaming = false;
            if mutable_name.is_empty() {
                config::set_asset_alias(file_name, file_name); // Set it to file name if blank
            }
        } else {
            response.request_focus(); // Request focus if it hasn't lost focus
        }
    }

    fn asset_buttons(
        &mut self,
        ui: &mut egui::Ui,
        category: logic::Category,
        focus_search_box: &mut bool,
        asset: Option<AssetInfo>,
    ) {
        if let Some(asset) = asset.clone() {
            if ui
                .button(locale::get_message(&self.locale, "button-open", None))
                .clicked()
            {
                self.open_asset(asset.clone());
                self.asset_context_menu_open = None;
            }
            if is_audio(&asset)
                && ui
                    .button(locale::get_message(&self.locale, "button-play", None))
                    .clicked()
            {
                self.player.play(asset.clone());
                self.asset_context_menu_open = None;
            }
            if ui
                .button(locale::get_message(
                    &self.locale,
                    "button-extract-file",
                    None,
                ))
                .clicked()
            {
                extract_file_button(asset.clone());
                self.asset_context_menu_open = None;
            }

            let targets = logic::convert::targets_for(&asset);
            if logic::convert::is_enabled() && !targets.is_empty() {
                ui.menu_button(
                    locale::get_message(&self.locale, "button-convert", None),
                    |ui| {
                        for &target in targets {
                            if ui.button(target.name()).clicked() {
                                extract_file_as(asset.clone(), Some(target));
                                self.asset_context_menu_open = None;
                            }
                        }
                    },
                );
            }
        }
        if ui
            .button(locale::get_message(&self.locale, "button-search", None))
            .clicked()
        {
            self.searching = !self.searching;
            *focus_search_box = true;
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(&self.locale, "button-rename", None))
            .clicked()
        {
            // Rename button
            self.renaming = !self.renaming;
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(
                &self.locale,
                "button-clear-cache",
                None,
            ))
            .clicked()
        {
            clear_cache();
            self.asset_context_menu_open = None;
        }

        if ui
            .button(locale::get_message(
                &self.locale,
                "button-extract-type",
                None,
            ))
            .clicked()
        {
            extract_all_of_type(category);
            self.asset_context_menu_open = None;
        }
        if ui
            .button(locale::get_message(&self.locale, "button-refresh", None))
            .clicked()
        {
            logic::refresh(category, false, false);
            self.asset_context_menu_open = None;
        }
        if ui
            .button(locale::get_message(&self.locale, "button-swap", None))
            .clicked()
        {
            self.toggle_edit_mode(EditMode::Swap, asset.clone());
            self.asset_context_menu_open = None;
        }
        if ui
            .button(locale::get_message(&self.locale, "button-copy", None))
            .clicked()
        {
            self.toggle_edit_mode(EditMode::Copy, asset.clone());
            self.asset_context_menu_open = None;
        }

        if matches!(category, logic::Category::Images | logic::Category::Ktx) {
            let message = if config::get_config_bool("display_image_preview").unwrap_or(false) {
                locale::get_message(&self.locale, "button-disable-display-image-preview", None)
            } else {
                locale::get_message(&self.locale, "button-display-image-preview", None)
            };

            if ui.button(message).clicked() {
                config::set_config_value(
                    "display_image_preview",
                    (!config::get_config_bool("display_image_preview").unwrap_or(false)).into(),
                );
                self.asset_context_menu_open = None;
            }
        }
    }

    // Function to handle asset response within asset list
    fn handle_asset_response(
        &mut self,
        response: egui::Response,
        visuals: &egui::Visuals,
        is_selected: bool,
        i: usize,
        scroll_to: Option<usize>,
        navigation_accepted: &mut bool,
        focus_search_box: &mut bool,
        asset: AssetInfo,
    ) -> (Color32, Color32) {
        // Highlight the background when selected
        let background_colour = if is_selected {
            visuals.selection.bg_fill // Primary colour
        } else {
            Color32::TRANSPARENT // No background colour
        };

        // Make the text have more contrast when selected
        let text_colour = if is_selected {
            visuals.strong_text_color() // Brighter
        } else {
            visuals.text_color() // Normal
        };

        // Handle the click/double click
        if response.clicked() && !self.renaming {
            self.selected = Some(i);
        }

        if response.secondary_clicked() {
            self.selected = Some(i);
            self.asset_context_menu_open = Some(i);
        }

        if let Some(asset_context_menu_open) = self.asset_context_menu_open {
            if asset_context_menu_open == i {
                response.context_menu(|ui| {
                    self.asset_buttons(ui, asset.category, focus_search_box, Some(asset.clone()));
                });
            }
        }

        if response.double_clicked() {
            self.open_asset(asset);
        }

        // Handle keyboard scrolling
        if scroll_to == Some(i) {
            *navigation_accepted = true;
            response.scroll_to_me(Some(egui::Align::Center)) // Align to center to prevent scrolling off the edge
        }

        (background_colour, text_colour)
    }

    pub fn ui(&mut self, tab: String, ui: &mut egui::Ui) {
        let category = match tab.as_str() {
            "music" => logic::Category::Music,
            "sounds" => logic::Category::Sounds,
            "images" => logic::Category::Images,
            "ktx-files" => logic::Category::Ktx,
            "rbxm-files" => logic::Category::Rbxm,
            _ => logic::Category::All,
        };

        // Detect if tab changed and do a refresh if so. Lists already loaded stay
        // on screen while the new one is made.
        if self.current_tab.as_ref() != Some(&tab) {
            self.current_tab = Some(tab.to_owned());
            self.selected = None;
            if logic::is_list_loaded(category) {
                logic::refresh_in_background(category);
            } else {
                logic::refresh(category, false, false);
            }
        }

        self.poll_pending_edit_mode();

        let file_list = self.update_view(category);

        let mut focus_search_box = false; // Focus the search box toggle for this frame

        let file_list_is_empty =
            logic::get_file_list(category).is_empty() && !logic::is_listing(category);

        // Handle key shortcuts here
        if ui.input(|i| i.key_pressed(egui::Key::F2)) {
            // Rename hotkey (F2)
            self.renaming = !self.renaming;
        }
        if ui.input(|i| i.key_pressed(egui::Key::F5)) {
            // Refresh (F5)
            logic::refresh(category, false, false);
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
            // Search (Ctrl+F)
            self.searching = !self.searching;
            focus_search_box = true;
        }
        if ui.input(|i| i.key_pressed(egui::Key::F3)) {
            // Extract all (F3)
            extract_all_of_type(category);
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            // Swap (Ctrl+D)
            let asset = self.selected.and_then(|i| file_list.get(i).cloned());
            self.toggle_edit_mode(EditMode::Swap, asset);
        }
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::E)) {
            // Ctrl+E (Extract)
            if let Some(selected) = self.selected {
                if let Some(asset) = file_list.get(selected) {
                    extract_file_button(asset.clone());
                }
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !self.searching {
            // Cancel actions (Esc)
            self.swapping_asset = None;
            self.copying = false;
            self.swapping = false;
        }

        // Shortcuts to deactivate during text entry
        if !self.renaming && !self.searching {
            if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                // Play/pause (Space)
                let selected = self
                    .selected
                    .and_then(|i| file_list.get(i))
                    .filter(|asset| is_audio(asset))
                    .cloned();
                self.player.toggle(selected);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                // Clear cache (del)
                clear_cache();
            }

            if ui.input(|inp| inp.events.iter().any(|ev| matches!(ev, egui::Event::Copy))) {
                // https://github.com/emilk/egui/issues/4065#issuecomment-2071047410
                // Copy (Ctrl+C)
                let asset = self.selected.and_then(|i| file_list.get(i).cloned());
                self.toggle_edit_mode(EditMode::Copy, asset);
            }
        }

        // GUI logic below here

        // Top UI buttons
        if config::get_config_bool("use_topbar_buttons").unwrap_or(true) {
            ui.push_id("Topbar buttons", |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.asset_buttons(ui, category, &mut focus_search_box, None);
                    });
                })
            });
        }

        // Player bar along the bottom, once something has been played
        if self.player.is_visible() {
            egui::TopBottomPanel::bottom("player")
                .show_inside(ui, |ui| self.player.ui(ui, &self.locale));
        }

        // Empty state
        if file_list_is_empty {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                let icon_size = 48.0;
                ui.add(
                    egui::Label::new(egui::RichText::new("📂").size(icon_size)).selectable(false), // Folder icon
                );
                ui.heading(locale::get_message(&self.locale, "empty-state-title", None));
                ui.label(locale::get_message(
                    &self.locale,
                    "empty-state-description",
                    None,
                ));
                ui.label(locale::get_message(&self.locale, "empty-state-hint", None));
                ui.add_space(20.0);
                if ui
                    .button(locale::get_message(&self.locale, "button-refresh", None))
                    .clicked()
                {
                    logic::refresh(category, false, false);
                }
            });
            return;
        }

        if self.swapping {
            if self.swapping_asset.as_ref().is_none() {
                ui.heading(locale::get_message(&self.locale, "swap-choose-file", None));
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set(
                    "asset",
                    config::get_asset_alias(&self.swapping_asset.as_ref().unwrap().name),
                );
                ui.heading(locale::get_message(&self.locale, "swap-with", Some(&args)));
            }
        }

        if self.copying {
            if self.swapping_asset.as_ref().is_none() {
                ui.heading(locale::get_message(&self.locale, "copy-choose-file", None));
            } else {
                let mut args = fluent_bundle::FluentArgs::new();
                args.set(
                    "asset",
                    config::get_asset_alias(&self.swapping_asset.as_ref().unwrap().name),
                );
                ui.heading(locale::get_message(
                    &self.locale,
                    "overwrite-with",
                    Some(&args),
                ));
            }
        }

        let file_list = if self.searching {
            let response = ui.text_edit_singleline(&mut self.search_query);

            if focus_search_box {
                response.request_focus();
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.searching = false; // Remove the search bar when the use presses escape
            }

            self.update_view(category) // The query may have changed
        } else {
            file_list
        };

        let display_image_preview = config::get_config_bool("display_image_preview")
            .unwrap_or(false)
            && (tab == "images" || tab == "ktx-files");

        // Ctrl+scroll resizes the previews.
        if display_image_preview {
            let old = config::get_config_u64("image_preview_size").unwrap_or(128);
            let new = gui::preview_size_from_scroll(ui.ctx(), old);
            if new != old {
                config::set_config_value("image_preview_size", new.into());
            }
        }

        let row_height = if display_image_preview {
            config::get_config_u64("image_preview_size").unwrap_or(128) as f32
        } else {
            ui.text_style_height(&egui::TextStyle::Body) + 4.0 // Room around the text
        };

        // Whole tiles per row; the old hardcoded gap over-packed rows, which is
        // why the list used to overflow sideways at small preview sizes.
        let amount_per_row = if display_image_preview {
            gui::tiles_per_row(
                ui.available_width(),
                row_height,
                ui.spacing().item_spacing.x,
            )
        } else {
            1
        };

        let total_rows = if display_image_preview {
            f32::ceil(file_list.len() as f32 / amount_per_row as f32) as usize
        // Show even unfilled rows
        } else {
            file_list.len()
        };

        let mut scroll_to: Option<usize> = None; // This is reset every frame, so it doesn't constantly scroll to the same label
        let mut none_selected: bool = false; // Used to scroll to the first value shown when none is selected

        // Only allow navigation of the user is not renaming
        if !self.renaming {
            // Grid mode reads left-to-right, so up/down move by a whole row.
            let step = amount_per_row.max(1);

            // If the user presses up, move the selection up
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                if let Some(selected) = self.selected {
                    // Check if it is large enough otherwise it'll attempt to select non-existant labels
                    let target = selected.saturating_sub(step);
                    if target != selected {
                        self.selected = Some(target);
                        scroll_to = Some(target); // This is also set to the same number, allowing for auto scrolling
                    }
                } else {
                    none_selected = true // Select the first visible entry
                }
            }

            // If the user presses down, move the selection down
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                if let Some(selected) = self.selected {
                    if selected + step < file_list.len() {
                        // Stop it from overflowing otherwise it'll attempt to select non-existant labels
                        self.selected = Some(selected + step);
                        scroll_to = Some(selected + step); // This is also set to the same number, allowing for auto scrolling
                    }
                } else {
                    none_selected = true // Select the first visible entry
                }
            }

            // Allow the user to confirm with enter
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(selected) = self.selected {
                    // Get file name after getting the selected value
                    if let Some(asset) = file_list.get(selected) {
                        self.open_asset(asset.clone());
                    }
                }
            }
        }

        let mut navigation_accepted: bool = false; // Used to check if the selected label is available to accept the keyboard navigation

        // Column headers, sticky above the scrolling rows
        let columns = if display_image_preview {
            Vec::new()
        } else {
            self.table
                .header_ui(ui, &self.locale, row_height + 4.0, category)
        };

        // File list for assets
        gui::begin_frame_textures(); // rotate pin sets
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            row_height,
            total_rows,
            |ui, row_range| {
                if display_image_preview {
                    for row_idx in row_range {
                        ui.horizontal(|ui| {
                            for amount in 0..amount_per_row {
                                let i = (row_idx * amount_per_row) + amount;
                                if let Some(asset) = file_list.get(i) {
                                    let file_name = &asset.name;
                                    let alias = config::get_asset_alias(file_name);

                                    let is_selected = if none_selected && i != 0 {
                                        // Selecting the very first causes some issues
                                        self.selected = Some(i); // If there is none selected, Set selected and return true
                                        none_selected = false; // Will select everything if this is not set to false immediately
                                        true
                                    } else {
                                        self.selected == Some(i) // Check if this current one is selected
                                    };

                                    // Draw the text
                                    if is_selected && self.renaming {
                                        self.handle_text_edit(ui, &alias, file_name, None);
                                    // Allow user to edit
                                    } else {
                                        let desired_size = egui::vec2(row_height, row_height); // Set height to the text style height
                                        let (rect, response) = ui.allocate_exact_size(
                                            desired_size,
                                            egui::Sense::click(),
                                        );

                                        if let Some(texture) =
                                            load_asset_image(asset.clone(), ui.ctx().clone())
                                        {
                                            texture.paint(ui, rect);
                                        }

                                        let visuals = ui.visuals();

                                        // Get colours and handle response
                                        let colours = self.handle_asset_response(
                                            response,
                                            visuals,
                                            is_selected,
                                            i,
                                            scroll_to,
                                            &mut navigation_accepted,
                                            &mut focus_search_box,
                                            asset.clone(),
                                        );

                                        let text_colour = colours.1;
                                        let background_colour = colours.0;

                                        // Draw the background colour
                                        ui.painter().rect_stroke(
                                            rect,
                                            0.0,
                                            egui::Stroke::new(row_height / 8.0, background_colour),
                                            egui::StrokeKind::Inside,
                                        );

                                        // Draw text ontop of image
                                        let text = egui::Label::new(
                                            egui::RichText::new(alias)
                                                .text_style(egui::TextStyle::Body)
                                                .color(text_colour),
                                        )
                                        .truncate()
                                        .selectable(false);

                                        let text_size =
                                            ui.text_style_height(&egui::TextStyle::Body);

                                        let text_rect = egui::Rect::from_min_size(
                                            rect.min
                                                + egui::vec2(
                                                    0.0,
                                                    (rect.height() - text_size) / 2.0,
                                                ),
                                            egui::vec2(row_height, text_size),
                                        );

                                        // Background to make text easier to read
                                        let background_colour = if visuals.dark_mode {
                                            egui::Color32::from_rgba_unmultiplied(27, 27, 27, 160)
                                        // Dark mode
                                        } else {
                                            egui::Color32::from_rgba_unmultiplied(
                                                248, 248, 248, 160,
                                            ) // Light mode
                                        };
                                        ui.painter().rect_filled(text_rect, 0.0, background_colour);

                                        ui.put(text_rect, text);
                                    }
                                }
                            }
                        });
                    }
                } else {
                    for i in row_range {
                        if let Some(asset) = file_list.get(i) {
                            let alias = config::get_asset_alias(&asset.name);
                            let is_selected = if none_selected && i != 0 {
                                self.selected = Some(i);
                                none_selected = false;
                                true
                            } else {
                                self.selected == Some(i)
                            };

                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::click(),
                            );

                            let visuals = ui.visuals().clone();
                            let (background_colour, text_colour) = self.handle_asset_response(
                                response,
                                &visuals,
                                is_selected,
                                i,
                                scroll_to,
                                &mut navigation_accepted,
                                &mut focus_search_box,
                                asset.clone(),
                            );

                            // Stripe every other row to make rows easier to follow
                            let background_colour = if !is_selected && i % 2 == 1 {
                                visuals.faint_bg_color
                            } else {
                                background_colour
                            };
                            ui.painter().rect_filled(rect, 0.0, background_colour);

                            let renaming = is_selected && self.renaming;
                            let cells: Vec<_> = columns
                                .iter()
                                .filter(|column| {
                                    !(renaming && column.column == table::Column::Name)
                                })
                                .copied()
                                .collect();
                            table::paint_row(ui, rect, &cells, asset, &alias, text_colour);

                            if renaming {
                                // Edit the name in place
                                if let Some(name) =
                                    columns.iter().find(|c| c.column == table::Column::Name)
                                {
                                    let cell = egui::Rect::from_min_size(
                                        rect.min + egui::vec2(name.x, 0.0),
                                        egui::vec2(name.width, rect.height()),
                                    );
                                    self.handle_text_edit(ui, &alias, &asset.name, Some(cell));
                                }
                            }
                        }
                    }
                }
            },
        );

        if !navigation_accepted && scroll_to.is_some() {
            // If the keyboard navigation wasn't accepted and there is keyboard navigation then...
            self.selected = None; // Set the selected to none, so it selects something on-screen
        }

        self.preview_selection(&file_list);
    }
}

impl Default for FileListUi {
    fn default() -> Self {
        Self {
            selected: None,
            current_tab: None,
            renaming: false,
            searching: false,
            search_query: "".to_owned(),
            swapping: false,
            swapping_asset: None,
            locale: locale::get_locale(None),
            asset_context_menu_open: None,
            copying: false,
            view: ListView::default(),
            table: table::TableLayout::load(),
            player: player::Player::default(),
            pending_edit_mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> AssetInfo {
        AssetInfo {
            name: name.to_owned(),
            size: 0,
            last_modified: None,
            created: None,
            format: None,
            from_file: true,
            from_sql: false,
            from_rbx_storage: false,
            category: logic::Category::Sounds,
        }
    }

    #[test]
    fn test_filter_assets_matches_name_case_insensitively() {
        let assets = vec![asset("Apple"), asset("banana"), asset("pineapple")];
        let filtered = filter_assets(&assets, "app");
        let names: Vec<_> = filtered.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Apple", "pineapple"]);
        assert!(filter_assets(&assets, "cherry").is_empty());
        assert_eq!(filter_assets(&assets, "").len(), 3);
    }
}

// Used for gui
use eframe::egui;
use egui::Color32;
use native_dialog::{DialogBuilder, MessageLevel};
use egui_dock::{DockArea, NodeIndex, DockState, SurfaceIndex, Style};
use fluent_bundle::{FluentBundle, FluentResource};
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use std::{sync::Arc, thread};

use std::collections::HashMap; // Used for input
use crate::{config, locale, log, logic, updater}; // Used for functionality
use eframe::egui::TextureHandle;

use lazy_static::lazy_static;

mod welcome;
mod settings;

const VERSION: &str = env!("CARGO_PKG_VERSION"); // Get version for use in the filename
const COMPILE_DATE: &str = env!("COMPILE_DATE");
const ICON: &[u8; 11400] = include_bytes!("../assets/icon.png");
const CONTRIBUTORS: [&str; 4] = [
    "AeEn123",
    "Vonercent",
    "MarcelDev",
    "aaditkumar2009",
];
const DEPENDENCIES: [[&str; 2]; 13] = [
    ["https://github.com/emilk/egui", ""],
    ["https://github.com/Adanos020/egui_dock", ""],
    ["https://github.com/lampsitter/egui_commonmark", ""],
    ["https://github.com/native-dialog-rs/native-dialog-rs", ""],
    ["https://github.com/rust-lang-nursery/lazy-static.rs", ""],
    ["https://github.com/projectfluent/fluent-rs", ""],
    ["https://github.com/1Password/sys-locale", ""],
    ["https://github.com/zbraniecki/unic-locale", ""],
    ["https://github.com/Stebalien/tempfile", ""],
    ["https://github.com/clap-rs/clap", ""],
    ["https://github.com/ardaku/whoami", ""],
    ["https://github.com/seanmonstar/reqwest", ""],
    ["https://github.com/serde-rs/json", ""],
];

lazy_static! {
    static ref IMAGES: Mutex<HashMap<String, TextureHandle>> = Mutex::new(HashMap::new());
    static ref ASSETS_LOADING: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

struct TabViewer<'a> {
    // passing selected label to TabViewer
    selected: &'a mut Option<usize>,
    current_tab: &'a mut Option<String>,
    renaming: &'a mut bool,
    searching: &'a mut bool,
    search_query: &'a mut String,
    swapping: &'a mut bool,
    swapping_asset_a: &'a mut Option<String>,
    locale: &'a mut FluentBundle<Arc<FluentResource>>,
    asset_context_menu_open: &'a mut Option<usize>,
    copying: &'a mut bool,
}

fn double_click(dir: PathBuf, value: String, mode: String, swapping: &mut bool, copying: &mut bool, swapping_asset_a: &mut Option<String>) {
    if *copying {
        if swapping_asset_a.is_none() {
            *swapping_asset_a = Some(value);
        } else {
            logic::copy_assets(dir, &swapping_asset_a.as_ref().unwrap(), &value);
        }
    } else if *swapping {
        if swapping_asset_a.is_none() {
            *swapping_asset_a = Some(value);
        } else {
            logic::swap_assets(dir, &swapping_asset_a.as_ref().unwrap(), &value);
            *swapping_asset_a = None;
            *swapping = false
        }
    } else {
        let temp_dir = logic::get_temp_dir(true);
        let alias = config::get_asset_alias(&value);
        let destination = temp_dir.join(alias);
        let origin = dir.join(value);
        let new_destination = logic::extract_file(origin, &mode, destination.clone(), true);
        if new_destination != PathBuf::new() {
            match open::that(new_destination) {
                Ok(()) => (),
                Err(err) => log_error!("Failed opening file: {}", err)
            }
        }
    }
}


fn delete_this_directory(cache_directory: PathBuf, locale: &FluentBundle<Arc<FluentResource>>) {
    // Confirmation dialog
    let yes = DialogBuilder::message()
    .set_level(MessageLevel::Info)
    .set_title(&locale::get_message(locale, "confirmation-delete-confirmation-title", None))
    .set_text(&locale::get_message(locale, "confirmation-delete-confirmation-description", None))
    .confirm().show()
    .unwrap();

    if yes {
        logic::delete_all_directory_contents(cache_directory);
    }
}

fn extract_all_of_type(cache_directory: PathBuf, mode: &str, locale: &FluentBundle<Arc<FluentResource>>) {
    let mut no = logic::get_list_task_running();

    // Confirmation dialog, the program is still listing files
    if no {
        // NOT result, will become false if user clicks yes
        no = !DialogBuilder::message()
        .set_level(MessageLevel::Info)
        .set_title(&locale::get_message(locale, "confirmation-filter-confirmation-title", None))
        .set_text(&locale::get_message(locale, "confirmation-filter-confirmation-description", None))
        .confirm().show()
        .unwrap();
    }

    // The user either agreed or the program is not listing files
    if !no {
        let option_path = DialogBuilder::file()
        .open_single_dir().show()
        .unwrap();

        // If the user provides a directory, the program will extract the assets to that directory
        if let Some(path) = option_path {
            logic::extract_dir(cache_directory, path, mode.to_string(), false,config::get_config_bool("use_alias").unwrap_or(false));
        }
    }
}
fn toggle_swap(swapping: &mut bool, swapping_asset_a: &mut Option<String>, locale: &FluentBundle<Arc<FluentResource>>) {
    let mut warning_acknowledged = config::get_config_bool("ban-warning-ack").unwrap_or(false);

    if !warning_acknowledged {
        warning_acknowledged = DialogBuilder::message()
        .set_level(MessageLevel::Info)
        .set_title(&locale::get_message(locale, "confirmation-ban-warning-title", None))
        .set_text(&locale::get_message(locale, "confirmation-ban-warning-description", None))
        .confirm().show()
        .unwrap();
    }

    if warning_acknowledged {
        config::set_config_value("ban-warning-ack", warning_acknowledged.into());
        if *swapping {
            *swapping_asset_a = None;
        }
        *swapping = !*swapping;

    }
}

fn extract_file_button(name: &str, cache_directory: PathBuf, tab: &str) {
    let alias = config::get_asset_alias(name);
    let origin = cache_directory.join(name);
    if let Some(destination) = native_dialog::DialogBuilder::file().set_filename(&alias).save_single_file().show().unwrap() {
        logic::extract_file(origin, tab.into(), destination, false);
    }
}

fn load_image(id: &str, data: &[u8], ctx: egui::Context) -> Result<TextureHandle, image::ImageError> {
    let images = {IMAGES.lock().unwrap().clone()};
    if let Some(texture) = images.get(id) {
        Ok(texture.clone())
    } else {
        let icon_image = image::load_from_memory(data)?;
        let icon_rgba = icon_image.to_rgba8();
        let icon_size = [icon_rgba.width() as usize, icon_rgba.height() as usize];
        let texture = ctx.load_texture(
            id,
            egui::ColorImage::from_rgba_unmultiplied(icon_size, icon_rgba.as_flat_samples().as_slice()),
            Default::default(),
        );
        let mut images = IMAGES.lock().unwrap();
        images.insert(id.to_string(), texture.clone());
        return Ok(texture);
    }
}

fn load_asset_image(id: String, tab: String, cache_directory: PathBuf, ctx: egui::Context) -> Option<TextureHandle> {
    let images = {IMAGES.lock().unwrap().clone()};
    if let Some(texture) = images.get(&id) {
        Some(texture.clone())
    } else {
        {
            let assets_loading = ASSETS_LOADING.lock().unwrap().clone();                            // Default to 2 CPU threads
            if assets_loading.contains(&id) || assets_loading.len() >= thread::available_parallelism().unwrap_or(NonZero::new(2).unwrap()).into() {
                return None // Don't load multiple at a time or more than CPU threads
            }
        }
        thread::spawn(move || {
            {
                let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                assets_loading.push(id.clone()); // Add the asset to the loading set
            }
            let path = cache_directory.join(&id);
            let bytes = logic::extract_file_to_bytes(path, &tab);
            match load_image(&id, &bytes.as_slice(), ctx) {
                Ok(_) => {
                    let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                    assets_loading.retain(|x| x != &id); // Remove the asset from the loading set
                },
                Err(_) => {
                    log_warn!("Failed to load {} as image, cooldown for 1000 ms", &id);
                    thread::sleep(Duration::from_millis(1000));
                    let mut assets_loading = ASSETS_LOADING.lock().unwrap();
                    assets_loading.retain(|x| x != &id); // Remove the asset from the loading set
                }
            }

        });
        None
    }
}

fn add_dependency_credit(dependency: [&str;2], ui: &mut egui::Ui, sponsor_message: &str) {
    if dependency[1] != "" {
        ui.horizontal(|ui| {
            ui.hyperlink_to(dependency[0].replace("https://github.com/", ""), dependency[0]);
            ui.label("|");
            ui.hyperlink_to(sponsor_message, dependency[1]);
        });
    } else {
        ui.hyperlink_to(dependency[0].replace("https://github.com/", ""), dependency[0]);
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let mut size = bytes as f64 / 1024.0;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

fn format_modified(time: std::time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

impl TabViewer<'_> {
    fn asset_buttons(&mut self, ui: &mut egui::Ui, cache_directory: PathBuf, tab: &str, focus_search_box: &mut bool, name: Option<&str>) {
        if let Some(name) = name {
            if ui.button(locale::get_message(self.locale, "button-open", None)).clicked() {
                double_click(cache_directory.clone(), name.to_string(), tab.to_string(), self.swapping, self.copying, self.swapping_asset_a);
                *self.asset_context_menu_open = None;
            }
            if ui.button(locale::get_message(self.locale, "button-extract-file", None)).clicked() {
                extract_file_button(name, cache_directory.clone(), tab);
                *self.asset_context_menu_open = None;
            }
        }
        if ui.button(locale::get_message(self.locale, "button-search", None)).clicked() {
            *self.searching = !*self.searching;
            *focus_search_box = true;
            *self.asset_context_menu_open = None;
        }
        
        if ui.button(locale::get_message(self.locale, "button-rename", None)).clicked() {
            // Rename button
            *self.renaming = !*self.renaming;
            *self.asset_context_menu_open = None;
        }
    
        if ui.button(locale::get_message(self.locale, "button-delete-this-dir", None)).clicked() {
            delete_this_directory(cache_directory.clone(), self.locale);
            *self.asset_context_menu_open = None;
        }
        if ui.button(locale::get_message(self.locale, "button-extract-type", None)).clicked() {
            extract_all_of_type(cache_directory.clone(), tab, self.locale);
            *self.asset_context_menu_open = None;
        }
        if ui.button(locale::get_message(self.locale, "button-refresh", None)).clicked() {
            logic::refresh(cache_directory.clone(), tab.to_string(), false, false);
            *self.asset_context_menu_open = None;
        }
        if ui.button(locale::get_message(self.locale, "button-swap", None)).clicked() {
            toggle_swap(self.swapping, self.swapping_asset_a, self.locale);
            *self.asset_context_menu_open = None;
    
            if let Some(n) = name {
                *self.swapping_asset_a = Some(n.to_string());
            } else {
                *self.swapping_asset_a = None;
            }
            
        }
        if ui.button(locale::get_message(self.locale, "button-copy", None)).clicked() {
            toggle_swap(self.copying, self.swapping_asset_a, self.locale);
            *self.asset_context_menu_open = None;
    
            if let Some(n) = name {
                *self.swapping_asset_a = Some(n.to_string());
            } else {
                *self.swapping_asset_a = None;
            }
        }

        if tab == "images" {
            let message = if config::get_config_bool("display_image_preview").unwrap_or(false) {
                locale::get_message(self.locale, "button-disable-display-image-preview", None)
            } else {
                locale::get_message(self.locale, "button-display-image-preview", None)
            };
    
            if ui.button(message).clicked() {
                config::set_config_value("display_image_preview", (!config::get_config_bool("display_image_preview").unwrap_or(false)).into());
                *self.asset_context_menu_open = None;
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
        cache_directory: PathBuf,
        tab: &str,
        mut focus_search_box: &mut bool,
        file_name: &str,
    ) -> (Color32, Color32) {
        // Highlight the background when selected
        let background_colour = if is_selected {
            visuals.selection.bg_fill // Primary colour
        } else {
            egui::Color32::TRANSPARENT // No background colour
        };

        // Make the text have more contrast when selected
        let text_colour = if is_selected {
            visuals.strong_text_color() // Brighter
        } else {
            visuals.text_color() // Normal
        };

        // Handle the click/double click
        if response.clicked() && !*self.renaming {
            *self.selected = Some(i);
        }

        if response.secondary_clicked() {
            *self.selected = Some(i);
            *self.asset_context_menu_open = Some(i);
        }

        if let Some(asset) = self.asset_context_menu_open {
            if *asset == i {
                response.context_menu(|ui| {
                    self.asset_buttons(ui, cache_directory.clone(), tab, &mut focus_search_box, Some(&file_name));
                });
            }

        }

        if response.double_clicked() {
            double_click(cache_directory, file_name.to_string(), tab.to_string(), self.swapping, self.copying, self.swapping_asset_a);
        }

        // Handle keyboard scrolling
        if scroll_to == Some(i) {
            *navigation_accepted = true;
            response.scroll_to_me(Some(egui::Align::Center)) // Align to center to prevent scrolling off the edge
        }

        return (background_colour, text_colour)
    }

    fn handle_text_edit(&mut self, ui: &mut egui::Ui, alias: &str, file_name: &str) {
        let mut mutable_name = alias.to_string();
        let response = egui::TextEdit::singleline(&mut mutable_name)
            .hint_text(file_name)
            .show(ui)
            .response;

        if mutable_name != *alias {
            config::set_asset_alias(file_name, &mutable_name);
        }

        if response.lost_focus() {
            *self.renaming = false;
            if mutable_name == "" {
                config::set_asset_alias(file_name, file_name); // Set it to file name if blank
            }
        } else {
            response.request_focus(); // Request focus if it hasn't lost focus
        }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        locale::get_message(self.locale, &*tab, None).into()
        
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let cache_directory = logic::get_mode_cache_directory(&tab);      

        let file_list = logic::get_file_list(); // Get the file list as it is used throughout the GUI

        if tab != "settings" && tab != "about" && tab != "logs" {
            // This is only shown on tabs other than settings (Extracting assets)

            // Detect if tab changed and do a refresh if so
            if let Some(current_tab) = self.current_tab {
                if current_tab.to_owned() != tab.to_owned() {
                    *self.current_tab = Some(tab.to_owned());
                    logic::refresh(cache_directory.to_owned(), tab.to_owned(), false, false);
                }
            } else {
                *self.current_tab = Some(tab.to_owned());
                logic::refresh(cache_directory.to_owned(), tab.to_owned(), false, false);
            }

            let mut focus_search_box = false; // Focus the search box toggle for this frame

            // Handle key shortcuts here
            if ui.input(|i| i.key_pressed(egui::Key::F2)) {
                // Rename hotkey
                *self.renaming = !*self.renaming;
            }
            if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
                // Ctrl+F (Search)
                *self.searching = !*self.searching;
                focus_search_box = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !*self.renaming {
                // del key used for editing, don't allow during editing
                delete_this_directory(cache_directory.clone(), self.locale);
            }
            if ui.input(|i| i.key_pressed(egui::Key::F3)) {
                extract_all_of_type(cache_directory.clone(), &tab, self.locale);
            }
            if ui.input(|i| i.key_pressed(egui::Key::F5)) {
                logic::refresh(cache_directory.clone(), tab.to_owned(), false, false);
            }
            if ui.input(|i| i.key_pressed(egui::Key::F4)) {
                toggle_swap(self.swapping, self.swapping_asset_a, self.locale);
            }
            if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
                // Ctrl+D (Copy)
                toggle_swap(self.copying, self.swapping_asset_a, self.locale);
            }

            // GUI logic below here

            // Top UI buttons
            if config::get_config_bool("use_topbar_buttons").unwrap_or(true) {
                ui.push_id("Topbar buttons", |ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            self.asset_buttons(ui, cache_directory.clone(), tab, &mut focus_search_box, None);
                        });
                    })
                });
            }
            
            let mut scroll_to: Option<usize> = None; // This is reset every frame, so it doesn't constantly scroll to the same label
            let mut none_selected: bool = false; // Used to scroll to the first value shown when none is selected
            
            // Only allow navigation of the user is not renaming
            if !*self.renaming {
                // If the user presses up, decrement the selected value
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(selected) = *self.selected {
                        if selected > 0 { // Check if it is larger than 0 otherwise it'll attempt to select non-existant labels
                            *self.selected = Some(selected - 1);
                            scroll_to = Some(selected - 1); // This is also set to the same number, allowing for auto scrolling
                        }
                    } else {
                        none_selected = true // Select the first visible entry
                    }
                }

                // If the user presses down, increment the selected value
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    if let Some(selected) = *self.selected {
                        if selected < file_list.len()-1 { // Stop it from overflowing otherwise it'll attempt to select non-existant labels
                            *self.selected = Some(selected + 1);
                            scroll_to = Some(selected + 1); // This is also set to the same number, allowing for auto scrolling
                        }
                    } else {
                        none_selected = true // Select the first visible entry
                    }
                }

                // Allow the user to confirm with enter
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(selected) = *self.selected {
                        // Get file name after getting the selected value
                        if let Some(asset) = file_list.get(selected) {
                            double_click(cache_directory.clone(), asset.name.to_string(), tab.to_string(), self.swapping, self.copying, self.swapping_asset_a);
                        }                   
                    }
                }

                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::E)) {
                    // Ctrl+E (Extract)
                    if let Some(selected) = *self.selected {
                        // Get file name after getting the selected value
                        if let Some(asset) = file_list.get(selected) {
                            extract_file_button(&asset.name, cache_directory.clone(), tab);
                        }                   
                    }
                }
            }


            let mut navigation_accepted: bool = false; // Used to check if the selected label is available to accept the keyboard navigation

            if *self.swapping {
                if self.swapping_asset_a.as_ref().is_none() {
                    ui.heading(locale::get_message(self.locale, "swap-choose-file", None));
                } else {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("asset", config::get_asset_alias(self.swapping_asset_a.as_ref().unwrap()));
                    ui.heading(locale::get_message(self.locale, "swap-with", Some(&args)));
                }
            }

            if *self.copying {
                if self.swapping_asset_a.as_ref().is_none() {
                    ui.heading(locale::get_message(self.locale, "copy-choose-file", None));
                } else {
                    let mut args = fluent_bundle::FluentArgs::new();
                    args.set("asset", config::get_asset_alias(self.swapping_asset_a.as_ref().unwrap()));
                    ui.heading(locale::get_message(self.locale, "overwrite-with", Some(&args)));
                }
            }

            let file_list = if *self.searching {
                let old_search_query = self.search_query.clone();

                let response = ui.text_edit_singleline(self.search_query);

                if focus_search_box {
                    response.request_focus();
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    *self.searching = false; // Remove the search bar when the use presses escape
                }

                if *self.search_query != old_search_query {
                    logic::filter_file_list(self.search_query.clone());
                }
                logic::get_filtered_file_list()
            } else {
                file_list
            };

            let display_image_preview = config::get_config_bool("display_image_preview").unwrap_or(false) && tab == "images";

            let row_height = if display_image_preview {
                config::get_config_u64("image_preview_size").unwrap_or(128) as f32
            } else {
                ui.text_style_height(&egui::TextStyle::Body)
            };

            let amount_per_row = if display_image_preview {
                ui.available_width() as usize / (row_height + 7.5) as usize // Account for padding because ui.horizontal adds padding
            } else {
                1
            };

            let total_rows = if display_image_preview {
                f32::ceil(file_list.len() as f32 / amount_per_row as f32) as usize // Show even unfilled rows
            } else {
                file_list.len()
            };

            // let mut table_properties = Vec::new();
            // table_properties.push(("name", 0.0));
            // table_properties.push(("size", 0.5));
            // table_properties.push(("modified", 0.75));



            // if !display_image_preview {
            //     // Display table headers
            //     let full_width = ui.available_width();
            //     let desired_size = egui::vec2(full_width, row_height);
            //     let rect = ui.allocate_exact_size(desired_size, egui::Sense::hover()).0;

            //     for property in table_properties {
            //         let size = rect.size();
            //         println!("{}", property.1*size.x);
            //         let property_rect = egui::Rect::from_min_size(
            //             rect.min + egui::vec2(property.1*size.x, 0.0),
            //             egui::vec2((1.0-property.1)*size.x, size.y)
            //         );

            //         ui.put(property_rect,
            //         egui::Label::new(property.0).truncate().selectable(false));
            //     }

            //     // // Column positions
            //     // let alias_x = rect.min.x + 5.0;
            //     // let size_x = rect.min.x + rect.width() * 0.7;
            //     // let modified_x = rect.min.x + rect.width() * 1.0 - 5.0; // adjust for padding

            //     // // Draw all columns
            //     // ui.painter().text(
            //     //     egui::pos2(alias_x+5, rect.min.y),
            //     //     egui::Align2::LEFT_TOP,
            //     //     "Name",
            //     //     egui::TextStyle::Body.resolve(ui.style()),
            //     //     visuals.text_color(),
            //     // );

            //     // ui.painter().text(
            //     //     egui::pos2(size_x+5, rect.min.y),
            //     //     egui::Align2::LEFT_TOP,
            //     //     "Size",
            //     //     egui::TextStyle::Body.resolve(ui.style()),
            //     //     visuals.text_color(),
            //     // );

            //     // ui.painter().text(
            //     //     egui::pos2(modified_x+5, rect.min.y),
            //     //     egui::Align2::LEFT_TOP,
            //     //     "Modified",
            //     //     egui::TextStyle::Body.resolve(ui.style()),
            //     //     visuals.text_color(),
            //     // );
            // }

            // File list for assets
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                row_height,
                total_rows,
                |ui, row_range| {
                    if display_image_preview {
                        for row_idx in row_range {
                            ui.horizontal(|ui| {
                                for amount in 0..amount_per_row {
                                    let i = (row_idx*amount_per_row)+amount;
                                    if let Some(asset) = file_list.get(i) {
                                        let file_name = &asset.name;
                                        let alias = config::get_asset_alias(&file_name);
                
                                        let is_selected  = if none_selected && i != 0 { // Selecting the very first causes some issues
                                            *self.selected = Some(i); // If there is none selected, Set selected and return true
                                            none_selected = false; // Will select everything if this is not set to false immediately
                                            true
                                        } else {
                                            *self.selected == Some(i) // Check if this current one is selected
                                        };
            
                                        // Draw the text
                                        if is_selected && *self.renaming {
                                            self.handle_text_edit(ui, &alias, &file_name); // Allow user to edit
                                        } else {
                                            let desired_size = egui::vec2(row_height, row_height); // Set height to the text style height
                                            let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
                                            
                                            // Only attempt to load if it's a real asset
                                            if asset.real_asset {
                                                if let Some(texture) = load_asset_image(file_name.to_string(), tab.to_string(), cache_directory.clone(), ui.ctx().clone()) {
                                                    egui::Image::new(&texture).maintain_aspect_ratio(true).max_height(row_height).paint_at(ui, rect);
                                                }
                                            }

        
                                            let visuals = ui.visuals();
        
                                            // Get colours and handle response
                                            let colours = self.handle_asset_response(response, visuals, is_selected, i, scroll_to, &mut navigation_accepted, cache_directory.clone(), &tab, &mut focus_search_box, &file_name);
        
                                            let text_colour = colours.1;
                                            let background_colour = colours.0;
        
                                            // Draw the background colour
                                            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(row_height/8.0, background_colour), egui::StrokeKind::Inside);
            
                                            // Draw text ontop of image
                                            let text = egui::Label::new(
                                                egui::RichText::new(alias)
                                                    .text_style(egui::TextStyle::Body)
                                                    .color(text_colour)
                                            ).truncate().selectable(false);
        
                                            let text_size = ui.text_style_height(&egui::TextStyle::Body);
        
                                            let text_rect = egui::Rect::from_min_size(
                                                rect.min + egui::vec2(0.0, (rect.height() - text_size) / 2.0),
                                                egui::vec2(row_height, text_size)
                                            );
        
                                            // Background to make text easier to read
                                            let background_colour = if visuals.dark_mode {
                                                egui::Color32::from_rgba_unmultiplied(27, 27, 27, 160) // Dark mode
                                            } else {
                                                egui::Color32::from_rgba_unmultiplied(248, 248, 248, 160) // Light mode
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
                                    *self.selected = Some(i);
                                    none_selected = false;
                                    true
                                } else {
                                    *self.selected == Some(i)
                                };
            
                                if is_selected && *self.renaming {
                                    self.handle_text_edit(ui, &alias, &asset.name);
                                } else {
                                    let full_width = ui.available_width();
                                    let desired_size = egui::vec2(full_width, row_height);
                                    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
            
                                    let visuals = ui.visuals();
                                    let colours = self.handle_asset_response(
                                        response, visuals, is_selected, i, scroll_to, 
                                        &mut navigation_accepted, cache_directory.clone(), &tab, 
                                        &mut focus_search_box, &asset.name
                                    );
            
                                    let text_colour = colours.1;
                                    let background_colour = colours.0;
            
                                    ui.painter().rect_filled(rect, 0.0, background_colour);
            
                                    // // Format metadata
                                    // let size = format_size(asset.size);
                                    // let modified = if asset.last_modified.is_some() {
                                    //     format_modified(asset.last_modified.unwrap())
                                    // } else {
                                    //     "".to_string()
                                    // };

                                    // Column positions (add padding)
                                    let alias_x = rect.min.x + 5.0;
                                    // let size_x = rect.min.x + rect.width() * 0.7;
                                    // let modified_x = rect.min.x + rect.width() * 1.0 - 5.0; // adjust for padding
            
                                    // Draw all columns
                                    ui.painter().text(
                                        egui::pos2(alias_x, rect.min.y),
                                        egui::Align2::LEFT_TOP,
                                        alias,
                                        egui::TextStyle::Body.resolve(ui.style()),
                                        text_colour,
                                    );
                                
                                // These are for later, broken rn
                                //     ui.painter().text(
                                //         egui::pos2(size_x, rect.min.y),
                                //         egui::Align2::RIGHT_TOP,
                                //         size,
                                //         egui::TextStyle::Body.resolve(ui.style()),
                                //         text_colour,
                                //     );
            
                                //     ui.painter().text(
                                //         egui::pos2(modified_x, rect.min.y),
                                //         egui::Align2::RIGHT_TOP,
                                //         modified,
                                //         egui::TextStyle::Body.resolve(ui.style()),
                                //         text_colour,
                                //     );
                                }
                            }
                        }
                    }
                }
            );

            if !navigation_accepted && scroll_to.is_some() {
                // If the keyboard navigation wasn't accepted and there is keyboard navigation then...
                *self.selected = None; // Set the selected to none, so it selects something on-screen
            }

        } else if tab == "settings" {
            // This is only shown in the settings tab

            settings::actions(ui, self.locale);
            settings::cache_dir_management(ui, self.locale);
            settings::behavior(ui, &self.locale);  
            settings::updates(ui, self.locale);

            if settings::language(ui, self.locale) {
                // This returns true if the locales need to be refreshed
                *self.locale = locale::get_locale(None);
            }

        } else if tab == "logs" {
            ui.heading(locale::get_message(self.locale, "logs", None));
            ui.label(locale::get_message(self.locale, "logs-description", None));

            let mut hide_username_from_logs = config::get_config_bool("hide_username_from_logs").unwrap_or(true);

            let logs = if hide_username_from_logs {
                log::get_anonymous_logs()
            } else {
                log::get_logs()
            };
            let lines = logs.lines();

            ui.horizontal(|ui| {
                ui.checkbox(&mut hide_username_from_logs, locale::get_message(&self.locale, "checkbox-hide-user-logs", None));
                config::set_config_value("hide_username_from_logs", hide_username_from_logs.into());

                if ui.button(locale::get_message(&self.locale, "button-copy-logs", None)).clicked() {
                    ui.ctx().copy_text(logs.clone());
                }
                if ui.button(locale::get_message(&self.locale, "button-export-logs", None)).clicked() {
                    if let Some(path) = DialogBuilder::file()
                        .save_single_file().show()
                        .unwrap()
                    {
                        if let Err(e) = std::fs::write(path, logs.clone()) {
                            log_critical!("Failed to save logs: {}", e);
                        }
                    }
                }
            });

            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                for line in lines {
                    let colour = if line.contains("WARN") {
                        egui::Color32::from_rgb(150,150,0)
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

            ui.heading(locale::get_message(self.locale, "support-project-donate", None));

            ui.horizontal(|ui| {
                ui.hyperlink_to(locale::get_message(self.locale, "support-sponsor", None), "https://github.com/sponsors/AeEn123");
                ui.label("|");
                ui.hyperlink_to("Roblox", "https://www.roblox.com/communities/10808976/Alfie-Likes-Computers#!/store")

            });

            ui.separator();

            ui.heading(locale::get_message(self.locale, "contributors", None));
            for contributor in CONTRIBUTORS {
                ui.hyperlink_to(format!("@{}",contributor), format!("https://github.com/{}", contributor));
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
    selected: Option<usize>, // Used for storing selected state to retain keyboard navigation as seen in the tkinter version
    current_tab: Option<String>, // Allows for detecting when the user changes tabs to refresh automatically
    renaming: bool,
    searching: bool,
    search_query: String,
    swapping: bool,
    swapping_asset_a: Option<String>,
    locale: FluentBundle<Arc<FluentResource>>,
    asset_context_menu_open: Option<usize>,
    copying: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        let tree = DockState::new(vec!["music".to_owned(), "sounds".to_owned(), "images".to_owned(), "rbxm-files".to_owned(), "ktx-files".to_owned(), "settings".to_owned(), "logs".to_owned(), "about".to_owned()]);

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
            selected: None,
            current_tab: None,
            renaming: false,
            searching: false,
            search_query: "".to_owned(),
            swapping: false,
            swapping_asset_a: None,
            locale: locale::get_locale(None),
            asset_context_menu_open: None,
            copying: false,
        }
    }
}

fn detect_japanese_font() -> Option<std::path::PathBuf> {
    let font_dirs = ["C:\\Windows\\Fonts\\msgothic.ttc", "/usr/share/fonts/noto-cjk/NotoSerifCJK-Regular.ttc", "~/.local/share/fonts/noto-cjk/NotoSerifCJK-Regular.ttc", "~/.fonts/noto-cjk/NotoSerifCJK-Regular.ttc"];
    
    for font in font_dirs {
        let resolved_font = PathBuf::from(logic::resolve_path(&font));
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
        
    };
    return None;
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
                    font.font_data.insert("japanese".to_owned(),egui::FontData::from_owned(bytes).into());
                    font.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("japanese".to_owned());
                    font.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("japanese".to_owned());
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
    match config::get_config_string("theme").unwrap_or("system".to_owned()).as_str() {
        "dark" => cc.egui_ctx.set_theme(egui::Theme::Dark),
        "light" => cc.egui_ctx.set_theme(egui::Theme::Light),
        _ => ()
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
                if ctx.input(|input| input.key_pressed(egui::Key::from_name(&i.to_string()).expect("Invalid key"))) {
                    if let Some(&(surface, node, tab)) = self.tab_map.get(&i) {
                        self.tree.set_active_tab((surface, node, egui_dock::TabIndex(tab)));
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
            .show(ctx, &mut TabViewer { 
                // Pass selected as a mutable reference
                selected: &mut self.selected,
                renaming: &mut self.renaming,
                searching: &mut self.searching,
                search_query: &mut self.search_query,
                swapping: &mut self.swapping,
                swapping_asset_a: &mut self.swapping_asset_a,
                current_tab: &mut self.current_tab,
                locale: &mut self.locale,
                asset_context_menu_open: &mut self.asset_context_menu_open,
                copying: &mut self.copying,
            });
        
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
            updater::check_for_updates(true, config::get_config_bool("automatically_install_updates").unwrap_or(false));
        }

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(
                    eframe::icon_data::from_png_bytes(&ICON[..])
                        .expect("Failed to load icon"),
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

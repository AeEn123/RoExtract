use crate::logic;
use std::sync::Arc;
use fluent_bundle::{FluentBundle, FluentResource, FluentArgs};
use native_dialog::{MessageDialog, FileDialog, MessageType};


pub fn actions(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.heading(logic::get_message(locale, "actions", None));

    // Clear cache description
    ui.label(logic::get_message(locale, "clear-cache-description", None));
    
    // Clear cache button
    if ui.button(logic::get_message(locale, "button-clear-cache", None)).clicked() || ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        // Confirmation dialog
        let yes = MessageDialog::new()
        .set_type(MessageType::Info)
        .set_title(&logic::get_message(locale, "confirmation-clear-cache-title", None))
        .set_text(&logic::get_message(locale, "confirmation-clear-cache-description", None))
        .show_confirm()
        .unwrap();

        if yes {
            logic::delete_all_directory_contents(logic::get_cache_directory().to_owned());
        }                    
    }

    // Extract all description
    ui.label(logic::get_message(locale, "extract-all-description", None));

    // Extract all button
    if ui.button(&logic::get_message(locale, "button-extract-all", None)).clicked() || ui.input(|i| i.key_pressed(egui::Key::F3)) {
        let mut no = logic::get_list_task_running();
    
        // Confirmation dialog, the program is still listing files
        if no {
            // NOT result, will become false if user clicks yes
            no = !MessageDialog::new()
            .set_type(MessageType::Info)
            .set_title(&logic::get_message(locale, "confirmation-filter-confirmation-title", None))
            .set_text(&logic::get_message(locale, "confirmation-filter-confirmation-description", None))
            .show_confirm()
            .unwrap();
        }
    
        // The user either agreed or the program is not listing files
        if !no {
            let option_path = FileDialog::new()
            .show_open_single_dir()
            .unwrap();
    
            // If the user provides a directory, the program will extract the assets to that directory
            if let Some(path) = option_path {
                logic::extract_all( path.to_string_lossy().to_string(), false, logic::get_config_bool("use_alias").unwrap_or(false))
            }
        }
    }
}

pub fn cache_dir_management(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.label(logic::get_message(locale, "custom-cache-dir-description", None));

    let mut args = FluentArgs::new();
    args.set("directory", logic::get_cache_directory());

    ui.label(logic::get_message(locale, "cache-directory", Some(&args)));

    ui.horizontal(|ui| {
        if ui.button(logic::get_message(locale, "button-change-cache-dir", None)).clicked() {
            let option_path = FileDialog::new()
            .show_open_single_dir()
            .unwrap();
    
            // If the user provides a directory, the program will change the cache directory to the new one
            if let Some(path) = option_path {
                // Validation checks
                match logic::validate_directory(&path.to_string_lossy().to_string()) {
                    Ok(directory) => {
                        logic::set_config_value("cache_directory", directory.into());
                        logic::set_cache_directory(logic::detect_directory()); // Set directory to new one
                    }
                    Err(_) => {
                        MessageDialog::new()
                        .set_type(MessageType::Info)
                        .set_title(&logic::get_message(locale, "error-invalid-directory-title", None))
                        .set_text(&logic::get_message(locale, "error-invalid-directory-description", None))
                        .show_alert()
                        .unwrap();
                    }
                }
            }
        }
        if ui.button(logic::get_message(locale, "button-reset-cache-dir", None)).clicked() {
            logic::set_config_value("cache_directory", "no directory set".into()); // Clear directory in config
            logic::set_cache_directory(logic::detect_directory()); // Set it back to default
        }
    });
}

pub fn updates(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    if !logic::get_system_config_bool("allow-updates").unwrap_or(true) {
        return
    }
    ui.separator();
    ui.heading(logic::get_message(locale, "updates", None));

    // Get configurations for use in checkboxes
    let mut check_for_updates = logic::get_config_bool("check_for_updates").unwrap_or(true);
    let mut automatically_install_updates = logic::get_config_bool("automatically_install_updates").unwrap_or(false);
    let mut include_prerelease = logic::get_config_bool("include_prerelease").unwrap_or(false);

    ui.checkbox(&mut check_for_updates, logic::get_message(locale, "check-for-updates", None));
    ui.checkbox(&mut automatically_install_updates, logic::get_message(locale, "automatically-install-updates", None));
    
    ui.label(logic::get_message(locale, "setting-below-restart-required", None)); // Restart is required to change this setting
    ui.checkbox(&mut include_prerelease, logic::get_message(locale, "download-development-build", None));

    // Add them to the config again
    logic::set_config_value("check_for_updates", check_for_updates.into());
    logic::set_config_value("automatically_install_updates", automatically_install_updates.into());
    logic::set_config_value("include_prerelease", include_prerelease.into());
}

pub fn behavior(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.heading(logic::get_message(locale, "behavior", None));

    egui::widgets::global_theme_preference_buttons(ui);
    match ui.ctx().options(|opt| opt.theme_preference) {
        egui::ThemePreference::Dark => logic::set_config_value("theme", "dark".into()),
        egui::ThemePreference::Light => logic::set_config_value("theme", "light".into()),
        egui::ThemePreference::System => logic::set_config_value("theme", "system".into())
    }
    
    
    ui.label(logic::get_message(locale, "use-alias-description", None));

    let mut use_alias = logic::get_config_bool("use_alias").unwrap_or(true);
    ui.checkbox(&mut use_alias, logic::get_message(locale, "use-alias", None));
    logic::set_config_value("use_alias", use_alias.into());

    let mut use_alias = logic::get_config_bool("refresh_before_extract").unwrap_or(false);
    ui.checkbox(&mut use_alias, logic::get_message(locale, "refresh-before-extract", None));
    logic::set_config_value("refresh_before_extract", use_alias.into());

    let mut use_topbar_buttons = logic::get_config_bool("use_topbar_buttons").unwrap_or(true);
    ui.checkbox(&mut use_topbar_buttons, logic::get_message(locale, "use-topbar-buttons", None));
    logic::set_config_value("use_topbar_buttons", use_topbar_buttons.into());

    let mut display_image_preview = logic::get_config_bool("display_image_preview").unwrap_or(false);
    ui.checkbox(&mut display_image_preview, logic::get_message(locale, "button-display-image-preview", None));
    logic::set_config_value("display_image_preview", display_image_preview.into());

    let mut image_preview_size = logic::get_config_u64("image_preview_size").unwrap_or(128);
    ui.add(egui::widgets::Slider::new(&mut image_preview_size, (16 as u64)..=(512 as u64))
    .text(logic::get_message(locale, "input-preview-size", None)));
    logic::set_config_value("image_preview_size", image_preview_size.into());
    
}

pub fn language(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) -> bool {
    ui.heading(logic::get_message(locale, "language-settings", None));

    let mut user_clicked = false;

    let languages = logic::get_language_list();
    egui::ScrollArea::vertical().show_rows(
        ui,
        ui.text_style_height(&egui::TextStyle::Body),
        languages.len(),
        |ui, row_range| {
        for i in row_range {
            let language = languages[i].clone();
            let lang_code = language.0;
            let is_selected = *lang_code == locale.locales[0].to_string();

            let visuals = ui.visuals();

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

    
            // Using a rect to allow the user to click across the entire list, not just the text
            let full_width = ui.available_width();
            let desired_size = egui::vec2(full_width, ui.text_style_height(&egui::TextStyle::Body)); // Set height to the text style height
            let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

            // Draw the background colour
            ui.painter().rect_filled(rect, 0.0, background_colour);

            // Draw the text
            ui.painter().text(
                rect.min + egui::vec2(5.0, 0.0), // Add a bit of padding for the label text
                egui::Align2::LEFT_TOP,
                language.1.clone(), // Text is the file name
                egui::TextStyle::Body.resolve(ui.style()),
                text_colour,
            );

            // Handle the click/double click
            if response.clicked() {
                logic::set_config_value("language", lang_code.to_string().into());
                user_clicked = true; // Refresh locales
            }
        }
    });
    return user_clicked; // Refresh depending on if the user clicked or not
}
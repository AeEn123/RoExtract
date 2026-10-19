use crate::{config, dialog, gui, locale, logic, logic::convert};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::{sync::Arc, thread};

pub fn actions(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.heading(locale::get_message(locale, "actions", None));

    // Clear cache description
    ui.label(locale::get_message(locale, "clear-cache-description", None));

    // Clear cache button
    if ui
        .button(locale::get_message(locale, "button-clear-cache", None))
        .clicked()
        || ui.input(|i| i.key_pressed(egui::Key::Delete))
    {
        // Runs on its own thread so the dialog doesn't block the GUI
        thread::spawn(|| {
            if dialog::confirm_localised(
                dialog::Level::Info,
                "confirmation-clear-cache-title",
                "confirmation-clear-cache-description",
            ) {
                logic::clear_cache();
            }
        });
    }

    // Extract all description
    ui.label(locale::get_message(locale, "extract-all-description", None));

    // Extract all button
    if ui
        .button(locale::get_message(locale, "button-extract-all", None))
        .clicked()
        || ui.input(|i| i.key_pressed(egui::Key::F3))
    {
        thread::spawn(|| {
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
                logic::extract_all(
                    path,
                    false,
                    config::get_config_bool("use_alias").unwrap_or(false),
                )
            }
        });
    }
}

pub fn cache_dir_management(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.label(locale::get_message(
        locale,
        "custom-cache-dir-description",
        None,
    ));

    let mut args = FluentArgs::new();
    args.set(
        "directory",
        logic::cache_directory::get_cache_directory()
            .to_string_lossy()
            .to_string(),
    );

    ui.label(locale::get_message(locale, "cache-directory", Some(&args)));

    ui.horizontal(|ui| {
        if ui
            .button(locale::get_message(locale, "button-change-cache-dir", None))
            .clicked()
        {
            thread::spawn(|| {
                // If the user provides a directory, the program will change the cache directory to the new one
                if let Some(path) = dialog::pick_folder() {
                    // Validation checks
                    match logic::cache_directory::validate_directory(
                        path.to_string_lossy().as_ref(),
                    ) {
                        Ok(directory) => {
                            config::set_config_value("cache_directory", directory.into());
                            logic::cache_directory::set_cache_directory(
                                logic::cache_directory::detect_directory(),
                            ); // Set directory to new one
                        }
                        Err(_) => dialog::alert_localised(
                            dialog::Level::Info,
                            "error-invalid-directory-title",
                            "error-invalid-directory-description",
                        ),
                    }
                }
            });
        }
        if ui
            .button(locale::get_message(locale, "button-reset-cache-dir", None))
            .clicked()
        {
            config::remove_config_value("cache_directory"); // Clear directory in config
            thread::spawn(|| {
                // Detection may need to ask the user, so it runs off the GUI thread
                logic::cache_directory::set_cache_directory(
                    logic::cache_directory::detect_directory(),
                ); // Set it back to default
            });
        }
    });
}

pub fn sql_db_management(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.label(locale::get_message(
        locale,
        "custom-sql-db-description",
        None,
    ));

    let mut args = FluentArgs::new();
    args.set(
        "path",
        logic::sql_database::get_db_path().unwrap_or("No database".to_string()),
    );

    ui.label(locale::get_message(locale, "sql-database", Some(&args)));

    ui.horizontal(|ui| {
        if ui
            .button(locale::get_message(locale, "button-change-sql-db", None))
            .clicked()
        {
            thread::spawn(|| {
                // If the user provides a path, the program will change the SQL database to the new one
                if let Some(path) = dialog::pick_file() {
                    // Validation checks
                    match logic::sql_database::validate_file(path.to_string_lossy().as_ref()) {
                        Ok(directory) => {
                            config::set_config_value("sql_database", directory.into());

                            // Close current db and open new one
                            let _ = logic::sql_database::reset_database();
                        }
                        Err(_) => dialog::alert_localised(
                            dialog::Level::Info,
                            "error-invalid-database-title",
                            "error-invalid-database-description",
                        ),
                    }
                }
            });
        }
        if ui
            .button(locale::get_message(locale, "button-reset-sql-db", None))
            .clicked()
        {
            config::remove_config_value("sql_database"); // Clear db in config

            // Close current db and open new one, which may need to ask the user
            thread::spawn(|| {
                let _ = logic::sql_database::reset_database();
            });
        }
    });
}

pub fn updates(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    if !config::get_system_config_bool("allow-updates").unwrap_or(true) {
        return;
    }
    ui.separator();
    ui.heading(locale::get_message(locale, "updates", None));

    // Get configurations for use in checkboxes
    let old_check = config::get_config_bool("check_for_updates").unwrap_or(true);
    let old_auto = config::get_config_bool("automatically_install_updates").unwrap_or(false);
    let old_pre = config::get_config_bool("include_prerelease").unwrap_or(false);

    let mut check_for_updates = old_check;
    let mut automatically_install_updates = old_auto;
    let mut include_prerelease = old_pre;

    ui.checkbox(
        &mut check_for_updates,
        locale::get_message(locale, "check-for-updates", None),
    );
    ui.checkbox(
        &mut automatically_install_updates,
        locale::get_message(locale, "automatically-install-updates", None),
    );

    ui.label(locale::get_message(
        locale,
        "setting-below-restart-required",
        None,
    )); // Restart is required to change this setting
    ui.checkbox(
        &mut include_prerelease,
        locale::get_message(locale, "download-development-build", None),
    );

    // Write only on change.
    if check_for_updates != old_check {
        config::set_config_value("check_for_updates", check_for_updates.into());
    }
    if automatically_install_updates != old_auto {
        config::set_config_value(
            "automatically_install_updates",
            automatically_install_updates.into(),
        );
    }
    if include_prerelease != old_pre {
        config::set_config_value("include_prerelease", include_prerelease.into());
    }
}

pub fn behavior(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.heading(locale::get_message(locale, "behavior", None));

    egui::widgets::global_theme_preference_buttons(ui);
    let theme_str = match ui.ctx().options(|opt| opt.theme_preference) {
        egui::ThemePreference::Dark => "dark",
        egui::ThemePreference::Light => "light",
        egui::ThemePreference::System => "system",
    };
    if config::get_config_string("theme").as_deref() != Some(theme_str) {
        config::set_config_value("theme", theme_str.into());
    }

    ui.label(locale::get_message(locale, "use-alias-description", None));

    let old = config::get_config_bool("use_alias").unwrap_or(true);
    let mut use_alias = old;
    ui.checkbox(
        &mut use_alias,
        locale::get_message(locale, "use-alias", None),
    );
    if use_alias != old {
        config::set_config_value("use_alias", use_alias.into());
    }

    let old = config::get_config_bool("refresh_before_extract").unwrap_or(false);
    let mut refresh_before_extract = old;
    ui.checkbox(
        &mut refresh_before_extract,
        locale::get_message(locale, "refresh-before-extract", None),
    );
    if refresh_before_extract != old {
        config::set_config_value("refresh_before_extract", refresh_before_extract.into());
    }

    let old = config::get_config_bool("use_builtin_player").unwrap_or(true);
    let mut use_builtin_player = old;
    ui.checkbox(
        &mut use_builtin_player,
        locale::get_message(locale, "use-builtin-player", None),
    );
    if use_builtin_player != old {
        config::set_config_value("use_builtin_player", use_builtin_player.into());
    }

    let old = config::get_config_bool("use_topbar_buttons").unwrap_or(true);
    let mut use_topbar_buttons = old;
    ui.checkbox(
        &mut use_topbar_buttons,
        locale::get_message(locale, "use-topbar-buttons", None),
    );
    if use_topbar_buttons != old {
        config::set_config_value("use_topbar_buttons", use_topbar_buttons.into());
    }

    let old = config::get_config_bool("display_image_preview").unwrap_or(false);
    let mut display_image_preview = old;
    ui.checkbox(
        &mut display_image_preview,
        locale::get_message(locale, "button-display-image-preview", None),
    );
    if display_image_preview != old {
        config::set_config_value("display_image_preview", display_image_preview.into());
    }

    let old = config::get_config_u64("image_preview_size").unwrap_or(128);
    let mut image_preview_size = old;
    ui.add(
        egui::widgets::Slider::new(
            &mut image_preview_size,
            gui::PREVIEW_SIZE_MIN..=gui::PREVIEW_SIZE_MAX,
        )
        .text(locale::get_message(locale, "input-preview-size", None)),
    );
    if image_preview_size != old {
        config::set_config_value("image_preview_size", image_preview_size.into());
    }
}

pub fn conversion(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.heading(locale::get_message(locale, "conversion", None));
    ui.label(locale::get_message(locale, "conversion-description", None));

    let old = convert::is_enabled();
    let mut enabled = old;
    ui.checkbox(
        &mut enabled,
        locale::get_message(locale, "enable-conversion", None),
    );
    if enabled != old {
        config::set_config_value("conversion_enabled", enabled.into());
    }

    ui.add_enabled_ui(enabled, |ui| {
        let old = config::get_config_bool("convert_on_extract").unwrap_or(true);
        let mut convert_on_extract = old;
        ui.checkbox(
            &mut convert_on_extract,
            locale::get_message(locale, "convert-on-extract", None),
        );
        if convert_on_extract != old {
            config::set_config_value("convert_on_extract", convert_on_extract.into());
        }

        format_choice(
            ui,
            "convert_image_format",
            &locale::get_message(locale, "image-format", None),
            &convert::Target::IMAGE,
        );
        format_choice(
            ui,
            "convert_audio_format",
            &locale::get_message(locale, "audio-format", None),
            &convert::Target::AUDIO,
        );
    });
}

/// A drop-down to pick which format to convert to, saved under `key`.
fn format_choice(ui: &mut egui::Ui, key: &str, label: &str, targets: &[convert::Target]) {
    let current = config::get_config_string(key)
        .and_then(|extension| convert::Target::from_extension(&extension))
        .filter(|target| targets.contains(target))
        .unwrap_or(targets[0]);
    let mut selected = current;
    egui::ComboBox::from_label(label)
        .selected_text(current.name())
        .show_ui(ui, |ui| {
            for &target in targets {
                ui.selectable_value(&mut selected, target, target.name());
            }
        });
    if selected != current {
        config::set_config_value(key, selected.extension().into());
    }
}

pub fn language(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) -> bool {
    ui.heading(locale::get_message(locale, "language-settings", None));

    let mut user_clicked = false;

    let languages = locale::get_language_list();
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
                let desired_size =
                    egui::vec2(full_width, ui.text_style_height(&egui::TextStyle::Body)); // Set height to the text style height
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
                    config::set_config_value("language", lang_code.to_string().into());
                    user_clicked = true; // Refresh locales
                }
            }
        },
    );
    user_clicked // Refresh depending on if the user clicked or not
}

pub fn rbx_storage_dir_management(ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
    ui.separator();
    ui.label(locale::get_message(
        locale,
        "custom-rbx-storage-dir-description",
        None,
    ));

    let mut args = FluentArgs::new();
    args.set(
        "directory",
        logic::rbx_storage_directory::get_rbx_storage_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| locale::get_message(locale, "no-directory", None)),
    );

    ui.label(locale::get_message(
        locale,
        "rbx-storage-directory",
        Some(&args),
    ));

    ui.horizontal(|ui| {
        if ui
            .button(locale::get_message(
                locale,
                "button-change-rbx-storage-dir",
                None,
            ))
            .clicked()
        {
            thread::spawn(|| {
                if let Some(path) = dialog::pick_folder() {
                    if path.is_dir() {
                        config::set_config_value(
                            "rbx_storage_directory",
                            path.to_string_lossy().to_string().into(),
                        );
                        logic::rbx_storage_directory::set_rbx_storage_dir(
                            logic::rbx_storage_directory::detect_directory(),
                        );
                    } else {
                        dialog::alert_localised(
                            dialog::Level::Info,
                            "error-invalid-directory-title",
                            "error-invalid-directory-description",
                        );
                    }
                }
            });
        }
        if ui
            .button(locale::get_message(
                locale,
                "button-reset-rbx-storage-dir",
                None,
            ))
            .clicked()
        {
            config::remove_config_value("rbx_storage_directory");
            logic::rbx_storage_directory::set_rbx_storage_dir(
                logic::rbx_storage_directory::detect_directory(),
            );
        }
    });
}

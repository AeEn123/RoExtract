use fluent_bundle::{FluentBundle, FluentResource};
use rusqlite::Connection;
use std::{fs, sync::{Arc, LazyLock, Mutex}};

use crate::{config, locale, logic};

const DEFAULT_PATHS: [&str; 2] = ["%localappdata%\\Roblox\\rbx-storage.db", "~/.var/app/org.vinegarhq.Sober/data/sober/appData/rbx-storage.db"]; // For windows and linux (sober)
const CONNECTION: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(||Mutex::new(open_database()));

pub fn open_database() -> Option<Connection> {
    let mut errors = "".to_owned();

    // User-specified path from config
    if let Some(path) = config::get_config_string("sql_database") {
        match validate_file(&path) {
            Ok(resolved_path) => {
                match Connection::open(resolved_path) {
                    Ok(connection) => return Some(connection),
                    Err(e) => errors.push_str(&e.to_string()),
                }
            },
            Err(e) => errors.push_str(&e),
        }

    }

    for path in DEFAULT_PATHS {
        match validate_file(&path) {
            Ok(resolved_path) => {
                match Connection::open(resolved_path) {
                    Ok(connection) => return Some(connection),
                    Err(e) => errors.push_str(&e.to_string()),
                }
            },
            Err(e) => errors.push_str(&e),
        }
    }

    // If it was unable to detect any path, tell the user
    let _ = native_dialog::DialogBuilder::message()
    .set_level(native_dialog::MessageLevel::Error)
    .set_title(&locale::get_message(&locale::get_locale(None), "error-sql-detection-title", None))
    .set_text(&locale::get_message(&locale::get_locale(None), "error-sql-detection-description", None))
    .alert().show();

    let yes = native_dialog::DialogBuilder::message()
    .set_level(native_dialog::MessageLevel::Error)
    .set_title(&locale::get_message(&locale::get_locale(None), "confirmation-custom-sql-title", None))
    .set_text(&locale::get_message(&locale::get_locale(None), "confirmation-custom-sql-description", None))
    .confirm().show()
    .unwrap();

    if yes {
        let option_path = native_dialog::DialogBuilder::file()
        .open_single_dir().show()
        .unwrap();
        if let Some(path) = option_path {
            config::set_config_value("sql_database", logic::resolve_path(&path.to_string_lossy().to_string()).into());
            return open_database();
        } else {
            log_critical!("Database detection failed! {}", errors);
        }
    } else {
        log_critical!("Database detection failed! {}", errors);
    }

    None
}

pub fn validate_file(path: &str) -> Result<String, String> {
    let resolved_path = logic::resolve_path(path);

    match fs::metadata(&resolved_path) { // Directory detection
        Ok(metadata) => {
            if metadata.is_file() {
                // Successfully detected a directory, we can return it
                return Ok(resolved_path);
            } else {
                return Err(format!("{}: Not a file", resolved_path));
            }
        }
        Err(e) => {
            return Err(e.to_string()); // Convert to correct data type
        }
    }
}


pub fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>) {
    let binding = CONNECTION;
    let connection = binding.lock().unwrap();
    if let Some(conn) = &*connection {
        conn.execute("DROP TABLE Files", ());
        // TODO finsih
    }
    
}
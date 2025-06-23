use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use rusqlite::Connection;
use std::{fs, sync::{Arc, LazyLock, Mutex}, time::SystemTime};
use rusqlite::params;

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
                    Err(e) => errors.push_str(&e.to_string()), // TODO: Don't silently fail on user-specified database
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

    logic::update_progress(0.0);

    // Args for formatting
    let mut args = FluentArgs::new();
    args.set("item", "0");
    args.set("total", "1");

    logic::update_status(locale::get_message(&locale, "deleting-files", Some(&args)));


    if let Some(conn) = &*connection {
        match conn.execute("DROP TABLE files", ()) {
            Ok(_) => {
                logic::update_progress(1.0);
                args.set("item", "1");
                args.set("total", "1");

                logic::update_status(locale::get_message(&locale, "deleting-files", Some(&args)));
            }
            Err(e) => {
                log_error!("Failed to DROP TABLE: {}", e);

                args.set("error", e.to_string());
                logic::update_progress(1.0);
                args.set("item", "1");
                args.set("total", "1");

                logic::update_status(locale::get_message(&locale, "failed-deleting-file", Some(&args)));

            }
        }
    } else {
        log_error!("No SQL Connection!");
        logic::update_status(locale::get_message(&locale, "failed-deleting-file", Some(&args)));

    }
    
}


pub fn refresh(category: logic::Category, cli_list_mode: bool, locale: &FluentBundle<Arc<FluentResource>>) {
    let headers = logic::get_headers(&category);
    let mut args = FluentArgs::new();

    let binding = CONNECTION;
    let connection = binding.lock().unwrap();

    if let Some(conn) = &*connection {

        let mut stmt = conn.prepare("SELECT id, size, ttl, substr(content, 1, 2048) as content_prefix FROM files").unwrap(); // TODO: Error handling

        let entries = stmt.query_map((), |row| {
            // TODO: Progress
            let last_modified_timestamp: u64 = row.get(2)?;
            let last_modified = SystemTime::UNIX_EPOCH
                .checked_add(std::time::Duration::from_secs(last_modified_timestamp));

            let bytes = row.get::<_, Vec<u8>>(3)?;

            let header_found = headers.iter().any(|header| { // Go through each header - if any returns true, we found it.
                logic::bytes_contains(&bytes, header.as_bytes())
            });

            // let header_found = true;

            if header_found {
                Ok(logic::AssetInfo {
                    name: hex::encode(row.get::<_, Vec<u8>>(0)?),
                    size: row.get(1)?,
                    last_modified,
                    from_file: false,
                    from_sql: true,
                    // category
                    category: if category == logic::Category::All { logic::determine_category(&bytes) } else { category } // Determine category if all
                })
            } else {
                Err(rusqlite::Error::InvalidQuery) // Return error for this asset as it doesn't match
            }


        }).unwrap(); // TODO: Error handling

        for entry in entries {
            if entry.is_ok() {
                logic::update_file_list(entry.unwrap(), cli_list_mode);
            }
        }
        
    }

    // TODO: This silently fails, add an error when the condition is false.
}

pub fn read_asset(asset: &logic::AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    let binding = CONNECTION;
    let connection = binding.lock().unwrap();

    if let Some(conn) = &*connection {
        let id_bytes = match hex::decode(&asset.name) {
            Ok(bytes) => bytes,
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)),
        };

        conn.query_row(
            "SELECT content FROM files WHERE id = ?1",
            params![id_bytes],
            |row| row.get(0),
        ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "No SQL connection!"))
    }
}
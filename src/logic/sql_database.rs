use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use rusqlite::params;
use rusqlite::Connection;
use std::{
    fs,
    sync::{Arc, LazyLock, Mutex},
    time::SystemTime,
};

use crate::{config, locale, logic};

const DEFAULT_PATHS: [&str; 2] = [
    "%localappdata%\\Roblox\\rbx-storage.db",
    "~/.var/app/org.vinegarhq.Sober/data/sober/appData/rbx-storage.db",
]; // For windows and linux (sober)
static CONNECTION: LazyLock<Mutex<Option<Connection>>> =
    LazyLock::new(|| Mutex::new(open_database()));

pub fn open_database() -> Option<Connection> {
    log_debug!("logic::sql_database::open_database()");
    let mut errors = "".to_owned();

    // User-specified path from config
    if let Some(path) = config::get_config_string("sql_database") {
        log_debug!("Trying user-specified path: {}", path);
        match validate_file(&path) {
            Ok(resolved_path) => match Connection::open(resolved_path) {
                Ok(connection) => return Some(connection),
                Err(e) => {
                    log_critical!("Detecting user-specified database failed: {}", e);
                    errors.push_str(&e.to_string())
                }
            },
            Err(e) => {
                log_critical!("Detecting user-specified database failed: {}", e);
                errors.push_str(&e)
            }
        }
    }

    for path in DEFAULT_PATHS {
        match validate_file(path) {
            Ok(resolved_path) => match Connection::open(resolved_path) {
                Ok(connection) => return Some(connection),
                Err(e) => errors.push_str(&e.to_string()),
            },
            Err(e) => errors.push_str(&e),
        }
    }

    // If it was unable to detect any path, tell the user
    let _ = native_dialog::DialogBuilder::message()
        .set_level(native_dialog::MessageLevel::Error)
        .set_title(locale::get_message(
            &locale::get_locale(None),
            "error-sql-detection-title",
            None,
        ))
        .set_text(locale::get_message(
            &locale::get_locale(None),
            "error-sql-detection-description",
            None,
        ))
        .alert()
        .show();

    let yes = native_dialog::DialogBuilder::message()
        .set_level(native_dialog::MessageLevel::Error)
        .set_title(locale::get_message(
            &locale::get_locale(None),
            "confirmation-custom-sql-title",
            None,
        ))
        .set_text(locale::get_message(
            &locale::get_locale(None),
            "confirmation-custom-sql-description",
            None,
        ))
        .confirm()
        .show()
        .unwrap();

    if yes {
        let option_path = native_dialog::DialogBuilder::file()
            .open_single_dir()
            .show()
            .unwrap();
        if let Some(path) = option_path {
            config::set_config_value(
                "sql_database",
                logic::resolve_path(path.to_string_lossy().as_ref()).into(),
            );
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
    log_debug!("logic::sql_database::validate_file({path})");
    let resolved_path = logic::resolve_path(path);

    match fs::metadata(&resolved_path) {
        // Directory detection
        Ok(metadata) => {
            if metadata.is_file() {
                // Successfully detected a directory, we can return it
                Ok(resolved_path)
            } else {
                Err(format!("{resolved_path}: Not a file"))
            }
        }
        Err(e) => {
            Err(e.to_string()) // Convert to correct data type
        }
    }
}

pub fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>) {
    log_debug!("logic::sql_database::clear_cache(locale)");

    let connection = CONNECTION.lock().unwrap();

    logic::update_progress(0.0);

    // Args for formatting
    let mut args = FluentArgs::new();
    args.set("item", "0");
    args.set("total", "1");

    logic::update_status(locale::get_message(locale, "deleting-files", Some(&args)));

    args.set("item", "1");
    args.set("total", "1");

    let path = if let Some(conn) = &*connection {
        conn.path()
    } else {
        None
    };
    
    // Disconnect from database before deleting
    match clean_up() {
        Ok(_) => log_info!("Disconnected from database"),
        Err(e) => log_error!("Failed disconnecting from database: {e:?}")
    }

    let storage_folder = if let Some(path) = path {
        if let Some(parent) = std::path::Path::new(path).parent() {
            Some(parent.join("rbx-storage"))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(conn) = &*connection {
        match conn.path() {
            Some(path) => {
                
                match std::fs::remove_file(path) {
                    Ok(_) => {
                        logic::update_progress(1.0);
                        logic::update_status(locale::get_message(
                            locale,
                            "deleting-files",
                            Some(&args),
                        ));
                    }
                    Err(e) => {
                        log_error!("Failed to delete file: {}", e);

                        args.set("error", e.to_string());

                        logic::update_progress(1.0);
                        logic::update_status(locale::get_message(
                            locale,
                            "failed-deleting-file",
                            Some(&args),
                        ));
                    }
            }
            },
            None => {
                log_error!("Failed to delete file: No path specified");

                args.set("error", "No path specified");

                logic::update_status(locale::get_message(
                    locale,
                    "failed-deleting-file",
                    Some(&args),
                ));
            }
        }
    } else {
        log_error!("No SQL Connection!");
        logic::update_status(locale::get_message(
            locale,
            "failed-deleting-file",
            Some(&args),
        ));
    }
}

pub fn refresh(
    category: logic::Category,
    cli_list_mode: bool,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    log_debug!("logic::sql_database::refresh({category}, {cli_list_mode}, locale)");

    if category == logic::Category::Music {
        return; // Music category is specific to /sounds folder.
    }

    let headers = logic::get_headers(&category);
    let mut args = FluentArgs::new();

    let connection = CONNECTION.lock().unwrap();

    if let Some(conn) = &*connection {
        let amount: Result<i64, _> =
            conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0));

        match conn
            .prepare("SELECT id, size, ttl, substr(content, 1, 2048) as content_prefix FROM files")
        {
            Ok(mut stmt) => {
                let mut count: i64 = 0;
                let result = stmt.query_map((), |row| {
                    if let Ok(total) = amount {
                        args.set("item", count);
                        args.set("total", total);
                        logic::update_progress(count as f32 / total as f32);
                        logic::update_status(locale::get_message(
                            locale,
                            "filtering-files",
                            Some(&args),
                        ));
                        count += 1;
                    }

                    let last_modified_timestamp: u64 = row.get(2)?;
                    let last_modified = SystemTime::UNIX_EPOCH
                        .checked_add(std::time::Duration::from_secs(last_modified_timestamp));

                    let bytes = row.get::<_, Vec<u8>>(3)?;

                    let header_found = headers.iter().any(|header| {
                        // Go through each header - if any returns true, we found it.
                        logic::bytes_contains(&bytes, header.as_bytes())
                    });

                    if header_found {
                        Ok(logic::AssetInfo {
                            name: hex::encode(row.get::<_, Vec<u8>>(0)?),
                            _size: row.get(1)?,
                            last_modified,
                            from_file: false,
                            from_sql: true,
                            category: if category == logic::Category::All {
                                logic::determine_category(&bytes)
                            } else {
                                category
                            }, // Determine category if all
                        })
                    } else {
                        Err(rusqlite::Error::InvalidQuery) // Return error for this asset as it doesn't match
                    }
                });

                match result {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            logic::update_file_list(entry, cli_list_mode);
                        }
                    }
                    Err(e) => log_error!("{}", e),
                }
            }
            Err(e) => {
                log_error!("Error happened when querying DB for listing files: {}", e);
                logic::update_status(locale::get_message(
                    locale,
                    "failed-opening-file",
                    Some(&args),
                ));
            }
        }
    } else {
        log_error!("No SQL Connection!");
        logic::update_status(locale::get_message(
            locale,
            "failed-opening-file",
            Some(&args),
        ));
    }
}

pub fn read_asset(asset: &logic::AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    log_debug!("logic::sql_database::read_asset({asset:?})");
    let connection = CONNECTION.lock().unwrap();

    if let Some(conn) = &*connection {
        let id_bytes = match hex::decode(&asset.name) {
            Ok(bytes) => bytes,
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)),
        };

        conn.query_row(
            "SELECT content FROM files WHERE id = ?1",
            params![id_bytes],
            |row| row.get(0),
        )
        .map_err(std::io::Error::other)
    } else {
        Err(std::io::Error::other("No SQL connection!"))
    }
}

pub fn create_asset_info(asset: &str, category: logic::Category) -> Option<logic::AssetInfo> {
    log_debug!("logic::sql_database::create_asset_info({asset}, {category})");
    let connection = CONNECTION.lock().unwrap();

    if let Some(conn) = &*connection {
        let id_bytes = match hex::decode(asset) {
            Ok(bytes) => bytes,
            Err(_) => return None,
        };
        conn.query_row(
            "SELECT id, size, ttl FROM files WHERE id = ?1",
            params![id_bytes],
            |row| {
                let last_modified_timestamp: u64 = row.get(2)?;
                let last_modified = SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(last_modified_timestamp)); // Convert u64 to SystemTime

                Ok(logic::AssetInfo {
                    name: asset.to_string(),
                    _size: row.get(1)?,
                    last_modified,
                    from_file: false,
                    from_sql: true,
                    category,
                })
            },
        )
        .ok()
    } else {
        None
    }
}

pub fn swap_assets(
    asset_a: &logic::AssetInfo,
    asset_b: &logic::AssetInfo,
) -> Result<(), rusqlite::Error> {
    log_debug!("logic::sql_database::swap_assets({asset_a:?}, {asset_b:?})");

    let mut connection = CONNECTION.lock().unwrap();

    if let Some(conn) = connection.as_mut() {
        let id_a = hex::decode(&asset_a.name).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
        })?;
        let id_b = hex::decode(&asset_b.name).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
        })?;

        let tx = conn.transaction()?;

        let content_a: Vec<u8> = tx.query_row(
            "SELECT content FROM files WHERE id = ?1",
            params![&id_a],
            |row| row.get(0),
        )?;
        let content_b: Vec<u8> = tx.query_row(
            "SELECT content FROM files WHERE id = ?1",
            params![&id_b],
            |row| row.get(0),
        )?;

        tx.execute(
            "UPDATE files SET content = ?1 WHERE id = ?2",
            params![&content_b, &id_a],
        )?;
        tx.execute(
            "UPDATE files SET content = ?1 WHERE id = ?2",
            params![&content_a, &id_b],
        )?;

        tx.commit()?;
        Ok(())
    } else {
        Err(rusqlite::Error::InvalidQuery)
    }
}

pub fn copy_assets(
    asset_a: &logic::AssetInfo,
    asset_b: &logic::AssetInfo,
) -> Result<(), rusqlite::Error> {
    log_debug!("logic::sql_database::copy_assets({asset_a:?}, {asset_b:?})");

    let connection = CONNECTION.lock().unwrap();

    if let Some(conn) = &*connection {
        let id_a = hex::decode(&asset_a.name).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
        })?;
        let id_b = hex::decode(&asset_b.name).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
        })?;

        let content_a: Vec<u8> = conn.query_row(
            "SELECT content FROM files WHERE id = ?1",
            params![&id_a],
            |row| row.get(0),
        )?;
        conn.execute(
            "UPDATE files SET content = ?1 WHERE id = ?2",
            params![&content_a, &id_b],
        )?;
        Ok(())
    } else {
        Err(rusqlite::Error::InvalidQuery)
    }
}

pub fn get_db_path() -> Option<String> {
    log_debug!("logic::sql_database::get_db_path()");

    let connection = CONNECTION.lock().unwrap();

    if let Some(conn) = &*connection {
        conn.path().map(|path| path.to_string())
    } else {
        None
    }
}

pub fn reset_database() -> Result<(), (Connection, rusqlite::Error)> {
    log_debug!("logic::sql_database::reset_database()");

    let result = clean_up();

    let mut connection = CONNECTION.lock().unwrap();
    *connection = open_database();

    result
}

pub fn clean_up() -> Result<(), (Connection, rusqlite::Error)> {
    log_debug!("logic::sql_database::clean_up()");

    let mut connection = CONNECTION.lock().unwrap();

    // Store result for later
    let result = if let Some(conn) = connection.take() {
        conn.close()
    } else {
        Ok(())
    };

    // Set connection to None, no need for it anymore
    *connection = None;

    result
}

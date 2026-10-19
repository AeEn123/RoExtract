use std::io::Read;
use std::{fs, path::PathBuf, sync::Mutex};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::sync::{Arc, LazyLock};

use crate::config;
use crate::dialog;
use crate::locale;
use crate::logic::{self, determine_category};

const DEFAULT_DIRECTORIES: [&str; 2] = [
    "%Temp%\\Roblox",
    "~/.var/app/org.vinegarhq.Sober/cache/sober",
]; // For windows and linux (sober)

static CACHE_DIRECTORY: LazyLock<Mutex<PathBuf>> = LazyLock::new(|| Mutex::new(detect_directory()));

fn create_asset_info_unchecked(
    path: &PathBuf,
    category: logic::Category,
    format: Option<&'static str>,
) -> logic::AssetInfo {
    let name = match path.file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => {
            log_warn!("Failed to get asset info: No filename");
            path.to_string_lossy().to_string()
        }
    };
    let metadata = fs::metadata(path)
        .map_err(|e| log_warn!("Failed to get asset info: {}", e))
        .ok();

    logic::AssetInfo {
        name,
        size: metadata.as_ref().map_or(0, |metadata| metadata.len()),
        last_modified: metadata.as_ref().and_then(|m| m.modified().ok()),
        created: metadata.as_ref().and_then(|m| m.created().ok()),
        format,
        from_file: true,
        from_sql: false,
        from_rbx_storage: false,
        category,
    }
}

fn get_category_cache_directory(category: logic::Category) -> PathBuf {
    let cache_dir = get_cache_directory();
    if category == logic::Category::Music {
        cache_dir.join("sounds") // Music located in /sounds
    } else {
        cache_dir.join("http") // Other stuff located in /http
    }
}

pub fn detect_directory() -> PathBuf {
    let mut errors = "".to_owned();
    if let Some(directory) = config::get_config().get("cache_directory") {
        // User-specified directory from config
        match validate_directory(&directory.to_string().replace('"', "")) {
            // It kept returning "value" instead of value
            Ok(resolved_directory) => return PathBuf::from(resolved_directory),
            Err(e) => {
                log_critical!("Detecting user-specified directory failed: {}", e);
                errors.push_str(&e.to_string());
            }
        }
    }
    // Directory detection
    for directory in DEFAULT_DIRECTORIES {
        match validate_directory(directory) {
            Ok(resolved_directory) => return PathBuf::from(resolved_directory),
            Err(e) => errors.push_str(&e.to_string()),
        }
    }

    // If it was unable to detect any directory, tell the user
    let locale = locale::get_locale(None);
    dialog::alert(
        dialog::Level::Error,
        &locale::get_message(&locale, "error-directory-detection-title", None),
        &locale::get_message(&locale, "error-directory-detection-description", None),
    );

    let yes = dialog::confirm(
        dialog::Level::Error,
        &locale::get_message(&locale, "confirmation-custom-directory-title", None),
        &locale::get_message(&locale, "confirmation-custom-directory-description", None),
    );

    if yes {
        if let Some(path) = dialog::pick_folder() {
            match validate_directory(path.to_string_lossy().as_ref()) {
                Ok(directory) => {
                    config::set_config_value("cache_directory", directory.into());
                    return detect_directory();
                }
                Err(e) => log_critical!("Directory detection failed! {}", e),
            }
        } else {
            log_critical!("Directory detection failed! {}", errors);
        }
    } else {
        log_critical!("Directory detection failed! {}", errors);
    }
    PathBuf::new()
}

pub fn validate_directory(directory: &str) -> Result<String, String> {
    let resolved_directory = logic::resolve_path(directory);

    match fs::metadata(&resolved_directory) {
        // Directory detection
        Ok(metadata) => {
            if metadata.is_dir() {
                // Successfully detected a directory, we can return it
                Ok(resolved_directory)
            } else {
                Err(format!("{resolved_directory}: Not a directory"))
            }
        }
        Err(e) => {
            Err(e.to_string()) // Convert to correct data type
        }
    }
}

pub fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>) {
    let dir = get_cache_directory();

    // Sanity check
    if dir == PathBuf::from("/") || dir == PathBuf::from("") || dir == PathBuf::new() {
        log_error!("Unable to clear cache - cache directory is not acceptable.");
        return;
    }
    assert_ne!(dir, PathBuf::from("/"));
    assert_ne!(dir, PathBuf::from(""));

    // Read directory
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(directory_read) => directory_read.collect(),
        Err(e) => {
            // Abort operation, error occurred
            logic::update_status(locale::get_message(locale, "error-check-logs", None));
            log_error!("Error listing directory: {e}");
            return;
        }
    };

    // Get amount and initialise counter for progress
    let total = entries.len();
    let mut count = 0;

    for entry in entries {
        // Args for formatting
        let mut args = FluentArgs::new();
        args.set("item", count);
        args.set("total", total);

        count += 1; // Increase counter for progress
        logic::update_progress(count as f32 / total as f32); // Convert to f32 to allow floating point output

        // Error checking
        if entry.is_err() {
            log_error!("Failed to delete file: {}: {}", count, entry.unwrap_err());
            logic::update_status(locale::get_message(
                locale,
                "failed-deleting-file",
                Some(&args),
            ));
            continue;
        }
        let path = entry.unwrap().path();

        if path.is_dir() {
            match fs::remove_dir_all(path) {
                // Error handling and update status
                Ok(_) => {
                    logic::update_status(locale::get_message(locale, "deleting-files", Some(&args)))
                }

                // If it's an error, log it and show on GUI
                Err(e) => {
                    log_error!("Failed to delete file: {}: {}", count, e);
                    logic::update_status(locale::get_message(
                        locale,
                        "failed-deleting-file",
                        Some(&args),
                    ));
                }
            }
        } else {
            match fs::remove_file(path) {
                // Error handling and update status
                Ok(_) => {
                    logic::update_status(locale::get_message(locale, "deleting-files", Some(&args)))
                }

                // If it's an error, log it and show on GUI
                Err(e) => {
                    log_error!("Failed to delete file: {}: {}", count, e);
                    logic::update_status(locale::get_message(
                        locale,
                        "failed-deleting-file",
                        Some(&args),
                    ));
                }
            }
        }
    }
}

pub fn refresh(
    category: logic::Category,
    cli_list_mode: bool,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    let dir = get_category_cache_directory(category);

    let headers = logic::get_headers(&category);

    // Read directory
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(directory_read) => directory_read.collect(),
        Err(e) => {
            // Abort operation, error occurred
            logic::update_status(locale::get_message(
                &locale::get_locale(None),
                "error-check-logs",
                None,
            ));
            log_error!("Error listing directory: {e}");
            return;
        }
    };

    // Get amount and initialise counter for progress
    let total = entries.len();
    let mut count = 0;

    // Filter the files out
    for entry in entries {
        if logic::get_stop_list_running() {
            break; // Stop if another thread requests to stop this task.
        }

        count += 1; // Increase counter for progress
        logic::update_progress(count as f32 / total as f32); // Convert to f32 to allow floating point output

        // Args for formatting
        let mut args = FluentArgs::new();
        args.set("item", count);
        args.set("total", total);

        let result = {
            let headers = &headers;
            let category = &category;
            move || -> std::io::Result<()> {
                let path = entry?.path();
                let mut file = fs::File::open(&path)?;

                // Reading the first 2048 bytes of the file
                let mut buffer = vec![0; 2048];
                let bytes_read = file.read(&mut buffer)?;
                buffer.truncate(bytes_read);

                // Decompress asset data so asset detection works with zstd-compressed data.
                let buffer = logic::maybe_decompress(buffer);

                if category == &logic::Category::Music {
                    // Everything in /sounds is music, no need to check headers
                    logic::update_file_list(
                        *category,
                        create_asset_info_unchecked(
                            &path,
                            *category,
                            logic::detect_format(*category, &buffer),
                        ),
                        cli_list_mode,
                    );
                } else {
                    for header in headers {
                        // Check if header is not empty before actually checking file
                        if !header.is_empty() {
                            // Add it to the list if the header is inside of the file.
                            if logic::bytes_contains(&buffer, header.as_bytes()) {
                                let detected_category = if *category == logic::Category::All {
                                    determine_category(&buffer)
                                } else {
                                    *category
                                };
                                let asset_info = create_asset_info_unchecked(
                                    &path,
                                    detected_category,
                                    logic::detect_format(detected_category, &buffer),
                                );
                                logic::update_file_list(*category, asset_info, cli_list_mode);
                                break; // Stop after the first match so a file isn't listed twice
                            }
                        }
                    }
                }

                Ok(())
            }
        }();
        match result {
            Ok(()) => {
                logic::update_status(locale::get_message(locale, "filtering-files", Some(&args)));
            }
            Err(e) => {
                log_error!("Couldn't open file: {}", e);
                logic::update_status(locale::get_message(
                    locale,
                    "failed-opening-file",
                    Some(&args),
                ));
            }
        }
    }
}

pub fn read_asset(asset: &logic::AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    let dir = get_category_cache_directory(asset.category);
    let asset_path = dir.join(&asset.name);
    fs::read(asset_path).map(logic::maybe_decompress)
}

pub fn swap_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir_a = get_category_cache_directory(asset_a.category);
    let dir_b = get_category_cache_directory(asset_b.category);

    let asset_a_path = dir_a.join(&asset_a.name);
    let asset_b_path = dir_b.join(&asset_b.name);

    let asset_a_bytes = fs::read(&asset_a_path)?;
    let asset_b_bytes = fs::read(&asset_b_path)?;

    fs::write(&asset_a_path, asset_b_bytes)?;
    fs::write(&asset_b_path, asset_a_bytes)?;
    Ok(())
}

pub fn copy_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir_a = get_category_cache_directory(asset_a.category);
    let dir_b = get_category_cache_directory(asset_b.category);

    let asset_a_path = dir_a.join(&asset_a.name);
    let asset_b_path = dir_b.join(&asset_b.name);

    let asset_a_bytes = fs::read(&asset_a_path)?;
    fs::write(&asset_b_path, asset_a_bytes)?;
    Ok(())
}

pub fn get_cache_directory() -> PathBuf {
    CACHE_DIRECTORY.lock().unwrap().clone()
}

pub fn set_cache_directory(value: PathBuf) {
    let mut cache_directory = CACHE_DIRECTORY.lock().unwrap();
    *cache_directory = value;
}

pub fn create_asset_info(asset: &str, category: logic::Category) -> Option<logic::AssetInfo> {
    let path = get_category_cache_directory(category).join(asset);

    if path.exists() {
        Some(create_asset_info_unchecked(&path, category, None))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;

    #[test]
    fn test_validate_directory() {
        let temp_dir = env::temp_dir();
        assert!(validate_directory(temp_dir.to_str().unwrap()).is_ok());

        let mut invalid_path = temp_dir.clone();
        invalid_path.push("roextract_nonexistent_dir");
        if invalid_path.exists() {
            fs::remove_dir_all(&invalid_path).unwrap();
        }
        assert!(validate_directory(invalid_path.to_str().unwrap()).is_err());

        let mut file_path = temp_dir.clone();
        file_path.push("roextract_test_file");
        File::create(&file_path).unwrap();
        assert!(validate_directory(file_path.to_str().unwrap()).is_err()); // It's a file, not a directory

        fs::remove_file(&file_path).unwrap();
    }
}

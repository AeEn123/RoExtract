use std::io::Read;
use std::{path::PathBuf, sync::Mutex, fs};

use std::sync::{Arc, LazyLock};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};

use crate::logic::{self, determine_category};
use crate::config;
use crate::locale;

const DEFAULT_DIRECTORIES: [&str; 2] = ["%Temp%\\Roblox", "~/.var/app/org.vinegarhq.Sober/cache/sober"]; // For windows and linux (sober)

static CACHE_DIRECTORY: LazyLock<Mutex<PathBuf>> = LazyLock::new(||Mutex::new(detect_directory()));

fn create_asset_info_unchecked(path: &PathBuf, category: logic::Category) -> logic::AssetInfo {
    match path.file_name() {
        Some(file_name) => {
            match fs::metadata(path) {
                Ok(metadata) => {
                    let size = metadata.len();
                    let last_modified = match metadata.modified() {
                        Ok(system_time) => Some(system_time),
                        Err(_) => None
                    };

                    return logic::AssetInfo {
                        name: file_name.to_string_lossy().to_string(),
                        size: size,
                        last_modified: last_modified,
                        from_file: true,
                        from_sql: false,
                        category: category,
                    }
                }
                Err(e) => {
                    log_warn!("Failed to get asset info: {}", e);
                    return logic::AssetInfo {
                        name: file_name.to_string_lossy().to_string(),
                        size: 0,
                        last_modified: None,
                        from_file: true,
                        from_sql: false,
                        category: category,
                    }
                }
            }
        }
        None => {
            log_warn!("Failed to get asset info: No filename");
            return logic::AssetInfo {
                name: path.to_string_lossy().to_string(),
                size: 0,
                last_modified: None,
                from_file: true,
                from_sql: false,
                category: category
            }
        }
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
        match validate_directory(&directory.to_string().replace('"',"")) { // It kept returning "value" instead of value
            Ok(resolved_directory) => return PathBuf::from(resolved_directory),
            Err(e) => {
                errors.push_str(&e.to_string());
            },
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
    let _ = native_dialog::DialogBuilder::message()
    .set_level(native_dialog::MessageLevel::Error)
    .set_title(&locale::get_message(&locale::get_locale(None), "error-directory-detection-title", None))
    .set_text(&locale::get_message(&locale::get_locale(None), "error-directory-detection-description", None))
    .alert().show();

    let yes = native_dialog::DialogBuilder::message()
    .set_level(native_dialog::MessageLevel::Error)
    .set_title(&locale::get_message(&locale::get_locale(None), "confirmation-custom-directory-title", None))
    .set_text(&locale::get_message(&locale::get_locale(None), "confirmation-custom-directory-description", None))
    .confirm().show()
    .unwrap();

    if yes {
        let option_path = native_dialog::DialogBuilder::file()
        .open_single_dir().show()
        .unwrap();
        if let Some(path) = option_path {
            config::set_config_value("cache_directory", validate_directory(&path.to_string_lossy().to_string()).unwrap().into());
            return detect_directory();
        } else {
            log_critical!("Directory detection failed! {}", errors);
        }
    } else {
        log_critical!("Directory detection failed! {}", errors);
    }
    return PathBuf::new();
}

pub fn validate_directory(directory: &str) -> Result<String, String> {
    let resolved_directory = logic::resolve_path(directory);

    match fs::metadata(&resolved_directory) { // Directory detection
        Ok(metadata) => {
            if metadata.is_dir() {
                // Successfully detected a directory, we can return it
                return Ok(resolved_directory);
            } else {
                return Err(format!("{}: Not a directory", resolved_directory));
            }
        }
        Err(e) => {
            return Err(e.to_string()); // Convert to correct data type
        }
    }
}

pub fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>) {
    let dir = get_cache_directory();

    // Sanity check
    assert_ne!(dir, PathBuf::from("/"));
    assert_ne!(dir, PathBuf::from(""));


    // Read directory
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(directory_read) => directory_read.collect(),
        Err(e) => {
            // Abort operation, error occurred
            logic::update_status(locale::get_message(&locale, "error-check-logs", None)); 
            log_error!("Error listing directory: {e}");
            return
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
        logic::update_progress(count as f32/total as f32); // Convert to f32 to allow floating point output

        // Error checking
        if entry.is_err() {
            log_error!("Failed to delete file: {}: {}", count, entry.unwrap_err());
            logic::update_status(locale::get_message(&locale, "failed-deleting-file", Some(&args)));
            continue;
        }
        let path = entry.unwrap().path();

        if path.is_dir() {
            match fs::remove_dir_all(path) {
                // Error handling and update status
                Ok(_) => logic::update_status(locale::get_message(&locale, "deleting-files", Some(&args))),

                // If it's an error, log it and show on GUI
                Err(e) => {
                    log_error!("Failed to delete file: {}: {}", count, e);
                    logic::update_status(locale::get_message(&locale, "failed-deleting-file", Some(&args)));
                }
            }
        } else {
            match fs::remove_file(path) {
                // Error handling and update status
                Ok(_) => logic::update_status(locale::get_message(&locale, "deleting-files", Some(&args))),

                // If it's an error, log it and show on GUI
                Err(e) => {
                    log_error!("Failed to delete file: {}: {}", count, e);
                    logic::update_status(locale::get_message(&locale, "failed-deleting-file", Some(&args)));
                }
            }    
        }
    }
}

pub fn refresh(category: logic::Category, cli_list_mode: bool, locale: &FluentBundle<Arc<FluentResource>>) {
    let dir = get_category_cache_directory(category);

    let headers = logic::get_headers(&category);

    // Read directory
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(directory_read) => directory_read.collect(),
        Err(e) => {
            // Abort operation, error occurred
            logic::update_status(locale::get_message(&locale::get_locale(None), "error-check-logs", None)); 
            log_error!("Error listing directory: {e}");
            return
        }
    };

    // Get amount and initialise counter for progress
    let total = entries.len();
    let mut count = 0;

    // Tell the user that there is no files to list to make it easy to tell that the program is working and it isn't broken
    if total == 0 {
        logic::update_file_list(logic::create_no_files(&locale), cli_list_mode);
    }
        // Filter the files out
        for entry in entries {
            if logic::get_stop_list_running() {
                break // Stop if another thread requests to stop this task.
            }
            
            count += 1; // Increase counter for progress
            logic::update_progress(count as f32/total as f32); // Convert to f32 to allow floating point output

            // Args for formatting
            let mut args = FluentArgs::new();
            args.set("item", count);
            args.set("total", total);

            let result = {
                let headers = &headers;
                let category = &category;
                move || -> std::io::Result<()> {
                let path = entry?.path();

                if category == &logic::Category::Music {
                    logic::update_file_list(create_asset_info_unchecked(&path, *category), cli_list_mode);
                } else {
                    let mut file = fs::File::open(&path)?;
                
                    // Reading the first 2048 bytes of the file
                    let mut buffer = vec![0; 2048];
                    let bytes_read = file.read(&mut buffer)?;
                    buffer.truncate(bytes_read);

                    for header in headers {
                        // Check if header is not empty before actually checking file
                        if header != "" {
                            // Add it to the list if the header is inside of the file.
                            if logic::bytes_contains(&buffer, header.as_bytes()) {  
                                let asset_info = if *category == logic::Category::All {
                                    create_asset_info_unchecked(&path, determine_category(&buffer))
                                } else {
                                    create_asset_info_unchecked(&path, *category)
                                };
                                logic::update_file_list(asset_info, cli_list_mode);
                            }
                        }
                    }    
                }


                Ok(())
            }}();
            match result {
                Ok(()) => {
                    logic::update_status(locale::get_message(&locale, "filtering-files", Some(&args)));
                }
                Err(e) => {
                    log_error!("Couldn't open file: {}", e);
                    logic::update_status(locale::get_message(&locale, "failed-opening-file", Some(&args)));
                }
            }
        }
}

pub fn read_asset(asset: &logic::AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    let dir = get_category_cache_directory(asset.category.clone());
    let asset_path = dir.join(&asset.name);
    Ok(fs::read(asset_path)?)
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
        return Some(create_asset_info_unchecked(&path, category))
    } else {
        return None
    }
    
}
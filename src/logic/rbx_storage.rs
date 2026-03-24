use std::io::Read;
use std::{fs, path::PathBuf, sync::Mutex};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::sync::{Arc, LazyLock};

use crate::locale;
use crate::logic::{self};

const DEFAULT_RBX_STORAGE_DIRECTORIES: [&str; 2] = [
    "%localappdata%\\Roblox\\rbx-storage",
    "~/.var/app/org.vinegarhq.Sober/data/sober/appData/rbx-storage",
]; // For windows and linux (sober)

static RBX_STORAGE_DIRECTORY: LazyLock<Mutex<Option<PathBuf>>> =
    LazyLock::new(|| Mutex::new(detect_directory()));

fn create_asset_info_unchecked(path: &PathBuf, category: logic::Category) -> logic::AssetInfo {
    match path.file_name() {
        Some(file_name) => match fs::metadata(path) {
            Ok(metadata) => {
                let size = metadata.len();
                let last_modified = metadata.modified().ok();

                logic::AssetInfo {
                    name: file_name.to_string_lossy().to_string(),
                    _size: size,
                    last_modified,
                    source: logic::AssetSource::RbxStorage,
                    category,
                }
            }
            Err(e) => {
                log_warn!("Failed to get asset info: {}", e);
                logic::AssetInfo {
                    name: file_name.to_string_lossy().to_string(),
                    _size: 0,
                    last_modified: None,
                    source: logic::AssetSource::RbxStorage,
                    category,
                }
            }
        },
        None => {
            log_warn!("Failed to get asset info: No filename");
            logic::AssetInfo {
                name: path.to_string_lossy().to_string(),
                _size: 0,
                last_modified: None,
                source: logic::AssetSource::RbxStorage,
                category,
            }
        }
    }
}

fn detect_directory() -> Option<PathBuf> {
    for directory in DEFAULT_RBX_STORAGE_DIRECTORIES {
        match validate_directory(directory) {
            Ok(resolved_directory) => return Some(PathBuf::from(resolved_directory)),
            Err(_) => continue,
        }
    }
    None
}

fn validate_directory(directory: &str) -> Result<String, String> {
    let resolved_directory = logic::resolve_path(directory);

    match fs::metadata(&resolved_directory) {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(resolved_directory)
            } else {
                Err(format!("{resolved_directory}: Not a directory"))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_directory() -> Option<PathBuf> {
    RBX_STORAGE_DIRECTORY.lock().unwrap().clone()
}

pub fn set_directory(value: PathBuf) {
    let mut directory = RBX_STORAGE_DIRECTORY.lock().unwrap();
    *directory = Some(value);
}

pub fn clear_cache(locale: &FluentBundle<Arc<FluentResource>>) {
    let dir = match get_directory() {
        Some(dir) => dir,
        None => {
            log_error!("Unable to clear rbx-storage - directory not found.");
            return;
        }
    };

    // Sanity check
    if dir == PathBuf::from("/") || dir == PathBuf::from("") {
        log_error!("Unable to clear rbx-storage - directory is not acceptable.");
        return;
    }

    // Read subdirectories
    let subdirs: Vec<_> = match fs::read_dir(&dir) {
        Ok(dirs) => dirs.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            log_error!("Error reading rbx-storage directory: {e}");
            return;
        }
    };

    let total = subdirs.len();
    let mut count = 0;

    for subdir_entry in subdirs {
        let subdir_path = subdir_entry.path();

        let mut args = FluentArgs::new();
        args.set("item", count);
        args.set("total", total);

        count += 1;
        logic::update_progress(count as f32 / total as f32);

        if subdir_path.is_dir() {
            match fs::remove_dir_all(&subdir_path) {
                Ok(_) => {
                    logic::update_status(locale::get_message(locale, "deleting-files", Some(&args)))
                }
                Err(e) => {
                    log_error!("Failed to delete rbx-storage subdirectory: {}: {}", count, e);
                    logic::update_status(locale::get_message(
                        locale,
                        "failed-deleting-file",
                        Some(&args),
                    ));
                }
            }
        }
    }

    // Finally remove the rbx-storage directory itself
    match fs::remove_dir_all(&dir) {
        Ok(_) => {
            logic::update_status(locale::get_message(locale, "deleted-files", None));
        }
        Err(e) => {
            log_error!("Failed to delete rbx-storage directory: {}", e);
            let mut args = FluentArgs::new();
            args.set("error", e.to_string());
            logic::update_status(locale::get_message(
                locale,
                "failed-deleting-file",
                Some(&args),
            ));
        }
    }
}

pub fn refresh(
    category: logic::Category,
    cli_list_mode: bool,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    // Only handle Music category for rbx-storage
    if category != logic::Category::Music {
        return;
    }

    let rbx_storage_dir = match get_directory() {
        Some(dir) => dir,
        None => return, // rbx-storage directory not found, skip
    };

    let headers = logic::get_headers(&logic::Category::Music);

    // Collect all files from sharded subdirectories
    let mut all_entries = Vec::new();

    // Read the rbx-storage directory (which contains subdirectories)
    let subdirs: Vec<_> = match fs::read_dir(&rbx_storage_dir) {
        Ok(dirs) => dirs.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            log_error!("Error reading rbx-storage directory: {e}");
            return;
        }
    };

    // Go through each subdirectory and collect files
    for subdir_entry in subdirs {
        let subdir_path = subdir_entry.path();
        if subdir_path.is_dir() {
            if let Ok(files) = fs::read_dir(&subdir_path) {
                all_entries.extend(files.filter_map(|e| e.ok()));
            }
        }
    }

    let total = all_entries.len();
    let mut count = 0;

    if total == 0 {
        return; // No files in rbx-storage
    }

    for entry in all_entries {
        if logic::get_stop_list_running() {
            break;
        }

        count += 1;
        logic::update_progress(count as f32 / total as f32);

        let mut args = FluentArgs::new();
        args.set("item", count);
        args.set("total", total);

        let path = entry.path();
        let result = {
            let headers = &headers;
            move || -> std::io::Result<()> {
                // For rbx-storage, we assume all files are potential audio files
                // and check their headers
                let mut file = fs::File::open(&path)?;
                let mut buffer = vec![0; 2048];
                let bytes_read = file.read(&mut buffer)?;
                buffer.truncate(bytes_read);

                // Decompress if needed (zstd or RBXH wrapper)
                let buffer = logic::maybe_decompress(buffer);

                for header in headers {
                    if !header.is_empty() && logic::bytes_contains(&buffer, header.as_bytes()) {
                        logic::update_file_list(
                            create_asset_info_unchecked(&path, logic::Category::Music),
                            cli_list_mode,
                        );
                        break;
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
                log_error!("Couldn't open rbx-storage file: {}", e);
            }
        }
    }
}

pub fn read_asset(asset: &logic::AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    // The asset name in rbx-storage is the hex-encoded filename
    // Files are sharded into subdirectories based on first 2 hex characters
    if asset.name.len() < 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Asset name too short for rbx-storage lookup",
        ));
    }

    let rbx_storage_dir = get_directory().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    let shard = &asset.name[0..2];
    let rbx_asset_path = rbx_storage_dir.join(shard).join(&asset.name);

    if rbx_asset_path.exists() {
        fs::read(rbx_asset_path).map(logic::maybe_decompress)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Asset '{}' not found in rbx-storage", asset.name),
        ))
    }
}

pub fn swap_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    // Both assets must be from rbx-storage
    if asset_a.source != logic::AssetSource::RbxStorage
        || asset_b.source != logic::AssetSource::RbxStorage
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Both assets must be from rbx-storage for swap",
        ));
    }

    let rbx_storage_dir = get_directory().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    let shard_a = &asset_a.name[0..2];
    let shard_b = &asset_b.name[0..2];

    let asset_a_path = rbx_storage_dir.join(shard_a).join(&asset_a.name);
    let asset_b_path = rbx_storage_dir.join(shard_b).join(&asset_b.name);

    let asset_a_bytes = fs::read(&asset_a_path)?;
    let asset_b_bytes = fs::read(&asset_b_path)?;

    fs::write(&asset_a_path, asset_b_bytes)?;
    fs::write(&asset_b_path, asset_a_bytes)?;
    Ok(())
}

pub fn copy_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    // Both assets must be from rbx-storage
    if asset_a.source != logic::AssetSource::RbxStorage
        || asset_b.source != logic::AssetSource::RbxStorage
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Both assets must be from rbx-storage for copy",
        ));
    }

    let rbx_storage_dir = get_directory().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    let shard_a = &asset_a.name[0..2];
    let shard_b = &asset_b.name[0..2];

    let asset_a_path = rbx_storage_dir.join(shard_a).join(&asset_a.name);
    let asset_b_path = rbx_storage_dir.join(shard_b).join(&asset_b.name);

    let asset_a_bytes = fs::read(&asset_a_path)?;
    fs::write(&asset_b_path, asset_a_bytes)?;
    Ok(())
}

pub fn create_asset_info(asset: &str, category: logic::Category) -> Option<logic::AssetInfo> {
    let rbx_storage_dir = get_directory()?;

    // The asset name in rbx-storage is the hex-encoded filename
    // Files are sharded into subdirectories based on first 2 hex characters
    if asset.len() < 2 {
        return None;
    }

    let shard = &asset[0..2];
    let path = rbx_storage_dir.join(shard).join(asset);

    if path.exists() {
        Some(create_asset_info_unchecked(&path, category))
    } else {
        None
    }
}

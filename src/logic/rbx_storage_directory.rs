use std::io::Read;
use std::{fs, path::PathBuf};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::sync::{Arc, LazyLock, Mutex};

use crate::logic::{self, determine_category};
use crate::{config, locale};

/// Default rbx-storage directory paths for each platform.
const DEFAULT_DIRECTORIES: [&str; 2] = [
    "%localappdata%\\Roblox\\rbx-storage",
    "~/.var/app/org.vinegarhq.Sober/cache/sober/rbx-storage",
];

static RBX_STORAGE_DIR: LazyLock<Mutex<Option<PathBuf>>> =
    LazyLock::new(|| Mutex::new(detect_directory()));

pub fn detect_directory() -> Option<PathBuf> {
    // User-specified path from config takes priority
    if let Some(path) = config::get_config_string("rbx_storage_directory") {
        let dir = PathBuf::from(&path);
        if dir.is_dir() {
            return Some(dir);
        }
        log_warn!(
            "User-specified rbx_storage_directory does not exist: {}",
            path
        );
    }

    // Try hardcoded defaults
    for default in DEFAULT_DIRECTORIES {
        let resolved = logic::resolve_path(default);
        let dir = PathBuf::from(&resolved);
        if dir.is_dir() {
            return Some(dir);
        }
    }

    None
}

pub fn get_rbx_storage_dir() -> Option<PathBuf> {
    RBX_STORAGE_DIR.lock().unwrap().clone()
}

pub fn set_rbx_storage_dir(value: Option<PathBuf>) {
    let mut dir = RBX_STORAGE_DIR.lock().unwrap();
    *dir = value;
}

/// Given a hash filename, derive its two-character subdirectory and return the full path.
/// e.g. "defcd8fbdc641282f02ab5e35c8b059f" → `<rbx-storage>/de/defcd8fbdc641282f02ab5e35c8b059f`
fn asset_path(dir: &PathBuf, hash: &str) -> PathBuf {
    let subdir = &hash[..2.min(hash.len())];
    dir.join(subdir).join(hash)
}

fn create_asset_info(hash: String, path: &PathBuf, category: logic::Category) -> logic::AssetInfo {
    let (size, last_modified) = match fs::metadata(path) {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(_) => (0, None),
    };

    logic::AssetInfo {
        name: hash,
        _size: size,
        last_modified,
        from_file: false,
        from_sql: false,
        from_rbx_storage: true,
        category,
    }
}

pub fn refresh(
    category: logic::Category,
    cli_list_mode: bool,
    locale: &FluentBundle<Arc<FluentResource>>,
) {
    if category == logic::Category::Music {
        return; // Music category is specific to /sounds folder.
    }

    let dir = match get_rbx_storage_dir() {
        Some(d) => d,
        None => {
            log_info!("rbx-storage directory not found, skipping.");
            return;
        }
    };

    let headers = logic::get_headers(&category);

    // Collect subdirectories (named by first two hex chars of hash)
    let subdirs: Vec<_> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect(),
        Err(e) => {
            log_error!("Error reading rbx-storage directory: {e}");
            logic::update_status(locale::get_message(
                &locale::get_locale(None),
                "error-check-logs",
                None,
            ));
            return;
        }
    };

    // Flatten all files — store only the hash filename as the asset name
    let mut all_files: Vec<(PathBuf, String)> = Vec::new();
    for subdir in &subdirs {
        if let Ok(entries) = fs::read_dir(subdir.path()) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    // Store just the filename (hash) — subdir is derived from first 2 chars
                    let hash = entry.file_name().to_string_lossy().to_string();
                    all_files.push((path, hash));
                }
            }
        }
    }

    let total = all_files.len();
    let mut count = 0;

    if total == 0 {
        return;
    }

    for (path, hash) in all_files {
        if logic::get_stop_list_running() {
            break;
        }

        count += 1;
        logic::update_progress(count as f32 / total as f32);

        let mut args = FluentArgs::new();
        args.set("item", count);
        args.set("total", total);

        let result = (|| -> std::io::Result<()> {
            let mut file = fs::File::open(&path)?;

            let mut buffer = vec![0u8; 2048];
            let bytes_read = file.read(&mut buffer)?;
            buffer.truncate(bytes_read);

            let buffer = logic::maybe_decompress(buffer);

            for header in &headers {
                if !header.is_empty() && logic::bytes_contains(&buffer, header.as_bytes()) {
                    let detected_category = if category == logic::Category::All {
                        determine_category(&buffer)
                    } else {
                        category
                    };
                    let asset_info = create_asset_info(hash.clone(), &path, detected_category);
                    logic::update_file_list(asset_info, cli_list_mode);
                    break;
                }
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                logic::update_status(locale::get_message(locale, "filtering-files", Some(&args)));
            }
            Err(e) => {
                log_error!("Couldn't open file in rbx-storage: {}", e);
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
    let dir = get_rbx_storage_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "rbx-storage directory not found",
        )
    })?;

    let asset_path = asset_path(&dir, &asset.name);
    fs::read(asset_path).map(logic::maybe_decompress)
}

pub fn swap_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir = get_rbx_storage_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "rbx-storage directory not found",
        )
    })?;

    let path_a = asset_path(&dir, &asset_a.name);
    let path_b = asset_path(&dir, &asset_b.name);

    let bytes_a = fs::read(&path_a)?;
    let bytes_b = fs::read(&path_b)?;

    fs::write(&path_a, &bytes_b)?;
    fs::write(&path_b, &bytes_a)?;
    Ok(())
}

pub fn copy_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir = get_rbx_storage_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "rbx-storage directory not found",
        )
    })?;

    let path_a = asset_path(&dir, &asset_a.name);
    let path_b = asset_path(&dir, &asset_b.name);

    let bytes_a = fs::read(&path_a)?;
    fs::write(&path_b, &bytes_a)?;
    Ok(())
}

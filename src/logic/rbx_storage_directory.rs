use std::io::Read;
use std::{fs, path::PathBuf};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::sync::Arc;

use crate::locale;
use crate::logic::{self, determine_category};

/// Default rbx-storage directory paths for each platform.
/// These are tried when the DB-derived path doesn't exist (e.g. Linux/Sober,
/// where the DB lives under `data/` but the cache dir is under `cache/`).
const DEFAULT_DIRECTORIES: [&str; 2] = [
    "%localappdata%\\Roblox\\rbx-storage",
    "~/.var/app/org.vinegarhq.Sober/cache/sober/rbx-storage",
];

/// Find the rbx-storage directory.
/// First tries to derive it from the SQL database path (works on Windows).
/// Falls back to the hardcoded default list (needed on Linux/Sober).
pub fn get_rbx_storage_dir() -> Option<PathBuf> {
    // Try deriving from the DB path first (works for Windows)
    if let Some(db_path) = logic::sql_database::get_db_path() {
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            let dir = parent.join("rbx-storage");
            if dir.is_dir() {
                return Some(dir);
            }
        }
    }

    // Fall back to hardcoded defaults (needed for Linux/Sober)
    for default in DEFAULT_DIRECTORIES {
        let resolved = logic::resolve_path(default);
        let dir = PathBuf::from(&resolved);
        if dir.is_dir() {
            return Some(dir);
        }
    }

    None
}

fn create_asset_info(
    relative_name: String,
    path: &PathBuf,
    category: logic::Category,
) -> logic::AssetInfo {
    let (size, last_modified) = match fs::metadata(path) {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(_) => (0, None),
    };

    logic::AssetInfo {
        name: relative_name,
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
    let dir = match get_rbx_storage_dir() {
        Some(d) => d,
        None => {
            log_debug!("rbx-storage directory not found, skipping.");
            return;
        }
    };

    let headers = logic::get_headers(&category);

    // Collect all files (one level of subdirectories: `rbx-storage/{2hex}/{hash}`)
    let subdirs: Vec<_> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).collect(),
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

    // Flatten all files across subdirs
    let mut all_files: Vec<(PathBuf, String)> = Vec::new();
    for subdir in &subdirs {
        let subdir_name = subdir.file_name().to_string_lossy().to_string();
        if let Ok(entries) = fs::read_dir(subdir.path()) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    // Relative name: "ab/abcdef123..." so we can reconstruct path later
                    let relative_name = format!("{}/{}", subdir_name, file_name);
                    all_files.push((path, relative_name));
                }
            }
        }
    }

    let total = all_files.len();
    let mut count = 0;

    if total == 0 {
        return;
    }

    for (path, relative_name) in all_files {
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
                    let asset_info = create_asset_info(relative_name.clone(), &path, detected_category);
                    logic::update_file_list(asset_info, cli_list_mode);
                    break; // Only add once per file
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
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    // asset.name is "ab/abcdef123..."
    let asset_path = dir.join(&asset.name);
    fs::read(asset_path).map(logic::maybe_decompress)
}

pub fn swap_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir = get_rbx_storage_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    let path_a = dir.join(&asset_a.name);
    let path_b = dir.join(&asset_b.name);

    let bytes_a = fs::read(&path_a)?;
    let bytes_b = fs::read(&path_b)?;

    fs::write(&path_a, &bytes_b)?;
    fs::write(&path_b, &bytes_a)?;
    Ok(())
}

pub fn copy_assets(asset_a: &logic::AssetInfo, asset_b: &logic::AssetInfo) -> std::io::Result<()> {
    let dir = get_rbx_storage_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "rbx-storage directory not found")
    })?;

    let path_a = dir.join(&asset_a.name);
    let path_b = dir.join(&asset_b.name);

    let bytes_a = fs::read(&path_a)?;
    fs::write(&path_b, &bytes_a)?;
    Ok(())
}

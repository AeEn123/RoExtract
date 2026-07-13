use std::{
    env, fs,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::SystemTime,
};

use clap::ValueEnum;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};

use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

use crate::{config, locale};

pub mod cache_directory;
pub mod rbx_storage_directory;
pub mod sql_database;

static TEMP_DIRECTORY: LazyLock<Mutex<PathBuf>> = LazyLock::new(|| Mutex::new(create_temp_dir()));

// Define global values
static STATUS: LazyLock<Mutex<String>> = LazyLock::new(|| {
    Mutex::new(locale::get_message(
        &locale::get_locale(None),
        "idling",
        None,
    ))
});
// Asset lists are `Arc<Vec<AssetInfo>>` so the GUI can snapshot them per-frame
// with a refcount bump instead of a deep clone; writers use `Arc::make_mut` for
// copy-on-write (clones only when a reader holds the previous snapshot).
static FILE_LIST: LazyLock<Mutex<Arc<Vec<AssetInfo>>>> =
    LazyLock::new(|| Mutex::new(Arc::new(Vec::new())));
static REQUEST_REPAINT: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static PROGRESS: LazyLock<Mutex<f32>> = LazyLock::new(|| Mutex::new(1.0));
static LIST_TASK_RUNNING: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static STOP_LIST_RUNNING: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static FILTERED_FILE_LIST: LazyLock<Mutex<Arc<Vec<AssetInfo>>>> =
    LazyLock::new(|| Mutex::new(Arc::new(Vec::new())));
static TASK_RUNNING: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false)); // Delete/extract

// CLI stuff
#[derive(ValueEnum, Clone, Debug, Eq, PartialEq, Hash, Copy, EnumIter, Display)]
pub enum Category {
    Music,
    Sounds,
    Images,
    Ktx,
    Rbxm,
    All,
}

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub name: String,
    pub _size: u64,
    pub last_modified: Option<SystemTime>,
    pub from_file: bool,
    pub from_sql: bool,
    pub from_rbx_storage: bool,
    pub category: Category,
}

// Define local functions
fn update_file_list(value: AssetInfo, cli_list_mode: bool) {
    // cli_list_mode will print out to console
    // It is done this way so it can read files and print to console in the same stage
    if cli_list_mode {
        println!("{}", value.name);
    }
    let mut file_list = FILE_LIST.lock().unwrap();
    // make_mut clones only if a snapshot is still being read elsewhere.
    Arc::make_mut(&mut file_list).push(value)
}

fn clear_file_list() {
    let mut file_list = FILE_LIST.lock().unwrap();
    *file_list = Arc::new(Vec::new())
}

/// Zstd magic bytes: 0xFD2FB528 (little-endian)
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// If `bytes` starts with the zstd magic number, decompress and return the
/// decompressed data.  Otherwise return `bytes` unchanged.
pub fn maybe_decompress(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.starts_with(&ZSTD_MAGIC) {
        match zstd::decode_all(bytes.as_slice()) {
            Ok(decompressed) => {
                log_debug!(
                    "Decompressed zstd-compressed cache file ({} → {} bytes)",
                    bytes.len(),
                    decompressed.len()
                );
                decompressed
            }
            Err(e) => {
                log_warn!("Failed to decompress zstd data, using raw bytes: {}", e);
                bytes
            }
        }
    } else {
        bytes
    }
}

fn bytes_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let len = needle.len();
    if len > 0 {
        haystack.windows(len).position(|window| window == needle)
    } else {
        None
    }
}

fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
    let len = needle.len();
    if len > 0 {
        haystack.windows(len).any(|window| window == needle)
    } else {
        false
    }
}

fn find_header(category: Category, bytes: &[u8]) -> Result<String, String> {
    // Get the header for the current category
    let headers = get_headers(&category);

    // iterate through headers to find the correct one for this file.
    for header in headers {
        if bytes_contains(bytes, header.as_bytes()) {
            return Ok(header.to_owned());
        }
    }
    Err("Headers not found in bytes".to_owned())
}

fn extract_bytes(header: &str, bytes: Vec<u8>) -> Vec<u8> {
    // Set offset depending on header
    let offset: usize = match header {
        "PNG" => 1,
        "KTX" => 1,
        "WEBP" => 8,
        _ => 0,
    };

    // Find the header in the file
    if let Some(index) = bytes_search(&bytes, header.as_bytes()) {
        // Apply the offset; saturate so a near-start header can't underflow.
        let index = index.saturating_sub(offset);
        return bytes[index..].to_vec();
    }
    log_warn!("Failed to extract a file!");
    // Return bytes instead if this fails
    bytes
}

fn create_no_files(locale: &FluentBundle<Arc<FluentResource>>) -> AssetInfo {
    AssetInfo {
        name: locale::get_message(locale, "no-files", None),
        _size: 0,
        last_modified: None,
        from_file: false,
        from_sql: false,
        from_rbx_storage: false,
        category: Category::All,
    }
}

fn read_asset(asset: &AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    if asset.from_file {
        cache_directory::read_asset(asset)
    } else if asset.from_sql {
        sql_database::read_asset(asset)
    } else if asset.from_rbx_storage {
        rbx_storage_directory::read_asset(asset)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Not from_file, from_sql, or from_rbx_storage",
        ))
    }
}

// Create temporary directory
pub fn create_temp_dir() -> PathBuf {
    let path = match config::get_system_config_string("temp-directory") {
        Some(dir) => PathBuf::from(dir),
        None => env::temp_dir().join("RoExtract"),
    };

    match fs::create_dir(&path) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                log_critical!("Failed to create temporary directory: {}", e);
            }
        }
    }

    path
}

// Define public functions
pub fn resolve_path(directory: &str) -> String {
    // There's probably a better way of doing this... It works though :D
    let resolved_path = directory
        .replace(
            "%Temp%",
            &format!("C:\\Users\\{}\\AppData\\Local\\Temp", whoami::username()),
        )
        .replace(
            "%localappdata%",
            &format!("C:\\Users\\{}\\AppData\\Local", whoami::username()),
        )
        .replace("~", &format!("/home/{}", whoami::username()));

    resolved_path
}

// Function to get temp directory, create it if it doesn't exist
pub fn get_temp_dir() -> PathBuf {
    return TEMP_DIRECTORY.lock().unwrap().clone();
}

pub fn clear_cache() {
    let running = {
        let task = TASK_RUNNING.lock().unwrap();
        *task
    };
    // Stop multiple threads from running
    if !running {
        thread::spawn(move || {
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = true; // Stop other threads from running
            }
            // Get locale for localised status messages
            let locale = locale::get_locale(None);

            sql_database::clear_cache(&locale);
            cache_directory::clear_cache(&locale);

            // Clear the file list for visual feedback to the user that the files are actually deleted
            clear_file_list();

            update_file_list(create_no_files(&locale), false);
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = false; // Allow other threads to run again
            }
            update_status(locale::get_message(&locale, "idling", None)); // Set the status back
        });
    }
}

pub fn refresh(category: Category, cli_list_mode: bool, yield_for_thread: bool) {
    // Get headers for use later
    let handle = thread::spawn(move || {
        // Get locale for localised status messages
        let locale = locale::get_locale(None);
        // This loop here is to make it wait until it is not running, and to set the STOP_LIST_RUNNING to true if it is running to make the other thread
        loop {
            let running = {
                let task = LIST_TASK_RUNNING.lock().unwrap();
                *task
            };
            if !running {
                break; // Break if not running
            } else {
                let mut stop = STOP_LIST_RUNNING.lock().unwrap(); // Tell the other thread to stop
                *stop = true;
            }
            thread::sleep(std::time::Duration::from_millis(10)); // Sleep for a bit to not be CPU intensive
        }
        {
            let mut task = LIST_TASK_RUNNING.lock().unwrap();
            *task = true; // Tell other threads that a task is running
            let mut stop = STOP_LIST_RUNNING.lock().unwrap();
            *stop = false; // Disable the stop, otherwise this thread will stop!
        }

        clear_file_list(); // Only list the files on the current tab

        sql_database::refresh(category, cli_list_mode, &locale);
        cache_directory::refresh(category, cli_list_mode, &locale);
        rbx_storage_directory::refresh(category, cli_list_mode, &locale);

        {
            let mut task = LIST_TASK_RUNNING.lock().unwrap();
            *task = false; // Allow other threads to run again
        }
        update_status(locale::get_message(&locale, "idling", None)); // Set the status back
    });

    if yield_for_thread {
        // Will wait for the thread instead of quitting immediately
        let _ = handle.join();
    }
}

pub fn extract_to_file(
    asset: AssetInfo,
    destination: PathBuf,
    add_extension: bool,
) -> Result<PathBuf, std::io::Error> {
    let mut destination = destination.clone(); // Get own mutable destination

    let bytes = read_asset(&asset)?;

    let header = find_header(asset.category, &bytes);
    let extracted_bytes = match header {
        Ok(header) => {
            // Add the extension if needed
            if add_extension {
                let extension = match header.as_str() {
                    "OggS" => "ogg",
                    "ID3" => "mp3",
                    "PNG" => "png",
                    "WEBP" => "webp",
                    "KTX" => "ktx",
                    "<roblox!" => "rbxm",
                    _ => "ogg",
                };

                destination.set_extension(extension);
            }

            extract_bytes(&header, bytes) // Extract between the header to the end of the file.
        }
        Err(_) => bytes, // No header found; write the raw bytes as-is.
    };

    // Ensure parent directory exists (needed when asset name contains subdirectories,
    // e.g. rbx-storage assets stored as "ab/abcdef...")
    if let Some(parent) = destination.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log_error!("Failed to create parent directory: {}", e);
        }
    }

    match fs::write(destination.clone(), extracted_bytes) {
        Ok(_) => (),
        Err(e) => log_error!("Error writing file: {}", e),
    };

    if let Some(sys_modified_time) = asset.last_modified {
        let modified_time = filetime::FileTime::from_system_time(sys_modified_time);
        match filetime::set_file_times(&destination, modified_time, modified_time) {
            Ok(_) => (),
            Err(e) => log_error!("Failed to write file modification time {}", e),
        };
    }

    Ok(destination)
}

pub fn extract_asset_to_bytes(asset: AssetInfo) -> Result<Vec<u8>, std::io::Error> {
    let bytes = read_asset(&asset)?;

    match find_header(asset.category, &bytes) {
        Ok(header) => Ok(extract_bytes(&header, bytes)), // Extract between the header to the end of the file.
        Err(_) => Ok(bytes),                             // No header was found.
    }
}

pub fn extract_dir(
    destination: PathBuf,
    category: Category,
    yield_for_thread: bool,
    use_alias: bool,
) {
    // Create directory if it doesn't exist
    match fs::create_dir_all(destination.clone()) {
        Ok(_) => (),
        Err(e) => log_error!("Error creating directory: {}", e),
    };
    let running = {
        let task = TASK_RUNNING.lock().unwrap();
        *task
    };
    // Stop multiple threads from running
    if !running {
        let handle = thread::spawn(move || {
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = true; // Stop other threads from running
            }

            // User has configured it to refresh before extracting
            if config::get_config_bool("refresh_before_extract").unwrap_or(false) {
                refresh(category, false, true); // true because it'll run both and have unfinished file list
            }

            let file_list = get_file_list();

            // Get locale for localised status messages
            let locale = locale::get_locale(None);

            // Get amount and initialise counter for progress
            let total = file_list.len();
            let mut count = 0;

            for entry in file_list.iter() {
                count += 1; // Increase counter for progress
                update_progress(count as f32 / total as f32); // Convert to f32 to allow floating point output

                let alias = if use_alias {
                    config::get_asset_alias(&entry.name)
                } else {
                    entry.name.clone()
                };

                let dest = destination.join(alias); // Local variable destination

                // Args for formatting
                let mut args = FluentArgs::new();
                args.set("item", count);
                args.set("total", total);

                match extract_to_file(entry.clone(), dest, true) {
                    Ok(_) => {
                        update_status(locale::get_message(
                            &locale,
                            "extracting-files",
                            Some(&args),
                        ));
                    }
                    Err(e) => {
                        update_status(locale::get_message(
                            &locale,
                            "extracting-files",
                            Some(&args),
                        ));
                        log_error!("Error extracting file ({}/{}): {}", count, total, e);
                    }
                }
            }
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = false; // Allow other threads to run again
            }
            update_status(locale::get_message(&locale, "all-extracted", None)); // Set the status to confirm to the user that all has finished
        });

        if yield_for_thread {
            // Will wait for the thread instead of quitting immediately
            let _ = handle.join();
        }
    }
}

pub fn extract_all(destination: PathBuf, yield_for_thread: bool, use_alias: bool) {
    let running = {
        let task = TASK_RUNNING.lock().unwrap();
        *task
    };
    // Stop multiple threads from running
    if !running {
        let handle = thread::spawn(move || {
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = true; // Stop other threads from running
            }

            // Get locale for localised status messages
            let locale = locale::get_locale(None);

            // Extract music directory
            extract_dir(destination.clone(), Category::Music, true, use_alias);

            // Extract http directory
            extract_dir(destination.clone(), Category::All, true, use_alias);

            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = false; // Allow other threads to run again
            }
            update_status(locale::get_message(&locale, "all-extracted", None)); // Set the status to confirm to the user that all has finished
        });

        if yield_for_thread {
            // Will wait for the thread instead of quitting immediately
            let _ = handle.join();
        }
    }
}

pub fn swap_assets(asset_a: AssetInfo, asset_b: AssetInfo) {
    let cache_directory_result = cache_directory::swap_assets(&asset_a, &asset_b);
    let sql_database_result = sql_database::swap_assets(&asset_a, &asset_b);
    let rbx_storage_result = rbx_storage_directory::swap_assets(&asset_a, &asset_b);

    // Confirmation and error messages
    let locale = locale::get_locale(None);
    let mut args = FluentArgs::new();

    if cache_directory_result.as_ref().is_err()
        && sql_database_result.as_ref().is_err()
        && rbx_storage_result.as_ref().is_err()
    {
        // cache_directory error
        args.set(
            "error",
            cache_directory_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!(
            "Error opening file '{}'",
            cache_directory_result.unwrap_err()
        );

        // sql_database error
        args.set(
            "error",
            sql_database_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!("Error opening file '{}'", sql_database_result.unwrap_err());

        // rbx_storage error
        args.set(
            "error",
            rbx_storage_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!("Error opening file '{}'", rbx_storage_result.unwrap_err());
    } else {
        args.set("item_a", asset_a.name);
        args.set("item_b", asset_b.name);
        update_status(locale::get_message(&locale, "swapped", Some(&args)));
    }
}

pub fn copy_assets(asset_a: AssetInfo, asset_b: AssetInfo) {
    let cache_directory_result = cache_directory::copy_assets(&asset_a, &asset_b);
    let sql_database_result = sql_database::copy_assets(&asset_a, &asset_b);
    let rbx_storage_result = rbx_storage_directory::copy_assets(&asset_a, &asset_b);

    // Confirmation and error messages
    let locale = locale::get_locale(None);
    let mut args = FluentArgs::new();

    if cache_directory_result.as_ref().is_err()
        && sql_database_result.as_ref().is_err()
        && rbx_storage_result.as_ref().is_err()
    {
        // cache_directory error
        args.set(
            "error",
            cache_directory_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!(
            "Error opening file '{}'",
            cache_directory_result.unwrap_err()
        );

        // sql_database error
        args.set(
            "error",
            sql_database_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!("Error opening file '{}'", sql_database_result.unwrap_err());

        // rbx_storage error
        args.set(
            "error",
            rbx_storage_result.as_ref().unwrap_err().to_string(),
        );
        update_status(locale::get_message(
            &locale,
            "failed-opening-file",
            Some(&args),
        ));
        log_error!("Error opening file '{}'", rbx_storage_result.unwrap_err());
    } else {
        args.set("item_a", asset_a.name);
        args.set("item_b", asset_b.name);
        update_status(locale::get_message(&locale, "copied", Some(&args)));
    }
}

pub fn filter_file_list(query: String) {
    let query_lower = query.to_lowercase();
    let file_list = get_file_list(); // Snapshot (Arc refcount bump)

    // Match case-insensitively on name and alias; collect once and assign under
    // a single lock rather than locking per match.
    let filtered: Vec<AssetInfo> = file_list
        .iter()
        .filter(|file| {
            file.name.to_lowercase().contains(&query_lower)
                || config::get_asset_alias(&file.name)
                    .to_lowercase()
                    .contains(&query_lower)
        })
        .cloned()
        .collect();

    *FILTERED_FILE_LIST.lock().unwrap() = Arc::new(filtered);
}

pub fn create_asset_info(asset: &str, category: Category) -> AssetInfo {
    if let Some(info) = sql_database::create_asset_info(asset, category) {
        return info;
    }

    if let Some(info) = cache_directory::create_asset_info(asset, category) {
        return info;
    }

    // Asset doesn't exist, but info is needed anyways
    AssetInfo {
        name: asset.to_string(),
        _size: 0,
        last_modified: None,
        from_file: false,
        from_sql: false,
        from_rbx_storage: false,
        category,
    }
}

pub fn determine_category(bytes: &[u8]) -> Category {
    // Music shares headers with Sounds and iterates first, so it would shadow
    // Sounds and mis-classify every OggS/ID3 file as Music. Skip both: this
    // classifies *unknown* bytes (e.g. an "All" listing), and Music is a
    // location-based (/sounds) category, not a byte-signature one.
    for category in Category::iter().filter(|&cat| cat != Category::All && cat != Category::Music) {
        for header in get_headers(&category) {
            // ID3 gets false positives, so also require an HTTP "binary/" marker.
            if header == "ID3" {
                if bytes_contains(bytes, header.as_bytes()) && bytes_contains(bytes, b"binary/") {
                    return category;
                }
            } else if bytes_contains(bytes, header.as_bytes()) {
                return category;
            }
        }
    }

    // No category found, return All
    Category::All
}

// File headers for each category.
//
// `Category::Music` reuses the same OggS/ID3 headers as `Sounds` because
// Roblox stores music as raw audio in /sounds with no HTTP header. This lets
// `find_header` succeed on a music file and resolve its extension via the
// existing header→extension table in `extract_to_file`, instead of needing a
// separate magic-byte detector for the no-header path.
pub fn get_headers(category: &Category) -> Vec<&'static str> {
    match category {
        Category::Music => {
            vec!["OggS", "ID3"]
        }
        Category::Sounds => {
            vec!["OggS", "ID3"]
        }
        Category::Ktx => {
            vec!["KTX"]
        }
        Category::Rbxm => {
            vec!["<roblox!"]
        }
        Category::Images => {
            vec!["PNG", "WEBP"]
        }
        Category::All => {
            // Aggregate headers from every category except `All` and `Music`.
            // Music shares its headers with Sounds (see above) and is location-based
            // (/sounds), so including it here would just duplicate the audio headers.
            Category::iter()
                .filter(|&cat| cat != Category::All && cat != Category::Music)
                .flat_map(|cat| get_headers(&cat))
                .collect()
        }
    }
}

pub fn update_status(value: String) {
    let mut status = STATUS.lock().unwrap();
    *status = value;
    let mut request = REQUEST_REPAINT.lock().unwrap();
    *request = true;
}

pub fn update_progress(value: f32) {
    let mut progress = PROGRESS.lock().unwrap();
    *progress = value;
    let mut request = REQUEST_REPAINT.lock().unwrap();
    *request = true;
}

pub fn get_file_list() -> Arc<Vec<AssetInfo>> {
    // Snapshot: bumps the Arc refcount, doesn't clone the Vec.
    FILE_LIST.lock().unwrap().clone()
}

pub fn get_filtered_file_list() -> Arc<Vec<AssetInfo>> {
    FILTERED_FILE_LIST.lock().unwrap().clone()
}

pub fn get_status() -> String {
    STATUS.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_decompress_no_zstd() {
        let bytes = vec![1, 2, 3, 4];
        let result = maybe_decompress(bytes.clone());
        assert_eq!(result, bytes);
    }

    fn dummy_asset(name: &str) -> AssetInfo {
        AssetInfo {
            name: name.to_owned(),
            _size: 0,
            last_modified: None,
            from_file: true,
            from_sql: false,
            from_rbx_storage: false,
            category: Category::Music,
        }
    }

    // A snapshot from get_file_list() must stay stable after later writes (the
    // copy-on-write guarantee the GUI relies on), and filter_file_list must
    // produce the matching subset. Kept in one test to avoid racing other tests
    // over the global list.
    #[test]
    fn test_file_list_arc_snapshot_and_filter() {
        clear_file_list();
        update_file_list(dummy_asset("apple"), false);

        // Take a snapshot, then mutate the live list.
        let snapshot = get_file_list();
        assert_eq!(snapshot.len(), 1);
        update_file_list(dummy_asset("banana"), false);

        // The snapshot is unchanged; the live list reflects the new push.
        assert_eq!(snapshot.len(), 1, "snapshot must not see later writes");
        assert_eq!(snapshot[0].name, "apple");
        let live = get_file_list();
        assert_eq!(live.len(), 2);
        // The write should have copied-on-write to a fresh allocation.
        assert!(!Arc::ptr_eq(&snapshot, &live));

        // Filtering matches on the asset name (case-insensitive).
        filter_file_list("APP".to_owned());
        let filtered = get_filtered_file_list();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "apple");

        clear_file_list();
        assert_eq!(get_file_list().len(), 0);
    }

    #[test]
    fn test_bytes_search() {
        let haystack = b"hello world";
        assert_eq!(bytes_search(haystack, b"hello"), Some(0));
        assert_eq!(bytes_search(haystack, b"world"), Some(6));
        assert_eq!(bytes_search(haystack, b"rust"), None);
        assert_eq!(bytes_search(haystack, b""), None);
    }

    #[test]
    fn test_bytes_contains() {
        let haystack = b"hello world";
        assert!(bytes_contains(haystack, b"hello"));
        assert!(bytes_contains(haystack, b"world"));
        assert!(!bytes_contains(haystack, b"rust"));
        assert!(!bytes_contains(haystack, b""));
    }

    #[test]
    fn test_determine_category_png() {
        let png_bytes = b"\x89PNG\r\n\x1a\n";
        assert_eq!(determine_category(png_bytes), Category::Images);
    }

    #[test]
    fn test_determine_category_webp() {
        let webp_bytes = b"RIFF....WEBP";
        assert_eq!(determine_category(webp_bytes), Category::Images);
    }

    #[test]
    fn test_determine_category_ktx() {
        let ktx_bytes = b"\xABKTX 11\xBB";
        assert_eq!(determine_category(ktx_bytes), Category::Ktx);
    }

    #[test]
    fn test_determine_category_rbxm() {
        let rbxm_bytes = b"<roblox!";
        assert_eq!(determine_category(rbxm_bytes), Category::Rbxm);
    }

    #[test]
    fn test_determine_category_mp3() {
        // ID3 header requires "binary/" for Category::Sounds in this app
        let mp3_bytes = b"ID3...binary/";
        assert_eq!(determine_category(mp3_bytes), Category::Sounds);
    }

    #[test]
    fn test_determine_category_unknown() {
        let unknown_bytes = b"unknown data";
        assert_eq!(determine_category(unknown_bytes), Category::All);
    }

    #[test]
    fn test_get_headers() {
        assert!(get_headers(&Category::Images).contains(&"PNG"));
        assert!(get_headers(&Category::Images).contains(&"WEBP"));
        // Music reuses the audio headers so find_header can resolve its extension.
        assert!(get_headers(&Category::Music).contains(&"OggS"));
        assert!(get_headers(&Category::Music).contains(&"ID3"));
    }

    #[test]
    fn test_get_headers_all_excludes_music_duplicates() {
        // All aggregates every category except All and Music, so the audio
        // headers appear exactly once (from Sounds), not twice.
        let all = get_headers(&Category::All);
        let oggs = all.iter().filter(|&&h| h == "OggS").count();
        let id3 = all.iter().filter(|&&h| h == "ID3").count();
        assert_eq!(oggs, 1);
        assert_eq!(id3, 1);
    }

    #[test]
    fn test_extract_bytes_offset_underflow() {
        // The WEBP offset is 8. If the header is found at an index smaller than
        // the offset, the index must saturate to 0 instead of underflowing and
        // panicking with an out-of-bounds slice.
        let bytes = b"WEBPdata".to_vec();
        let result = extract_bytes("WEBP", bytes.clone());
        assert_eq!(result, bytes);
    }

    #[test]
    fn test_extract_bytes_png_offset() {
        // PNG has an offset of 1, so extraction starts one byte before "PNG".
        let bytes = b"\x89PNG\r\n".to_vec();
        let result = extract_bytes("PNG", bytes.clone());
        assert_eq!(result, bytes);
    }

    #[test]
    fn test_extract_bytes_no_header_returns_input() {
        let bytes = b"no header here".to_vec();
        let result = extract_bytes("OggS", bytes.clone());
        assert_eq!(result, bytes);
    }
}

pub fn get_progress() -> f32 {
    *PROGRESS.lock().unwrap()
}

pub fn get_list_task_running() -> bool {
    *LIST_TASK_RUNNING.lock().unwrap()
}

pub fn get_stop_list_running() -> bool {
    *STOP_LIST_RUNNING.lock().unwrap()
}

pub fn get_request_repaint() -> bool {
    let mut request_repaint = REQUEST_REPAINT.lock().unwrap();
    let old_request_repaint = *request_repaint;
    *request_repaint = false; // Set to false when this function is called to acknowledge
    old_request_repaint
}

// Delete the temp directory
pub fn clean_up() {
    let temp_dir = get_temp_dir();
    // Just in case if it somehow resolves to "/"
    if temp_dir != PathBuf::new() && temp_dir != PathBuf::from("/") {
        log_info!("Cleaning up {}", temp_dir.display());
        match fs::remove_dir_all(temp_dir) {
            Ok(_) => log_info!("Done cleaning up directory"),
            Err(e) => log_error!("Failed to clean up directory: {}", e),
        }
    }

    match sql_database::clean_up() {
        Ok(_) => (),
        Err(e) => log_error!("Failed to clean up SQL database: {:?}", e),
    }
}

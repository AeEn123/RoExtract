use std::{
    collections::HashMap,
    env, fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, LazyLock, Mutex,
    },
    thread,
    time::SystemTime,
};

use clap::ValueEnum;
use fluent_bundle::FluentArgs;

use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

use crate::{config, locale};

pub mod audio;
pub mod cache_directory;
pub mod convert;
pub mod ktx;
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
// One asset list per category, so switching tabs doesn't throw the others away.
static FILE_LISTS: LazyLock<Mutex<HashMap<Category, FileList>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
static REQUEST_REPAINT: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static PROGRESS: LazyLock<Mutex<f32>> = LazyLock::new(|| Mutex::new(1.0));
// Only one listing runs at a time; starting another stops it.
static LISTING: LazyLock<Mutex<Listing>> = LazyLock::new(|| Mutex::new(Listing::default()));
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
    pub size: u64,
    pub last_modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    /// File extension of the detected format, e.g. "png". None if unknown.
    pub format: Option<&'static str>,
    pub from_file: bool,
    pub from_sql: bool,
    pub from_rbx_storage: bool,
    pub category: Category,
}

/// The assets listed for one category.
///
/// `assets` is an `Arc<Vec<AssetInfo>>` so the GUI can snapshot it per-frame
/// with a refcount bump instead of a deep clone; writers use `Arc::make_mut` for
/// copy-on-write (clones only when a reader holds the previous snapshot).
#[derive(Default)]
struct FileList {
    assets: Arc<Vec<AssetInfo>>,
    /// Changes whenever `assets` does, so readers can cache work derived from it.
    generation: u64,
    /// A listing ran to completion at least once.
    loaded: bool,
}

impl FileList {
    fn set_assets(&mut self, assets: Vec<AssetInfo>) {
        self.assets = Arc::new(assets);
        self.generation = next_generation();
    }
}

#[derive(Default)]
struct Listing {
    running: bool,
    stop: bool,
    category: Option<Category>,
    /// Background refreshes collect here and replace the list once finished,
    /// so the old list stays visible meanwhile.
    staging: Option<Vec<AssetInfo>>,
}

// Define local functions
fn next_generation() -> u64 {
    NEXT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

fn update_file_list(category: Category, value: AssetInfo, cli_list_mode: bool) {
    // cli_list_mode will print out to console
    // It is done this way so it can read files and print to console in the same stage
    if cli_list_mode {
        println!("{}", value.name);
    }
    {
        let mut listing = LISTING.lock().unwrap();
        if listing.category == Some(category) {
            if let Some(staging) = listing.staging.as_mut() {
                staging.push(value);
                return;
            }
        }
    }
    let mut file_lists = FILE_LISTS.lock().unwrap();
    let file_list = file_lists.entry(category).or_default();
    // make_mut clones only if a snapshot is still being read elsewhere.
    Arc::make_mut(&mut file_list.assets).push(value);
    file_list.generation = next_generation();
}

fn clear_file_list(category: Category) {
    let mut file_lists = FILE_LISTS.lock().unwrap();
    file_lists
        .entry(category)
        .or_default()
        .set_assets(Vec::new());
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

/// The file extension for a header found by `find_header`.
fn header_extension(header: &str) -> &'static str {
    match header {
        "OggS" => "ogg",
        "ID3" => "mp3",
        "PNG" => "png",
        "WEBP" => "webp",
        "KTX" => "ktx",
        "<roblox!" => "rbxm",
        _ => "ogg",
    }
}

/// The format of an asset from its first bytes, as a file extension.
pub fn detect_format(category: Category, bytes: &[u8]) -> Option<&'static str> {
    find_header(category, bytes)
        .ok()
        .map(|header| header_extension(&header))
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

/// Detect the cache directory and open the database now rather than on first use.
pub fn init_sources() {
    cache_directory::get_cache_directory();
    sql_database::get_db_path();
    rbx_storage_directory::get_rbx_storage_dir();
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

            audio::clear();

            // Clear the file lists for visual feedback to the user that the files are actually deleted
            for file_list in FILE_LISTS.lock().unwrap().values_mut() {
                file_list.set_assets(Vec::new());
                file_list.loaded = true;
            }
            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = false; // Allow other threads to run again
            }
            update_status(locale::get_message(&locale, "idling", None)); // Set the status back
        });
    }
}

/// List the assets of `category`, clearing its list first so they show up as they're found.
pub fn refresh(category: Category, cli_list_mode: bool, yield_for_thread: bool) {
    list_assets(category, cli_list_mode, yield_for_thread, false);
}

/// List the assets of `category`, keeping the current list until the new one is complete.
pub fn refresh_in_background(category: Category) {
    list_assets(category, false, false, true);
}

fn list_assets(category: Category, cli_list_mode: bool, yield_for_thread: bool, staged: bool) {
    let handle = thread::spawn(move || {
        // Get locale for localised status messages
        let locale = locale::get_locale(None);
        // Wait until no other listing is running, telling it to stop meanwhile
        loop {
            {
                let mut listing = LISTING.lock().unwrap();
                if !listing.running {
                    *listing = Listing {
                        running: true,
                        stop: false,
                        category: Some(category),
                        staging: staged.then(Vec::new),
                    };
                    break;
                }
                listing.stop = true; // Tell the other thread to stop
            }
            thread::sleep(std::time::Duration::from_millis(10)); // Sleep for a bit to not be CPU intensive
        }

        if !staged {
            clear_file_list(category);
        }

        sql_database::refresh(category, cli_list_mode, &locale);
        cache_directory::refresh(category, cli_list_mode, &locale);
        rbx_storage_directory::refresh(category, cli_list_mode, &locale);

        let (completed, staging) = {
            let mut listing = LISTING.lock().unwrap();
            let completed = !listing.stop;
            let staging = listing.staging.take();
            *listing = Listing::default(); // Allow other threads to run again
            (completed, staging)
        };
        let is_empty = {
            let mut file_lists = FILE_LISTS.lock().unwrap();
            let file_list = file_lists.entry(category).or_default();
            match staging {
                Some(staging) if completed => file_list.set_assets(staging),
                Some(_) => (),                        // Interrupted, keep the old list
                None => file_list.loaded = completed, // An interrupted list is incomplete
            }
            if completed {
                file_list.loaded = true;
            }
            file_list.assets.is_empty()
        };

        if cli_list_mode && completed && is_empty {
            println!("{}", locale::get_message(&locale, "no-files", None));
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
    extract_to_file_as(asset, destination, add_extension, None)
}

/// Like `extract_to_file`, converting to `target` when given. If conversion
/// fails the asset is written as it is.
pub fn extract_to_file_as(
    asset: AssetInfo,
    destination: PathBuf,
    add_extension: bool,
    target: Option<convert::Target>,
) -> Result<PathBuf, std::io::Error> {
    let mut destination = destination.clone(); // Get own mutable destination

    let bytes = read_asset(&asset)?;

    let header = find_header(asset.category, &bytes);
    let extracted_bytes = match header {
        Ok(header) => {
            // Add the extension if needed
            if add_extension {
                destination.set_extension(header_extension(&header));
            }

            extract_bytes(&header, bytes) // Extract between the header to the end of the file.
        }
        Err(_) => bytes, // No header found; write the raw bytes as-is.
    };

    let extracted_bytes = match target {
        Some(target) => match convert::convert(&asset, &extracted_bytes, target) {
            Ok(converted) => {
                if add_extension {
                    destination.set_extension(target.extension());
                }
                converted
            }
            Err(e) => {
                log_error!(
                    "Failed to convert {} to {}: {}",
                    asset.name,
                    target.name(),
                    e
                );
                extracted_bytes
            }
        },
        None => extracted_bytes,
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

            extract_category(&destination, category, use_alias);

            {
                let mut task = TASK_RUNNING.lock().unwrap();
                *task = false; // Allow other threads to run again
            }
            let locale = locale::get_locale(None);
            update_status(locale::get_message(&locale, "all-extracted", None)); // Set the status to confirm to the user that all has finished
        });

        if yield_for_thread {
            // Will wait for the thread instead of quitting immediately
            let _ = handle.join();
        }
    }
}

/// Extract every asset of `category` into `destination`. Blocks until done.
fn extract_category(destination: &PathBuf, category: Category, use_alias: bool) {
    // Create directory if it doesn't exist
    match fs::create_dir_all(destination) {
        Ok(_) => (),
        Err(e) => log_error!("Error creating directory: {}", e),
    };

    // List first if the user configured it, or if this category was never listed
    if config::get_config_bool("refresh_before_extract").unwrap_or(false)
        || !is_list_loaded(category)
    {
        refresh(category, false, true);
    }

    let file_list = get_file_list(category);

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

        update_status(locale::get_message(
            &locale,
            "extracting-files",
            Some(&args),
        ));
        let target = convert::target_on_extract(entry);
        if let Err(e) = extract_to_file_as(entry.clone(), dest, true, target) {
            log_error!("Error extracting file ({}/{}): {}", count, total, e);
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
            extract_category(&destination, Category::Music, use_alias);

            // Extract http directory
            extract_category(&destination, Category::All, use_alias);

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
        audio::clear(); // The contents changed
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
        audio::clear(); // The contents changed
        args.set("item_a", asset_a.name);
        args.set("item_b", asset_b.name);
        update_status(locale::get_message(&locale, "copied", Some(&args)));
    }
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
        size: 0,
        last_modified: None,
        created: None,
        format: None,
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

/// Ask the GUI to redraw soon, e.g. after background work finished.
pub fn request_repaint() {
    *REQUEST_REPAINT.lock().unwrap() = true;
}

pub fn update_progress(value: f32) {
    let mut progress = PROGRESS.lock().unwrap();
    *progress = value;
    let mut request = REQUEST_REPAINT.lock().unwrap();
    *request = true;
}

pub fn get_file_list(category: Category) -> Arc<Vec<AssetInfo>> {
    // Snapshot: bumps the Arc refcount, doesn't clone the Vec.
    FILE_LISTS
        .lock()
        .unwrap()
        .get(&category)
        .map(|file_list| file_list.assets.clone())
        .unwrap_or_default()
}

/// Changes whenever the list for `category` does.
pub fn get_file_list_generation(category: Category) -> u64 {
    FILE_LISTS
        .lock()
        .unwrap()
        .get(&category)
        .map(|file_list| file_list.generation)
        .unwrap_or(0)
}

/// True once a listing of `category` ran to completion.
pub fn is_list_loaded(category: Category) -> bool {
    FILE_LISTS
        .lock()
        .unwrap()
        .get(&category)
        .is_some_and(|file_list| file_list.loaded)
}

/// True while `category` is being listed.
pub fn is_listing(category: Category) -> bool {
    LISTING.lock().unwrap().category == Some(category)
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
            size: 0,
            last_modified: None,
            created: None,
            format: None,
            from_file: true,
            from_sql: false,
            from_rbx_storage: false,
            category: Category::Music,
        }
    }

    // A snapshot from get_file_list() must stay stable after later writes (the
    // copy-on-write guarantee the GUI relies on), and each category keeps its
    // own list. Kept in one test to avoid racing other tests over the globals.
    #[test]
    fn test_file_list_arc_snapshot_and_categories() {
        clear_file_list(Category::Music);
        clear_file_list(Category::Images);
        update_file_list(Category::Music, dummy_asset("apple"), false);

        // Take a snapshot, then mutate the live list.
        let snapshot = get_file_list(Category::Music);
        let generation = get_file_list_generation(Category::Music);
        assert_eq!(snapshot.len(), 1);
        update_file_list(Category::Music, dummy_asset("banana"), false);

        // The snapshot is unchanged; the live list reflects the new push.
        assert_eq!(snapshot.len(), 1, "snapshot must not see later writes");
        assert_eq!(snapshot[0].name, "apple");
        let live = get_file_list(Category::Music);
        assert_eq!(live.len(), 2);
        assert_ne!(get_file_list_generation(Category::Music), generation);
        // The write should have copied-on-write to a fresh allocation.
        assert!(!Arc::ptr_eq(&snapshot, &live));

        // Other categories are untouched.
        assert!(get_file_list(Category::Images).is_empty());

        clear_file_list(Category::Music);
        assert_eq!(get_file_list(Category::Music).len(), 0);
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
    fn test_detect_format() {
        assert_eq!(detect_format(Category::Images, b"\x89PNG\r\n"), Some("png"));
        assert_eq!(detect_format(Category::Music, b"ID3\x04"), Some("mp3"));
        assert_eq!(detect_format(Category::Sounds, b"OggS\0"), Some("ogg"));
        assert_eq!(detect_format(Category::All, b"<roblox!"), Some("rbxm"));
        assert_eq!(detect_format(Category::Images, b"OggS"), None);
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
    LISTING.lock().unwrap().running
}

pub fn get_stop_list_running() -> bool {
    LISTING.lock().unwrap().stop
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

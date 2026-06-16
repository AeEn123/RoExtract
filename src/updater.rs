use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        LazyLock, Mutex,
    },
    thread,
};

use eframe::egui;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::{config, logic};

pub mod gui;

#[cfg(target_os = "windows")]
use std::ffi::OsString;

static UPDATE_FILE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// An update that has been detected by a background check and is waiting to be
/// shown to the user as an in-app prompt. Holds the release and the resolved
/// download URL for the current platform.
static AVAILABLE_UPDATE: LazyLock<Mutex<Option<(Release, String)>>> =
    LazyLock::new(|| Mutex::new(None));

/// Set while a binary download is in progress so the GUI can show feedback and
/// avoid starting a second download.
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

static URL: &str = "https://api.github.com/repos/AeEn123/RoExtract/releases/latest";
static PRERELEASE_URL: &str = "https://api.github.com/repos/AeEn123/RoExtract/releases";

#[derive(Deserialize, Debug, Clone)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Release {
    name: String,
    tag_name: String,
    body: String,
    assets: Vec<Asset>, // List of assets
}

fn clean_version_number(version: &str) -> String {
    version
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect()
}

fn detect_download_binary(assets: &[Asset]) -> Option<&Asset> {
    let os = std::env::consts::OS; // Get the user's operating system to download the correct binary

    for asset in assets {
        let name = asset.name.to_lowercase();

        // Download installer based on system config
        let installer = if config::get_system_config_bool("prefer-installers").unwrap_or(false) {
            name.contains("install")
        } else {
            !name.contains("install")
        };

        if name.contains(os) && installer {
            return Some(asset); // Return the correct binary based on OS
        }
    }

    log_warn!("Failed to find asset, going for first asset listed.");
    assets.first() // None if the release has no assets, avoiding an index panic
}

fn update_action(json: Release, auto_download_update: bool) {
    log_info!("An update is available.");
    log_info!("{}", &json.name);
    log_info!("{}", &json.body);

    if auto_download_update {
        let correct_asset = match detect_download_binary(&json.assets) {
            Some(asset) => asset,
            None => {
                log_error!("Update available but the release contains no downloadable assets.");
                return;
            }
        };

        let tag_name = if json.tag_name.contains("dev-build") {
            Some(json.tag_name.as_str())
        } else {
            None
        };
        download_update(&correct_asset.browser_download_url, tag_name);
    }
}

#[cfg(target_family = "unix")]
fn save_install_script() -> PathBuf {
    let temp_dir = logic::get_temp_dir();
    let path = temp_dir.join("installer.sh");

    if temp_dir != PathBuf::new() {
        match fs::write(&path, include_str!("installer/installer.sh")) {
            Ok(_) => log_info!("File written to {}", path.display()),
            Err(e) => log_error!("Failed to write to {}: {}", path.display(), e),
        }

        path
    } else {
        PathBuf::new()
    }
}

#[cfg(target_os = "windows")]
fn save_install_script() -> PathBuf {
    let temp_dir = logic::get_temp_dir();
    let path = temp_dir.join("installer.bat");

    if temp_dir != PathBuf::new() {
        match fs::write(&path, include_str!("installer/installer.bat")) {
            Ok(_) => log_info!("File written to {}", path.display()),
            Err(e) => log_error!("Failed to write to {}: {}", path.display(), e),
        }

        return path;
    } else {
        return PathBuf::new();
    }
}

pub fn download_update(url: &str, tag_name: Option<&str>) {
    if !config::get_system_config_bool("allow-updates").unwrap_or(true) {
        log_warn!("Updating has been disabled by the system.");
        return;
    }
    let client = Client::new();
    let filename = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let temp_dir = logic::get_temp_dir();

    let response = client
        .get(url)
        .header("User-Agent", "RoExtract (Rust)") // Set a User-Agent otherwise it returns 403
        .send();

    match response {
        Ok(data) => match data.bytes() {
            Ok(bytes) => {
                let path = temp_dir.join(filename);
                match fs::write(path.clone(), bytes) {
                    Ok(_) => {
                        set_update_file(path);
                        config::set_config_value("current_tag_name", tag_name.into());
                        config::save_config_file();
                    }
                    Err(e) => log_error!("Failed to write file: {}", e),
                }
            }
            Err(e) => log_error!("Download failed: Failed to parse: {}", e),
        },
        Err(e) => log_error!("Failed to download: {}", e),
    }
}

pub fn set_update_file(file: PathBuf) {
    let mut update_file = UPDATE_FILE.lock().unwrap();
    *update_file = Some(file)
}

pub fn run_install_script(run_afterwards: bool) -> bool {
    if let Some(update_file) = { UPDATE_FILE.lock().unwrap().clone() } {
        log_info!("Installing from {}", update_file.display());
        if config::get_system_config_bool("prefer-installers").unwrap_or(false) {
            // Just run the installer
            match open::that(update_file) {
                Ok(_) => (),
                Err(e) => log_error!("Installer failed to launch {} ", e),
            }

            std::process::exit(0);
        } else {
            // Run install script
            let install_script = save_install_script();
            if install_script != PathBuf::new() {
                #[cfg(target_os = "windows")]
                let mut command = std::process::Command::new("cmd");
                #[cfg(target_family = "unix")]
                let mut command = std::process::Command::new("sh");

                let program_path = std::env::current_exe().unwrap();

                #[cfg(target_family = "unix")]
                if run_afterwards {
                    command
                        .args([
                            install_script,
                            update_file,
                            program_path.clone(),
                            program_path,
                        ])
                        .spawn()
                        .expect("failed to start update script");
                } else {
                    command
                        .args([install_script, update_file, program_path])
                        .spawn()
                        .expect("failed to start update script");
                }

                #[cfg(target_os = "windows")] // cmd /c
                if run_afterwards {
                    command.args([
                        OsString::from("/c"),
                        install_script.into_os_string(),
                        update_file.into_os_string(),
                        program_path.clone().into_os_string(),
                        program_path.into_os_string(),
                    ]);
                } else {
                    command.args([
                        OsString::from("/c"),
                        install_script.into_os_string(),
                        update_file.into_os_string(),
                        program_path.into_os_string(),
                        OsString::from("exit"),
                    ]);
                }

                std::process::exit(0);
            }

            true
        }
    } else {
        false
    }
}

/// Performs the (blocking) network request to GitHub and returns the release
/// that should be offered as an update, or `None` if there is nothing newer or
/// the request failed. This is the only function that touches the network for
/// update checks; callers decide whether to run it on a background thread.
fn fetch_available_update() -> Option<Release> {
    let include_prerelease = config::get_config_bool("include_prerelease").unwrap_or(false);

    let client = Client::new();

    let response = if include_prerelease {
        client
            .get(PRERELEASE_URL)
            .header("User-Agent", "RoExtract (Rust)")
            .send()
    } else {
        client
            .get(URL)
            .header("User-Agent", "RoExtract (Rust)") // Set a User-Agent otherwise it returns 403
            .send()
    };

    match response {
        Ok(data) => {
            let text = data.text().unwrap_or("No text".to_string());
            if include_prerelease {
                match serde_json::from_str::<Vec<Release>>(&text) {
                    Ok(data) => {
                        if let Some(json) = data.into_iter().next() {
                            let current_tag = config::get_config_string("current_tag_name")
                                .unwrap_or("None".to_string());
                            if current_tag != json.tag_name {
                                return Some(json);
                            }
                        } else {
                            log_info!("No releases returned by the update server.");
                        }
                    }
                    Err(e) => log_error!("Updater failed to parse json: {}", e),
                };
            } else {
                match serde_json::from_str::<Release>(&text) {
                    Ok(json) => {
                        let clean_tag_name = clean_version_number(&json.tag_name);
                        let clean_version = clean_version_number(env!("CARGO_PKG_VERSION"));
                        if (clean_tag_name != clean_version)
                            | config::get_config_string("current_tag_name").is_some()
                        {
                            // Update back to stable version if user has opted out of development builds
                            return Some(json);
                        } else {
                            log_info!("No updates are available.")
                        }
                    }
                    Err(e) => log_error!("Updater Failed to parse json: {}", e),
                }
            }
        }
        Err(e) => log_error!("Failed to check for update: {}", e),
    }

    None
}

/// Blocking update check used by the CLI (`--check-for-updates`,
/// `--download-new-update`). The CLI's whole job is this one request and the
/// process exits straight after, so blocking is correct here.
pub fn check_for_updates(auto_download_update: bool) {
    if let Some(json) = fetch_available_update() {
        update_action(json, auto_download_update);
    }
}

/// Non-blocking update check used by the GUI. Runs the network request on a
/// background thread so the main window can open immediately. If an update is
/// found it is either auto-downloaded (when enabled) or stored for the GUI to
/// surface as an in-app prompt; `ctx` is used to wake the GUI when that happens.
pub fn check_for_updates_background(ctx: egui::Context, auto_download_update: bool) {
    thread::spawn(move || {
        let Some(json) = fetch_available_update() else {
            return;
        };

        if auto_download_update {
            update_action(json, true);
            return;
        }

        // Resolve the download URL up front so the GUI doesn't have to.
        let url = match detect_download_binary(&json.assets) {
            Some(asset) => asset.browser_download_url.clone(),
            None => {
                log_error!("Update available but the release contains no downloadable assets.");
                return;
            }
        };

        log_info!("An update is available: {}", json.name);
        *AVAILABLE_UPDATE.lock().unwrap() = Some((json, url));
        ctx.request_repaint(); // Wake the GUI so it can show the prompt
    });
}

/// Returns and clears the pending update detected by a background check, if any.
pub fn take_available_update() -> Option<(Release, String)> {
    AVAILABLE_UPDATE.lock().unwrap().take()
}

/// Whether a binary download is currently in progress.
pub fn is_downloading() -> bool {
    DOWNLOADING.load(Ordering::Relaxed)
}

/// Downloads the update binary on a background thread and, once finished,
/// launches the install script. Non-blocking so the GUI stays responsive while
/// the (potentially large) binary downloads.
pub fn download_and_install(ctx: egui::Context, url: String, tag_name: Option<String>) {
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return; // A download is already running
    }

    thread::spawn(move || {
        download_update(&url, tag_name.as_deref());

        // On success this hands off to the install script and exits the process.
        if !run_install_script(true) {
            log_error!("Update download failed; not installing.");
            DOWNLOADING.store(false, Ordering::SeqCst);
            ctx.request_repaint(); // Let the GUI re-show the prompt buttons
        }
    });
}

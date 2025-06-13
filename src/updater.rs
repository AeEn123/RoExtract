use std::{fs, path::PathBuf, sync::Mutex};

use reqwest::blocking::Client;
use serde::Deserialize;
use lazy_static::lazy_static;

use crate::{config, logic};

mod gui;

#[cfg(target_os = "windows")]
use std::ffi::OsString;

lazy_static! {
    static ref UPDATE_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);
}

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
    version.chars().filter(|c| c.is_digit(10) || *c == '.').collect()
}

fn detect_download_binary(assets: &Vec<Asset>) -> &Asset {
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
            return asset // Return the correct binary based on OS
        }
    }

    log_warn!("Failed to find asset, going for first asset listed.");
    return &assets[0];
}

fn update_action(json: Release, run_gui: bool, auto_download_update: bool) {
    log_info!("An update is available.");
    log_info!("{}", &json.name);
    log_info!("{}", &json.body);

    let correct_asset = detect_download_binary(&json.assets);

    if auto_download_update {
        let tag_name = if json.tag_name.contains("dev-build") {
            Some(json.tag_name.as_str())
        } else {
            None
        };
        download_update(&correct_asset.browser_download_url, tag_name);
    } else if run_gui {
        match gui::run_gui(json.clone(), correct_asset.browser_download_url.clone()) {
            Ok(_) => log_info!("User exited GUI"),
            Err(e) => log_critical!("GUI failed: {}",e)
        }
    }
}

#[cfg(target_family = "unix")]
fn save_install_script() -> PathBuf {
    let temp_dir = logic::get_temp_dir(false);
    let path = temp_dir.join("installer.sh");

    if temp_dir != PathBuf::new() {
        match fs::write(&path, include_str!("installer/installer.sh")) {
            Ok(_) => log_info!("File written to {}", path.display()),
            Err(e) => log_error!("Failed to write to {}: {}", path.display(), e)
        }
        
        return path;
    } else {
        return PathBuf::new();
    }
}

#[cfg(target_os = "windows")]
fn save_install_script() -> PathBuf {
    let temp_dir = logic::get_temp_dir(false);
    let path = temp_dir.join("installer.bat");

    if temp_dir != PathBuf::new() {
        match fs::write(&path, include_str!("installer/installer.bat")) {
            Ok(_) => log_info!("File written to {}", path.display()),
            Err(e) => log_error!("Failed to write to {}: {}", path.display(), e)
        }
        
        return path;
    } else {
        return PathBuf::new();
    }
}

pub fn download_update(url: &str, tag_name: Option<&str>) {
    if !config::get_system_config_bool("allow-updates").unwrap_or(true) {
        log_warn!("Updating has been disabled by the system.");
        return
    }
    let client = Client::new();
    let filename = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().to_string();
    let temp_dir = logic::get_temp_dir(true);

    let response = client
        .get(url)
        .header("User-Agent", "RoExtract (Rust)") // Set a User-Agent otherwise it returns 403
        .send();

    match response {
        Ok(data) => {
            match data.bytes() {
                Ok(bytes) => {
                    let path = temp_dir.join(filename);
                    match fs::write(path.clone(), bytes) {
                        Ok(_) => {
                            set_update_file(path);
                            config::set_config_value("current_tag_name", tag_name.clone().into());
                        },
                        Err(e) => log_error!("Failed to write file: {}", e)
                    }
                }
                Err(e) => log_error!("Download failed: Failed to parse: {}", e)
            }
        }
        Err(e) => log_error!("Failed to download: {}", e),
    }
}

pub fn set_update_file(file: PathBuf) {
    let mut update_file = UPDATE_FILE.lock().unwrap();
    *update_file = Some(file)
}

pub fn run_install_script(run_afterwards: bool) -> bool {
    if let Some(update_file) = {UPDATE_FILE.lock().unwrap().clone()} {
        log_info!("Installing from {}", update_file.display());
        if config::get_system_config_bool("prefer-installers").unwrap_or(false) {
            // Just run the installer
            match open::that(update_file) {
                Ok(_) => (),
                Err(e) => log_error!("Installer failed to launch {} ", e)
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
                    command.args([install_script, update_file, program_path.clone(), program_path]).spawn().expect("failed to start update script");
                } else {
                    command.args([install_script, update_file, program_path]).spawn().expect("failed to start update script");
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
    
            return true;
        }

    } else {
        return false;
    }
}

pub fn check_for_updates(run_gui: bool, auto_download_update: bool) {
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
                      let json = data[0].clone();
                      let current_tag = config::get_config_string("current_tag_name").unwrap_or("None".to_string());
                      if current_tag != json.tag_name {
                        update_action(json, run_gui, auto_download_update);
                      }                      
                    },
                    Err(e) => log_error!("Updater failed to parse json: {}", e)
                };
            } else {
                match serde_json::from_str::<Release>(&text) {
                    Ok(json) => {
                        let clean_tag_name = clean_version_number(&json.tag_name);
                        let clean_version = clean_version_number(env!("CARGO_PKG_VERSION"));
                        if (clean_tag_name != clean_version) | config::get_config_string("current_tag_name").is_some() { // Update back to stable version if user has opted out of development builds
                            update_action(json, run_gui, auto_download_update);
                        } else {
                            log_info!("No updates are available.")
                        }
                    }
                    Err(e) => log_error!("Updater Failed to parse json: {}", e)
                }
            }
        }
        Err(e) => log_error!("Failed to check for update: {}", e),
    }
}

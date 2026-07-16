use serde_json::{json, Value};
use std::sync::LazyLock;
use std::{fs, path::PathBuf, sync::Mutex};

use crate::logic;

static CONFIG: LazyLock<Mutex<Value>> = LazyLock::new(|| Mutex::new(read_config_file()));
static SYSTEM_CONFIG: LazyLock<Mutex<Value>> = LazyLock::new(|| Mutex::new(read_system_config()));
static CONFIG_FILE: LazyLock<Mutex<PathBuf>> = LazyLock::new(|| Mutex::new(detect_config_file()));

const SYSTEM_CONFIG_FILE: &str = "RoExtract-system.json";
const DEFAULT_CONFIG_FILE: &str = "RoExtract-config.json";

// Define local functions
fn detect_config_file() -> PathBuf {
    if let Some(config_path) = get_system_config_string("config-path") {
        PathBuf::from(logic::resolve_path(&config_path))
    } else {
        DEFAULT_CONFIG_FILE.into()
    }
}

fn read_config_file() -> Value {
    match fs::read(CONFIG_FILE.lock().unwrap().clone()) {
        Ok(bytes) => {
            match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    log_warn!("Failed to parse config file! {}", e);
                    json!({}) // Blank config by default
                }
            }
        }

        Err(_e) => {
            // Most likely no such file or directory
            json!({})
        }
    }
}

fn read_system_config() -> Value {
    let path = match std::env::current_exe() {
        Ok(path) => path.parent().unwrap_or(&path).join(SYSTEM_CONFIG_FILE),
        Err(_) => std::path::PathBuf::new().join(SYSTEM_CONFIG_FILE),
    };

    match fs::read(path) {
        Ok(bytes) => {
            match serde_json::from_slice(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    log_warn!("Failed to parse config file! {}", e);
                    json!({}) // Blank config by default
                }
            }
        }

        Err(_e) => {
            // Most likely no such file or directory
            json!({})
        }
    }
}

pub fn get_config() -> Value {
    CONFIG.lock().unwrap().clone()
}

pub fn get_config_string(key: &str) -> Option<String> {
    let config = CONFIG.lock().unwrap();
    let value = config.get(key)?;
    Some(value.as_str()?.to_owned().replace('"', ""))
}

pub fn get_config_bool(key: &str) -> Option<bool> {
    let config = CONFIG.lock().unwrap();
    config.get(key)?.as_bool()
}

pub fn get_config_u64(key: &str) -> Option<u64> {
    let config = CONFIG.lock().unwrap();
    config.get(key)?.as_u64()
}

pub fn get_asset_alias(asset: &str) -> String {
    let config = CONFIG.lock().unwrap();
    if let Some(aliases) = config.get("aliases") {
        if let Some(value) = aliases.get(asset) {
            return value.as_str().unwrap().to_owned().replace('"', "");
        }
    }
    asset.to_string()
}

pub fn set_config_value(key: &str, value: Value) {
    let mut config = CONFIG.lock().unwrap();
    config[key] = value;
}

pub fn remove_config_value(key: &str) {
    let mut config = CONFIG.lock().unwrap();
    config.as_object_mut().map(|obj| obj.remove(key));
}

pub fn set_asset_alias(asset: &str, value: &str) {
    let mut config = CONFIG.lock().unwrap();
    if config.get("aliases").is_none() {
        config["aliases"] = json!({});
    }

    config["aliases"][asset] = value.replace('"', "").into();
}

pub fn get_system_config_string(key: &str) -> Option<String> {
    let config = SYSTEM_CONFIG.lock().unwrap();
    let value = config.get(key)?;
    Some(value.as_str()?.to_owned().replace('"', ""))
}

pub fn get_system_config_bool(key: &str) -> Option<bool> {
    let config = SYSTEM_CONFIG.lock().unwrap();
    config.get(key)?.as_bool()
}

pub fn save_config_file() {
    let config = CONFIG.lock().unwrap().clone();
    match serde_json::to_vec_pretty(&config) {
        Ok(data) => {
            let result = fs::write(CONFIG_FILE.lock().unwrap().clone(), data);
            if result.is_err() {
                log_critical!(
                    "Failed to write config file: {}",
                    result.as_ref().unwrap_err()
                )
            }
        }
        Err(e) => {
            log_critical!("Failed to write config file: {}", e);
        }
    }
}

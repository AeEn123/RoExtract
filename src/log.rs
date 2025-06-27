use std::sync::{LazyLock, Mutex};

static LOG: LazyLock<Mutex<String>> = LazyLock::new(||Mutex::new(String::new()));


pub fn log(log_type: &str, message: &str, file: &str, line: u32, column: u32) {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_message = format!(
        "{}  {}{} [{}:{}:{}]",
        now, log_type, message, file, line, column
    );

    println!("{}", log_message);

    let mut log = LOG.lock().unwrap();
    log.push_str(&format!("{}\n", log_message));
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        crate::log::log("INFO:  ", &format!($($arg)*), file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        crate::log::log("WARN:  ", &format!($($arg)*), file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        crate::log::log("ERROR: ", &format!($($arg)*), file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            crate::log::log("DEBUG: ", &format!($($arg)*), file!(), line!(), column!())
        }
    };
}


#[macro_export]
macro_rules! log_critical {
    ($($arg:tt)*) => {{
        let formatted = format!($($arg)*);
        crate::log::log("CRITICAL: ", &formatted, file!(), line!(), column!());

        let _ = native_dialog::DialogBuilder::message()
            .set_level(native_dialog::MessageLevel::Error)
            .set_title(&crate::locale::get_message(
                &crate::locale::get_locale(None),
                "generic-error-critical",
                None,
            ))
            .set_text(&formatted)
            .alert()
            .show();
    }};
}

pub fn get_logs() -> String {
    return LOG.lock().unwrap().clone();
}

pub fn get_anonymous_logs() -> String {
    let logs = LOG.lock().unwrap().clone();
    // Remove all possible information
    let logs = logs.replace(&whoami::username(), "username");
    let logs = logs.replace(&whoami::realname(), "Real Name");
    let logs = logs.replace(&whoami::devicename(), "devicename");
    logs
}
// src/log.rs
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static LOG_FILE: &str = ".store/debug.log";
static LOG_LOCK: Mutex<()> = Mutex::new(());
static START: OnceLock<Instant> = OnceLock::new();

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

fn fmt_elapsed(ms: u128) -> String {
    let total_ms = ms as u64;
    let h = total_ms / 3_600_000;
    let m = (total_ms % 3_600_000) / 60_000;
    let s = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

/// Internal logging function
pub fn write_log(level: &str, msg: &str) {
    let elapsed = fmt_elapsed(start().elapsed().as_millis());
    let line = format!("[{elapsed}][{level}] {msg}\n");

    if let Ok(_guard) = LOG_LOCK.lock() {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

/// Info-level logging
#[macro_export]
macro_rules! logf {
    ($($arg:tt)*) => {
        $crate::log::write_log("INFO", &format!($($arg)*))
    };
}

/// Debug-level logging
#[macro_export]
macro_rules! logd {
    ($($arg:tt)*) => {
        $crate::log::write_log("DEBUG", &format!($($arg)*))
    };
}

/// Error-level logging
#[macro_export]
macro_rules! loge {
    ($($arg:tt)*) => {
        $crate::log::write_log("ERROR", &format!($($arg)*))
    };
}

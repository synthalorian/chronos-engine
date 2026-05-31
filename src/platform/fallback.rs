//! Fallback platform helpers for unsupported operating systems.
//!
//! Uses `std::env::var_os("HOME")` as a best-effort home directory and
//! returns `None` for dialogs.

use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn config_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".config"))
}

pub fn data_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".local/share"))
}

pub fn cache_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".cache"))
}

pub fn open_file_dialog(_title: &str) -> Option<PathBuf> {
    None
}

pub fn save_file_dialog(_title: &str, _default_name: &str) -> Option<PathBuf> {
    None
}

pub fn open_dir_dialog(_title: &str) -> Option<PathBuf> {
    None
}

pub fn message_box(_title: &str, _message: &str) {}

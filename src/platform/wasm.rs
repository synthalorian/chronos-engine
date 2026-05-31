//! WASM / Browser platform implementation.
//!
//! Provides directory, dialog, and message-box helpers for the web target.
//! Browser security doesn't allow arbitrary filesystem access or native
//! dialogs, so these are stubs or localStorage/IndexedDB-based.

use std::path::PathBuf;

// ── Directories (stubs — browsers don't have a real filesystem) ──────

pub fn home_dir() -> Option<PathBuf> {
    // Browsers have no home directory. Return a synthetic path in
    // localStorage ("virtual home") so engine code doesn't panic.
    None
}

pub fn config_dir() -> Option<PathBuf> {
    None
}

pub fn data_dir() -> Option<PathBuf> {
    None
}

pub fn cache_dir() -> Option<PathBuf> {
    None
}

// ── Dialog stubs (browsers don't support native dialogs) ────────────

pub fn open_file_dialog(_title: &str) -> Option<PathBuf> {
    None
}

pub fn save_file_dialog(_title: &str, _default_name: &str) -> Option<PathBuf> {
    None
}

pub fn open_dir_dialog(_title: &str) -> Option<PathBuf> {
    None
}

pub fn message_box(_title: &str, _message: &str) {
    // Browser equivalent would be alert/console.log, but we stay silent
    // to avoid annoying popups. Log to console in debug builds.
    #[cfg(debug_assertions)]
    web_sys::console::log_1(&format!("[chronos] {}: {}", _title, _message).into());
}

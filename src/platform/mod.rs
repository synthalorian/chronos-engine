//! Cross-platform abstractions for directories, file dialogs, and OS paths.
//!
//! Provides helpers for locating user directories (home, config, data, cache)
//! and platform-native file dialogs. All functions are `no_std`-friendly where
//! possible and use `std::env` / conditional compilation rather than external
//! crates to keep the core dependency-free.
//!
//! # Usage
//! ```ignore
//! use chronos_engine::platform::{home_dir, config_dir, open_dir_dialog};
//!
//! if let Some(home) = home_dir() {
//!     println!("Home: {}", home.display());
//! }
//!
//! if let Some(path) = open_dir_dialog("Select project folder") {
//!     println!("Selected: {}", path.display());
//! }
//! ```

use std::path::PathBuf;

// ── Internal platform implementations ─────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
use wasm as imp;

#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
mod windows;
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
use windows as imp;

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
mod macos;
#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
use macos as imp;

#[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
mod linux;
#[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
use linux as imp;

#[cfg(all(not(target_arch = "wasm32"), not(any(target_os = "windows", target_os = "macos", target_os = "linux"))))]
mod fallback;
#[cfg(all(not(target_arch = "wasm32"), not(any(target_os = "windows", target_os = "macos", target_os = "linux"))))]
use fallback as imp;

// ── Public API ────────────────────────────────────────────────────────

/// Return the user's home directory.
///
/// | Platform | Source |
/// |----------|--------|
/// | Linux    | `$HOME` |
/// | macOS    | `$HOME` |
/// | Windows  | `%USERPROFILE%` |
pub fn home_dir() -> Option<PathBuf> {
    imp::home_dir()
}

/// Return the user's config directory for Chronos Engine.
///
/// | Platform | Path |
/// |----------|------|
/// | Linux    | `$XDG_CONFIG_HOME/chronos-engine` or `~/.config/chronos-engine` |
/// | macOS    | `~/Library/Application Support/chronos-engine` |
/// | Windows  | `%APPDATA%/chronos-engine` |
pub fn config_dir() -> Option<PathBuf> {
    imp::config_dir().map(|p| p.join("chronos-engine"))
}

/// Return the user's data directory for Chronos Engine.
///
/// | Platform | Path |
/// |----------|------|
/// | Linux    | `$XDG_DATA_HOME/chronos-engine` or `~/.local/share/chronos-engine` |
/// | macOS    | `~/Library/Application Support/chronos-engine` |
/// | Windows  | `%APPDATA%/chronos-engine` |
pub fn data_dir() -> Option<PathBuf> {
    imp::data_dir().map(|p| p.join("chronos-engine"))
}

/// Return the user's cache directory for Chronos Engine.
///
/// | Platform | Path |
/// |----------|------|
/// | Linux    | `$XDG_CACHE_HOME/chronos-engine` or `~/.cache/chronos-engine` |
/// | macOS    | `~/Library/Caches/chronos-engine` |
/// | Windows  | `%LOCALAPPDATA%/chronos-engine/Cache` |
pub fn cache_dir() -> Option<PathBuf> {
    imp::cache_dir().map(|p| p.join("chronos-engine"))
}

/// Return a temporary directory for the engine.
///
/// Uses `std::env::temp_dir()` with a `chronos-engine` subdirectory.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("chronos-engine")
}

// ── Dialog stubs (backed by optional `rfd` feature in future) ────────

/// Show a native file-open dialog.
///
/// Returns `None` if the user cancels or if dialogs are unsupported on the
/// current platform (e.g. headless servers).
///
/// The `title` is shown as the dialog window caption.
pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    imp::open_file_dialog(title)
}

/// Show a native file-save dialog.
///
/// `default_name` is pre-filled as the suggested filename.
pub fn save_file_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    imp::save_file_dialog(title, default_name)
}

/// Show a native directory-open dialog.
///
/// `title` is shown as the dialog window caption.
pub fn open_dir_dialog(title: &str) -> Option<PathBuf> {
    imp::open_dir_dialog(title)
}

/// Show a message box with an OK button.
///
/// This is a no-op on headless targets.
pub fn message_box(title: &str, message: &str) {
    imp::message_box(title, message);
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_dir_returns_some() {
        assert!(home_dir().is_some(), "home_dir should return Some on all supported platforms");
    }

    #[test]
    fn config_dir_contains_engine_name() {
        if let Some(dir) = config_dir() {
            let file_name = dir.file_name().unwrap().to_str().unwrap();
            assert_eq!(file_name, "chronos-engine");
        }
    }

    #[test]
    fn data_dir_contains_engine_name() {
        if let Some(dir) = data_dir() {
            let file_name = dir.file_name().unwrap().to_str().unwrap();
            assert_eq!(file_name, "chronos-engine");
        }
    }

    #[test]
    fn cache_dir_contains_engine_name() {
        if let Some(dir) = cache_dir() {
            let file_name = dir.file_name().unwrap().to_str().unwrap();
            assert_eq!(file_name, "chronos-engine");
        }
    }

    #[test]
    fn temp_dir_contains_engine_name() {
        let dir = temp_dir();
        let file_name = dir.file_name().unwrap().to_str().unwrap();
        assert_eq!(file_name, "chronos-engine");
    }

    #[test]
    fn dialog_stubs_return_none_or_some() {
        // These are allowed to return None on headless CI, but should not panic.
        let _ = open_file_dialog("Test");
        let _ = save_file_dialog("Test", "file.txt");
        let _ = open_dir_dialog("Test");
        message_box("Test", "Hello");
    }
}

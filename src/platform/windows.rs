//! Windows platform helpers — known folders, COM dialogs.

use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

pub fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("Cache"))
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    #[cfg(feature = "dialogs")]
    {
        return rfd::FileDialog::new()
            .set_title(title)
            .pick_file()
            .map(std::path::PathBuf::from);
    }
    #[cfg(not(feature = "dialogs"))]
    {
        let _ = title;
        None
    }
}

pub fn save_file_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    #[cfg(feature = "dialogs")]
    {
        return rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(default_name)
            .save_file()
            .map(std::path::PathBuf::from);
    }
    #[cfg(not(feature = "dialogs"))]
    {
        let _ = (title, default_name);
        None
    }
}

pub fn open_dir_dialog(title: &str) -> Option<PathBuf> {
    #[cfg(feature = "dialogs")]
    {
        return rfd::FileDialog::new()
            .set_title(title)
            .pick_folder()
            .map(std::path::PathBuf::from);
    }
    #[cfg(not(feature = "dialogs"))]
    {
        let _ = title;
        None
    }
}

pub fn message_box(title: &str, message: &str) {
    #[cfg(feature = "dialogs")]
    {
        let _ = rfd::MessageDialog::new()
            .set_title(title)
            .set_description(message)
            .set_level(rfd::MessageLevel::Info)
            .show();
    }
    #[cfg(not(feature = "dialogs"))]
    {
        let _ = (title, message);
    }
}

//! Linux platform helpers — XDG directories, zenity/kdialog dialogs.

use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")))
}

pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".local/share")))
}

pub fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".cache")))
}

use std::sync::OnceLock;

fn has_display() -> bool {
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn has_command(name: &str) -> bool {
    static CACHE: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        let mut set = std::collections::HashSet::new();
        for cmd in &["zenity", "kdialog"] {
            if std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                set.insert(cmd.to_string());
            }
        }
        set
    });
    cache.contains(name)
}

fn zenity_file_dialog(title: &str, args: &[&str]) -> Option<PathBuf> {
    let mut cmd = std::process::Command::new("zenity");
    cmd.arg("--file-selection")
        .arg("--title")
        .arg(title);
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

fn kdialog_file_dialog(title: &str, args: &[&str]) -> Option<PathBuf> {
    let mut cmd = std::process::Command::new("kdialog");
    cmd.arg("--title").arg(title)
        .arg("--getopenfilename").arg(".");
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

pub fn open_file_dialog(title: &str) -> Option<PathBuf> {
    if !has_display() {
        return None;
    }
    if has_command("zenity") {
        zenity_file_dialog(title, &[])
    } else if has_command("kdialog") {
        kdialog_file_dialog(title, &[])
    } else {
        None
    }
}

fn kdialog_save_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("kdialog")
        .arg("--title").arg(title)
        .arg("--getsavefilename")
        .arg(format!("./{default_name}"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

fn kdialog_dir_dialog(title: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("kdialog")
        .arg("--title").arg(title)
        .arg("--getexistingdirectory")
        .arg(".")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

pub fn save_file_dialog(title: &str, default_name: &str) -> Option<PathBuf> {
    if !has_display() {
        return None;
    }
    if has_command("zenity") {
        zenity_file_dialog(title, &["--save"])
    } else if has_command("kdialog") {
        kdialog_save_dialog(title, default_name)
    } else {
        None
    }
}

pub fn open_dir_dialog(title: &str) -> Option<PathBuf> {
    if !has_display() {
        return None;
    }
    if has_command("zenity") {
        zenity_file_dialog(title, &["--directory"])
    } else if has_command("kdialog") {
        kdialog_dir_dialog(title)
    } else {
        None
    }
}

pub fn message_box(title: &str, message: &str) {
    if !has_display() {
        return;
    }
    if has_command("zenity") {
        let _ = std::process::Command::new("zenity")
            .arg("--info")
            .arg("--title")
            .arg(title)
            .arg("--text")
            .arg(message)
            .output();
    } else if has_command("kdialog") {
        let _ = std::process::Command::new("kdialog")
            .arg("--msgbox")
            .arg(message)
            .arg("--title")
            .arg(title)
            .output();
    }
}

//! Script hot-reload watcher for the Chronos Engine.
//!
//! Polling-based file watcher that detects script changes by comparing
//! timestamps. Supports in-memory script loading for testing and
//! configurable reload policies (`Immediate`, `Debounced`, `Manual`).
//!
//! Unlike the `Asset` hot-reload system (which uses the `notify` crate),
//! this module is entirely self-contained and uses a monotonically
//! increasing timestamp counter for deterministic change detection.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

// ──────────────────────────────────────────────
// Timestamp helper
// ──────────────────────────────────────────────

/// Monotonically increasing counter used as a stand-in for wall-clock time.
///
/// Using a counter instead of `SystemTime::now()` guarantees deterministic
/// ordering in tests — every call returns a strictly greater value.
static CURRENT_TIMESTAMP: AtomicU64 = AtomicU64::new(1);

/// Returns the next monotonically increasing timestamp.
fn next_timestamp() -> u64 {
    CURRENT_TIMESTAMP.fetch_add(1, Ordering::Relaxed)
}

// ──────────────────────────────────────────────
// ScriptFile
// ──────────────────────────────────────────────

/// Represents a script file, either loaded from disk or created in-memory.
#[derive(Debug, Clone)]
pub struct ScriptFile {
    /// File path on disk (empty string for in-memory scripts).
    pub path: String,
    /// Script name (filename without extension).
    pub name: String,
    /// Current source code content.
    pub source: String,
    /// Timestamp of last modification (monotonically increasing).
    pub last_modified: u64,
}

impl ScriptFile {
    /// Create a `ScriptFile` by reading from a file on disk.
    ///
    /// The script name is derived from the filename stem (without extension).
    /// If the file does not exist, the source will be empty but no error is
    /// raised — callers that need guaranteed existence should use
    /// [`ScriptWatcher::load_file`] instead.
    pub fn new(path: &str) -> Self {
        let p = Path::new(path);
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let source = std::fs::read_to_string(path).unwrap_or_default();

        ScriptFile {
            path: path.to_string(),
            name,
            source,
            last_modified: next_timestamp(),
        }
    }

    /// Create a `ScriptFile` from a name and source code (in-memory).
    pub fn from_source(name: &str, source: &str) -> Self {
        ScriptFile {
            path: String::new(),
            name: name.to_string(),
            source: source.to_string(),
            last_modified: next_timestamp(),
        }
    }
}

// ──────────────────────────────────────────────
// ScriptChange
// ──────────────────────────────────────────────

/// Describes a detected change to a script file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptChange {
    /// A new script was detected.
    Created {
        name: String,
    },
    /// An existing script was modified.
    Modified {
        name: String,
    },
    /// A script was removed.
    Deleted {
        name: String,
    },
}

// ──────────────────────────────────────────────
// ScriptWatchError
// ──────────────────────────────────────────────

/// Errors that can occur during script watching operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptWatchError {
    /// The specified file was not found on disk.
    FileNotFound(String),
    /// An I/O error occurred while reading a file.
    IoError(String),
    /// The script name derived from the path is invalid.
    InvalidName(String),
}

impl fmt::Display for ScriptWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptWatchError::FileNotFound(path) => {
                write!(f, "script file not found: {}", path)
            }
            ScriptWatchError::IoError(msg) => {
                write!(f, "script I/O error: {}", msg)
            }
            ScriptWatchError::InvalidName(name) => {
                write!(f, "invalid script name: {}", name)
            }
        }
    }
}

// ──────────────────────────────────────────────
// ScriptWatcher
// ──────────────────────────────────────────────

/// Polls for script file changes by comparing timestamps.
///
/// Tracks loaded scripts and detects changes between calls to
/// [`ScriptWatcher::check_changes`]. The first call after loading a script
/// reports it as `Created`; subsequent calls report `Modified` if the source
/// was updated (e.g. via [`ScriptWatcher::update_source`]) or `Deleted` if
/// the script was removed.
#[derive(Debug, Clone)]
pub struct ScriptWatcher {
    /// Loaded scripts indexed by name.
    scripts: HashMap<String, ScriptFile>,
    /// Directories registered for watching (used for bookkeeping).
    watch_directories: Vec<String>,
    /// Snapshot of timestamps from the last `check_changes` call.
    snapshot: HashMap<String, u64>,
}

impl ScriptWatcher {
    /// Create a new empty watcher.
    pub fn new() -> Self {
        ScriptWatcher {
            scripts: HashMap::new(),
            watch_directories: Vec::new(),
            snapshot: HashMap::new(),
        }
    }

    /// Add a directory to the watch list.
    ///
    /// Duplicates are silently ignored.
    pub fn watch_directory(&mut self, dir: &str) {
        if !self.watch_directories.contains(&dir.to_string()) {
            self.watch_directories.push(dir.to_string());
        }
    }

    /// Load a script directly from a name and source string.
    ///
    /// If a script with the same name already exists it is replaced.
    pub fn load_script(&mut self, name: &str, source: &str) {
        let file = ScriptFile::from_source(name, source);
        self.scripts.insert(name.to_string(), file);
    }

    /// Load a script from a file path.
    ///
    /// Returns an error if the file does not exist, cannot be read, or has
    /// no valid filename stem.
    pub fn load_file(&mut self, path: &str) -> Result<(), ScriptWatchError> {
        let p = Path::new(path);

        if !p.exists() {
            return Err(ScriptWatchError::FileNotFound(path.to_string()));
        }

        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        if name.is_empty() || name == "unknown" {
            return Err(ScriptWatchError::InvalidName(path.to_string()));
        }

        let source = std::fs::read_to_string(path)
            .map_err(|e| ScriptWatchError::IoError(e.to_string()))?;

        let mut file = ScriptFile::from_source(&name, &source);
        file.path = path.to_string();

        // Prefer actual file modification time when available.
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    file.last_modified = duration.as_millis() as u64;
                }
            }
        }

        self.scripts.insert(name, file);
        Ok(())
    }

    /// Get the current source code for a script by name.
    pub fn get_source(&self, name: &str) -> Option<&str> {
        self.scripts.get(name).map(|f| f.source.as_str())
    }

    /// Poll for changes since the last call.
    ///
    /// Compares each script's current `last_modified` timestamp against the
    /// snapshot from the previous check:
    ///
    /// | current state    | snapshot state | result     |
    /// |------------------|----------------|------------|
    /// | present          | absent         | `Created`  |
    /// | present, changed | present        | `Modified` |
    /// | absent           | present        | `Deleted`  |
    pub fn check_changes(&mut self) -> Vec<ScriptChange> {
        let mut changes = Vec::new();

        // Detect created and modified scripts.
        for (name, file) in &self.scripts {
            match self.snapshot.get(name) {
                None => {
                    changes.push(ScriptChange::Created {
                        name: name.clone(),
                    });
                }
                Some(&prev_ts) if file.last_modified != prev_ts => {
                    changes.push(ScriptChange::Modified {
                        name: name.clone(),
                    });
                }
                _ => {}
            }
        }

        // Detect deleted scripts.
        for name in self.snapshot.keys() {
            if !self.scripts.contains_key(name) {
                changes.push(ScriptChange::Deleted {
                    name: name.clone(),
                });
            }
        }

        // Refresh the snapshot.
        self.snapshot.clear();
        for (name, file) in &self.scripts {
            self.snapshot.insert(name.clone(), file.last_modified);
        }

        changes
    }

    /// Update the source code of an existing script (simulates hot-reload).
    ///
    /// Sets a new modification timestamp so the next call to
    /// [`ScriptWatcher::check_changes`] will report a `Modified` change.
    ///
    /// Returns `true` if the script existed and was updated, `false` otherwise.
    pub fn update_source(&mut self, name: &str, new_source: &str) -> bool {
        if let Some(file) = self.scripts.get_mut(name) {
            file.source = new_source.to_string();
            file.last_modified = next_timestamp();
            true
        } else {
            false
        }
    }

    /// Remove a script by name.
    ///
    /// Returns `true` if the script existed and was removed.
    pub fn remove_script(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    /// List all loaded script names in sorted order.
    pub fn list_scripts(&self) -> Vec<String> {
        let mut names: Vec<String> = self.scripts.keys().cloned().collect();
        names.sort();
        names
    }

    /// Number of currently loaded scripts.
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    /// Check whether a script with the given name is loaded.
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }
}

impl Default for ScriptWatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// ReloadPolicy
// ──────────────────────────────────────────────

/// Controls how hot-reload behaves when changes are detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadPolicy {
    /// Reload immediately when a change is detected.
    Immediate,
    /// Wait for changes to settle before reloading (delay in milliseconds).
    Debounced {
        delay_ms: u64,
    },
    /// Only reload when explicitly requested via
    /// [`ScriptReloader::force_reload`].
    Manual,
}

// ──────────────────────────────────────────────
// ScriptReloader
// ──────────────────────────────────────────────

/// Manages the script reload process with a configurable [`ReloadPolicy`].
///
/// Wraps a [`ScriptWatcher`] and decides *when* to surface changed scripts
/// based on the active policy.
#[derive(Debug, Clone)]
pub struct ScriptReloader {
    watcher: ScriptWatcher,
    policy: ReloadPolicy,
    last_check_time: u64,
    /// Scripts queued for reload (used by `Debounced` and `Manual` policies).
    pending_reloads: Vec<String>,
    /// Total number of reloads performed.
    reload_count: u64,
}

impl ScriptReloader {
    /// Create a new reloader with the given watcher and policy.
    pub fn new(watcher: ScriptWatcher, policy: ReloadPolicy) -> Self {
        ScriptReloader {
            watcher,
            policy,
            last_check_time: 0,
            pending_reloads: Vec::new(),
            reload_count: 0,
        }
    }

    /// Poll the watcher for changes and return names of scripts that should
    /// be reloaded according to the active policy.
    ///
    /// - **Immediate**: returns every changed script right away.
    /// - **Debounced**: returns changed scripts only after the configured
    ///   delay has elapsed since the last check; otherwise they are queued in
    ///   `pending_reloads`.
    /// - **Manual**: never returns changed scripts from `poll` — they are
    ///   always queued for explicit retrieval via
    ///   [`ScriptReloader::force_reload`] or
    ///   [`ScriptReloader::get_reloaded_sources`].
    pub fn poll(&mut self) -> Vec<String> {
        let changes = self.watcher.check_changes();
        let now = next_timestamp();

        let mut reloaded = Vec::new();

        for change in changes {
            let name = match &change {
                ScriptChange::Created { name } | ScriptChange::Modified { name } => name.clone(),
                ScriptChange::Deleted { name } => {
                    // Clean up any pending entry for deleted scripts.
                    self.pending_reloads.retain(|n| n != name);
                    continue;
                }
            };

            match &self.policy {
                ReloadPolicy::Immediate => {
                    self.reload_count += 1;
                    reloaded.push(name);
                }
                ReloadPolicy::Debounced { delay_ms } => {
                    if self.last_check_time == 0 || now >= self.last_check_time + *delay_ms {
                        self.reload_count += 1;
                        reloaded.push(name);
                    } else if !self.pending_reloads.contains(&name) {
                        self.pending_reloads.push(name);
                    }
                }
                ReloadPolicy::Manual => {
                    if !self.pending_reloads.contains(&name) {
                        self.pending_reloads.push(name);
                    }
                }
            }
        }

        self.last_check_time = now;
        reloaded
    }

    /// Force a reload of a specific script by name.
    ///
    /// Returns `true` if the script exists in the watcher, `false` otherwise.
    /// Removes the script from the pending queue and increments the reload
    /// counter.
    pub fn force_reload(&mut self, name: &str) -> bool {
        if self.watcher.has_script(name) {
            self.reload_count += 1;
            self.pending_reloads.retain(|n| n != name);
            true
        } else {
            false
        }
    }

    /// Drain pending reloads and return `(name, source)` pairs.
    ///
    /// Primarily useful with `Debounced` and `Manual` policies where changed
    /// scripts are queued rather than returned immediately by
    /// [`ScriptReloader::poll`].
    pub fn get_reloaded_sources(&mut self) -> Vec<(String, String)> {
        let pending = std::mem::take(&mut self.pending_reloads);
        let mut result = Vec::new();

        for name in &pending {
            if let Some(source) = self.watcher.get_source(name) {
                result.push((name.clone(), source.to_string()));
            }
        }

        result
    }

    /// Total number of reloads performed since the reloader was created.
    pub fn reload_count(&self) -> u64 {
        self.reload_count
    }

    /// Number of scripts currently queued for reload.
    pub fn pending_count(&self) -> usize {
        self.pending_reloads.len()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: ScriptFile creation from source ──

    #[test]
    fn script_file_from_source() {
        let file = ScriptFile::from_source("test_script", "print('hello')");
        assert_eq!(file.name, "test_script");
        assert_eq!(file.source, "print('hello')");
        assert!(file.path.is_empty());
        assert!(file.last_modified > 0);
    }

    // ── Test 2: ScriptFile from_source vs new ──

    #[test]
    fn script_file_from_source_vs_new() {
        let in_mem = ScriptFile::from_source("memory", "let x = 1;");
        assert!(in_mem.path.is_empty());
        assert_eq!(in_mem.name, "memory");
        assert_eq!(in_mem.source, "let x = 1;");

        // new() on a nonexistent file produces empty source but keeps the path.
        let from_disk = ScriptFile::new("/tmp/__chronos_nonexistent_xyz__.rhai");
        assert_eq!(from_disk.source, "");
        assert!(!from_disk.path.is_empty());
    }

    // ── Test 3: ScriptChange variants ──

    #[test]
    fn script_change_variants() {
        let created = ScriptChange::Created {
            name: "foo".to_string(),
        };
        let modified = ScriptChange::Modified {
            name: "bar".to_string(),
        };
        let deleted = ScriptChange::Deleted {
            name: "baz".to_string(),
        };

        assert_eq!(
            created,
            ScriptChange::Created {
                name: "foo".to_string()
            }
        );
        assert_eq!(
            modified,
            ScriptChange::Modified {
                name: "bar".to_string()
            }
        );
        assert_eq!(
            deleted,
            ScriptChange::Deleted {
                name: "baz".to_string()
            }
        );

        // Verify inequality across variants.
        assert_ne!(created, modified);
        assert_ne!(modified, deleted);
    }

    // ── Test 4: ScriptWatcher creation ──

    #[test]
    fn script_watcher_creation() {
        let watcher = ScriptWatcher::new();
        assert_eq!(watcher.script_count(), 0);
        assert!(watcher.list_scripts().is_empty());
    }

    // ── Test 5: ScriptWatcher load_script and get_source ──

    #[test]
    fn watcher_load_script_and_get_source() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("player", "fn update() { }");

        assert_eq!(watcher.get_source("player"), Some("fn update() { }"));
        assert_eq!(watcher.get_source("nonexistent"), None);

        // Overwriting replaces the source.
        watcher.load_script("player", "fn update() { /* v2 */ }");
        assert!(watcher.get_source("player").unwrap().contains("v2"));
    }

    // ── Test 6: ScriptWatcher update_source detects Modified change ──

    #[test]
    fn watcher_update_source_detects_modification() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("enemy", "fn ai() { }");

        // First check: newly loaded script is Created.
        let changes = watcher.check_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            ScriptChange::Created { name } if name == "enemy"
        ));

        // No more changes until something happens.
        let changes = watcher.check_changes();
        assert!(changes.is_empty());

        // Update source.
        let updated = watcher.update_source("enemy", "fn ai() { /* improved */ }");
        assert!(updated);
        assert!(watcher.get_source("enemy").unwrap().contains("improved"));

        // Change detected as Modified.
        let changes = watcher.check_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            ScriptChange::Modified { name } if name == "enemy"
        ));

        // Updating a nonexistent script returns false.
        assert!(!watcher.update_source("ghost", "nope"));
    }

    // ── Test 7: ScriptWatcher remove_script ──

    #[test]
    fn watcher_remove_script() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("npc", "fn talk() { }");

        // Snapshot the loaded state.
        let _ = watcher.check_changes();

        // Remove the script.
        assert!(watcher.remove_script("npc"));
        assert!(!watcher.has_script("npc"));

        // Double-remove returns false.
        assert!(!watcher.remove_script("npc"));

        // Change detected as Deleted.
        let changes = watcher.check_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            ScriptChange::Deleted { name } if name == "npc"
        ));
    }

    // ── Test 8: ScriptWatcher list_scripts and count ──

    #[test]
    fn watcher_list_scripts_and_count() {
        let mut watcher = ScriptWatcher::new();
        assert_eq!(watcher.script_count(), 0);

        watcher.load_script("z_script", "let z = 0;");
        watcher.load_script("a_script", "let a = 1;");
        watcher.load_script("m_script", "let m = 2;");

        assert_eq!(watcher.script_count(), 3);

        let list = watcher.list_scripts();
        assert_eq!(list, vec!["a_script", "m_script", "z_script"]);
    }

    // ── Test 9: ScriptWatcher has_script ──

    #[test]
    fn watcher_has_script() {
        let mut watcher = ScriptWatcher::new();
        assert!(!watcher.has_script("absent"));

        watcher.load_script("present", "print(42);");
        assert!(watcher.has_script("present"));
        assert!(!watcher.has_script("absent"));
    }

    // ── Test 10: ReloadPolicy variants ──

    #[test]
    fn reload_policy_variants() {
        assert_eq!(ReloadPolicy::Immediate, ReloadPolicy::Immediate);
        assert_eq!(
            ReloadPolicy::Debounced { delay_ms: 100 },
            ReloadPolicy::Debounced { delay_ms: 100 }
        );
        assert_ne!(
            ReloadPolicy::Debounced { delay_ms: 50 },
            ReloadPolicy::Debounced { delay_ms: 100 }
        );
        assert_eq!(ReloadPolicy::Manual, ReloadPolicy::Manual);
    }

    // ── Test 11: ScriptReloader creation ──

    #[test]
    fn script_reloader_creation() {
        let watcher = ScriptWatcher::new();
        let reloader = ScriptReloader::new(watcher, ReloadPolicy::Immediate);
        assert_eq!(reloader.reload_count(), 0);
        assert_eq!(reloader.pending_count(), 0);
    }

    // ── Test 12: ScriptReloader poll returns changed scripts ──

    #[test]
    fn reloader_poll_returns_changed_scripts() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("player", "fn update() {}");
        watcher.load_script("enemy", "fn ai() {}");

        let mut reloader = ScriptReloader::new(watcher, ReloadPolicy::Immediate);

        // First poll: both scripts are new.
        let changed = reloader.poll();
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&"player".to_string()));
        assert!(changed.contains(&"enemy".to_string()));
        assert_eq!(reloader.reload_count(), 2);

        // Subsequent poll: nothing changed.
        let changed = reloader.poll();
        assert!(changed.is_empty());
        assert_eq!(reloader.reload_count(), 2);
    }

    // ── Test 13: ScriptReloader force_reload ──

    #[test]
    fn reloader_force_reload() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("weapon", "fn fire() {}");

        let mut reloader = ScriptReloader::new(watcher, ReloadPolicy::Manual);

        // Drain the initial Created change (Manual queues it but doesn't reload).
        let _ = reloader.poll();
        assert_eq!(reloader.pending_count(), 1);

        // Force-reload the existing script.
        assert!(reloader.force_reload("weapon"));
        assert_eq!(reloader.reload_count(), 1);
        assert_eq!(reloader.pending_count(), 0);

        // Force-reload a nonexistent script.
        assert!(!reloader.force_reload("phantom"));
        assert_eq!(reloader.reload_count(), 1);
    }

    // ── Test 14: ScriptReloader get_reloaded_sources ──

    #[test]
    fn reloader_get_reloaded_sources() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("spell", "fn cast() {}");

        let mut reloader = ScriptReloader::new(watcher, ReloadPolicy::Manual);

        // poll() under Manual policy queues "spell" but does not reload.
        let _ = reloader.poll();
        assert_eq!(reloader.pending_count(), 1);

        // get_reloaded_sources drains pending and returns (name, source).
        let sources = reloader.get_reloaded_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].0, "spell");
        assert!(sources[0].1.contains("cast"));

        // Pending is now drained.
        assert_eq!(reloader.pending_count(), 0);

        // Calling again yields nothing.
        let sources = reloader.get_reloaded_sources();
        assert!(sources.is_empty());
    }

    // ── Test 15: ScriptWatchError display ──

    #[test]
    fn script_watch_error_display() {
        let not_found = ScriptWatchError::FileNotFound("scripts/main.rhai".to_string());
        let displayed = not_found.to_string();
        assert!(displayed.contains("scripts/main.rhai"));
        assert!(displayed.contains("not found"));

        let io_err = ScriptWatchError::IoError("permission denied".to_string());
        let displayed = io_err.to_string();
        assert!(displayed.contains("permission denied"));
        assert!(displayed.contains("I/O"));

        let invalid = ScriptWatchError::InvalidName("".to_string());
        let displayed = invalid.to_string();
        assert!(displayed.contains("invalid"));
    }

    // ── Test 16: Debounced policy — first poll returns immediately ──

    #[test]
    fn reloader_debounced_first_poll_returns_immediately() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("particle", "fn emit() {}");

        let policy = ReloadPolicy::Debounced { delay_ms: 999_999 };
        let mut reloader = ScriptReloader::new(watcher, policy);

        // last_check_time starts at 0, so the first poll should fire immediately.
        let changed = reloader.poll();
        assert_eq!(changed.len(), 1);
        assert!(changed.contains(&"particle".to_string()));
        assert_eq!(reloader.reload_count(), 1);
    }

    // ── Test 17: Watcher handles multiple updates to the same script ──

    #[test]
    fn watcher_multiple_updates_same_script() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("game", "fn init() {}");

        // Establish the snapshot.
        let _ = watcher.check_changes();

        // Two rapid updates — only one Modified change should be reported.
        watcher.update_source("game", "fn init() { /* v2 */ }");
        watcher.update_source("game", "fn init() { /* v3 */ }");

        let changes = watcher.check_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            ScriptChange::Modified { name } if name == "game"
        ));

        // Source reflects the latest update.
        assert!(watcher.get_source("game").unwrap().contains("v3"));
    }

    // ── Test 18: Manual policy queues but does not return from poll ──

    #[test]
    fn reloader_manual_queues_without_returning() {
        let mut watcher = ScriptWatcher::new();
        watcher.load_script("quest", "fn start() {}");

        let mut reloader = ScriptReloader::new(watcher, ReloadPolicy::Manual);

        let changed = reloader.poll();
        assert!(changed.is_empty(), "Manual policy should not return from poll");
        assert_eq!(reloader.pending_count(), 1);

        // Force-reload dequeues it.
        assert!(reloader.force_reload("quest"));
        assert_eq!(reloader.pending_count(), 0);
        assert_eq!(reloader.reload_count(), 1);
    }

    // ── Test 19: ScriptWatcher default trait ──

    #[test]
    fn script_watcher_default() {
        let watcher = ScriptWatcher::default();
        assert_eq!(watcher.script_count(), 0);
    }

    // ── Test 20: watch_directory deduplication ──

    #[test]
    fn watcher_watch_directory_deduplicates() {
        let mut watcher = ScriptWatcher::new();
        watcher.watch_directory("scripts/");
        watcher.watch_directory("scripts/");
        watcher.watch_directory("mods/");

        // Only two unique directories should be tracked.
        let count = watcher
            .watch_directories
            .iter()
            .filter(|d| *d == "scripts/")
            .count();
        assert_eq!(count, 1);
        assert_eq!(watcher.watch_directories.len(), 2);
    }
}

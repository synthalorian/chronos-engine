//! Asset Hot-Reload — watches source asset files and re-imports them when
//! they change.
//!
//! Uses a lightweight polling-based approach (file modification timestamps)
//! so it works without additional dependencies. For projects that also
//! enable the `dev-tools` feature, `notify`-based watching is available
//! in `asset::HotReloadWatcher`; this module provides the same capability
//! specifically for the Phase-10 import pipeline.

#![cfg(feature = "asset-pipeline")]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::metadata::{MetaError, MetaManager};
use super::processor::{AssetProcessor, ProcessedAsset, ProcessorError};

// ──────────────────────────────────────────────────────────────
// Reload Policy
// ──────────────────────────────────────────────────────────────

/// Controls how aggressively the watcher triggers re-imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadPolicy {
    /// Re-import on every detected change.
    Immediate,
    /// Collect changes and process them together on demand.
    Coalesced,
    /// Only mark as changed; caller decides when to process.
    Manual,
}

// ──────────────────────────────────────────────────────────────
// Watched Asset
// ──────────────────────────────────────────────────────────────

/// Internal state for a single watched file.
#[derive(Debug, Clone)]
struct WatchedAsset {
    path: PathBuf,
    last_modified: u64,
    last_checked: u64,
    changed: bool,
}

// ──────────────────────────────────────────────────────────────
// Watcher Error
// ──────────────────────────────────────────────────────────────

/// Errors from the hot-reload watcher.
#[derive(Debug)]
pub enum WatcherError {
    Io(std::io::Error),
    Meta(MetaError),
    Processor(ProcessorError),
}

impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherError::Io(e) => write!(f, "watcher I/O error: {}", e),
            WatcherError::Meta(e) => write!(f, "watcher meta error: {}", e),
            WatcherError::Processor(e) => write!(f, "watcher processor error: {}", e),
        }
    }
}

impl std::error::Error for WatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WatcherError::Io(e) => Some(e),
            WatcherError::Meta(e) => Some(e),
            WatcherError::Processor(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for WatcherError {
    fn from(e: std::io::Error) -> Self {
        WatcherError::Io(e)
    }
}

impl From<MetaError> for WatcherError {
    fn from(e: MetaError) -> Self {
        WatcherError::Meta(e)
    }
}

impl From<ProcessorError> for WatcherError {
    fn from(e: ProcessorError) -> Self {
        WatcherError::Processor(e)
    }
}

// ──────────────────────────────────────────────────────────────
// AssetWatcher
// ──────────────────────────────────────────────────────────────

/// Polls source asset files for changes and drives the import pipeline
/// to re-process anything that has been modified.
///
/// # Usage
///
/// ```ignore
/// use chronos_engine::import::hotreload::{AssetWatcher, ReloadPolicy};
/// let mut watcher = AssetWatcher::new(ReloadPolicy::Immediate);
/// watcher.watch_dir(Path::new("assets"));
///
/// // In your game loop / editor update:
/// let changes = watcher.poll(&mut processor)?;
/// ```
pub struct AssetWatcher {
    policy: ReloadPolicy,
    watched: HashMap<PathBuf, WatchedAsset>,
    meta_manager: MetaManager,
    poll_interval: Duration,
    last_poll: u64,
}

impl AssetWatcher {
    /// Create a new watcher with the given reload policy.
    pub fn new(policy: ReloadPolicy) -> Self {
        AssetWatcher {
            policy,
            watched: HashMap::new(),
            meta_manager: MetaManager::new(),
            poll_interval: Duration::from_secs(1),
            last_poll: 0,
        }
    }

    /// Set the minimum time between polls (default: 1s).
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Recursively scan a directory and register every asset file for
    /// watching. Existing watches are updated; new files are added.
    pub fn watch_dir(&mut self, dir: &Path) -> Result<usize, WatcherError> {
        let metas = self.meta_manager.scan_directory(dir)?;
        let mut added = 0;

        for meta in metas {
            let path = meta.source_path.clone();
            let mtime = file_modified_timestamp(&path).unwrap_or(0);

            self.watched.insert(
                path.clone(),
                WatchedAsset {
                    path,
                    last_modified: mtime,
                    last_checked: now_timestamp(),
                    changed: false,
                },
            );
            added += 1;
        }

        Ok(added)
    }

    /// Add a single file to the watch list.
    pub fn watch_file(&mut self, path: &Path) -> Result<(), WatcherError> {
        let mtime = file_modified_timestamp(path).unwrap_or(0);
        self.watched.insert(
            path.to_path_buf(),
            WatchedAsset {
                path: path.to_path_buf(),
                last_modified: mtime,
                last_checked: now_timestamp(),
                changed: false,
            },
        );
        Ok(())
    }

    /// Remove a file from the watch list.
    pub fn unwatch(&mut self, path: &Path) {
        self.watched.remove(path);
    }

    /// Poll all watched files for changes.
    ///
    /// Returns a list of paths that have changed since the last poll.
    /// With [`ReloadPolicy::Immediate`], changed assets are automatically
    /// re-processed via the provided [`AssetProcessor`].
    pub fn poll(
        &mut self,
        processor: &mut AssetProcessor,
    ) -> Result<Vec<ProcessedAsset>, WatcherError> {
        let now = now_timestamp();
        if now.saturating_sub(self.last_poll) < self.poll_interval.as_millis() as u64 {
            return Ok(Vec::new());
        }
        self.last_poll = now;

        let mut changed_paths = Vec::new();

        for watched in self.watched.values_mut() {
            let current_mtime = file_modified_timestamp(&watched.path).unwrap_or(0);
            watched.last_checked = now;

            if current_mtime > watched.last_modified {
                watched.last_modified = current_mtime;
                watched.changed = true;
                changed_paths.push(watched.path.clone());
            }
        }

        if changed_paths.is_empty() {
            return Ok(Vec::new());
        }

        match self.policy {
            ReloadPolicy::Immediate => {
                let mut results = Vec::with_capacity(changed_paths.len());
                for path in &changed_paths {
                    match processor.rebuild_asset(path) {
                        Ok(pa) => {
                            if let Some(w) = self.watched.get_mut(path) {
                                w.changed = false;
                            }
                            results.push(pa);
                        }
                        Err(e) => {
                            eprintln!("[AssetWatcher] Failed to reload {}: {}", path.display(), e);
                        }
                    }
                }
                Ok(results)
            }
            ReloadPolicy::Coalesced => {
                // Process all changed files in one batch.
                let mut results = Vec::with_capacity(changed_paths.len());
                for path in &changed_paths {
                    match processor.rebuild_asset(path) {
                        Ok(pa) => {
                            if let Some(w) = self.watched.get_mut(path) {
                                w.changed = false;
                            }
                            results.push(pa);
                        }
                        Err(e) => {
                            eprintln!("[AssetWatcher] Failed to reload {}: {}", path.display(), e);
                        }
                    }
                }
                Ok(results)
            }
            ReloadPolicy::Manual => {
                // Don't process; just return what changed. Caller must call
                // process_changed() explicitly.
                Ok(Vec::new())
            }
        }
    }

    /// Process all assets that have been marked as changed. Useful with
    /// [`ReloadPolicy::Manual`].
    pub fn process_changed(
        &mut self,
        processor: &mut AssetProcessor,
    ) -> Result<Vec<ProcessedAsset>, WatcherError> {
        let to_process: Vec<PathBuf> = self
            .watched
            .iter()
            .filter(|(_, w)| w.changed)
            .map(|(p, _)| p.clone())
            .collect();

        let mut results = Vec::with_capacity(to_process.len());
        for path in &to_process {
            match processor.rebuild_asset(path) {
                Ok(pa) => {
                    if let Some(w) = self.watched.get_mut(path) {
                        w.changed = false;
                    }
                    results.push(pa);
                }
                Err(e) => {
                    eprintln!("[AssetWatcher] Failed to reload {}: {}", path.display(), e);
                }
            }
        }
        Ok(results)
    }

    /// Return the list of paths that have changed since the last poll but
    /// have not yet been processed.
    pub fn changed_paths(&self) -> Vec<&Path> {
        self.watched
            .values()
            .filter(|w| w.changed)
            .map(|w| w.path.as_path())
            .collect()
    }

    /// Number of files currently being watched.
    pub fn watched_count(&self) -> usize {
        self.watched.len()
    }

    /// Clear all watches.
    pub fn clear(&mut self) {
        self.watched.clear();
    }
}

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

fn file_modified_timestamp(path: &Path) -> Result<u64, std::io::Error> {
    let meta = fs::metadata(path)?;
    let modified = meta.modified()?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64)
}

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "asset-pipeline"))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(format!("/tmp/chronos_hotreload_tests_{}", id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    // Test 1: new watcher starts empty.
    #[test]
    fn new_watcher_empty() {
        let watcher = AssetWatcher::new(ReloadPolicy::Immediate);
        assert_eq!(watcher.watched_count(), 0);
        assert!(watcher.changed_paths().is_empty());
    }

    // Test 2: watch_file registers a file.
    #[test]
    fn watch_file_increments_count() {
        let dir = test_dir();
        let path = dir.join("test.bin");
        fs::write(&path, b"v1").unwrap();

        let mut watcher = AssetWatcher::new(ReloadPolicy::Immediate);
        watcher.watch_file(&path).expect("watch");
        assert_eq!(watcher.watched_count(), 1);
    }

    // Test 3: poll detects a modified file.
    #[test]
    fn poll_detects_modification() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([255, 0, 0, 255]));
        let path = source.join("watch.png");
        img.save(&path).unwrap();

        let mut watcher =
            AssetWatcher::new(ReloadPolicy::Manual).with_poll_interval(Duration::from_millis(0));
        watcher.watch_file(&path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);

        // First poll — no changes yet.
        let first = watcher.poll(&mut proc).expect("poll");
        assert!(first.is_empty());

        // Modify the file.
        thread::sleep(Duration::from_millis(50));
        let img2 = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([0, 255, 0, 255]));
        img2.save(&path).unwrap();

        // Second poll — should detect change.
        let second = watcher.poll(&mut proc).expect("poll");
        // With Manual policy, poll doesn't process, just marks.
        assert!(second.is_empty());
        assert_eq!(watcher.changed_paths().len(), 1);
    }

    // Test 4: Immediate policy processes on poll.
    #[test]
    fn immediate_policy_processes() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([255, 0, 0, 255]));
        let path = source.join("immed.png");
        img.save(&path).unwrap();

        let mut watcher =
            AssetWatcher::new(ReloadPolicy::Immediate).with_poll_interval(Duration::from_millis(0));
        watcher.watch_file(&path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        // Initial process.
        let _ = watcher.poll(&mut proc).expect("poll");

        // Modify.
        thread::sleep(Duration::from_millis(50));
        let img2 = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([0, 0, 255, 255]));
        img2.save(&path).unwrap();

        let results = watcher.poll(&mut proc).expect("poll");
        assert_eq!(results.len(), 1);
        assert!(results[0].rebuilt);
    }

    // Test 5: unwatch removes file.
    #[test]
    fn unwatch_removes() {
        let dir = test_dir();
        let path = dir.join("remove.bin");
        fs::write(&path, b"x").unwrap();

        let mut watcher = AssetWatcher::new(ReloadPolicy::Immediate);
        watcher.watch_file(&path).unwrap();
        assert_eq!(watcher.watched_count(), 1);

        watcher.unwatch(&path);
        assert_eq!(watcher.watched_count(), 0);
    }

    // Test 6: watch_dir scans recursively.
    #[test]
    fn watch_dir_scans() {
        let dir = test_dir();
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(dir.join("a.png"), b"x").unwrap();
        fs::write(sub.join("b.png"), b"y").unwrap();

        let mut watcher = AssetWatcher::new(ReloadPolicy::Manual);
        let count = watcher.watch_dir(&dir).expect("watch_dir");
        assert_eq!(
            count, 2,
            "should find 2 assets (ignoring dot-dirs and .meta)"
        );
    }

    // Test 7: process_changed with Manual policy.
    #[test]
    fn process_changed_manual() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([255, 0, 0, 255]));
        let path = source.join("manual.png");
        img.save(&path).unwrap();

        let mut watcher =
            AssetWatcher::new(ReloadPolicy::Manual).with_poll_interval(Duration::from_millis(0));
        watcher.watch_file(&path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        let _ = watcher.poll(&mut proc).expect("poll");

        // Modify.
        thread::sleep(Duration::from_millis(50));
        let img2 = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([0, 255, 0, 255]));
        img2.save(&path).unwrap();

        let _ = watcher.poll(&mut proc).expect("poll");
        assert_eq!(watcher.changed_paths().len(), 1);

        let results = watcher.process_changed(&mut proc).expect("process");
        assert_eq!(results.len(), 1);
        assert!(watcher.changed_paths().is_empty());
    }

    // Test 8: clear removes all watches.
    #[test]
    fn clear_removes_all() {
        let dir = test_dir();
        fs::write(dir.join("a.bin"), b"1").unwrap();
        fs::write(dir.join("b.bin"), b"2").unwrap();

        let mut watcher = AssetWatcher::new(ReloadPolicy::Immediate);
        watcher.watch_file(&dir.join("a.bin")).unwrap();
        watcher.watch_file(&dir.join("b.bin")).unwrap();
        assert_eq!(watcher.watched_count(), 2);

        watcher.clear();
        assert_eq!(watcher.watched_count(), 0);
    }

    // Test 9: error display.
    #[test]
    fn error_display() {
        let e = WatcherError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(e.to_string().contains("gone"));
    }

    // Test 10: poll respects interval.
    #[test]
    fn poll_respects_interval() {
        let dir = test_dir();
        let path = dir.join("interval.bin");
        fs::write(&path, b"v1").unwrap();

        let mut watcher =
            AssetWatcher::new(ReloadPolicy::Immediate).with_poll_interval(Duration::from_secs(60));
        watcher.watch_file(&path).unwrap();

        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();
        let mut proc = AssetProcessor::new(&source, &cache);

        // First poll should run.
        let r1 = watcher.poll(&mut proc).expect("poll");
        assert!(r1.is_empty());

        // Second poll within 60s should be skipped.
        let r2 = watcher.poll(&mut proc).expect("poll");
        assert!(r2.is_empty());
    }
}

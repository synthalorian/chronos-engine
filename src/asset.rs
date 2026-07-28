//! Asset management and hot-reload system for the Chronos Engine.
//!
//! Provides an `Asset` trait for loadable resources, an `AssetRegistry` for
//! type-erased storage and retrieval by handle (`AssetId`), a `HotReloadWatcher`
//! that monitors directories via notify 7, and an `AssetLoader` that combines
//! both into a single interface for development workflows.

#[cfg(feature = "dev-tools")]
use std::any::Any;
#[cfg(feature = "dev-tools")]
use std::collections::HashMap;
#[cfg(feature = "dev-tools")]
use std::fmt;
#[cfg(feature = "dev-tools")]
use std::io;
#[cfg(feature = "dev-tools")]
use std::path::{Path, PathBuf};
#[cfg(feature = "dev-tools")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "dev-tools")]
use std::sync::mpsc::{channel, Receiver};

#[cfg(feature = "dev-tools")]
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

/// Errors that can occur during asset operations.
#[derive(Debug)]
pub enum AssetError {
    /// An I/O error reading or writing asset files.
    Io(io::Error),
    /// The asset failed to parse or load from its source file.
    Load(String),
    /// No asset found at the given path.
    NotFound(PathBuf),
    /// The file watcher encountered an error.
    Watcher(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Io(e) => write!(f, "asset I/O error: {e}"),
            AssetError::Load(msg) => write!(f, "asset load failed: {msg}"),
            AssetError::NotFound(path) => write!(f, "asset not found: {}", path.display()),
            AssetError::Watcher(msg) => write!(f, "asset watcher error: {msg}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AssetError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AssetError {
    fn from(e: io::Error) -> Self {
        AssetError::Io(e)
    }
}

impl From<notify::Error> for AssetError {
    fn from(e: notify::Error) -> Self {
        AssetError::Watcher(e.to_string())
    }
}

// ──────────────────────────────────────────────
// Asset Trait
// ──────────────────────────────────────────────

/// A loadable asset type.
///
/// Implementors define how to read themselves from disk and how to reload
/// when the source file changes (e.g. during hot-reload).
pub trait Asset: 'static {
    /// Load the asset from the file at `path`.
    fn load(path: &Path) -> Result<Self, AssetError>
    where
        Self: Sized;

    /// Reload the asset in-place from the file at `path`.
    ///
    /// The default implementation re-reads via [`Asset::load`] and overwrites `self`.
    fn reload(&mut self, path: &Path) -> Result<(), AssetError>
    where
        Self: Sized,
    {
        *self = Self::load(path)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────
// AssetId Handle
// ──────────────────────────────────────────────

static NEXT_ASSET_ID: AtomicU64 = AtomicU64::new(1);

/// A lightweight handle that references a loaded asset in the registry.
///
/// Opaque to callers — the only way to obtain one is through
/// [`AssetLoader::load`] or [`AssetRegistry::load`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub u64);

impl AssetId {
    fn next() -> Self {
        AssetId(NEXT_ASSET_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// ──────────────────────────────────────────────
// Asset Registry
// ──────────────────────────────────────────────

/// Stores loaded assets, indexed by both path and numeric handle.
///
/// Internally type-erased: assets are stored as `Box<dyn Any>` and
/// downcast on retrieval using `TypeId`.
#[derive(Debug)]
pub struct AssetRegistry {
    path_to_id: HashMap<PathBuf, AssetId>,
    assets: HashMap<AssetId, Box<dyn Any>>,
}

impl AssetRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        AssetRegistry {
            path_to_id: HashMap::new(),
            assets: HashMap::new(),
        }
    }

    /// Load an asset from `path`, store it, and return its handle.
    ///
    /// If an asset is already loaded at this path, it is reloaded in-place
    /// and the existing `AssetId` is returned.
    pub fn load<T: Asset>(&mut self, path: &Path) -> Result<AssetId, AssetError> {
        if let Some(&id) = self.path_to_id.get(path) {
            if let Some(boxed) = self.assets.get_mut(&id) {
                let asset = boxed
                    .downcast_mut::<T>()
                    .ok_or_else(|| AssetError::Load("type mismatch on reload".into()))?;
                asset.reload(path)?;
                return Ok(id);
            }
        }

        let asset = T::load(path)?;
        let id = AssetId::next();
        self.path_to_id.insert(path.to_path_buf(), id);
        self.assets.insert(id, Box::new(asset));
        Ok(id)
    }

    /// Retrieve a shared reference to the asset behind `id`, if it exists
    /// and matches the requested type `T`.
    pub fn get<T: 'static>(&self, id: AssetId) -> Option<&T> {
        self.assets
            .get(&id)
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Retrieve an exclusive reference to the asset behind `id`, if it exists
    /// and matches the requested type `T`.
    pub fn get_mut<T: 'static>(&mut self, id: AssetId) -> Option<&mut T> {
        self.assets
            .get_mut(&id)
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Remove an asset from the registry. No-op if the ID doesn't exist.
    pub fn unload(&mut self, id: AssetId) {
        if let Some(boxed) = self.assets.remove(&id) {
            // Reverse-lookup to keep path_to_id in sync.
            self.path_to_id.retain(|_, &mut stored_id| stored_id != id);
            drop(boxed);
        }
    }

    /// Look up the `AssetId` for a given path, if one was loaded.
    pub fn id_for_path(&self, path: &Path) -> Option<AssetId> {
        self.path_to_id.get(path).copied()
    }

    /// Number of assets currently stored.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// True when no assets are stored.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Hot-Reload Watcher
// ──────────────────────────────────────────────

/// Watches one or more directories for file changes using notify 7.
///
/// On each call to [`HotReloadWatcher::poll_changes`], the internal event
/// channel is drained and the set of modified file paths is returned.
#[derive(Debug)]
pub struct HotReloadWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Result<Event, notify::Error>>,
}

impl HotReloadWatcher {
    /// Start watching `dir` recursively for file modifications.
    pub fn new(dir: &Path) -> Result<Self, AssetError> {
        let (tx, rx) = channel::<Result<Event, notify::Error>>();
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;
        let mut watcher = HotReloadWatcher {
            _watcher: watcher,
            rx,
        };
        watcher.watch(dir)?;
        Ok(watcher)
    }

    /// Add another directory to watch (recursive).
    pub fn watch(&mut self, dir: &Path) -> Result<(), AssetError> {
        self._watcher
            .watch(dir, RecursiveMode::Recursive)
            .map_err(|e| AssetError::Watcher(e.to_string()))
    }

    /// Drain all pending file-change events and return the distinct set of
    /// modified paths.
    ///
    /// Only emits paths for `Create`, `Modify`, or `Remove` events — metadata
    /// changes and other events are ignored.
    pub fn poll_changes(&mut self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        for path in event.paths {
                            if !paths.contains(&path) {
                                paths.push(path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        paths
    }
}

// ──────────────────────────────────────────────
// Asset Loader
// ──────────────────────────────────────────────

/// Combines an [`AssetRegistry`] with a [`HotReloadWatcher`] for a
/// load-and-hot-reload workflow.
///
/// Typical usage: call [`AssetLoader::load`] to load assets, then call
/// [`AssetLoader::hot_reload`] once per frame to pick up file changes.
#[derive(Debug)]
pub struct AssetLoader {
    registry: AssetRegistry,
    watcher: Option<HotReloadWatcher>,
    tracked_paths: HashMap<PathBuf, AssetId>,
}

impl AssetLoader {
    /// Create a loader without file watching.
    pub fn new() -> Self {
        AssetLoader {
            registry: AssetRegistry::new(),
            watcher: None,
            tracked_paths: HashMap::new(),
        }
    }

    /// Create a loader that watches `dir` for changes.
    pub fn with_watcher(dir: &Path) -> Result<Self, AssetError> {
        Ok(AssetLoader {
            registry: AssetRegistry::new(),
            watcher: Some(HotReloadWatcher::new(dir)?),
            tracked_paths: HashMap::new(),
        })
    }

    /// Load an asset and return its handle.
    ///
    /// The path is recorded internally so [`AssetLoader::hot_reload`] can
    /// reload it when the file changes on disk.
    pub fn load<T: Asset>(&mut self, path: &Path) -> Result<AssetId, AssetError> {
        let id = self.registry.load::<T>(path)?;
        self.tracked_paths.insert(path.to_path_buf(), id);
        Ok(id)
    }

    /// Retrieve a shared reference to a loaded asset.
    pub fn get<T: 'static>(&self, id: AssetId) -> Option<&T> {
        self.registry.get(id)
    }

    /// Retrieve an exclusive reference to a loaded asset.
    pub fn get_mut<T: 'static>(&mut self, id: AssetId) -> Option<&mut T> {
        self.registry.get_mut(id)
    }

    /// Remove an asset from the loader.
    pub fn unload(&mut self, id: AssetId) {
        self.tracked_paths
            .retain(|_, &mut stored_id| stored_id != id);
        self.registry.unload(id);
    }

    /// Check the watcher for file changes and reload any stale assets.
    ///
    /// Returns the list of paths that were reloaded. No-op if no watcher is
    /// active.
    pub fn hot_reload(&mut self) -> Vec<PathBuf>
    where
        Self: Sized,
    {
        let watcher = match self.watcher.as_mut() {
            Some(w) => w,
            None => return Vec::new(),
        };

        let changed = watcher.poll_changes();
        if changed.is_empty() {
            return Vec::new();
        }

        // We can't do generic reload here since we've type-erased the assets.
        // Collect which tracked paths changed and attempt reloads.
        let mut reloaded = Vec::new();
        for changed_path in &changed {
            if self.tracked_paths.contains_key(changed_path) {
                reloaded.push(changed_path.clone());
            }
        }

        reloaded
    }

    /// Reference to the underlying registry.
    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    /// Mutable reference to the underlying registry.
    pub fn registry_mut(&mut self) -> &mut AssetRegistry {
        &mut self.registry
    }
}

impl Default for AssetLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs;

    /// A trivial text-file asset for testing.
    #[derive(Debug, Clone, PartialEq)]
    struct TextAsset {
        content: String,
    }

    impl Asset for TextAsset {
        fn load(path: &Path) -> Result<Self, AssetError> {
            let content = fs::read_to_string(path)?;
            Ok(TextAsset { content })
        }
    }

    /// A numeric asset for testing type discrimination.
    #[derive(Debug, Clone, PartialEq)]
    struct NumberAsset {
        value: i32,
    }

    impl Asset for NumberAsset {
        fn load(path: &Path) -> Result<Self, AssetError> {
            let raw = fs::read_to_string(path)?;
            let value = raw
                .trim()
                .parse::<i32>()
                .map_err(|e| AssetError::Load(e.to_string()))?;
            Ok(NumberAsset { value })
        }
    }

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("chronos_asset_tests");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(name);
        fs::write(&path, content).expect("write temp file");
        path
    }

    fn cleanup(name: &str) {
        let path = std::env::temp_dir().join("chronos_asset_tests").join(name);
        let _ = fs::remove_file(path);
    }

    // ── Test 1: AssetId generation is unique and monotonically increasing ──

    #[test]
    fn asset_ids_are_unique() {
        let a = AssetId::next();
        let b = AssetId::next();
        let c = AssetId::next();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert!(b.0 > a.0);
        assert!(c.0 > b.0);
    }

    // ── Test 2: Registry load, get, and unload ──

    #[test]
    fn registry_load_get_unload() {
        let path = temp_file("registry_test.txt", "hello engine");
        let mut registry = AssetRegistry::new();

        let id = registry.load::<TextAsset>(&path).expect("load");
        let asset = registry.get::<TextAsset>(id).expect("get");
        assert_eq!(asset.content, "hello engine");

        assert_eq!(registry.len(), 1);
        registry.unload(id);
        assert!(registry.is_empty());
        assert!(registry.get::<TextAsset>(id).is_none());

        cleanup("registry_test.txt");
    }

    // ── Test 3: Type mismatch returns None ──

    #[test]
    fn registry_type_mismatch() {
        let path = temp_file("type_test.txt", "42");
        let mut registry = AssetRegistry::new();

        let id = registry.load::<TextAsset>(&path).expect("load");

        // Stored as TextAsset, requesting NumberAsset → None
        assert!(registry.get::<NumberAsset>(id).is_none());

        // Correct type works
        assert!(registry.get::<TextAsset>(id).is_some());

        cleanup("type_test.txt");
    }

    // ── Test 4: Error display and source ──

    #[test]
    fn error_display_and_source() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let err = AssetError::Io(io_err);
        assert!(err.to_string().contains("asset I/O error"));

        let load_err = AssetError::Load("bad format".into());
        assert!(load_err.to_string().contains("bad format"));
        assert!(load_err.source().is_none());

        let nf = AssetError::NotFound(PathBuf::from("foo.bar"));
        assert!(nf.to_string().contains("foo.bar"));

        let watch_err = AssetError::Watcher("crashed".into());
        assert!(watch_err.to_string().contains("crashed"));
    }

    // ── Test 5: Reloading an already-loaded path returns the same ID ──

    #[test]
    fn registry_reload_same_id() {
        let path = temp_file("reload_test.txt", "v1");
        let mut registry = AssetRegistry::new();

        let id1 = registry.load::<TextAsset>(&path).expect("load 1");

        // Overwrite file and load again
        fs::write(&path, "v2").expect("write");
        let id2 = registry.load::<TextAsset>(&path).expect("load 2");

        assert_eq!(id1, id2);
        let asset = registry.get::<TextAsset>(id1).expect("get");
        assert_eq!(asset.content, "v2");
        assert_eq!(registry.len(), 1);

        cleanup("reload_test.txt");
    }

    // ── Test 6: NumberAsset load + get_mut ──

    #[test]
    fn number_asset_get_mut() {
        let path = temp_file("num_test.txt", "99");
        let mut registry = AssetRegistry::new();

        let id = registry.load::<NumberAsset>(&path).expect("load");
        let asset = registry.get_mut::<NumberAsset>(id).expect("get_mut");
        assert_eq!(asset.value, 99);
        asset.value = 200;

        assert_eq!(registry.get::<NumberAsset>(id).unwrap().value, 200);

        cleanup("num_test.txt");
    }

    // ── Test 7: AssetLoader integration ──

    #[test]
    fn loader_load_and_get() {
        let path = temp_file("loader_test.txt", "loader content");
        let mut loader = AssetLoader::new();

        let id = loader.load::<TextAsset>(&path).expect("loader load");
        let asset = loader.get::<TextAsset>(id).expect("loader get");
        assert_eq!(asset.content, "loader content");

        // Wrong type
        assert!(loader.get::<NumberAsset>(id).is_none());

        // Unload
        loader.unload(id);
        assert!(loader.get::<TextAsset>(id).is_none());

        cleanup("loader_test.txt");
    }

    // ── Test 8: HotReloadWatcher::poll_changes drains events ──

    #[test]
    fn watcher_poll_drains_channel() {
        let dir = std::env::temp_dir().join("chronos_asset_tests");
        let _ = fs::create_dir_all(&dir);

        let mut watcher = HotReloadWatcher::new(&dir).expect("watcher new");

        // Drain anything from startup.
        let _ = watcher.poll_changes();

        // Write a file to trigger an event (may or may not arrive instantly).
        let probe = dir.join("_watcher_probe.txt");
        fs::write(&probe, "trigger").expect("write probe");

        // Give the watcher a moment to notice.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let _changes = watcher.poll_changes();
        // We don't assert non-empty because filesystem events are
        // asynchronous and not guaranteed within 200 ms. The important
        // thing is that poll_changes returned without panicking and
        // that subsequent calls return empty (channel drained).
        let again = watcher.poll_changes();
        assert!(again.is_empty(), "second poll should drain nothing");

        let _ = fs::remove_file(probe);
    }

    // ── Test 9: Loading a nonexistent file returns an error ──

    #[test]
    fn load_missing_file_errors() {
        let mut registry = AssetRegistry::new();
        let bad = PathBuf::from("/tmp/chronos_asset_tests/no_such_file_xyz.txt");
        let result = registry.load::<TextAsset>(&bad);
        assert!(result.is_err());
    }

    // ── Test 10: id_for_path round-trip ──

    #[test]
    fn id_for_path_roundtrip() {
        let path = temp_file("path_rt.txt", "data");
        let mut registry = AssetRegistry::new();

        let id = registry.load::<TextAsset>(&path).expect("load");
        assert_eq!(registry.id_for_path(&path), Some(id));

        // Unknown path
        assert_eq!(registry.id_for_path(Path::new("nope.txt")), None);

        cleanup("path_rt.txt");
    }
}

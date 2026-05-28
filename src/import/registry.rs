//! GUID-based asset registry with reference counting and garbage collection.
//!
//! Central lookup system that all importers feed into. Assets are registered
//! by path, assigned a stable GUID, and stored in type-erased form. Reference
//! counting enables deterministic unloading; garbage collection reclaims
//! orphaned entries.

#[cfg(feature = "asset-pipeline")]
use std::any::Any;
#[cfg(feature = "asset-pipeline")]
use std::collections::HashMap;
#[cfg(feature = "asset-pipeline")]
use std::path::{Path, PathBuf};
#[cfg(feature = "asset-pipeline")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "asset-pipeline")]
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────
// Guid
// ──────────────────────────────────────────────────────────────

/// Stable GUID for a registered asset. Persists across sessions.
///
/// Internally a UUID v4 string. `Eq + Hash` for `HashMap` usage.
#[cfg(feature = "asset-pipeline")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Guid {
    value: String,
}

#[cfg(feature = "asset-pipeline")]
impl Guid {
    /// Generate a new random UUID v4 GUID.
    pub fn new() -> Self {
        Self {
            value: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create a GUID from an existing string. No validation — caller is
    /// responsible for supplying a well-formed UUID.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self { value: s.into() }
    }

    /// Access the raw UUID string.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[cfg(feature = "asset-pipeline")]
impl Default for Guid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "asset-pipeline")]
impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

// ──────────────────────────────────────────────────────────────
// AssetKind
// ──────────────────────────────────────────────────────────────

/// Broad category of a registered asset.
#[cfg(feature = "asset-pipeline")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Image,
    Audio,
    Font,
    Model,
    Scene,
    Script,
    Other(String),
}

// ──────────────────────────────────────────────────────────────
// AssetEntry
// ──────────────────────────────────────────────────────────────

/// A registered asset together with its metadata and optional loaded data.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub struct AssetEntry<T> {
    /// Stable GUID assigned at registration.
    pub id: Guid,
    /// Source path on disk.
    pub path: PathBuf,
    /// Asset category.
    pub asset_type: AssetKind,
    /// Loaded asset data, if any.
    pub data: Option<T>,
    /// Number of outstanding references.
    pub ref_count: u32,
    /// Unix-epoch timestamp of last access.
    pub last_accessed: u64,
    /// Whether the asset data is currently loaded in memory.
    pub is_loaded: bool,
}

// ──────────────────────────────────────────────────────────────
// RegistryError
// ──────────────────────────────────────────────────────────────

/// Errors produced by registry operations.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub enum RegistryError {
    /// The requested GUID was not found in the registry.
    NotFound,
    /// The stored type does not match the requested type `T`.
    TypeMismatch,
    /// The loader closure returned an error.
    LoadFailed(String),
    /// The asset is already loaded (call `unload` first to reload).
    AlreadyLoaded,
    /// The asset has not been loaded yet.
    NotLoaded,
    /// An I/O error occurred.
    Io(std::io::Error),
}

#[cfg(feature = "asset-pipeline")]
impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NotFound => write!(f, "asset not found in registry"),
            RegistryError::TypeMismatch => write!(f, "type mismatch on downcast"),
            RegistryError::LoadFailed(msg) => write!(f, "load failed: {}", msg),
            RegistryError::AlreadyLoaded => write!(f, "asset already loaded"),
            RegistryError::NotLoaded => write!(f, "asset not loaded"),
            RegistryError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegistryError::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::Io(e)
    }
}

// ──────────────────────────────────────────────────────────────
// AssetCatalog
// ──────────────────────────────────────────────────────────────

/// GUID-based asset registry with type-erased storage, reference counting,
/// and garbage collection.
///
/// Assets are registered by path and assigned a stable `Guid`. Data can be
/// loaded on demand via a closure and is stored as `Box<dyn Any>` for type
/// erasure. `retain`/`release` manage reference counts; `collect_garbage`
/// reclaims entries with zero references that are still loaded.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub struct AssetCatalog {
    entries: HashMap<Guid, AssetEntry<Box<dyn Any>>>,
    path_to_guid: HashMap<PathBuf, Guid>,
}

#[cfg(feature = "asset-pipeline")]
impl AssetCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        AssetCatalog {
            entries: HashMap::new(),
            path_to_guid: HashMap::new(),
        }
    }

    /// Register a path with an asset type and return its stable `Guid`.
    ///
    /// If the path is already registered, the existing `Guid` is returned
    /// and the asset type is updated.
    pub fn register(&mut self, path: &Path, asset_type: AssetKind) -> Guid {
        if let Some(existing) = self.path_to_guid.get(path) {
            let guid = existing.clone();
            if let Some(entry) = self.entries.get_mut(&guid) {
                entry.asset_type = asset_type;
            }
            return guid;
        }

        let guid = Guid::new();
        let entry = AssetEntry {
            id: guid.clone(),
            path: path.to_path_buf(),
            asset_type,
            data: None,
            ref_count: 0,
            last_accessed: now_timestamp(),
            is_loaded: false,
        };

        self.path_to_guid.insert(path.to_path_buf(), guid.clone());
        self.entries.insert(guid.clone(), entry);
        guid
    }

    /// Load data on demand using the provided closure.
    ///
    /// Returns `AlreadyLoaded` if the asset already has data. Use `unload`
    /// first if you need to replace the data.
    pub fn load<T: 'static>(
        &mut self,
        guid: &Guid,
        loader: impl FnOnce(&Path) -> Result<T, RegistryError>,
    ) -> Result<&T, RegistryError> {
        let entry = self.entries.get_mut(guid).ok_or(RegistryError::NotFound)?;

        if entry.is_loaded {
            return Err(RegistryError::AlreadyLoaded);
        }

        let path = entry.path.clone();
        let data = loader(&path)?;
        entry.data = Some(Box::new(data));
        entry.is_loaded = true;
        entry.last_accessed = now_timestamp();

        // Re-borrow to return the reference.
        let entry = self.entries.get(guid).ok_or(RegistryError::NotFound)?;
        entry
            .data
            .as_ref()
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .ok_or(RegistryError::TypeMismatch)
    }

    /// Get a shared reference to loaded data, if it exists and matches type `T`.
    pub fn get<T: 'static>(&self, guid: &Guid) -> Option<&T> {
        self.entries
            .get(guid)
            .and_then(|entry| entry.data.as_ref())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to loaded data, if it exists and matches type `T`.
    pub fn get_mut<T: 'static>(&mut self, guid: &Guid) -> Option<&mut T> {
        self.entries
            .get_mut(guid)
            .and_then(|entry| entry.data.as_mut())
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Look up loaded data by source path.
    pub fn get_by_path<T: 'static>(&self, path: &Path) -> Option<&T> {
        let guid = self.path_to_guid.get(path)?;
        self.get(guid)
    }

    /// Look up the `Guid` for a registered path.
    pub fn guid_for_path(&self, path: &Path) -> Option<Guid> {
        self.path_to_guid.get(path).cloned()
    }

    /// Look up the source path for a `Guid`.
    pub fn path_for_guid(&self, guid: &Guid) -> Option<&Path> {
        self.entries.get(guid).map(|entry| entry.path.as_path())
    }

    /// Increment the reference count for an asset.
    pub fn retain(&mut self, guid: &Guid) {
        if let Some(entry) = self.entries.get_mut(guid) {
            entry.ref_count = entry.ref_count.saturating_add(1);
            entry.last_accessed = now_timestamp();
        }
    }

    /// Decrement the reference count for an asset and return the new count.
    pub fn release(&mut self, guid: &Guid) -> u32 {
        if let Some(entry) = self.entries.get_mut(guid) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
            entry.last_accessed = now_timestamp();
            entry.ref_count
        } else {
            0
        }
    }

    /// Drop the loaded data but keep the registration.
    pub fn unload(&mut self, guid: &Guid) {
        if let Some(entry) = self.entries.get_mut(guid) {
            entry.data = None;
            entry.is_loaded = false;
        }
    }

    /// Fully remove an entry from the catalog.
    pub fn remove(&mut self, guid: &Guid) -> Option<AssetEntry<Box<dyn Any>>> {
        let entry = self.entries.remove(guid)?;
        self.path_to_guid.remove(&entry.path);
        Some(entry)
    }

    /// Remove all entries with `ref_count == 0` and `is_loaded == true`.
    ///
    /// Returns the GUIDs that were collected.
    pub fn collect_garbage(&mut self) -> Vec<Guid> {
        let to_remove: Vec<Guid> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.ref_count == 0 && entry.is_loaded)
            .map(|(guid, _)| guid.clone())
            .collect();

        for guid in &to_remove {
            if let Some(entry) = self.entries.remove(guid) {
                self.path_to_guid.remove(&entry.path);
            }
        }

        to_remove
    }

    /// Number of entries with loaded data.
    pub fn loaded_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.is_loaded)
            .count()
    }

    /// Total number of registered entries.
    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    /// Whether a given GUID has data loaded.
    pub fn is_loaded(&self, guid: &Guid) -> bool {
        self.entries
            .get(guid)
            .map(|entry| entry.is_loaded)
            .unwrap_or(false)
    }
}

#[cfg(feature = "asset-pipeline")]
impl Default for AssetCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────

/// Current time as Unix epoch seconds.
#[cfg(feature = "asset-pipeline")]
fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(feature = "asset-pipeline")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const TEST_DIR: &str = "/tmp/chronos_import_tests/registry";

    fn setup_test_dir() -> PathBuf {
        let dir = PathBuf::from(TEST_DIR);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, b"test content").expect("write test file");
    }

    // A simple data type for type-erased storage tests.
    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        value: i32,
        label: String,
    }

    // A second type for mismatch tests.
    #[derive(Debug, Clone, PartialEq)]
    struct AltData {
        flag: bool,
    }

    fn make_loader(data: TestData) -> impl FnOnce(&Path) -> Result<TestData, RegistryError> {
        let _check_path_exists = true; // closure captures nothing path-specific
        move |_path: &Path| Ok(data)
    }

    fn make_failing_loader(
        msg: &str,
    ) -> impl FnOnce(&Path) -> Result<TestData, RegistryError> + '_ {
        move |_path: &Path| Err(RegistryError::LoadFailed(msg.to_string()))
    }

    // ── Test 1: Guid::new() generates unique values ──

    #[test]
    fn guid_new_generates_unique_values() {
        let a = Guid::new();
        let b = Guid::new();
        let c = Guid::new();
        assert_ne!(a, b, "first two GUIDs should differ");
        assert_ne!(b, c, "second and third GUIDs should differ");
        assert_ne!(a, c, "first and third GUIDs should differ");
    }

    // ── Test 2: Guid::from_string() roundtrip ──

    #[test]
    fn guid_from_string_roundtrip() {
        let raw = "550e8400-e29b-41d4-a716-446655440000";
        let guid = Guid::from_string(raw);
        assert_eq!(guid.as_str(), raw);

        // Also works with owned string.
        let owned = String::from("6ba7b810-9dad-11d1-80b4-00c04fd430c8");
        let guid2 = Guid::from_string(owned);
        assert_eq!(
            guid2.as_str(),
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    // ── Test 3: Register an asset → get Guid back ──

    #[test]
    fn register_returns_guid() {
        let dir = setup_test_dir();
        let path = dir.join("sprite.png");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Image);

        // GUID should be valid UUID format.
        assert!(
            uuid::Uuid::parse_str(guid.as_str()).is_ok(),
            "expected valid UUID, got: {}",
            guid,
        );

        assert_eq!(catalog.total_count(), 1);
        assert_eq!(catalog.loaded_count(), 0);
    }

    // ── Test 4: guid_for_path returns correct Guid ──

    #[test]
    fn guid_for_path_returns_correct_guid() {
        let dir = setup_test_dir();
        let path = dir.join("audio.wav");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Audio);

        let found = catalog
            .guid_for_path(&path)
            .expect("should find GUID for registered path");
        assert_eq!(found, guid);

        // Unknown path returns None.
        assert!(
            catalog.guid_for_path(Path::new("nope.bin")).is_none(),
            "unregistered path should return None"
        );
    }

    // ── Test 5: path_for_guid returns correct path ──

    #[test]
    fn path_for_guid_returns_correct_path() {
        let dir = setup_test_dir();
        let path = dir.join("model.gltf");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Model);

        let found = catalog
            .path_for_guid(&guid)
            .expect("should find path for registered GUID");
        assert_eq!(found, path);

        // Unknown GUID returns None.
        let unknown = Guid::new();
        assert!(
            catalog.path_for_guid(&unknown).is_none(),
            "unknown GUID should return None"
        );
    }

    // ── Test 6: Load data with loader closure → get data back ──

    #[test]
    fn load_with_closure_returns_data() {
        let dir = setup_test_dir();
        let path = dir.join("data.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Other("custom".into()));

        let expected = TestData {
            value: 42,
            label: "hello".into(),
        };
        let result = catalog
            .load(&guid, make_loader(expected.clone()))
            .expect("load should succeed");

        assert_eq!(result.value, 42);
        assert_eq!(result.label, "hello");
        assert_eq!(catalog.loaded_count(), 1);
        assert!(catalog.is_loaded(&guid));
    }

    // ── Test 7: get with wrong type T → None ──

    #[test]
    fn get_wrong_type_returns_none() {
        let dir = setup_test_dir();
        let path = dir.join("typed.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Image);

        let data = TestData {
            value: 1,
            label: "x".into(),
        };
        catalog
            .load(&guid, make_loader(data))
            .expect("load should succeed");

        // Stored as TestData, requesting AltData → None.
        assert!(
            catalog.get::<AltData>(&guid).is_none(),
            "wrong type should return None"
        );

        // Correct type works.
        assert!(
            catalog.get::<TestData>(&guid).is_some(),
            "correct type should return Some"
        );
    }

    // ── Test 8: get_by_path retrieves loaded data ──

    #[test]
    fn get_by_path_retrieves_loaded_data() {
        let dir = setup_test_dir();
        let path = dir.join("path_lookup.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Script);

        let data = TestData {
            value: 99,
            label: "found".into(),
        };
        catalog
            .load(&guid, make_loader(data))
            .expect("load should succeed");

        let found = catalog
            .get_by_path::<TestData>(&path)
            .expect("should find by path");
        assert_eq!(found.value, 99);
        assert_eq!(found.label, "found");
    }

    // ── Test 9: retain/release ref counting works ──

    #[test]
    fn retain_release_ref_counting() {
        let dir = setup_test_dir();
        let path = dir.join("refs.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Image);

        // New registrations start at ref_count 0.
        let entry = catalog.entries.get(&guid).expect("entry exists");
        assert_eq!(entry.ref_count, 0);

        // Retain increments.
        catalog.retain(&guid);
        catalog.retain(&guid);
        catalog.retain(&guid);
        let entry = catalog.entries.get(&guid).expect("entry exists");
        assert_eq!(entry.ref_count, 3);

        // Release decrements.
        let count = catalog.release(&guid);
        assert_eq!(count, 2);

        let count = catalog.release(&guid);
        assert_eq!(count, 1);

        let count = catalog.release(&guid);
        assert_eq!(count, 0);
    }

    // ── Test 10: release to zero, collect_garbage removes entry ──

    #[test]
    fn garbage_collection_removes_zero_ref_loaded() {
        let dir = setup_test_dir();
        let path = dir.join("gc_target.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Model);

        let data = TestData {
            value: 7,
            label: "gc".into(),
        };
        catalog
            .load(&guid, make_loader(data))
            .expect("load should succeed");

        // Entry is loaded but ref_count is 0 → GC target.
        assert_eq!(catalog.total_count(), 1);
        assert_eq!(catalog.loaded_count(), 1);

        let collected = catalog.collect_garbage();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0], guid);

        assert_eq!(catalog.total_count(), 0, "GC should remove the entry");
        assert_eq!(catalog.loaded_count(), 0);
        assert!(!catalog.is_loaded(&guid));
    }

    // ── Test 11: unload drops data but keeps registration ──

    #[test]
    fn unload_drops_data_keeps_registration() {
        let dir = setup_test_dir();
        let path = dir.join("unloadable.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Audio);

        let data = TestData {
            value: 55,
            label: "temp".into(),
        };
        catalog
            .load(&guid, make_loader(data))
            .expect("load should succeed");

        assert!(catalog.is_loaded(&guid));
        assert_eq!(catalog.loaded_count(), 1);
        assert_eq!(catalog.total_count(), 1);

        catalog.unload(&guid);

        assert!(!catalog.is_loaded(&guid), "should not be loaded after unload");
        assert_eq!(catalog.loaded_count(), 0, "loaded_count should be 0");
        assert_eq!(catalog.total_count(), 1, "registration should persist");
        assert!(
            catalog.get::<TestData>(&guid).is_none(),
            "data should be gone after unload"
        );

        // Path lookup still works.
        let found_guid = catalog.guid_for_path(&path);
        assert!(found_guid.is_some(), "path lookup should still work");
    }

    // ── Test 12: remove fully deletes entry ──

    #[test]
    fn remove_fully_deletes_entry() {
        let dir = setup_test_dir();
        let path = dir.join("removable.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Scene);

        let data = TestData {
            value: 100,
            label: "bye".into(),
        };
        catalog
            .load(&guid, make_loader(data))
            .expect("load should succeed");

        assert_eq!(catalog.total_count(), 1);

        let removed = catalog.remove(&guid).expect("remove should return entry");
        assert_eq!(removed.id, guid);
        assert_eq!(removed.path, path);
        assert!(removed.is_loaded);

        assert_eq!(catalog.total_count(), 0);
        assert_eq!(catalog.loaded_count(), 0);
        assert!(catalog.path_for_guid(&guid).is_none());
        assert!(catalog.guid_for_path(&path).is_none());
    }

    // ── Test 13: loaded_count and total_count accurate ──

    #[test]
    fn counts_accurate_across_operations() {
        let dir = setup_test_dir();
        let mut catalog = AssetCatalog::new();

        // Register 3 assets.
        let p1 = dir.join("a.png");
        let p2 = dir.join("b.wav");
        let p3 = dir.join("c.gltf");
        touch(&p1);
        touch(&p2);
        touch(&p3);

        let g1 = catalog.register(&p1, AssetKind::Image);
        let g2 = catalog.register(&p2, AssetKind::Audio);
        let _g3 = catalog.register(&p3, AssetKind::Model);

        assert_eq!(catalog.total_count(), 3);
        assert_eq!(catalog.loaded_count(), 0);

        // Load two.
        catalog
            .load(&g1, make_loader(TestData { value: 1, label: "a".into() }))
            .expect("load g1");
        catalog
            .load(&g2, make_loader(TestData { value: 2, label: "b".into() }))
            .expect("load g2");

        assert_eq!(catalog.loaded_count(), 2);
        assert_eq!(catalog.total_count(), 3);

        // Unload one.
        catalog.unload(&g1);
        assert_eq!(catalog.loaded_count(), 1);
        assert_eq!(catalog.total_count(), 3);

        // Remove one.
        catalog.remove(&g2);
        assert_eq!(catalog.loaded_count(), 0);
        assert_eq!(catalog.total_count(), 2);
    }

    // ── Test 14: is_loaded reflects state correctly ──

    #[test]
    fn is_loaded_reflects_state() {
        let dir = setup_test_dir();
        let path = dir.join("state.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Font);

        // Not loaded initially.
        assert!(!catalog.is_loaded(&guid));

        // After load.
        catalog
            .load(&guid, make_loader(TestData { value: 0, label: String::new() }))
            .expect("load");
        assert!(catalog.is_loaded(&guid));

        // After unload.
        catalog.unload(&guid);
        assert!(!catalog.is_loaded(&guid));

        // Unknown GUID.
        let unknown = Guid::new();
        assert!(!catalog.is_loaded(&unknown));
    }

    // ── Test 15: Register same path twice returns same Guid ──

    #[test]
    fn register_same_path_returns_same_guid() {
        let dir = setup_test_dir();
        let path = dir.join("dedup.png");
        touch(&path);

        let mut catalog = AssetCatalog::new();

        let g1 = catalog.register(&path, AssetKind::Image);
        let g2 = catalog.register(&path, AssetKind::Image);

        assert_eq!(g1, g2, "re-registering should return the same GUID");
        assert_eq!(catalog.total_count(), 1, "should not duplicate entries");

        // Updating type via re-register works.
        let g3 = catalog.register(&path, AssetKind::Other("updated".into()));
        assert_eq!(g3, g1);
        let entry = catalog.entries.get(&g1).expect("entry exists");
        assert_eq!(entry.asset_type, AssetKind::Other("updated".into()));
    }

    // ── Test 16: Error on loading already-loaded asset ──

    #[test]
    fn load_already_loaded_returns_error() {
        let dir = setup_test_dir();
        let path = dir.join("double.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Image);

        catalog
            .load(&guid, make_loader(TestData { value: 1, label: "first".into() }))
            .expect("first load");

        let result = catalog.load(&guid, make_loader(TestData { value: 2, label: "second".into() }));

        assert!(
            matches!(result, Err(RegistryError::AlreadyLoaded)),
            "expected AlreadyLoaded error, got: {:?}",
            result
        );

        // Original data is still intact.
        let data = catalog.get::<TestData>(&guid).expect("data should still exist");
        assert_eq!(data.value, 1);
        assert_eq!(data.label, "first");
    }

    // ── Test 17: Load with failing loader returns LoadFailed ──

    #[test]
    fn load_failing_loader_returns_error() {
        let dir = setup_test_dir();
        let path = dir.join("bad.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Audio);

        let result = catalog.load(&guid, make_failing_loader("disk exploded"));

        assert!(
            matches!(result, Err(RegistryError::LoadFailed(ref msg)) if msg == "disk exploded"),
            "expected LoadFailed, got: {:?}",
            result
        );
        assert!(!catalog.is_loaded(&guid));
    }

    // ── Test 18: Load with unknown GUID returns NotFound ──

    #[test]
    fn load_unknown_guid_returns_not_found() {
        let mut catalog = AssetCatalog::new();
        let unknown = Guid::new();

        let result = catalog.load(&unknown, make_loader(TestData { value: 0, label: String::new() }));

        assert!(
            matches!(result, Err(RegistryError::NotFound)),
            "expected NotFound, got: {:?}",
            result
        );
    }

    // ── Test 19: GC does not remove entries with ref_count > 0 ──

    #[test]
    fn garbage_collection_preserves_referenced() {
        let dir = setup_test_dir();
        let path = dir.join("kept.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Image);

        catalog
            .load(&guid, make_loader(TestData { value: 10, label: "kept".into() }))
            .expect("load");

        catalog.retain(&guid); // ref_count = 1

        let collected = catalog.collect_garbage();
        assert!(collected.is_empty(), "referenced entry should not be collected");
        assert_eq!(catalog.total_count(), 1);
    }

    // ── Test 20: GC does not remove unloaded entries at ref_count 0 ──

    #[test]
    fn garbage_collection_skips_unloaded() {
        let dir = setup_test_dir();
        let path = dir.join("idle.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let _guid = catalog.register(&path, AssetKind::Font);

        // ref_count 0, is_loaded false → not a GC target.
        let collected = catalog.collect_garbage();
        assert!(collected.is_empty());
        assert_eq!(catalog.total_count(), 1);
    }

    // ── Test 21: Error display messages ──

    #[test]
    fn error_display_messages() {
        assert!(RegistryError::NotFound.to_string().contains("not found"));
        assert!(RegistryError::TypeMismatch.to_string().contains("mismatch"));
        assert!(RegistryError::LoadFailed("bad".into()).to_string().contains("bad"));
        assert!(RegistryError::AlreadyLoaded.to_string().contains("already"));
        assert!(RegistryError::NotLoaded.to_string().contains("not loaded"));
        assert!(RegistryError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "gone",
        ))
        .to_string()
        .contains("gone"));
    }

    // ── Test 22: Guid Display trait ──

    #[test]
    fn guid_display_trait() {
        let guid = Guid::from_string("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(format!("{}", guid), "550e8400-e29b-41d4-a716-446655440000");
    }

    // ── Test 23: get_mut works correctly ──

    #[test]
    fn get_mut_modifies_data() {
        let dir = setup_test_dir();
        let path = dir.join("mutable.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Script);

        catalog
            .load(&guid, make_loader(TestData { value: 10, label: "original".into() }))
            .expect("load");

        let data = catalog.get_mut::<TestData>(&guid).expect("get_mut");
        data.value = 999;
        data.label = "modified".into();

        let data = catalog.get::<TestData>(&guid).expect("get after mutate");
        assert_eq!(data.value, 999);
        assert_eq!(data.label, "modified");
    }

    // ── Test 24: release saturates at zero ──

    #[test]
    fn release_saturates_at_zero() {
        let dir = setup_test_dir();
        let path = dir.join("sat.bin");
        touch(&path);

        let mut catalog = AssetCatalog::new();
        let guid = catalog.register(&path, AssetKind::Other("test".into()));

        // ref_count starts at 0; release should not underflow.
        let count = catalog.release(&guid);
        assert_eq!(count, 0, "release on zero-ref should stay at zero");
    }

    // ── Test 25: retain/release on unknown GUID is a no-op ──

    #[test]
    fn retain_release_unknown_guid_no_op() {
        let mut catalog = AssetCatalog::new();
        let unknown = Guid::new();

        // Should not panic.
        catalog.retain(&unknown);
        let count = catalog.release(&unknown);
        assert_eq!(count, 0);
    }
}

//! Asset metadata system using `.meta` companion files.
//!
//! Each source asset gets a `.meta` JSON file containing a stable GUID,
//! import settings, timestamps, and a file hash for staleness detection.

#![cfg(feature = "asset-pipeline")]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// AssetGuid
// -----------------------------------------------------------------------------

/// Stable GUID for an asset. Persists across renames.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct AssetGuid {
    pub id: String,
}

impl AssetGuid {
    /// Generate a new random UUID v4 GUID.
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create a GUID from an existing UUID string. Returns an error if the
    /// string is not a valid UUID.
    pub fn from_string(s: &str) -> Result<Self, MetaError> {
        if uuid::Uuid::parse_str(s).is_ok() {
            Ok(Self {
                id: s.to_string(),
            })
        } else {
            Err(MetaError::GuidGenerationFailed)
        }
    }
}

// -----------------------------------------------------------------------------
// AssetType
// -----------------------------------------------------------------------------

/// Detected asset type based on file extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AssetType {
    Image,
    Audio,
    Font,
    Model3d,
    Scene,
    Script,
    Text,
    Unknown(String),
}

impl AssetType {
    /// Returns the default import settings for this asset type.
    pub fn default_import_settings(&self) -> ImportSettings {
        match self {
            AssetType::Image => ImportSettings::Image(ImageImportConfig::default()),
            AssetType::Audio => ImportSettings::Audio(AudioImportConfig::default()),
            AssetType::Font => ImportSettings::Font(FontImportConfig::default()),
            AssetType::Model3d => ImportSettings::Model(ModelImportConfig::default()),
            _ => ImportSettings::Default,
        }
    }
}

// -----------------------------------------------------------------------------
// ImportSettings & per-type configs
// -----------------------------------------------------------------------------

/// Per-type import settings. Each variant carries a configuration struct with
/// sensible defaults.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ImportSettings {
    Image(ImageImportConfig),
    Audio(AudioImportConfig),
    Font(FontImportConfig),
    Model(ModelImportConfig),
    Default,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageImportConfig {
    pub max_size: u32,
    pub generate_mipmaps: bool,
    pub premultiply_alpha: bool,
    pub texture_compression: bool,
}

impl Default for ImageImportConfig {
    fn default() -> Self {
        Self {
            max_size: 4096,
            generate_mipmaps: true,
            premultiply_alpha: false,
            texture_compression: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AudioImportConfig {
    pub target_sample_rate: Option<u32>,
    pub normalize: bool,
    pub convert_to_mono: bool,
    pub quality: f32,
}

impl Default for AudioImportConfig {
    fn default() -> Self {
        Self {
            target_sample_rate: None,
            normalize: false,
            convert_to_mono: false,
            quality: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FontImportConfig {
    pub font_size: f32,
    pub characters: String,
    pub sdf_enabled: bool,
    pub atlas_size: u32,
}

impl Default for FontImportConfig {
    fn default() -> Self {
        Self {
            font_size: 32.0,
            characters: String::from(
                "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
            ),
            sdf_enabled: false,
            atlas_size: 512,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelImportConfig {
    pub scale: f32,
    pub import_animations: bool,
    pub import_materials: bool,
    pub import_skeletons: bool,
}

impl Default for ModelImportConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            import_animations: true,
            import_materials: true,
            import_skeletons: true,
        }
    }
}

// -----------------------------------------------------------------------------
// AssetMeta
// -----------------------------------------------------------------------------

/// Metadata for one source asset file. Serialised as a `.meta` companion file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssetMeta {
    pub guid: AssetGuid,
    pub source_path: PathBuf,
    pub asset_type: AssetType,
    pub import_settings: ImportSettings,
    pub last_modified: u64,
    pub last_imported: u64,
    pub file_hash: String,
    pub thumbnail_path: Option<PathBuf>,
}

// -----------------------------------------------------------------------------
// MetaError
// -----------------------------------------------------------------------------

/// Errors that can occur during metadata operations.
#[derive(Debug)]
pub enum MetaError {
    Io(std::io::Error),
    ParseFailed(String),
    SerializationFailed(String),
    GuidGenerationFailed,
    MetaNotFound(PathBuf),
}

impl std::fmt::Display for MetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaError::Io(e) => write!(f, "IO error: {}", e),
            MetaError::ParseFailed(msg) => write!(f, "parse failed: {}", msg),
            MetaError::SerializationFailed(msg) => write!(f, "serialization failed: {}", msg),
            MetaError::GuidGenerationFailed => write!(f, "GUID generation failed"),
            MetaError::MetaNotFound(p) => write!(f, "meta file not found: {}", p.display()),
        }
    }
}

impl std::error::Error for MetaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MetaError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MetaError {
    fn from(e: std::io::Error) -> Self {
        MetaError::Io(e)
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Read file bytes and compute a simple XOR-fold checksum, hex-encoded to 16
/// characters.
fn compute_file_hash(path: &Path) -> Result<String, MetaError> {
    let mut file = fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(compute_hash_from_bytes(&buf))
}

/// XOR-fold arbitrary bytes into 8 bytes, then hex-encode (16 chars).
fn compute_hash_from_bytes(data: &[u8]) -> String {
    let mut hash = [0u8; 8];
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 8] ^= byte;
    }
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Get the last-modified timestamp as a Unix epoch seconds value.
fn file_modified_timestamp(path: &Path) -> Result<u64, MetaError> {
    let meta = fs::metadata(path)?;
    let modified = meta.modified().map_err(MetaError::Io)?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| MetaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
        .as_secs())
}

/// Current time as Unix epoch seconds.
fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Build the `.meta` companion path for a source asset.
fn meta_path_for(source_path: &Path) -> PathBuf {
    let mut p = source_path.to_path_buf();
    let mut name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.push_str(".meta");
    p.set_file_name(name);
    p
}

// -----------------------------------------------------------------------------
// MetaManager
// -----------------------------------------------------------------------------

/// Manages `.meta` companion files for a project directory.
pub struct MetaManager;

impl MetaManager {
    pub fn new() -> Self {
        Self
    }

    /// Detect the asset type from a file extension.
    pub fn detect_type(&self, path: &Path) -> AssetType {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "webp" => AssetType::Image,
            "wav" | "ogg" | "mp3" | "flac" => AssetType::Audio,
            "ttf" | "otf" | "woff" | "woff2" => AssetType::Font,
            "gltf" | "glb" | "obj" | "fbx" => AssetType::Model3d,
            "scene" | "chronos" => AssetType::Scene,
            "rhai" => AssetType::Script,
            "txt" | "md" | "json" | "toml" | "yaml" => AssetType::Text,
            other => AssetType::Unknown(other.to_string()),
        }
    }

    /// Generate a new `.meta` file for the given source asset.
    pub fn generate_meta(&self, source_path: &Path) -> Result<AssetMeta, MetaError> {
        let asset_type = self.detect_type(source_path);
        let import_settings = asset_type.default_import_settings();
        let last_modified = file_modified_timestamp(source_path)?;
        let file_hash = compute_file_hash(source_path)?;

        let meta = AssetMeta {
            guid: AssetGuid::new(),
            source_path: source_path.to_path_buf(),
            asset_type,
            import_settings,
            last_modified,
            last_imported: now_timestamp(),
            file_hash,
            thumbnail_path: None,
        };

        self.save_meta(&meta)?;
        Ok(meta)
    }

    /// Load an existing `.meta` file for the given source asset.
    pub fn load_meta(&self, source_path: &Path) -> Result<AssetMeta, MetaError> {
        let mp = meta_path_for(source_path);
        if !mp.exists() {
            return Err(MetaError::MetaNotFound(mp));
        }
        let data = fs::read_to_string(&mp)?;
        serde_json::from_str(&data).map_err(|e| MetaError::ParseFailed(e.to_string()))
    }

    /// Write a `.meta` file to disk.
    pub fn save_meta(&self, meta: &AssetMeta) -> Result<(), MetaError> {
        let mp = meta_path_for(&meta.source_path);
        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| MetaError::SerializationFailed(e.to_string()))?;
        fs::write(&mp, json)?;
        Ok(())
    }

    /// Check if the source file has been modified since the last import.
    pub fn is_stale(&self, meta: &AssetMeta) -> bool {
        match file_modified_timestamp(&meta.source_path) {
            Ok(ts) => ts > meta.last_imported,
            Err(_) => true,
        }
    }

    /// Recursively scan a directory, generating `.meta` files for any asset
    /// that does not already have one. Skips `.meta` files and dot-directories.
    pub fn scan_directory(&self, dir: &Path) -> Result<Vec<AssetMeta>, MetaError> {
        let mut results = Vec::new();
        self.scan_directory_recursive(dir, &mut results)?;
        Ok(results)
    }

    fn scan_directory_recursive(
        &self,
        dir: &Path,
        results: &mut Vec<AssetMeta>,
    ) -> Result<(), MetaError> {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden directories.
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            if path.is_dir() {
                self.scan_directory_recursive(&path, results)?;
                continue;
            }

            // Skip .meta files.
            if let Some(ext) = path.extension() {
                // A file like `hero.png.meta` has extension `meta`.
                if ext == "meta" {
                    continue;
                }
            }

            // Also skip if the filename itself ends with `.meta` (no other
            // extension case — e.g. just `foo.meta`).
            let fname = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if fname.ends_with(".meta") {
                continue;
            }

            let mp = meta_path_for(&path);
            let meta = if mp.exists() {
                self.load_meta(&path)?
            } else {
                self.generate_meta(&path)?
            };
            results.push(meta);
        }
        Ok(())
    }

    /// Load the meta for a path and return its GUID.
    pub fn guid_for_path(&self, path: &Path) -> Result<AssetGuid, MetaError> {
        let meta = self.load_meta(path)?;
        Ok(meta.guid)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(format!("/tmp/chronos_import_tests_{}", id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, b"test content").expect("write test file");
    }

    // 1. AssetGuid::new() generates valid UUID format.
    #[test]
    fn guid_new_is_valid_uuid() {
        let guid = AssetGuid::new();
        assert!(
            uuid::Uuid::parse_str(&guid.id).is_ok(),
            "expected valid UUID, got: {}",
            guid.id,
        );
    }

    // 2. AssetGuid::from_string() accepts valid UUID.
    #[test]
    fn guid_from_string_valid() {
        let s = "550e8400-e29b-41d4-a716-446655440000";
        let guid = AssetGuid::from_string(s).expect("should parse valid UUID");
        assert_eq!(guid.id, s);
    }

    // 3. AssetGuid::from_string() rejects invalid UUID.
    #[test]
    fn guid_from_string_invalid() {
        let result = AssetGuid::from_string("not-a-uuid");
        assert!(result.is_err(), "expected error for invalid UUID");
    }

    // 4. AssetType detection: .png → Image.
    #[test]
    fn detect_type_image_png() {
        let mgr = MetaManager::new();
        let t = mgr.detect_type(Path::new("hero.png"));
        assert_eq!(t, AssetType::Image);
    }

    // 5. AssetType detection: .wav → Audio.
    #[test]
    fn detect_type_audio_wav() {
        let mgr = MetaManager::new();
        let t = mgr.detect_type(Path::new("shot.wav"));
        assert_eq!(t, AssetType::Audio);
    }

    // 6. AssetType detection: .gltf → Model3d.
    #[test]
    fn detect_type_model_gltf() {
        let mgr = MetaManager::new();
        let t = mgr.detect_type(Path::new("character.gltf"));
        assert_eq!(t, AssetType::Model3d);
    }

    // 7. AssetType detection: .xyz → Unknown.
    #[test]
    fn detect_type_unknown() {
        let mgr = MetaManager::new();
        let t = mgr.detect_type(Path::new("weird.xyz"));
        assert_eq!(t, AssetType::Unknown("xyz".to_string()));
    }

    // 8. Generate .meta file and verify JSON structure.
    #[test]
    fn generate_meta_creates_valid_json() {
        let dir = test_dir();
        let asset_path = dir.join("textures").join("hero.png");
        touch(&asset_path);

        let mgr = MetaManager::new();
        let meta = mgr.generate_meta(&asset_path).expect("generate meta");

        // Check the .meta file exists on disk.
        let mp = meta_path_for(&asset_path);
        assert!(mp.exists(), ".meta file should exist");

        // Parse it back as generic JSON and verify structure.
        let raw = fs::read_to_string(&mp).expect("read meta file");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");

        assert!(v["guid"].is_string(), "guid should be a string");
        assert!(v["source_path"].is_string(), "source_path should be a string");
        assert!(v["asset_type"].is_string(), "asset_type should be a string");
        assert!(v["import_settings"].is_object(), "import_settings should be an object");
        assert!(v["last_modified"].is_number(), "last_modified should be a number");
        assert!(v["last_imported"].is_number(), "last_imported should be a number");
        assert!(v["file_hash"].is_string(), "file_hash should be a string");

        // Image-specific settings present.
        let settings = &v["import_settings"]["Image"];
        assert!(settings.is_object(), "expected Image settings");
        assert_eq!(settings["max_size"], 4096);
        assert_eq!(settings["generate_mipmaps"], true);

        // Verify the in-memory struct.
        assert_eq!(meta.asset_type, AssetType::Image);
        assert!(meta.thumbnail_path.is_none());
    }

    // 9. Roundtrip: generate → save → load → compare.
    #[test]
    fn meta_roundtrip() {
        let dir = test_dir();
        let asset_path = dir.join("audio").join("explosion.ogg");
        touch(&asset_path);

        let mgr = MetaManager::new();
        let original = mgr.generate_meta(&asset_path).expect("generate");
        let loaded = mgr.load_meta(&asset_path).expect("load");

        assert_eq!(original.guid, loaded.guid);
        assert_eq!(original.source_path, loaded.source_path);
        assert_eq!(original.asset_type, loaded.asset_type);
        assert_eq!(original.last_modified, loaded.last_modified);
        assert_eq!(original.file_hash, loaded.file_hash);
    }

    // 10. is_stale returns true when source is newer.
    #[test]
    fn is_stale_true_when_source_newer() {
        let dir = test_dir();
        let asset_path = dir.join("stale_check.bin");
        touch(&asset_path);

        let mgr = MetaManager::new();
        let mut meta = mgr.generate_meta(&asset_path).expect("generate");

        // Backdate last_imported so the source file is newer.
        meta.last_imported = 0;

        assert!(mgr.is_stale(&meta), "should be stale when source is newer");
    }

    // 11. is_stale returns false when source is older.
    #[test]
    fn is_stale_false_when_source_older() {
        let dir = test_dir();
        let asset_path = dir.join("fresh_check.bin");
        touch(&asset_path);

        let mgr = MetaManager::new();
        let meta = mgr.generate_meta(&asset_path).expect("generate");

        // last_imported was set to now, source was touched just before,
        // so it should not be stale. Sleep briefly to ensure ordering.
        assert!(!mgr.is_stale(&meta), "should not be stale right after generate");
    }

    // 12. scan_directory finds assets and generates missing .meta files.
    #[test]
    fn scan_directory_generates_missing_meta() {
        let dir = test_dir();
        touch(&dir.join("img1.png"));
        touch(&dir.join("sound.wav"));
        touch(&dir.join("model.obj"));
        // Create a dot-directory that should be skipped.
        let dot = dir.join(".hidden");
        let _ = fs::create_dir_all(&dot);
        touch(&dot.join("secret.txt"));

        let mgr = MetaManager::new();
        let metas = mgr.scan_directory(&dir).expect("scan");

        // Should find exactly 3 assets (dot-dir skipped).
        assert_eq!(metas.len(), 3, "expected 3 assets, got {}", metas.len());

        // All .meta files should exist.
        assert!(meta_path_for(&dir.join("img1.png")).exists());
        assert!(meta_path_for(&dir.join("sound.wav")).exists());
        assert!(meta_path_for(&dir.join("model.obj")).exists());

        // Verify types.
        let types: HashSet<String> = metas
            .iter()
            .map(|m| {
                let v = serde_json::to_string(&m.asset_type).unwrap();
                v.trim_matches('"').to_string()
            })
            .collect();
        assert!(types.contains("Image"), "expected Image type");
        assert!(types.contains("Audio"), "expected Audio type");
        assert!(types.contains("Model3d"), "expected Model3d type");
    }

    // 13. ImportSettings defaults are sensible for each type.
    #[test]
    fn import_settings_defaults() {
        let img = ImageImportConfig::default();
        assert_eq!(img.max_size, 4096);
        assert!(img.generate_mipmaps);
        assert!(!img.premultiply_alpha);
        assert!(!img.texture_compression);

        let audio = AudioImportConfig::default();
        assert!(audio.target_sample_rate.is_none());
        assert!(!audio.normalize);
        assert!(!audio.convert_to_mono);
        assert!((audio.quality - 1.0).abs() < f32::EPSILON);

        let font = FontImportConfig::default();
        assert!(!font.characters.is_empty());
        assert!(!font.sdf_enabled);
        assert_eq!(font.atlas_size, 512);

        let model = ModelImportConfig::default();
        assert!(model.import_animations);
        assert!(model.import_materials);
        assert!(model.import_skeletons);
        assert!((model.scale - 1.0).abs() < f32::EPSILON);
    }

    // Additional: compute_hash_from_bytes produces 16-char hex string.
    #[test]
    fn hash_is_16_hex_chars() {
        let h = compute_hash_from_bytes(b"hello world");
        assert_eq!(h.len(), 16, "hash should be 16 hex chars");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // Additional: guid_for_path works.
    #[test]
    fn guid_for_path_returns_guid() {
        let dir = test_dir();
        let asset_path = dir.join("guided.png");
        touch(&asset_path);

        let mgr = MetaManager::new();
        mgr.generate_meta(&asset_path).expect("generate");

        let guid = mgr.guid_for_path(&asset_path).expect("get guid");
        assert!(
            uuid::Uuid::parse_str(&guid.id).is_ok(),
            "guid should be valid UUID",
        );
    }

    // Additional: load_meta returns MetaNotFound when no .meta file exists.
    #[test]
    fn load_meta_not_found() {
        let dir = test_dir();
        let missing = dir.join("nonexistent.png");
        let mgr = MetaManager::new();
        let result = mgr.load_meta(&missing);
        assert!(matches!(result, Err(MetaError::MetaNotFound(_))));
    }

    // Additional: detect_type covers all text extensions.
    #[test]
    fn detect_type_text_variants() {
        let mgr = MetaManager::new();
        assert_eq!(mgr.detect_type(Path::new("readme.md")), AssetType::Text);
        assert_eq!(mgr.detect_type(Path::new("data.json")), AssetType::Text);
        assert_eq!(mgr.detect_type(Path::new("config.toml")), AssetType::Text);
        assert_eq!(mgr.detect_type(Path::new("notes.yaml")), AssetType::Text);
        assert_eq!(mgr.detect_type(Path::new("plain.txt")), AssetType::Text);
    }

    // Additional: scan skips .meta files.
    #[test]
    fn scan_skips_meta_files() {
        let dir = test_dir();
        touch(&dir.join("sprite.png"));
        // Manually create a .meta file that shouldn't be treated as an asset.
        touch(&dir.join("orphan.meta"));

        let mgr = MetaManager::new();
        let metas = mgr.scan_directory(&dir).expect("scan");
        assert_eq!(metas.len(), 1, "should find only the .png, not the .meta");
    }
}

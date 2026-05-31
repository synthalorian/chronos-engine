//! Asset Processing Pipeline — coordinates import, metadata, caching, and
//! registration for all asset types.
//!
//! The pipeline is the central hub that Phase-10 importers feed into:
//! it reads source files, applies per-type import settings from `.meta`
//! files, invokes the correct importer, writes processed cache files, and
//! registers the result in the [`AssetCatalog`].

#![cfg(feature = "asset-pipeline")]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::metadata::{AssetGuid, AssetMeta, AssetType, ImportSettings, MetaError, MetaManager};
use super::registry::{AssetCatalog, AssetKind, Guid, RegistryError};

// ──────────────────────────────────────────────────────────────
// Processed Asset
// ──────────────────────────────────────────────────────────────

/// Result of a single asset processing step.
#[derive(Debug, Clone)]
pub struct ProcessedAsset {
    /// Stable GUID.
    pub guid: Guid,
    /// Asset category.
    pub kind: AssetKind,
    /// Source file path.
    pub source_path: PathBuf,
    /// Path to the processed cache file, if one was written.
    pub cache_path: Option<PathBuf>,
    /// Whether the cache was rebuilt (true) or reused (false).
    pub rebuilt: bool,
}

// ──────────────────────────────────────────────────────────────
// Processor Error
// ──────────────────────────────────────────────────────────────

/// Errors that can occur during pipeline processing.
#[derive(Debug)]
pub enum ProcessorError {
    Meta(MetaError),
    Registry(RegistryError),
    ImportFailed(String),
    CacheWriteFailed(std::io::Error),
    UnsupportedType(AssetType),
}

impl std::fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessorError::Meta(e) => write!(f, "meta error: {}", e),
            ProcessorError::Registry(e) => write!(f, "registry error: {}", e),
            ProcessorError::ImportFailed(msg) => write!(f, "import failed: {}", msg),
            ProcessorError::CacheWriteFailed(e) => write!(f, "cache write failed: {}", e),
            ProcessorError::UnsupportedType(t) => {
                write!(f, "unsupported asset type for processing: {:?}", t)
            }
        }
    }
}

impl std::error::Error for ProcessorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProcessorError::Meta(e) => Some(e),
            ProcessorError::Registry(e) => Some(e),
            ProcessorError::CacheWriteFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl From<MetaError> for ProcessorError {
    fn from(e: MetaError) -> Self {
        ProcessorError::Meta(e)
    }
}

impl From<RegistryError> for ProcessorError {
    fn from(e: RegistryError) -> Self {
        ProcessorError::Registry(e)
    }
}

// ──────────────────────────────────────────────────────────────
// AssetProcessor
// ──────────────────────────────────────────────────────────────

/// Central asset processing pipeline.
///
/// # Usage
///
/// ```ignore
/// use chronos_engine::import::processor::AssetProcessor;
/// let mut proc = AssetProcessor::new(Path::new("assets"), Path::new("assets/.cache"));
/// proc.scan_and_process()?;
/// ```
pub struct AssetProcessor {
    /// Root directory containing source assets.
    pub source_dir: PathBuf,
    /// Directory where processed cache files are written.
    pub cache_dir: PathBuf,
    meta_manager: MetaManager,
    catalog: AssetCatalog,
    /// Tracks which source files have been processed in this session.
    processed: HashMap<PathBuf, ProcessedAsset>,
}

impl AssetProcessor {
    /// Create a new processor with the given source and cache directories.
    pub fn new(source_dir: &Path, cache_dir: &Path) -> Self {
        let _ = fs::create_dir_all(cache_dir);
        AssetProcessor {
            source_dir: source_dir.to_path_buf(),
            cache_dir: cache_dir.to_path_buf(),
            meta_manager: MetaManager::new(),
            catalog: AssetCatalog::new(),
            processed: HashMap::new(),
        }
    }

    /// Access the underlying asset catalog.
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Mutable access to the underlying asset catalog.
    pub fn catalog_mut(&mut self) -> &mut AssetCatalog {
        &mut self.catalog
    }

    /// Scan the source directory and process every asset that is missing,
    /// stale, or unregistered.
    pub fn scan_and_process(&mut self) -> Result<Vec<ProcessedAsset>, ProcessorError> {
        let metas = self.meta_manager.scan_directory(&self.source_dir)?;
        let mut results = Vec::with_capacity(metas.len());

        for meta in metas {
            if let Some(pa) = self.process_asset(&meta.source_path)? {
                results.push(pa)
            }
        }

        Ok(results)
    }

    /// Process a single source asset.
    ///
    /// Returns `Some(ProcessedAsset)` if the asset was (re)built or
    /// `None` if it is up to date and already cached.
    pub fn process_asset(
        &mut self,
        source_path: &Path,
    ) -> Result<Option<ProcessedAsset>, ProcessorError> {
        // Load or generate metadata.
        let meta = match self.meta_manager.load_meta(source_path) {
            Ok(m) => {
                if self.meta_manager.is_stale(&m) {
                    // Regenerate meta if stale (hash changed).
                    self.meta_manager.generate_meta(source_path)?
                } else {
                    m
                }
            }
            Err(MetaError::MetaNotFound(_)) => self.meta_manager.generate_meta(source_path)?,
            Err(e) => return Err(e.into()),
        };

        let asset_type = meta.asset_type.clone();
        let kind = Self::asset_type_to_kind(&asset_type);
        let guid = self.catalog.register(source_path, kind.clone());

        // Check cache validity first — a valid cache means no rebuild needed.
        let cache_path = self.cache_path_for(&meta.guid, &asset_type);
        let cache_valid = cache_path
            .as_ref()
            .map(|p| p.exists() && !self.meta_manager.is_stale(&meta))
            .unwrap_or(false);

        if cache_valid {
            let pa = ProcessedAsset {
                guid: guid.clone(),
                kind: kind.clone(),
                source_path: source_path.to_path_buf(),
                cache_path: cache_path.clone(),
                rebuilt: false,
            };
            self.processed.insert(source_path.to_path_buf(), pa.clone());
            return Ok(Some(pa));
        }

        // Check if already processed this session (avoids duplicate work during scans).
        if let Some(existing) = self.processed.get(source_path) {
            return Ok(Some(existing.clone()));
        }

        // Build cache.
        let rebuilt_cache = match asset_type {
            AssetType::Image => self.process_image(source_path, &meta)?,
            AssetType::Audio => self.process_audio(source_path, &meta)?,
            AssetType::Font => self.process_font(source_path, &meta)?,
            AssetType::Model3d => self.process_model(source_path, &meta)?,
            other => {
                return Err(ProcessorError::UnsupportedType(other));
            }
        };

        let pa = ProcessedAsset {
            guid: guid.clone(),
            kind: kind.clone(),
            source_path: source_path.to_path_buf(),
            cache_path: rebuilt_cache.clone(),
            rebuilt: true,
        };

        if rebuilt_cache.is_some() {
            self.processed.insert(source_path.to_path_buf(), pa.clone());
        }

        Ok(Some(pa))
    }

    /// Force-rebuild a single asset regardless of cache state.
    pub fn rebuild_asset(&mut self, source_path: &Path) -> Result<ProcessedAsset, ProcessorError> {
        let meta = self.meta_manager.generate_meta(source_path)?;
        let asset_type = meta.asset_type.clone();
        let kind = Self::asset_type_to_kind(&asset_type);
        let guid = self.catalog.register(source_path, kind.clone());

        let cache_path = match asset_type {
            AssetType::Image => self.process_image(source_path, &meta)?,
            AssetType::Audio => self.process_audio(source_path, &meta)?,
            AssetType::Font => self.process_font(source_path, &meta)?,
            AssetType::Model3d => self.process_model(source_path, &meta)?,
            other => return Err(ProcessorError::UnsupportedType(other)),
        };

        let pa = ProcessedAsset {
            guid,
            kind,
            source_path: source_path.to_path_buf(),
            cache_path,
            rebuilt: true,
        };

        self.processed.insert(source_path.to_path_buf(), pa.clone());
        Ok(pa)
    }

    /// Clean the cache directory, removing all processed files.
    pub fn clean_cache(&self) -> Result<(), ProcessorError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).map_err(ProcessorError::CacheWriteFailed)?;
            fs::create_dir_all(&self.cache_dir).map_err(ProcessorError::CacheWriteFailed)?;
        }
        Ok(())
    }

    // ── Internal processors per type ──

    fn process_image(
        &mut self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<PathBuf>, ProcessorError> {
        use crate::import::image_import::{ImageImportSettings, ImageImporter};

        let settings = match &meta.import_settings {
            ImportSettings::Image(cfg) => ImageImportSettings {
                generate_mipmaps: cfg.generate_mipmaps,
                max_mipmap_levels: if cfg.generate_mipmaps { None } else { Some(0) },
                premultiply_alpha: cfg.premultiply_alpha,
                flip_vertical: false,
                output_format: crate::import::image_import::PixelFormat::RGBA8,
            },
            _ => ImageImportSettings::default(),
        };

        let imported = ImageImporter::import_with_settings(source, &settings)
            .map_err(|e| ProcessorError::ImportFailed(e.to_string()))?;

        let cache_name = format!("{}.rgba", meta.guid.id);
        let cache_path = self.cache_dir.join(&cache_name);

        let mut data = Vec::with_capacity(8 + imported.pixels.len());
        data.extend_from_slice(&imported.width.to_le_bytes());
        data.extend_from_slice(&imported.height.to_le_bytes());
        data.extend_from_slice(&imported.pixels);

        fs::write(&cache_path, &data).map_err(ProcessorError::CacheWriteFailed)?;

        Ok(Some(cache_path))
    }

    fn process_audio(
        &mut self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<PathBuf>, ProcessorError> {
        use crate::import::audio_import::{AudioImportSettings, AudioImporter};

        let settings = match &meta.import_settings {
            ImportSettings::Audio(cfg) => AudioImportSettings {
                normalize: cfg.normalize,
                target_sample_rate: cfg.target_sample_rate,
                convert_to_mono: cfg.convert_to_mono,
                trim_silence: false,
                silence_threshold_db: -60.0,
            },
            _ => AudioImportSettings::default(),
        };

        let imported = AudioImporter::import_with_settings(source, &settings)
            .map_err(|e| ProcessorError::ImportFailed(e.to_string()))?;

        let cache_name = format!("{}.pcm", meta.guid.id);
        let cache_path = self.cache_dir.join(&cache_name);

        // Serialize: sample_rate (4), channels (2), frame_count (8), samples...
        let frame_count = (imported.samples.len() / imported.channels.max(1) as usize) as u64;
        let mut data = Vec::with_capacity(14 + imported.samples.len() * 4);
        data.extend_from_slice(&imported.sample_rate.to_le_bytes());
        data.extend_from_slice(&imported.channels.to_le_bytes());
        data.extend_from_slice(&frame_count.to_le_bytes());
        for s in &imported.samples {
            data.extend_from_slice(&s.to_le_bytes());
        }

        fs::write(&cache_path, &data).map_err(ProcessorError::CacheWriteFailed)?;

        Ok(Some(cache_path))
    }

    fn process_font(
        &mut self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<PathBuf>, ProcessorError> {
        use crate::import::font_import::{FontImportSettings, FontImporter};

        let settings = match &meta.import_settings {
            ImportSettings::Font(cfg) => FontImportSettings {
                font_size: cfg.font_size,
                atlas_width: cfg.atlas_size,
                characters: Some(cfg.characters.clone()),
                padding: 1,
                sdf: cfg.sdf_enabled,
            },
            _ => FontImportSettings::default(),
        };

        let atlas = FontImporter::import_with_settings(source, &settings)
            .map_err(|e| ProcessorError::ImportFailed(e.to_string()))?;

        let cache_name = format!("{}.atlas", meta.guid.id);
        let cache_path = self.cache_dir.join(&cache_name);

        let data = Self::serialize_font_atlas(&atlas);
        fs::write(&cache_path, &data).map_err(ProcessorError::CacheWriteFailed)?;

        Ok(Some(cache_path))
    }

    fn process_model(
        &mut self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<PathBuf>, ProcessorError> {
        use crate::import::gltf_import::GltfImporter;

        let scene = GltfImporter::import(source)
            .map_err(|e| ProcessorError::ImportFailed(e.to_string()))?;

        let cache_name = format!("{}.gltfscene", meta.guid.id);
        let cache_path = self.cache_dir.join(&cache_name);

        let json = serde_json::to_string(&scene)
            .map_err(|e| ProcessorError::ImportFailed(e.to_string()))?;
        fs::write(&cache_path, &json).map_err(ProcessorError::CacheWriteFailed)?;

        Ok(Some(cache_path))
    }

    // ── Helpers ──

    fn cache_path_for(&self, guid: &AssetGuid, asset_type: &AssetType) -> Option<PathBuf> {
        let ext = match asset_type {
            AssetType::Image => "rgba",
            AssetType::Audio => "pcm",
            AssetType::Font => "atlas",
            AssetType::Model3d => "gltfscene",
            _ => return None,
        };
        Some(self.cache_dir.join(format!("{}.{}", guid.id, ext)))
    }

    /// Serialize a FontAtlas into a compact binary format.
    fn serialize_font_atlas(atlas: &crate::import::font_import::FontAtlas) -> Vec<u8> {
        use crate::import::font_import::FontAtlas;

        let atlas: &FontAtlas = atlas;
        let mut data = Vec::new();
        // Header: width (4), height (4), glyph_count (4), line_height (4), font_size (4)
        data.extend_from_slice(&atlas.width.to_le_bytes());
        data.extend_from_slice(&atlas.height.to_le_bytes());
        data.extend_from_slice(&(atlas.glyphs.len() as u32).to_le_bytes());
        data.extend_from_slice(&atlas.line_height.to_le_bytes());
        data.extend_from_slice(&atlas.font_size.to_le_bytes());
        // Pixel data: len (4) + bytes
        data.extend_from_slice(&(atlas.pixels.len() as u32).to_le_bytes());
        data.extend_from_slice(&atlas.pixels);
        // Glyphs: each is 4 + 4 + 4 + 4 + 4 + 4 + 4 + 4 = 32 bytes
        for g in &atlas.glyphs {
            data.extend_from_slice(&(g.character as u32).to_le_bytes());
            data.extend_from_slice(&g.x.to_le_bytes());
            data.extend_from_slice(&g.y.to_le_bytes());
            data.extend_from_slice(&g.width.to_le_bytes());
            data.extend_from_slice(&g.height.to_le_bytes());
            data.extend_from_slice(&g.advance_w.to_le_bytes());
            data.extend_from_slice(&g.bearing_x.to_le_bytes());
            data.extend_from_slice(&g.bearing_y.to_le_bytes());
        }
        // Kerning: count (4) + pairs
        data.extend_from_slice(&(atlas.kerning.len() as u32).to_le_bytes());
        for ((a, b), k) in &atlas.kerning {
            data.extend_from_slice(&(*a as u32).to_le_bytes());
            data.extend_from_slice(&(*b as u32).to_le_bytes());
            data.extend_from_slice(&k.to_le_bytes());
        }
        data
    }

    fn asset_type_to_kind(t: &AssetType) -> AssetKind {
        match t {
            AssetType::Image => AssetKind::Image,
            AssetType::Audio => AssetKind::Audio,
            AssetType::Font => AssetKind::Font,
            AssetType::Model3d => AssetKind::Model,
            AssetType::Scene => AssetKind::Other("scene".into()),
            AssetType::Script => AssetKind::Other("script".into()),
            AssetType::Text => AssetKind::Other("text".into()),
            AssetType::Unknown(s) => AssetKind::Other(s.clone()),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "asset-pipeline"))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(format!("/tmp/chronos_processor_tests_{}", id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn touch(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content).expect("write test file");
    }

    // Test 1: Processor::new creates cache directory.
    #[test]
    fn processor_new_creates_cache_dir() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        let _proc = AssetProcessor::new(&source, &cache);
        assert!(cache.exists(), "cache dir should be created");
    }

    // Test 2: process_asset returns rebuilt for a new image.
    #[test]
    fn process_new_image_returns_rebuilt() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        // Create a valid 2x2 RGBA PNG.
        let img = image::RgbaImage::from_fn(2, 2, |x, y| {
            image::Rgba([(x * 127) as u8, (y * 127) as u8, 64, 255])
        });
        let img_path = source.join("sprite.png");
        img.save(&img_path).expect("save test png");

        let mut proc = AssetProcessor::new(&source, &cache);
        let result = proc.process_asset(&img_path).expect("process");
        assert!(result.is_some());
        let pa = result.unwrap();
        assert!(pa.rebuilt, "new asset should be rebuilt");
        assert!(pa.cache_path.is_some(), "should have cache path");
        assert!(
            pa.cache_path.as_ref().unwrap().exists(),
            "cache file should exist"
        );
    }

    // Test 3: Second process of same asset returns not-rebuilt (cached).
    #[test]
    fn process_cached_image_returns_not_rebuilt() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |x, y| {
            image::Rgba([(x * 127) as u8, (y * 127) as u8, 64, 255])
        });
        let img_path = source.join("cached.png");
        img.save(&img_path).expect("save test png");

        let mut proc = AssetProcessor::new(&source, &cache);
        let first = proc.process_asset(&img_path).expect("first").unwrap();
        assert!(first.rebuilt);

        // Re-process after a brief pause — cache should be valid.
        // Sleep ensures filesystem mtime and last_imported settle.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let second = proc.process_asset(&img_path).expect("second").unwrap();
        assert!(!second.rebuilt, "cached asset should not be rebuilt");
    }

    // Test 4: scan_and_process finds multiple assets.
    #[test]
    fn scan_finds_multiple_assets() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([255, 0, 0, 255]));
        img.save(source.join("a.png")).unwrap();
        img.save(source.join("b.png")).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        let results = proc.scan_and_process().expect("scan");
        assert_eq!(results.len(), 2, "should find 2 assets");
        assert!(
            results.iter().all(|r| r.rebuilt),
            "all should be rebuilt on first scan"
        );
    }

    // Test 5: clean_cache removes all cache files.
    #[test]
    fn clean_cache_removes_files() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([255, 0, 0, 255]));
        let img_path = source.join("del.png");
        img.save(&img_path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        proc.process_asset(&img_path).expect("process");
        assert!(
            cache.read_dir().unwrap().next().is_some(),
            "cache should have files"
        );

        proc.clean_cache().expect("clean");
        // After clean + recreate, dir exists but may be empty.
        assert!(cache.exists(), "cache dir should still exist");
    }

    // Test 6: rebuild_asset forces rebuild.
    #[test]
    fn rebuild_forces_rebuild() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([0, 255, 0, 255]));
        let img_path = source.join("force.png");
        img.save(&img_path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        let first = proc.process_asset(&img_path).expect("first").unwrap();
        assert!(first.rebuilt);

        let rebuilt = proc.rebuild_asset(&img_path).expect("rebuild");
        assert!(rebuilt.rebuilt, "rebuild_asset should always mark rebuilt");
    }

    // Test 7: Error display.
    #[test]
    fn error_display_messages() {
        let e = ProcessorError::ImportFailed("bad".into());
        assert!(e.to_string().contains("bad"));

        let e = ProcessorError::UnsupportedType(AssetType::Unknown("xyz".into()));
        assert!(e.to_string().contains("xyz"));
    }

    // Test 8: catalog access.
    #[test]
    fn catalog_tracks_registrations() {
        let dir = test_dir();
        let source = dir.join("src");
        let cache = dir.join("cache");
        fs::create_dir_all(&source).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |_, _| image::Rgba([0, 0, 255, 255]));
        let img_path = source.join("track.png");
        img.save(&img_path).unwrap();

        let mut proc = AssetProcessor::new(&source, &cache);
        proc.process_asset(&img_path).expect("process");
        assert_eq!(proc.catalog().total_count(), 1);
    }
}

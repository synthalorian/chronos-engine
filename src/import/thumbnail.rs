//! Thumbnail Generation — produces small preview images for the asset browser.
//!
//! Supports thumbnails for images (resize), fonts (render sample text), and
//! placeholder thumbnails for models/audio. Thumbnails are saved as small
//! RGBA8 PNG files next to the `.meta` files.

#![cfg(feature = "asset-pipeline")]

use std::fs;
use std::path::{Path, PathBuf};

use super::metadata::{AssetMeta, AssetType, MetaManager};

// ──────────────────────────────────────────────────────────────
// Thumbnail
// ──────────────────────────────────────────────────────────────

/// A generated thumbnail with pixel data and its saved path.
#[derive(Debug, Clone)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub path: PathBuf,
}

// ──────────────────────────────────────────────────────────────
// Thumbnail Error
// ──────────────────────────────────────────────────────────────

/// Errors during thumbnail generation.
#[derive(Debug)]
pub enum ThumbnailError {
    Io(std::io::Error),
    ImportFailed(String),
    UnsupportedType(AssetType),
}

impl std::fmt::Display for ThumbnailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbnailError::Io(e) => write!(f, "thumbnail I/O error: {}", e),
            ThumbnailError::ImportFailed(msg) => write!(f, "thumbnail import failed: {}", msg),
            ThumbnailError::UnsupportedType(t) => {
                write!(f, "thumbnail unsupported for type: {:?}", t)
            }
        }
    }
}

impl std::error::Error for ThumbnailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ThumbnailError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ThumbnailError {
    fn from(e: std::io::Error) -> Self {
        ThumbnailError::Io(e)
    }
}

// ──────────────────────────────────────────────────────────────
// ThumbnailGenerator
// ──────────────────────────────────────────────────────────────

/// Default thumbnail dimensions.
pub const THUMB_WIDTH: u32 = 128;
pub const THUMB_HEIGHT: u32 = 128;

/// Generates preview thumbnails for assets.
pub struct ThumbnailGenerator {
    pub thumb_width: u32,
    pub thumb_height: u32,
    /// Directory where thumbnails are stored. Defaults to `<source>/.thumbnails`.
    pub thumb_dir: PathBuf,
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        ThumbnailGenerator {
            thumb_width: THUMB_WIDTH,
            thumb_height: THUMB_HEIGHT,
            thumb_dir: PathBuf::from(".thumbnails"),
        }
    }
}

impl ThumbnailGenerator {
    /// Create a generator with default size and a given thumbnail directory.
    pub fn new(thumb_dir: &Path) -> Self {
        let _ = fs::create_dir_all(thumb_dir);
        ThumbnailGenerator {
            thumb_width: THUMB_WIDTH,
            thumb_height: THUMB_HEIGHT,
            thumb_dir: thumb_dir.to_path_buf(),
        }
    }

    /// Generate a thumbnail for the given source asset and its metadata.
    ///
    /// Returns `Some(Thumbnail)` on success, or `None` if the type does not
    /// support thumbnails.
    pub fn generate(
        &self,
        source_path: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<Thumbnail>, ThumbnailError> {
        match meta.asset_type {
            AssetType::Image => self.generate_image_thumbnail(source_path, meta),
            AssetType::Font => self.generate_font_thumbnail(source_path, meta),
            AssetType::Model3d => self.generate_model_thumbnail(source_path, meta),
            AssetType::Audio => self.generate_audio_thumbnail(source_path, meta),
            _ => Ok(None),
        }
    }

    /// Generate thumbnails for every asset in a directory tree.
    pub fn generate_all(&self, source_dir: &Path) -> Result<Vec<Thumbnail>, ThumbnailError> {
        let meta_mgr = MetaManager::new();
        let metas = meta_mgr
            .scan_directory(source_dir)
            .map_err(|e| ThumbnailError::ImportFailed(e.to_string()))?;

        let mut results = Vec::new();
        for meta in metas {
            if let Some(thumb) = self.generate(&meta.source_path, &meta)? {
                results.push(thumb);
            }
        }
        Ok(results)
    }

    // ── Per-type generators ──

    fn generate_image_thumbnail(
        &self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<Thumbnail>, ThumbnailError> {
        use crate::import::image_import::ImageImporter;

        let imported = ImageImporter::import(source)
            .map_err(|e| ThumbnailError::ImportFailed(e.to_string()))?;

        let (w, h, pixels) =
            if imported.width <= self.thumb_width && imported.height <= self.thumb_height {
                // No resize needed.
                (imported.width, imported.height, imported.pixels)
            } else {
                // Simple box-filter downscale to thumbnail size.
                let (tw, th) = fit_rect(
                    imported.width,
                    imported.height,
                    self.thumb_width,
                    self.thumb_height,
                );
                let scaled =
                    box_filter_resize(&imported.pixels, imported.width, imported.height, tw, th);
                (tw, th, scaled)
            };

        let path = self.thumb_path(source, meta);
        self.write_png(&path, w, h, &pixels)?;

        Ok(Some(Thumbnail {
            width: w,
            height: h,
            pixels,
            path,
        }))
    }

    fn generate_font_thumbnail(
        &self,
        source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<Thumbnail>, ThumbnailError> {
        use crate::import::font_import::{FontImportSettings, FontImporter};

        let settings = FontImportSettings {
            font_size: 24.0,
            atlas_width: self.thumb_width,
            ..Default::default()
        };

        let atlas = FontImporter::import_with_settings(source, &settings)
            .map_err(|e| ThumbnailError::ImportFailed(e.to_string()))?;

        // The atlas pixel data is already RGBA8; crop/pad to thumb size.
        let (w, h, pixels) = crop_or_pad_rgba(
            &atlas.pixels,
            atlas.width,
            atlas.height,
            self.thumb_width,
            self.thumb_height,
        );

        let path = self.thumb_path(source, meta);
        self.write_png(&path, w, h, &pixels)?;

        Ok(Some(Thumbnail {
            width: w,
            height: h,
            pixels,
            path,
        }))
    }

    fn generate_model_thumbnail(
        &self,
        _source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<Thumbnail>, ThumbnailError> {
        // Placeholder: generate a solid-colour cube icon.
        let pixels = generate_placeholder_rgba(
            self.thumb_width,
            self.thumb_height,
            [100, 180, 255, 255], // light blue
        );
        let path = self.thumb_path(_source, meta);
        self.write_png(&path, self.thumb_width, self.thumb_height, &pixels)?;

        Ok(Some(Thumbnail {
            width: self.thumb_width,
            height: self.thumb_height,
            pixels,
            path,
        }))
    }

    fn generate_audio_thumbnail(
        &self,
        _source: &Path,
        meta: &AssetMeta,
    ) -> Result<Option<Thumbnail>, ThumbnailError> {
        // Placeholder: generate a waveform-style icon.
        let mut pixels = generate_placeholder_rgba(
            self.thumb_width,
            self.thumb_height,
            [80, 200, 120, 255], // green
        );

        // Draw a simple sine wave in white across the center row.
        let cy = self.thumb_height / 2;
        let amplitude = (self.thumb_height / 4) as i32;
        for x in 0..self.thumb_width {
            let angle = (x as f32) / (self.thumb_width as f32) * std::f32::consts::TAU * 3.0;
            let y = cy as i32 + (angle.sin() * amplitude as f32) as i32;
            let y = y.clamp(0, self.thumb_height as i32 - 1) as u32;
            let idx = ((y * self.thumb_width + x) * 4) as usize;
            if idx + 3 < pixels.len() {
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }

        let path = self.thumb_path(_source, meta);
        self.write_png(&path, self.thumb_width, self.thumb_height, &pixels)?;

        Ok(Some(Thumbnail {
            width: self.thumb_width,
            height: self.thumb_height,
            pixels,
            path,
        }))
    }

    // ── Helpers ──

    fn thumb_path(&self, _source: &Path, meta: &AssetMeta) -> PathBuf {
        let file_name = format!("{}.thumb.png", meta.guid.id);
        self.thumb_dir.join(file_name)
    }

    fn write_png(&self, path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), ThumbnailError> {
        let img = image::RgbaImage::from_raw(w, h, rgba.to_vec())
            .ok_or_else(|| ThumbnailError::ImportFailed("invalid rgba dimensions".into()))?;
        img.save(path)
            .map_err(|e| ThumbnailError::ImportFailed(e.to_string()))?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// Internal image helpers
// ──────────────────────────────────────────────────────────────

/// Compute a rectangle that fits inside `max_w × max_h` while preserving
/// the original aspect ratio.
fn fit_rect(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let src_aspect = src_w as f32 / src_h.max(1) as f32;
    let max_aspect = max_w as f32 / max_h.max(1) as f32;

    if src_aspect > max_aspect {
        let w = max_w;
        let h = (max_w as f32 / src_aspect).round() as u32;
        (w, h.max(1))
    } else {
        let h = max_h;
        let w = (max_h as f32 * src_aspect).round() as u32;
        (w.max(1), h)
    }
}

/// Box-filter resize RGBA8 data.
fn box_filter_resize(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];

    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx0 = (dx as f32 * x_ratio) as u32;
            let sx1 = ((dx as f32 + 1.0) * x_ratio).ceil() as u32;
            let sy0 = (dy as f32 * y_ratio) as u32;
            let sy1 = ((dy as f32 + 1.0) * y_ratio).ceil() as u32;

            let sx1 = sx1.min(src_w);
            let sy1 = sy1.min(src_h);
            let count = ((sx1 - sx0) * (sy1 - sy0)).max(1);

            let mut r: u32 = 0;
            let mut g: u32 = 0;
            let mut b: u32 = 0;
            let mut a: u32 = 0;

            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let idx = ((sy * src_w + sx) * 4) as usize;
                    r += src[idx] as u32;
                    g += src[idx + 1] as u32;
                    b += src[idx + 2] as u32;
                    a += src[idx + 3] as u32;
                }
            }

            let didx = ((dy * dst_w + dx) * 4) as usize;
            dst[didx] = (r / count) as u8;
            dst[didx + 1] = (g / count) as u8;
            dst[didx + 2] = (b / count) as u8;
            dst[didx + 3] = (a / count) as u8;
        }
    }

    dst
}

/// Crop or pad RGBA data to fit `target_w × target_h`.
fn crop_or_pad_rgba(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    target_w: u32,
    target_h: u32,
) -> (u32, u32, Vec<u8>) {
    if src_w == target_w && src_h == target_h {
        return (src_w, src_h, src.to_vec());
    }

    let mut dst = vec![0u8; (target_w * target_h * 4) as usize];
    let copy_w = src_w.min(target_w);
    let copy_h = src_h.min(target_h);

    for y in 0..copy_h {
        for x in 0..copy_w {
            let sidx = ((y * src_w + x) * 4) as usize;
            let didx = ((y * target_w + x) * 4) as usize;
            dst[didx] = src[sidx];
            dst[didx + 1] = src[sidx + 1];
            dst[didx + 2] = src[sidx + 2];
            dst[didx + 3] = src[sidx + 3];
        }
    }

    (target_w, target_h, dst)
}

/// Fill an RGBA buffer with a solid colour.
fn generate_placeholder_rgba(w: u32, h: u32, colour: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        pixels.extend_from_slice(&colour);
    }
    pixels
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
        let dir = PathBuf::from(format!("/tmp/chronos_thumbnail_tests_{}", id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    // Test 1: fit_rect preserves aspect ratio.
    #[test]
    fn fit_rect_preserves_aspect() {
        let (w, h) = fit_rect(100, 50, 64, 64);
        assert_eq!(w, 64);
        assert_eq!(h, 32);

        let (w, h) = fit_rect(50, 100, 64, 64);
        assert_eq!(w, 32);
        assert_eq!(h, 64);
    }

    // Test 2: box_filter_resize produces correct dimensions.
    #[test]
    fn box_filter_resize_dimensions() {
        let src = vec![255u8; 16 * 16 * 4];
        let dst = box_filter_resize(&src, 16, 16, 4, 4);
        assert_eq!(dst.len(), 4 * 4 * 4);
    }

    // Test 3: generate_placeholder_rgba size.
    #[test]
    fn placeholder_size() {
        let p = generate_placeholder_rgba(10, 10, [1, 2, 3, 4]);
        assert_eq!(p.len(), 10 * 10 * 4);
        assert_eq!(p[0], 1);
        assert_eq!(p[3], 4);
    }

    // Test 4: crop_or_pad_rgba exact match.
    #[test]
    fn crop_or_pad_exact() {
        let src = vec![255u8; 8 * 8 * 4];
        let (w, h, dst) = crop_or_pad_rgba(&src, 8, 8, 8, 8);
        assert_eq!(w, 8);
        assert_eq!(h, 8);
        assert_eq!(dst, src);
    }

    // Test 5: ThumbnailGenerator::new creates dir.
    #[test]
    fn generator_new_creates_dir() {
        let dir = test_dir().join("thumbs");
        let gen = ThumbnailGenerator::new(&dir);
        assert!(gen.thumb_dir.exists());
    }

    // Test 6: generate_image_thumbnail from real PNG.
    #[test]
    fn generate_image_thumbnail_real() {
        let dir = test_dir();
        // Use a 256x256 image so it gets downscaled to 128x128 by fit_rect.
        let img = image::RgbaImage::from_fn(256, 256, |x, y| {
            image::Rgba([
                (x as u8).wrapping_mul(1),
                (y as u8).wrapping_mul(1),
                128,
                255,
            ])
        });
        let path = dir.join("test.png");
        img.save(&path).unwrap();

        let meta_mgr = MetaManager::new();
        let meta = meta_mgr.generate_meta(&path).unwrap();

        let gen = ThumbnailGenerator::new(&dir.join("thumbs"));
        let thumb = gen.generate(&path, &meta).expect("generate").expect("some");

        assert_eq!(thumb.width, 128);
        assert_eq!(thumb.height, 128);
        assert!(thumb.path.exists(), "thumbnail file should exist");
    }

    // Test 7: generate_model_thumbnail produces placeholder.
    #[test]
    fn generate_model_thumbnail_placeholder() {
        let dir = test_dir();
        let path = dir.join("model.gltf");
        fs::write(&path, b"{}").unwrap();

        let meta_mgr = MetaManager::new();
        let meta = meta_mgr.generate_meta(&path).unwrap();

        let gen = ThumbnailGenerator::new(&dir.join("thumbs"));
        let thumb = gen.generate(&path, &meta).expect("generate").expect("some");
        assert_eq!(thumb.width, 128);
        assert_eq!(thumb.height, 128);
    }

    // Test 8: generate_audio_thumbnail produces waveform.
    #[test]
    fn generate_audio_thumbnail_waveform() {
        let dir = test_dir();
        let path = dir.join("sound.wav");
        fs::write(&path, b"RIFF\x00\x00\x00\x00WAVE").unwrap();

        let meta_mgr = MetaManager::new();
        let meta = meta_mgr.generate_meta(&path).unwrap();

        let gen = ThumbnailGenerator::new(&dir.join("thumbs"));
        let thumb = gen.generate(&path, &meta).expect("generate").expect("some");
        assert_eq!(thumb.width, 128);
        assert_eq!(thumb.height, 128);
    }

    // Test 9: unsupported type returns None.
    #[test]
    fn unsupported_type_returns_none() {
        let dir = test_dir();
        let path = dir.join("script.rhai");
        fs::write(&path, b"// script").unwrap();

        let meta_mgr = MetaManager::new();
        let meta = meta_mgr.generate_meta(&path).unwrap();

        let gen = ThumbnailGenerator::new(&dir.join("thumbs"));
        let result = gen.generate(&path, &meta).expect("generate");
        assert!(result.is_none(), "script should not have thumbnail");
    }

    // Test 10: generate_all scans directory.
    #[test]
    fn generate_all_scans_dir() {
        let dir = test_dir();
        let img = image::RgbaImage::from_fn(8, 8, |_, _| image::Rgba([255, 0, 0, 255]));
        img.save(dir.join("a.png")).unwrap();
        img.save(dir.join("b.png")).unwrap();

        let gen = ThumbnailGenerator::new(&dir.join("thumbs"));
        let thumbs = gen.generate_all(&dir).expect("generate_all");
        assert_eq!(thumbs.len(), 2, "should generate 2 thumbnails");
    }
}

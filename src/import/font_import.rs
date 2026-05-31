//! Font importer that rasterizes TTF/OTF fonts into bitmap glyph atlases.
//!
//! Produces [`FontAtlas`] data compatible with the engine's `BitmapFont` system
//! but does not directly depend on it. Uses `ab_glyph` for font parsing and
//! glyph rasterization.

#[cfg(feature = "asset-pipeline")]
use ab_glyph::{Font, FontVec, Glyph, GlyphId, OutlinedGlyph, Point, PxScale, ScaleFont};
#[cfg(feature = "asset-pipeline")]
use std::collections::HashMap;
#[cfg(feature = "asset-pipeline")]
use std::path::Path;

// =====================================================================
// Data Types
// =====================================================================

/// Per-glyph metadata and position within the atlas.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct GlyphEntry {
    pub character: char,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub advance_w: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

/// Rasterized font atlas with glyph positions and RGBA8 pixel data.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub glyphs: Vec<GlyphEntry>,
    pub line_height: f32,
    pub font_size: f32,
    /// Kerning offsets between character pairs, scaled to pixels.
    pub kerning: HashMap<(char, char), f32>,
}

#[cfg(feature = "asset-pipeline")]
impl FontAtlas {
    /// Creates an empty atlas with zero dimensions.
    pub fn empty() -> Self {
        FontAtlas {
            width: 0,
            height: 0,
            pixels: Vec::new(),
            glyphs: Vec::new(),
            line_height: 0.0,
            font_size: 0.0,
            kerning: HashMap::new(),
        }
    }
}

/// Configuration for font import operations.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct FontImportSettings {
    /// Font size in pixels (default: 32.0).
    pub font_size: f32,
    /// Atlas texture width in pixels (default: 512).
    pub atlas_width: u32,
    /// Characters to include, or `None` for ASCII printable 0x20-0x7E.
    pub characters: Option<String>,
    /// Padding between glyphs in pixels (default: 1).
    pub padding: u32,
    /// Enable signed distance field mode (default: false).
    ///
    /// *Note:* SDF mode is reserved for future implementation.
    /// Currently glyphs are rasterized with standard anti-aliased coverage.
    pub sdf: bool,
}

#[cfg(feature = "asset-pipeline")]
impl Default for FontImportSettings {
    fn default() -> Self {
        Self {
            font_size: 32.0,
            atlas_width: 512,
            characters: None,
            padding: 1,
            sdf: false,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl FontImportSettings {
    /// Returns the standard ASCII printable character set (0x20-0x7E).
    pub fn ascii_printable() -> String {
        (0x20u8..=0x7E).map(|b| b as char).collect()
    }

    fn resolve_characters(&self) -> Vec<char> {
        match &self.characters {
            Some(s) => s.chars().collect(),
            None => Self::ascii_printable().chars().collect(),
        }
    }
}

/// Text measurement result without rasterization.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline_y: f32,
}

/// Errors that can occur during font import.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub enum FontImportError {
    /// Filesystem I/O error.
    Io(std::io::Error),
    /// Font file could not be parsed.
    InvalidFont(String),
    /// Glyph rasterization failed.
    RasterizationFailed(String),
    /// Atlas dimensions insufficient for all glyphs.
    AtlasTooSmall,
}

#[cfg(feature = "asset-pipeline")]
impl std::fmt::Display for FontImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::InvalidFont(msg) => write!(f, "Invalid font: {}", msg),
            Self::RasterizationFailed(msg) => write!(f, "Rasterization failed: {}", msg),
            Self::AtlasTooSmall => write!(f, "Atlas dimensions too small for all glyphs"),
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl std::error::Error for FontImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl From<std::io::Error> for FontImportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

// =====================================================================
// Internal Types
// =====================================================================

/// Intermediate glyph metrics used during packing.
#[cfg(feature = "asset-pipeline")]
struct GlyphMetrics {
    character: char,
    width: u32,
    height: u32,
    advance_w: f32,
    bearing_x: f32,
    bearing_y: f32,
}

// =====================================================================
// FontImporter
// =====================================================================

/// Maximum atlas height to prevent unbounded memory allocation.
#[cfg(feature = "asset-pipeline")]
const MAX_ATLAS_HEIGHT: u32 = 8192;

/// Font importer that rasterizes TTF/OTF fonts into bitmap glyph atlases.
///
/// # Example
///
/// ```ignore
/// use chronos_engine::import::font_import::{FontImporter, FontImportSettings};
///
/// let atlas = FontImporter::import(Path::new("assets/fonts/DejaVuSans.ttf"))?;
/// println!("Atlas: {}x{}, {} glyphs", atlas.width, atlas.height, atlas.glyphs.len());
/// ```
#[cfg(feature = "asset-pipeline")]
pub struct FontImporter;

#[cfg(feature = "asset-pipeline")]
impl FontImporter {
    /// Import a font file with default settings.
    pub fn import(path: &Path) -> Result<FontAtlas, FontImportError> {
        Self::import_with_settings(path, &FontImportSettings::default())
    }

    /// Import a font file with custom settings.
    pub fn import_with_settings(
        path: &Path,
        settings: &FontImportSettings,
    ) -> Result<FontAtlas, FontImportError> {
        let bytes = std::fs::read(path)?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|e| FontImportError::InvalidFont(e.to_string()))?;
        Self::rasterize_font(&font, settings)
    }

    /// List characters that have renderable glyphs in the font.
    ///
    /// Scans code points 0x20-0x24F (ASCII + Latin Extended) and returns
    /// characters that have valid glyph outlines at the given size.
    pub fn list_characters(path: &Path, font_size: f32) -> Result<Vec<char>, FontImportError> {
        let bytes = std::fs::read(path)?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|e| FontImportError::InvalidFont(e.to_string()))?;

        let scale = PxScale::from(font_size);
        let scaled = font.as_scaled(scale);

        let mut result = Vec::new();
        for code_point in 0x20u32..=0x24F {
            let ch = match char::from_u32(code_point) {
                Some(c) => c,
                None => continue,
            };
            let gid = font.glyph_id(ch);
            if gid.0 == 0 {
                continue;
            }
            let glyph = Glyph {
                id: gid,
                scale,
                position: Point { x: 0.0, y: 0.0 },
            };
            if scaled.outline_glyph(glyph).is_some() {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Measure text dimensions without rasterizing.
    pub fn measure_text(
        path: &Path,
        text: &str,
        font_size: f32,
    ) -> Result<TextMetrics, FontImportError> {
        let bytes = std::fs::read(path)?;
        let font = FontVec::try_from_vec(bytes)
            .map_err(|e| FontImportError::InvalidFont(e.to_string()))?;

        let scale = PxScale::from(font_size);
        let scaled = font.as_scaled(scale);

        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_height = ascent - descent + scaled.line_gap();
        let mut max_x = 0.0f32;
        let mut cursor_x = 0.0f32;
        let mut lines = 1u32;
        let mut prev_gid: Option<GlyphId> = None;

        for ch in text.chars() {
            if ch == '\n' {
                max_x = max_x.max(cursor_x);
                cursor_x = 0.0;
                lines += 1;
                prev_gid = None;
                continue;
            }

            let gid = font.glyph_id(ch);

            if let Some(prev) = prev_gid {
                cursor_x += scaled.kern(prev, gid);
            }

            cursor_x += scaled.h_advance(gid);
            prev_gid = Some(gid);
        }

        max_x = max_x.max(cursor_x);

        Ok(TextMetrics {
            width: max_x,
            height: lines as f32 * line_height,
            baseline_y: ascent,
        })
    }

    // -----------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------

    fn rasterize_font(
        font: &FontVec,
        settings: &FontImportSettings,
    ) -> Result<FontAtlas, FontImportError> {
        let scale = PxScale::from(settings.font_size);
        let scaled = font.as_scaled(scale);

        let ascent = scaled.ascent();
        let descent = scaled.descent();
        let line_height = ascent - descent + scaled.line_gap();

        let chars = settings.resolve_characters();

        // Phase 1: Collect glyph outlines and metrics
        let mut metrics_list = Vec::with_capacity(chars.len());
        let mut outlines: Vec<Option<OutlinedGlyph>> = Vec::with_capacity(chars.len());

        for &ch in &chars {
            let gid = font.glyph_id(ch);
            let advance_w = scaled.h_advance(gid);
            let bearing_x = scaled.h_side_bearing(gid);

            let glyph = Glyph {
                id: gid,
                scale,
                position: Point { x: 0.0, y: 0.0 },
            };
            let outlined = scaled.outline_glyph(glyph);

            let (w, h, bearing_y) = match &outlined {
                Some(o) => {
                    let bounds = o.px_bounds();
                    let gw = (bounds.max.x - bounds.min.x).ceil().max(0.0) as u32;
                    let gh = (bounds.max.y - bounds.min.y).ceil().max(0.0) as u32;
                    (gw, gh, bounds.min.y)
                }
                None => (0, 0, 0.0),
            };

            metrics_list.push(GlyphMetrics {
                character: ch,
                width: w,
                height: h,
                advance_w,
                bearing_x,
                bearing_y,
            });
            outlines.push(outlined);
        }

        // Phase 2: Pack glyphs left-to-right, top-to-bottom
        let (entries, atlas_height) =
            Self::pack_glyphs(&metrics_list, settings.atlas_width, settings.padding)?;

        // Phase 3: Rasterize into RGBA8 pixel buffer
        let atlas_width = settings.atlas_width;
        let pixel_count = (atlas_width as usize) * (atlas_height as usize) * 4;
        let mut pixels = vec![0u8; pixel_count];

        for (entry, outlined) in entries.iter().zip(outlines.iter()) {
            if let Some(ref o) = outlined {
                let w = atlas_width;
                let h = atlas_height;
                o.draw(|x, y, v| {
                    let px = entry.x + x;
                    let py = entry.y + y;
                    if px < w && py < h {
                        let idx = ((py * w + px) * 4) as usize;
                        if idx + 3 < pixels.len() {
                            pixels[idx] = 255; // R
                            pixels[idx + 1] = 255; // G
                            pixels[idx + 2] = 255; // B
                            pixels[idx + 3] = (v * 255.0).round() as u8; // A
                        }
                    }
                });
            }
        }

        // Phase 4: Compute kerning pairs for the character set
        let mut kerning = HashMap::new();
        for &ch_a in &chars {
            let gid_a = font.glyph_id(ch_a);
            for &ch_b in &chars {
                let gid_b = font.glyph_id(ch_b);
                let k = scaled.kern(gid_a, gid_b);
                if k.abs() > f32::EPSILON {
                    kerning.insert((ch_a, ch_b), k);
                }
            }
        }

        Ok(FontAtlas {
            width: atlas_width,
            height: atlas_height,
            pixels,
            glyphs: entries,
            line_height,
            font_size: settings.font_size,
            kerning,
        })
    }

    /// Simple left-to-right, top-to-bottom bin packing.
    ///
    /// Returns `(glyph entries, atlas_height)`.
    fn pack_glyphs(
        glyphs: &[GlyphMetrics],
        atlas_width: u32,
        padding: u32,
    ) -> Result<(Vec<GlyphEntry>, u32), FontImportError> {
        let mut cursor_x: u32 = 0;
        let mut cursor_y: u32 = 0;
        let mut row_height: u32 = 0;
        let mut entries = Vec::with_capacity(glyphs.len());

        for gm in glyphs {
            let padded_w = gm.width.saturating_add(padding * 2);
            let padded_h = gm.height.saturating_add(padding * 2);

            // Wrap to next row if current glyph doesn't fit
            if cursor_x > 0 && cursor_x.saturating_add(padded_w) > atlas_width {
                cursor_x = 0;
                cursor_y = cursor_y.saturating_add(row_height);
                row_height = 0;
            }

            // Check atlas height limit
            if cursor_y.saturating_add(padded_h) > MAX_ATLAS_HEIGHT {
                return Err(FontImportError::AtlasTooSmall);
            }

            entries.push(GlyphEntry {
                character: gm.character,
                x: cursor_x.saturating_add(padding),
                y: cursor_y.saturating_add(padding),
                width: gm.width,
                height: gm.height,
                advance_w: gm.advance_w,
                bearing_x: gm.bearing_x,
                bearing_y: gm.bearing_y,
            });

            cursor_x = cursor_x.saturating_add(padded_w);
            row_height = row_height.max(padded_h);
        }

        let atlas_height = cursor_y.saturating_add(row_height).max(1);
        Ok((entries, atlas_height))
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(all(test, feature = "asset-pipeline"))]
mod tests {
    use super::*;

    // --- Settings Tests ---

    #[test]
    fn test_settings_defaults() {
        let s = FontImportSettings::default();
        assert!((s.font_size - 32.0).abs() < f32::EPSILON);
        assert_eq!(s.atlas_width, 512);
        assert!(s.characters.is_none());
        assert_eq!(s.padding, 1);
        assert!(!s.sdf);
    }

    #[test]
    fn test_ascii_printable_range() {
        let ascii = FontImportSettings::ascii_printable();
        // 0x7E - 0x20 + 1 = 95 characters
        assert_eq!(ascii.len(), 95);
        assert_eq!(ascii.chars().next(), Some(' '));
        assert_eq!(ascii.chars().last(), Some('~'));
        assert!(ascii.contains('A'));
        assert!(ascii.contains('z'));
        assert!(ascii.contains('0'));
        assert!(!ascii.contains('\n'));
    }

    #[test]
    fn test_settings_custom_characters() {
        let s = FontImportSettings {
            characters: Some("ABC".into()),
            ..Default::default()
        };
        let chars = s.resolve_characters();
        assert_eq!(chars, vec!['A', 'B', 'C']);
    }

    // --- Data Structure Tests ---

    #[test]
    fn test_glyph_entry_construction() {
        let entry = GlyphEntry {
            character: 'A',
            x: 10,
            y: 20,
            width: 15,
            height: 18,
            advance_w: 16.0,
            bearing_x: 1.0,
            bearing_y: -14.0,
        };
        assert_eq!(entry.character, 'A');
        assert_eq!(entry.x, 10);
        assert_eq!(entry.y, 20);
        assert_eq!(entry.width, 15);
        assert_eq!(entry.height, 18);
        assert!((entry.advance_w - 16.0).abs() < f32::EPSILON);
        assert!((entry.bearing_x - 1.0).abs() < f32::EPSILON);
        assert!((entry.bearing_y - (-14.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_atlas_empty() {
        let atlas = FontAtlas::empty();
        assert_eq!(atlas.width, 0);
        assert_eq!(atlas.height, 0);
        assert!(atlas.pixels.is_empty());
        assert!(atlas.glyphs.is_empty());
        assert!(atlas.kerning.is_empty());
        assert!((atlas.line_height).abs() < f32::EPSILON);
        assert!((atlas.font_size).abs() < f32::EPSILON);
    }

    #[test]
    fn test_text_metrics_construction() {
        let tm = TextMetrics {
            width: 120.5,
            height: 32.0,
            baseline_y: 24.0,
        };
        assert!((tm.width - 120.5).abs() < f32::EPSILON);
        assert!((tm.height - 32.0).abs() < f32::EPSILON);
        assert!((tm.baseline_y - 24.0).abs() < f32::EPSILON);
    }

    // --- Error Tests ---

    #[test]
    fn test_error_display_messages() {
        let io_err = FontImportError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let msg = io_err.to_string();
        assert!(
            msg.contains("IO error:"),
            "expected 'IO error:' in '{}'",
            msg
        );
        assert!(
            msg.contains("file not found"),
            "expected 'file not found' in '{}'",
            msg
        );

        let invalid = FontImportError::InvalidFont("bad magic".into());
        let msg = invalid.to_string();
        assert!(
            msg.contains("Invalid font:"),
            "expected 'Invalid font:' in '{}'",
            msg
        );
        assert!(
            msg.contains("bad magic"),
            "expected 'bad magic' in '{}'",
            msg
        );

        let raster = FontImportError::RasterizationFailed("overflow".into());
        let msg = raster.to_string();
        assert!(
            msg.contains("Rasterization failed:"),
            "expected 'Rasterization failed:' in '{}'",
            msg
        );

        let atlas = FontImportError::AtlasTooSmall;
        let msg = atlas.to_string();
        assert!(
            msg.contains("too small"),
            "expected 'too small' in '{}'",
            msg
        );
    }

    #[test]
    fn test_error_source_chain() {
        let io_err = FontImportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(std::error::Error::source(&io_err).is_some());

        let invalid = FontImportError::InvalidFont("x".into());
        assert!(std::error::Error::source(&invalid).is_none());

        let raster = FontImportError::RasterizationFailed("x".into());
        assert!(std::error::Error::source(&raster).is_none());

        let atlas = FontImportError::AtlasTooSmall;
        assert!(std::error::Error::source(&atlas).is_none());
    }

    #[test]
    fn test_error_from_io() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: FontImportError = io.into();
        match err {
            FontImportError::Io(_) => {}
            other => panic!("expected Io variant, got {:?}", other),
        }
    }

    // --- Packing Tests ---

    #[test]
    fn test_pack_three_glyphs_in_row() {
        let glyphs = vec![
            GlyphMetrics {
                character: 'A',
                width: 10,
                height: 10,
                advance_w: 12.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
            GlyphMetrics {
                character: 'B',
                width: 10,
                height: 10,
                advance_w: 12.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
            GlyphMetrics {
                character: 'C',
                width: 10,
                height: 10,
                advance_w: 12.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
        ];

        let (entries, atlas_height) = FontImporter::pack_glyphs(&glyphs, 100, 1).unwrap();
        assert_eq!(entries.len(), 3);

        // All on the same row: y = padding
        assert_eq!(entries[0].y, 1);
        assert_eq!(entries[1].y, 1);
        assert_eq!(entries[2].y, 1);

        // x positions advance by width + 2*padding
        // Glyph 0: x = padding = 1
        // Glyph 1: x = 1 + (10 + 2) = 13
        // Glyph 2: x = 13 + (10 + 2) = 25
        assert_eq!(entries[0].x, 1);
        assert_eq!(entries[1].x, 13);
        assert_eq!(entries[2].x, 25);

        // Atlas height: padding + glyph_height + padding = 12
        assert_eq!(atlas_height, 12);
    }

    #[test]
    fn test_pack_row_wrapping() {
        let glyphs = vec![
            GlyphMetrics {
                character: 'A',
                width: 40,
                height: 10,
                advance_w: 42.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
            GlyphMetrics {
                character: 'B',
                width: 40,
                height: 10,
                advance_w: 42.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
            GlyphMetrics {
                character: 'C',
                width: 40,
                height: 15,
                advance_w: 42.0,
                bearing_x: 0.0,
                bearing_y: 0.0,
            },
        ];

        // Atlas width 100: glyph A padded_w=42, cursor=42. glyph B padded_w=42, cursor=84.
        // glyph C: 84 + 42 = 126 > 100, wraps to next row.
        let (entries, atlas_height) = FontImporter::pack_glyphs(&glyphs, 100, 1).unwrap();
        assert_eq!(entries.len(), 3);

        // First two on row 0
        assert_eq!(entries[0].y, 1);
        assert_eq!(entries[1].y, 1);

        // Third on next row. Row 0 height = 10 + 2 = 12.
        // Row 1 starts at y = 12, glyph placed at y = 12 + 1 = 13.
        assert_eq!(entries[2].y, 13);

        // Atlas height: row 0 (12) + row 1 (15 + 2 = 17) = 29
        assert_eq!(atlas_height, 29);
    }

    #[test]
    fn test_pack_atlas_too_small() {
        let glyphs = vec![GlyphMetrics {
            character: 'X',
            width: 10,
            height: MAX_ATLAS_HEIGHT + 1,
            advance_w: 12.0,
            bearing_x: 0.0,
            bearing_y: 0.0,
        }];

        let result = FontImporter::pack_glyphs(&glyphs, 100, 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            FontImportError::AtlasTooSmall => {}
            other => panic!("expected AtlasTooSmall, got {:?}", other),
        }
    }

    #[test]
    fn test_pack_empty_glyphs() {
        let glyphs: Vec<GlyphMetrics> = vec![];
        let (entries, atlas_height) = FontImporter::pack_glyphs(&glyphs, 256, 1).unwrap();
        assert!(entries.is_empty());
        assert_eq!(atlas_height, 1); // minimum height
    }

    // --- Integration Tests ---

    #[test]
    fn test_import_nonexistent_file() {
        let result = FontImporter::import(Path::new("/nonexistent/path/font.ttf"));
        assert!(result.is_err());
        match result.unwrap_err() {
            FontImportError::Io(_) => {}
            other => panic!("expected Io error, got {:?}", other),
        }
    }

    #[test]
    fn test_import_invalid_bytes() {
        let tmp = std::env::temp_dir().join("chronos_test_invalid_font.ttf");
        std::fs::write(&tmp, b"this is not a font file").ok();
        let result = FontImporter::import(&tmp);
        assert!(result.is_err());
        match result.unwrap_err() {
            FontImportError::InvalidFont(_) => {}
            other => panic!("expected InvalidFont, got {:?}", other),
        }
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_kerning_lookup() {
        let mut kerning = HashMap::new();
        kerning.insert(('A', 'V'), -2.5f32);
        kerning.insert(('T', 'o'), -1.0f32);

        let atlas = FontAtlas {
            width: 128,
            height: 32,
            pixels: vec![0u8; 128 * 32 * 4],
            glyphs: vec![],
            line_height: 32.0,
            font_size: 32.0,
            kerning,
        };

        assert_eq!(atlas.kerning.get(&('A', 'V')), Some(&-2.5f32));
        assert_eq!(atlas.kerning.get(&('T', 'o')), Some(&-1.0f32));
        assert_eq!(atlas.kerning.get(&('X', 'Y')), None);
    }
}

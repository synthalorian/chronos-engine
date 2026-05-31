//! Image importer for loading PNG, JPG, BMP, TGA files into raw pixel data
//! with optional mipmap generation via box-filter downscaling.

#[cfg(feature = "asset-pipeline")]
use std::path::Path;

// ──────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────

#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Bmp,
    Tga,
    Unknown,
}

#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    RGBA8,
    RGB8,
    Grayscale8,
}

#[cfg(feature = "asset-pipeline")]
impl PixelFormat {
    #[allow(dead_code)]
    fn channels(self) -> usize {
        match self {
            PixelFormat::RGBA8 => 4,
            PixelFormat::RGB8 => 3,
            PixelFormat::Grayscale8 => 1,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct MipmapLevel {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct ImportedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub format: ImageFormat,
    pub mipmaps: Vec<MipmapLevel>,
}

#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone)]
pub struct ImageImportSettings {
    pub generate_mipmaps: bool,
    pub max_mipmap_levels: Option<u32>,
    pub premultiply_alpha: bool,
    pub flip_vertical: bool,
    pub output_format: PixelFormat,
}

#[cfg(feature = "asset-pipeline")]
impl Default for ImageImportSettings {
    fn default() -> Self {
        ImageImportSettings {
            generate_mipmaps: true,
            max_mipmap_levels: None,
            premultiply_alpha: false,
            flip_vertical: false,
            output_format: PixelFormat::RGBA8,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Error
// ──────────────────────────────────────────────────────────────

#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub enum ImageImportError {
    Io(std::io::Error),
    DecodeFailed(String),
    UnsupportedFormat(ImageFormat),
    InvalidDimensions { width: u32, height: u32 },
}

#[cfg(feature = "asset-pipeline")]
impl std::fmt::Display for ImageImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageImportError::Io(e) => write!(f, "IO error: {}", e),
            ImageImportError::DecodeFailed(msg) => write!(f, "Decode failed: {}", msg),
            ImageImportError::UnsupportedFormat(fmt) => {
                write!(f, "Unsupported image format: {:?}", fmt)
            }
            ImageImportError::InvalidDimensions { width, height } => {
                write!(f, "Invalid dimensions: {}x{}", width, height)
            }
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl std::error::Error for ImageImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImageImportError::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl From<std::io::Error> for ImageImportError {
    fn from(e: std::io::Error) -> Self {
        ImageImportError::Io(e)
    }
}

// ──────────────────────────────────────────────────────────────
// Importer
// ──────────────────────────────────────────────────────────────

#[cfg(feature = "asset-pipeline")]
pub struct ImageImporter;

#[cfg(feature = "asset-pipeline")]
impl ImageImporter {
    /// Import an image using default settings.
    pub fn import(path: &Path) -> Result<ImportedImage, ImageImportError> {
        let settings = ImageImportSettings::default();
        Self::import_with_settings(path, &settings)
    }

    /// Import an image with custom settings.
    pub fn import_with_settings(
        path: &Path,
        settings: &ImageImportSettings,
    ) -> Result<ImportedImage, ImageImportError> {
        let format = Self::detect_format(path);

        let img = image::open(path).map_err(|e| {
            ImageImportError::DecodeFailed(format!("Failed to open {}: {}", path.display(), e))
        })?;

        if img.width() == 0 || img.height() == 0 {
            return Err(ImageImportError::InvalidDimensions {
                width: img.width(),
                height: img.height(),
            });
        }

        let mut rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        if settings.flip_vertical {
            image::imageops::flip_vertical_in_place(&mut rgba);
        }

        let mut pixels = rgba.into_raw();

        if settings.premultiply_alpha {
            premultiply_alpha(&mut pixels);
        }

        let output_pixels = match settings.output_format {
            PixelFormat::RGBA8 => pixels.clone(),
            PixelFormat::RGB8 => rgba8_to_rgb8(&pixels),
            PixelFormat::Grayscale8 => rgba8_to_gray8(&pixels),
        };

        let mipmaps = if settings.generate_mipmaps {
            // Use original RGBA pixels for mipmap generation to keep box-filter consistent
            let mip_pixels = if settings.output_format == PixelFormat::RGBA8 {
                output_pixels.clone()
            } else {
                // Reconvert for mipmap generation at each level
                pixels.clone()
            };
            Self::generate_mipmaps(&mip_pixels, width, height, settings.max_mipmap_levels)
        } else {
            Vec::new()
        };

        Ok(ImportedImage {
            width,
            height,
            pixels: output_pixels,
            format,
            mipmaps,
        })
    }

    /// Detect image format from file extension.
    pub fn detect_format(path: &Path) -> ImageFormat {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("png") => ImageFormat::Png,
            Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
            Some("bmp") => ImageFormat::Bmp,
            Some("tga") => ImageFormat::Tga,
            _ => ImageFormat::Unknown,
        }
    }

    /// Generate a mipmap chain using box-filter downscaling.
    ///
    /// Level 0 is the original image. Each subsequent level halves width and
    /// height (rounding up). Each pixel is the average of its 4 parent pixels
    /// (or fewer at edges). Continues until 1x1 or `max_levels` is reached.
    pub fn generate_mipmaps(
        pixels: &[u8],
        width: u32,
        height: u32,
        max_levels: Option<u32>,
    ) -> Vec<MipmapLevel> {
        const ABSOLUTE_MAX_LEVELS: u32 = 14;

        let limit = max_levels
            .map(|m| m.min(ABSOLUTE_MAX_LEVELS))
            .unwrap_or(ABSOLUTE_MAX_LEVELS);

        let mut levels = Vec::new();

        let mut src_data = pixels.to_vec();
        let mut src_w = width;
        let mut src_h = height;

        for _ in 0..limit {
            let dst_w = src_w.div_ceil(2);
            let dst_h = src_h.div_ceil(2);

            if dst_w == 0 && dst_h == 0 {
                break;
            }

            let dst_data = box_filter_downsample(&src_data, src_w, src_h, dst_w, dst_h);

            levels.push(MipmapLevel {
                width: dst_w,
                height: dst_h,
                data: dst_data.clone(),
            });

            if dst_w <= 1 && dst_h <= 1 {
                break;
            }

            src_data = dst_data;
            src_w = dst_w;
            src_h = dst_h;
        }

        levels
    }
}

// ──────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────

/// Box-filter downsample: each output pixel averages the 2x2 block (or fewer
/// at edges) of the source image. Expects RGBA8 input.
#[cfg(feature = "asset-pipeline")]
#[allow(clippy::manual_checked_ops)]
fn box_filter_downsample(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = Vec::with_capacity((dst_w * dst_h * 4) as usize);

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = dx * 2;
            let sy = dy * 2;

            let mut r_sum: u32 = 0;
            let mut g_sum: u32 = 0;
            let mut b_sum: u32 = 0;
            let mut a_sum: u32 = 0;
            let mut count: u32 = 0;

            for oy in 0..2u32 {
                for ox in 0..2u32 {
                    let px = sx + ox;
                    let py = sy + oy;
                    if px < src_w && py < src_h {
                        let idx = ((py * src_w + px) * 4) as usize;
                        r_sum += src[idx] as u32;
                        g_sum += src[idx + 1] as u32;
                        b_sum += src[idx + 2] as u32;
                        a_sum += src[idx + 3] as u32;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                dst.push((r_sum / count) as u8);
                dst.push((g_sum / count) as u8);
                dst.push((b_sum / count) as u8);
                dst.push((a_sum / count) as u8);
            } else {
                dst.push(0);
                dst.push(0);
                dst.push(0);
                dst.push(0);
            }
        }
    }

    dst
}

/// Premultiply RGB channels by alpha (in-place).
#[cfg(feature = "asset-pipeline")]
fn premultiply_alpha(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let a = chunk[3] as u32;
        chunk[0] = ((chunk[0] as u32 * a + 127) / 255) as u8;
        chunk[1] = ((chunk[1] as u32 * a + 127) / 255) as u8;
        chunk[2] = ((chunk[2] as u32 * a + 127) / 255) as u8;
    }
}

/// Convert RGBA8 pixel buffer to RGB8.
#[cfg(feature = "asset-pipeline")]
fn rgba8_to_rgb8(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|c| &c[..3])
        .copied()
        .collect()
}

/// Convert RGBA8 pixel buffer to Grayscale8 using luminance weights.
#[cfg(feature = "asset-pipeline")]
fn rgba8_to_gray8(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .map(|c| {
            let r = c[0] as u32;
            let g = c[1] as u32;
            let b = c[2] as u32;
            ((r * 77 + g * 150 + b * 29 + 128) / 256) as u8
        })
        .collect()
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(feature = "asset-pipeline")]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const TEST_DIR: &str = "/tmp/chronos_import_tests";

    fn setup_test_dir() {
        let _ = fs::create_dir_all(TEST_DIR);
    }

    /// Create a small RGBA test image buffer with a simple pattern.
    fn make_test_image(width: u32, height: u32) -> image::RgbaImage {
        image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                (x * 60 + 10) as u8,
                (y * 60 + 20) as u8,
                ((x + y) * 30 + 40) as u8,
                255,
            ])
        })
    }

    /// Create a small RGBA test image with varying alpha.
    fn make_test_image_alpha(width: u32, height: u32) -> image::RgbaImage {
        image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                (x * 60 + 10) as u8,
                (y * 60 + 20) as u8,
                ((x + y) * 30 + 40) as u8,
                ((x * 64) + 1).min(255) as u8,
            ])
        })
    }

    // Test 1: Import a valid PNG
    #[test]
    fn test_import_valid_png() {
        setup_test_dir();
        let path = format!("{}/test_4x4.png", TEST_DIR);
        let img = make_test_image(4, 4);
        img.save(&path).expect("save test png");

        let result = ImageImporter::import(Path::new(&path));
        assert!(result.is_ok(), "PNG import should succeed");
        let imported = result.unwrap();
        assert_eq!(imported.width, 4);
        assert_eq!(imported.height, 4);
        assert_eq!(imported.format, ImageFormat::Png);
        assert_eq!(imported.pixels.len(), 4 * 4 * 4); // RGBA8
    }

    // Test 2: Import a valid JPG
    #[test]
    fn test_import_valid_jpg() {
        setup_test_dir();
        let path = format!("{}/test_4x4.jpg", TEST_DIR);
        // JPEG does not support alpha; create an RGB image.
        let img = image::RgbImage::from_fn(4, 4, |x, y| {
            image::Rgb([
                (x * 60 + 10) as u8,
                (y * 60 + 20) as u8,
                ((x + y) * 30 + 40) as u8,
            ])
        });
        img.save(&path).expect("save test jpg");

        let result = ImageImporter::import(Path::new(&path));
        assert!(result.is_ok(), "JPG import should succeed");
        let imported = result.unwrap();
        assert_eq!(imported.width, 4);
        assert_eq!(imported.height, 4);
        assert_eq!(imported.format, ImageFormat::Jpeg);
        // JPG is lossy so pixel values may differ, but size should be correct
        assert_eq!(imported.pixels.len(), 4 * 4 * 4);
    }

    // Test 3: Import with custom settings (mipmaps disabled)
    #[test]
    fn test_import_mipmaps_disabled() {
        setup_test_dir();
        let path = format!("{}/test_mip_off.png", TEST_DIR);
        let img = make_test_image(8, 8);
        img.save(&path).expect("save test png");

        let settings = ImageImportSettings {
            generate_mipmaps: false,
            ..ImageImportSettings::default()
        };

        let result = ImageImporter::import_with_settings(Path::new(&path), &settings);
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert!(
            imported.mipmaps.is_empty(),
            "Mipmaps should be empty when disabled"
        );
    }

    // Test 4: Detect format from .png extension
    #[test]
    fn test_detect_format_png() {
        assert_eq!(
            ImageImporter::detect_format(Path::new("sprite.png")),
            ImageFormat::Png
        );
    }

    // Test 5: Detect format from .jpg extension
    #[test]
    fn test_detect_format_jpg() {
        assert_eq!(
            ImageImporter::detect_format(Path::new("photo.jpg")),
            ImageFormat::Jpeg
        );
        assert_eq!(
            ImageImporter::detect_format(Path::new("photo.jpeg")),
            ImageFormat::Jpeg
        );
    }

    // Test 6: Detect unknown extension
    #[test]
    fn test_detect_format_unknown() {
        assert_eq!(
            ImageImporter::detect_format(Path::new("file.webp")),
            ImageFormat::Unknown
        );
        assert_eq!(
            ImageImporter::detect_format(Path::new("file")),
            ImageFormat::Unknown
        );
        assert_eq!(
            ImageImporter::detect_format(Path::new("file.xyz")),
            ImageFormat::Unknown
        );
    }

    // Test 7: Generate mipmaps for 4x4 image → 3 levels (4→2→1)
    #[test]
    fn test_mipmaps_4x4() {
        let img = make_test_image(4, 4);
        let pixels = img.into_raw();

        let mipmaps = ImageImporter::generate_mipmaps(&pixels, 4, 4, None);

        assert_eq!(
            mipmaps.len(),
            2,
            "4x4 should produce 2 mipmap levels (2x2, 1x1)"
        );
        assert_eq!(mipmaps[0].width, 2);
        assert_eq!(mipmaps[0].height, 2);
        assert_eq!(mipmaps[0].data.len(), 2 * 2 * 4);
        assert_eq!(mipmaps[1].width, 1);
        assert_eq!(mipmaps[1].height, 1);
        assert_eq!(mipmaps[1].data.len(), 1 * 1 * 4);
    }

    // Test 8: Generate mipmaps for 3x3 (odd dimensions) → 2 levels (3→2→1)
    #[test]
    fn test_mipmaps_3x3_odd() {
        let img = make_test_image(3, 3);
        let pixels = img.into_raw();

        let mipmaps = ImageImporter::generate_mipmaps(&pixels, 3, 3, None);

        assert_eq!(
            mipmaps.len(),
            2,
            "3x3 should produce 2 mipmap levels (2x2, 1x1)"
        );
        assert_eq!(mipmaps[0].width, 2);
        assert_eq!(mipmaps[0].height, 2);
        assert_eq!(mipmaps[1].width, 1);
        assert_eq!(mipmaps[1].height, 1);
    }

    // Test 9: Max levels cap
    #[test]
    fn test_mipmaps_max_levels_one() {
        let img = make_test_image(8, 8);
        let pixels = img.into_raw();

        let mipmaps = ImageImporter::generate_mipmaps(&pixels, 8, 8, Some(1));

        assert_eq!(
            mipmaps.len(),
            1,
            "max_levels=1 should produce only 1 mipmap"
        );
        assert_eq!(mipmaps[0].width, 4);
        assert_eq!(mipmaps[0].height, 4);
    }

    // Test 10: Error on nonexistent file
    #[test]
    fn test_import_nonexistent_file() {
        let result = ImageImporter::import(Path::new("/tmp/chronos_import_tests/nonexistent.png"));
        assert!(result.is_err(), "Should fail on nonexistent file");
        match result.unwrap_err() {
            ImageImportError::DecodeFailed(_) => {} // expected
            other => panic!("Expected DecodeFailed, got: {:?}", other),
        }
    }

    // Test 11: Premultiply alpha
    #[test]
    fn test_premultiply_alpha() {
        setup_test_dir();
        let path = format!("{}/test_alpha.png", TEST_DIR);
        let img = make_test_image_alpha(4, 4);
        img.save(&path).expect("save alpha png");

        let settings = ImageImportSettings {
            premultiply_alpha: true,
            generate_mipmaps: false,
            ..ImageImportSettings::default()
        };

        let result = ImageImporter::import_with_settings(Path::new(&path), &settings);
        assert!(result.is_ok());
        let imported = result.unwrap();

        // Verify pixels are premultiplied: for each pixel, R <= A, G <= A, B <= A
        for chunk in imported.pixels.chunks_exact(4) {
            let r = chunk[0] as u32;
            let g = chunk[1] as u32;
            let b = chunk[2] as u32;
            let a = chunk[3] as u32;
            assert!(
                r <= a || a == 0,
                "Red channel ({}) should be <= alpha ({}) after premultiply",
                r,
                a
            );
            assert!(
                g <= a || a == 0,
                "Green channel ({}) should be <= alpha ({}) after premultiply",
                g,
                a
            );
            assert!(
                b <= a || a == 0,
                "Blue channel ({}) should be <= alpha ({}) after premultiply",
                b,
                a
            );
        }
    }

    // Test 12: Flip vertical
    #[test]
    fn test_flip_vertical() {
        setup_test_dir();
        let path = format!("{}/test_flip.png", TEST_DIR);

        // Create a 2x2 image with known pixel values per row
        let mut img = image::RgbaImage::new(2, 2);
        // Top row: red
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([255, 0, 0, 255]));
        // Bottom row: blue
        img.put_pixel(0, 1, image::Rgba([0, 0, 255, 255]));
        img.put_pixel(1, 1, image::Rgba([0, 0, 255, 255]));
        img.save(&path).expect("save flip png");

        let settings = ImageImportSettings {
            flip_vertical: true,
            generate_mipmaps: false,
            ..ImageImportSettings::default()
        };

        let result = ImageImporter::import_with_settings(Path::new(&path), &settings);
        assert!(result.is_ok());
        let imported = result.unwrap();

        // After flip, first pixel (top-left) should be blue (was bottom row)
        assert_eq!(imported.pixels[0], 0, "R of first pixel after flip");
        assert_eq!(imported.pixels[2], 255, "B of first pixel after flip");
    }

    // Bonus: verify mipmap pixel values are correct averages
    #[test]
    fn test_mipmap_box_filter_values() {
        // 2x2 image: all four pixels have distinct RGBA values
        let pixels: Vec<u8> = vec![
            // (0,0) - red
            255, 0, 0, 255, // (1,0) - green
            0, 255, 0, 255, // (0,1) - blue
            0, 0, 255, 255, // (1,1) - white
            255, 255, 255, 255,
        ];

        let mipmaps = ImageImporter::generate_mipmaps(&pixels, 2, 2, None);
        assert_eq!(mipmaps.len(), 1);
        assert_eq!(mipmaps[0].width, 1);
        assert_eq!(mipmaps[0].height, 1);

        // The single 1x1 pixel should be the average of all four:
        // R = (255+0+0+255)/4 = 127
        // G = (0+255+0+255)/4 = 127
        // B = (0+0+255+255)/4 = 127
        // A = (255+255+255+255)/4 = 255
        let m = &mipmaps[0].data;
        assert_eq!(m[0], 127, "Averaged R");
        assert_eq!(m[1], 127, "Averaged G");
        assert_eq!(m[2], 127, "Averaged B");
        assert_eq!(m[3], 255, "Averaged A");
    }

    // Bonus: BMP and TGA format detection
    #[test]
    fn test_detect_format_bmp_tga() {
        assert_eq!(
            ImageImporter::detect_format(Path::new("img.bmp")),
            ImageFormat::Bmp
        );
        assert_eq!(
            ImageImporter::detect_format(Path::new("img.tga")),
            ImageFormat::Tga
        );
    }

    // Bonus: output format RGB8
    #[test]
    fn test_output_format_rgb8() {
        setup_test_dir();
        let path = format!("{}/test_rgb8.png", TEST_DIR);
        let img = make_test_image(4, 4);
        img.save(&path).expect("save rgb8 png");

        let settings = ImageImportSettings {
            output_format: PixelFormat::RGB8,
            generate_mipmaps: false,
            ..ImageImportSettings::default()
        };

        let result = ImageImporter::import_with_settings(Path::new(&path), &settings);
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(
            imported.pixels.len(),
            4 * 4 * 3,
            "RGB8 should have 3 bytes per pixel"
        );
    }

    // Bonus: output format Grayscale8
    #[test]
    fn test_output_format_grayscale8() {
        setup_test_dir();
        let path = format!("{}/test_gray.png", TEST_DIR);
        let img = make_test_image(4, 4);
        img.save(&path).expect("save gray png");

        let settings = ImageImportSettings {
            output_format: PixelFormat::Grayscale8,
            generate_mipmaps: false,
            ..ImageImportSettings::default()
        };

        let result = ImageImporter::import_with_settings(Path::new(&path), &settings);
        assert!(result.is_ok());
        let imported = result.unwrap();
        assert_eq!(
            imported.pixels.len(),
            4 * 4,
            "Grayscale8 should have 1 byte per pixel"
        );
    }

    // Bonus: InvalidDimensions error for zero-size (simulated)
    #[test]
    fn test_error_display() {
        let err = ImageImportError::InvalidDimensions {
            width: 0,
            height: 5,
        };
        let msg = format!("{}", err);
        assert!(
            msg.contains("0x5"),
            "Error message should contain dimensions"
        );

        let io_err = ImageImportError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file gone",
        ));
        assert!(format!("{}", io_err).contains("file gone"));
    }
}

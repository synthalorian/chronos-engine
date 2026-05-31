//! Bitmap font rendering via glyph atlas.
//!
//! Converts text strings into `RenderSprite` batches using a
//! texture atlas where each glyph occupies a known sub-region.

use crate::render::RenderSprite;
use crate::texture::TextureAtlas;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance_w: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

#[derive(Debug, Clone)]
struct GlyphInfo {
    frame_name: String,
    metrics: GlyphMetrics,
}

pub struct BitmapFont {
    glyph_size: u32,
    glyphs: HashMap<char, GlyphInfo>,
    line_height: f32,
    kerning: HashMap<(char, char), f32>,
}

impl BitmapFont {
    pub fn new(glyph_size: u32, line_height: f32) -> Self {
        BitmapFont {
            glyph_size,
            glyphs: HashMap::new(),
            line_height,
            kerning: HashMap::new(),
        }
    }

    pub fn register_ascii_grid(&mut self, first_char: char, columns: u32) {
        for (i, ch) in (first_char..='\x7e').enumerate() {
            let _frame_name = format!("glyph_{}", ch as u32);
            let col = i as u32 % columns;
            let row = i as u32 / columns;
            self.glyphs.insert(
                ch,
                GlyphInfo {
                    frame_name: format!("tile_{}", row * columns + col),
                    metrics: GlyphMetrics {
                        advance_w: self.glyph_size as f32,
                        bearing_x: 0.0,
                        bearing_y: 0.0,
                    },
                },
            );
        }
    }

    pub fn register_glyph(
        &mut self,
        ch: char,
        frame_name: &str,
        advance_w: f32,
        bearing_x: f32,
        bearing_y: f32,
    ) {
        self.glyphs.insert(
            ch,
            GlyphInfo {
                frame_name: frame_name.to_string(),
                metrics: GlyphMetrics {
                    advance_w,
                    bearing_x,
                    bearing_y,
                },
            },
        );
    }

    pub fn set_kerning(&mut self, left: char, right: char, offset: f32) {
        self.kerning.insert((left, right), offset);
    }

    pub fn register_atlas_frames(&self, atlas: &mut TextureAtlas, columns: u32, rows: u32) {
        atlas.define_grid(self.glyph_size, self.glyph_size, columns, rows);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_text(
        &self,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: [f32; 4],
        layer: i32,
        atlas: &TextureAtlas,
    ) -> Vec<RenderSprite> {
        let mut sprites = Vec::new();
        let mut cursor_x = x;
        let mut cursor_y = y;
        let mut prev_char: Option<char> = None;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_y += self.line_height * scale;
                cursor_x = x;
                prev_char = None;
                continue;
            }

            if let Some(glyph) = self.glyphs.get(&ch) {
                if let Some(frame) = atlas.get_frame(&glyph.frame_name) {
                    if let Some(pc) = prev_char {
                        if let Some(k) = self.kerning.get(&(pc, ch)) {
                            cursor_x += k * scale;
                        }
                    }

                    let gx = cursor_x + glyph.metrics.bearing_x * scale;
                    let gy = cursor_y + glyph.metrics.bearing_y * scale;
                    let gw = frame.pixel_w as f32 * scale;
                    let gh = frame.pixel_h as f32 * scale;

                    sprites.push(
                        RenderSprite::new(gx + gw * 0.5, gy + gh * 0.5, gw, gh)
                            .with_uv(frame.u, frame.v, frame.u + frame.du, frame.v + frame.dv)
                            .with_layer(layer)
                            .with_color(color[0], color[1], color[2], color[3]),
                    );

                    cursor_x += glyph.metrics.advance_w * scale;
                } else {
                    cursor_x += self.glyph_size as f32 * scale;
                }
            }
            prev_char = Some(ch);
        }

        sprites
    }

    pub fn measure_text(&self, text: &str, scale: f32) -> (f32, f32) {
        let mut max_x = 0.0f32;
        let mut cursor_x = 0.0f32;
        let mut lines = 1usize;
        let mut prev_char: Option<char> = None;

        for ch in text.chars() {
            if ch == '\n' {
                max_x = max_x.max(cursor_x);
                cursor_x = 0.0;
                lines += 1;
                prev_char = None;
                continue;
            }

            if let Some(glyph) = self.glyphs.get(&ch) {
                if let Some(pc) = prev_char {
                    if let Some(k) = self.kerning.get(&(pc, ch)) {
                        cursor_x += k * scale;
                    }
                }
                cursor_x += glyph.metrics.advance_w * scale;
            }
            prev_char = Some(ch);
        }

        max_x = max_x.max(cursor_x);
        (max_x, lines as f32 * self.line_height * scale)
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }
}

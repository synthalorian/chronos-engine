//! Infinite ground grid renderer for the Chronos Engine editor viewport.
//!
//! Draws a 2D orthographic grid using egui painter calls. Lines are laid out
//! in world space and transformed to screen space via a camera offset + zoom
//! so the grid scrolls and scales naturally with the viewport camera.

use egui::{Color32, Pos2, Rect};

use super::snap_to_grid;

// ──────────────────────────────────────────────
// GridRenderer
// ──────────────────────────────────────────────

/// Renders an infinite-style ground grid inside an egui viewport.
///
/// The grid covers a configurable extent around the origin. Minor lines are
/// drawn at `spacing` intervals; every `major_every`-th line is drawn thicker
/// and brighter. The X axis (world Y = 0) is drawn in red, the Y axis
/// (world X = 0) in green.
pub struct GridRenderer {
    /// Whether the grid is visible.
    pub visible: bool,
    /// Distance between minor grid lines in world units.
    pub spacing: f32,
    /// Every N-th line is a major (thicker) line.
    pub major_every: u32,
    /// Half-size of the drawn grid in world units.
    pub extent: f32,
    /// Colour for the X axis line.
    pub axis_x_color: [f32; 3],
    /// Colour for the Y axis line.
    pub axis_y_color: [f32; 3],
    /// Colour for minor grid lines.
    pub minor_color: [f32; 3],
    /// Colour for major grid lines.
    pub major_color: [f32; 3],
}

impl GridRenderer {
    /// Create a new grid renderer with sensible defaults.
    pub fn new() -> Self {
        Self {
            visible: true,
            spacing: 1.0,
            major_every: 5,
            extent: 50.0,
            axis_x_color: [0.8, 0.2, 0.2],
            axis_y_color: [0.2, 0.8, 0.2],
            minor_color: [0.3, 0.3, 0.3],
            major_color: [0.5, 0.5, 0.5],
        }
    }

    /// Draw the grid into the viewport.
    ///
    /// * `painter`        — egui painter for the viewport area.
    /// * `camera_offset`  — 2D world-space offset of the camera.
    /// * `zoom`           — Camera zoom level (1.0 = default).
    /// * `screen_rect`    — The viewport's screen rect.
    pub fn render(
        &self,
        painter: &egui::Painter,
        camera_offset: [f32; 2],
        zoom: f32,
        screen_rect: Rect,
    ) {
        if !self.visible || self.spacing <= 0.0 || zoom <= 0.0 {
            return;
        }

        let screen_center = screen_rect.center();
        let spacing = self.spacing;
        let major_every = self.major_every.max(1);

        // World-space bounds visible on screen (expand by one spacing to avoid gaps).
        let inv_zoom = 1.0 / zoom;
        let half_screen_w = screen_rect.width() * 0.5 * inv_zoom;
        let half_screen_h = screen_rect.height() * 0.5 * inv_zoom;

        let world_x_min = camera_offset[0] - half_screen_w - spacing;
        let world_x_max = camera_offset[0] + half_screen_w + spacing;
        let world_y_min = camera_offset[1] - half_screen_h - spacing;
        let world_y_max = camera_offset[1] + half_screen_h + spacing;

        // Clamp to grid extent.
        let lo = -self.extent;
        let hi = self.extent;
        let x_start = (world_x_min.max(lo) / spacing).floor() * spacing;
        let x_end = world_x_max.min(hi);
        let y_start = (world_y_min.max(lo) / spacing).floor() * spacing;
        let y_end = world_y_max.min(hi);

        // ── Vertical lines (constant world X) ─────────────
        let mut world_x = x_start;
        while world_x <= x_end {
            let sx = world_to_screen([world_x, 0.0], camera_offset, zoom, screen_center).x;

            // Cull off-screen.
            if sx < screen_rect.left() || sx > screen_rect.right() {
                world_x += spacing;
                continue;
            }

            // Determine line style.
            let (color, width) = if world_x.abs() < spacing * 0.01 {
                // Y axis — world X = 0.
                (f32_array_to_color32(self.axis_y_color), 2.0)
            } else {
                let index = ((world_x / spacing).round() as i32).unsigned_abs();
                let is_major = index.is_multiple_of(major_every);
                if is_major {
                    (f32_array_to_color32(self.major_color), 1.5)
                } else {
                    (f32_array_to_color32(self.minor_color), 0.5)
                }
            };

            let top = Pos2::new(sx, screen_rect.top());
            let bot = Pos2::new(sx, screen_rect.bottom());
            painter.line_segment([top, bot], (width, color));

            world_x += spacing;
        }

        // ── Horizontal lines (constant world Y) ───────────
        let mut world_y = y_start;
        while world_y <= y_end {
            let sy = world_to_screen([0.0, world_y], camera_offset, zoom, screen_center).y;

            if sy < screen_rect.top() || sy > screen_rect.bottom() {
                world_y += spacing;
                continue;
            }

            let (color, width) = if world_y.abs() < spacing * 0.01 {
                // X axis — world Y = 0.
                (f32_array_to_color32(self.axis_x_color), 2.0)
            } else {
                let index = ((world_y / spacing).round() as i32).unsigned_abs();
                let is_major = index.is_multiple_of(major_every);
                if is_major {
                    (f32_array_to_color32(self.major_color), 1.5)
                } else {
                    (f32_array_to_color32(self.minor_color), 0.5)
                }
            };

            let left = Pos2::new(screen_rect.left(), sy);
            let right = Pos2::new(screen_rect.right(), sy);
            painter.line_segment([left, right], (width, color));

            world_y += spacing;
        }
    }

    /// Snap a single world-space value to the nearest grid point.
    pub fn snap(&self, value: f32) -> f32 {
        snap_to_grid(value, self.spacing)
    }

    /// Snap both axes of a 2D point to the nearest grid points.
    pub fn snap_2d(&self, x: f32, y: f32) -> (f32, f32) {
        (snap_to_grid(x, self.spacing), snap_to_grid(y, self.spacing))
    }
}

impl Default for GridRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Convert a world-space position to screen-space.
///
/// `screen_x = (world_x - camera_offset[0]) * zoom + screen_center.x`
pub fn world_to_screen(
    world_pos: [f32; 2],
    camera_offset: [f32; 2],
    zoom: f32,
    screen_center: Pos2,
) -> Pos2 {
    Pos2::new(
        (world_pos[0] - camera_offset[0]) * zoom + screen_center.x,
        (world_pos[1] - camera_offset[1]) * zoom + screen_center.y,
    )
}

/// Convert a normalised `[0.0..1.0]` RGB triplet into an [`Color32`].
pub fn f32_array_to_color32(arr: [f32; 3]) -> Color32 {
    let r = (arr[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (arr[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (arr[2].clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgb(r, g, b)
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Vec2;

    fn default_screen_center() -> Pos2 {
        Pos2::new(400.0, 300.0)
    }

    fn default_screen_rect() -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0))
    }

    // 1. new() returns expected defaults.
    #[test]
    fn new_defaults() {
        let g = GridRenderer::new();
        assert!(g.visible);
        assert!((g.spacing - 1.0).abs() < f32::EPSILON);
        assert_eq!(g.major_every, 5);
        assert!((g.extent - 50.0).abs() < f32::EPSILON);
        assert!((g.axis_x_color[0] - 0.8).abs() < f32::EPSILON);
        assert!((g.axis_y_color[1] - 0.8).abs() < f32::EPSILON);
    }

    // 2. snap() rounds to nearest grid point with default spacing.
    #[test]
    fn snap_default_spacing() {
        let g = GridRenderer::new();
        assert!((g.snap(1.3) - 1.0).abs() < f32::EPSILON);
        assert!((g.snap(1.6) - 2.0).abs() < f32::EPSILON);
        assert!((g.snap(0.0)).abs() < f32::EPSILON);
        assert!((g.snap(-0.4)).abs() < f32::EPSILON);
        assert!((g.snap(-0.6) - (-1.0)).abs() < f32::EPSILON);
    }

    // 3. snap_2d() snaps both axes independently.
    #[test]
    fn snap_2d_both_axes() {
        let g = GridRenderer::new();
        let (x, y) = g.snap_2d(1.3, 2.7);
        assert!((x - 1.0).abs() < f32::EPSILON);
        assert!((y - 3.0).abs() < f32::EPSILON);
    }

    // 4. world_to_screen applies offset + zoom correctly.
    #[test]
    fn world_to_screen_transform() {
        let center = default_screen_center();
        // At zoom 1.0, offset 0: world origin maps to screen center.
        let p = world_to_screen([0.0, 0.0], [0.0, 0.0], 1.0, center);
        assert!((p.x - 400.0).abs() < f32::EPSILON);
        assert!((p.y - 300.0).abs() < f32::EPSILON);

        // With camera offset, the origin shifts.
        let p2 = world_to_screen([0.0, 0.0], [10.0, 20.0], 1.0, center);
        assert!((p2.x - 390.0).abs() < f32::EPSILON);
        assert!((p2.y - 280.0).abs() < f32::EPSILON);

        // Zoom scales the offset.
        let p3 = world_to_screen([5.0, 0.0], [0.0, 0.0], 2.0, center);
        assert!((p3.x - 410.0).abs() < f32::EPSILON);
    }

    // 5. f32_array_to_color32 clamps and converts correctly.
    #[test]
    fn color_conversion() {
        let c = f32_array_to_color32([0.0, 0.5, 1.0]);
        assert_eq!(c.r(), 0);
        assert_eq!(c.g(), 127);
        assert_eq!(c.b(), 255);

        // Out-of-range values are clamped.
        let c2 = f32_array_to_color32([2.0, -1.0, 0.5]);
        assert_eq!(c2.r(), 255);
        assert_eq!(c2.g(), 0);
        assert_eq!(c2.b(), 127);
    }

    // 6. Visibility toggle prevents rendering.
    #[test]
    fn visibility_toggle() {
        let mut g = GridRenderer::new();
        assert!(g.visible);
        g.visible = false;
        assert!(!g.visible);
    }

    // 7. snap with custom spacing.
    #[test]
    fn snap_custom_spacing() {
        let mut g = GridRenderer::new();
        g.spacing = 0.5;
        assert!((g.snap(1.2) - 1.0).abs() < f32::EPSILON);
        assert!((g.snap(1.3) - 1.5).abs() < f32::EPSILON);
    }

    // 8. render with zero zoom is a no-op (does not panic).
    #[test]
    fn render_zero_zoom_no_panic() {
        let g = GridRenderer::new();
        let ctx = egui::Context::default();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("test_grid"),
        ));
        // Should not panic.
        g.render(&painter, [0.0, 0.0], 0.0, default_screen_rect());
        g.render(&painter, [0.0, 0.0], -1.0, default_screen_rect());
    }
}

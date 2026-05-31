#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Gizmo system for the editor viewport — Phase 7C.
//!
//! Renders and handles translate / rotate / scale gizmos in the 2-D editor
//! viewport. Each gizmo draws three coloured axes (X = red, Y = green,
//! Z = blue) and responds to mouse drag input to produce deltas that can be
//! applied to entity transforms.
//!
//! **Coordinate convention** (screen-space):
//! - X axis → rightward
//! - Y axis → upward (inverted from egui's downward-positive Y)
//! - Z axis → diagonally (upper-right for the 3-D illusion)

use crate::editor_panels::GizmoMode;

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

/// Default gizmo size in screen pixels.
const DEFAULT_GIZMO_SIZE: f32 = 80.0;

/// Default axis length in world units (mapped to screen pixels).
const DEFAULT_AXIS_LENGTH: f32 = 1.5;

/// Maximum distance (in screen pixels) for a mouse to be considered
/// "hovering" an axis during hit-testing.
const HIT_THRESHOLD: f32 = 8.0;

/// Thickness of a normal (non-active) axis line.
const AXIS_THICKNESS: f32 = 2.0;

/// Thickness of the active (hovered / dragged) axis line.
const ACTIVE_AXIS_THICKNESS: f32 = 3.5;

/// Arrow head size (side length of the triangle cap).
const ARROW_HEAD_SIZE: f32 = 10.0;

/// Box cap half-size for scale mode.
const BOX_CAP_HALF: f32 = 5.0;

/// Arc radius for rotate mode (screen pixels).
const ARC_RADIUS: f32 = 30.0;

/// Z-axis angle from horizontal (radians) — gives a pseudo-3-D look.
const Z_AXIS_ANGLE: f32 = -std::f32::consts::FRAC_PI_4; // 45° upper-right

// ──────────────────────────────────────────────
// Axis colours
// ──────────────────────────────────────────────

const X_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 60, 60);
const Y_COLOR: egui::Color32 = egui::Color32::from_rgb(60, 200, 60);
const Z_COLOR: egui::Color32 = egui::Color32::from_rgb(60, 100, 240);

/// Brighter variants used when an axis is active.
const X_COLOR_ACTIVE: egui::Color32 = egui::Color32::from_rgb(255, 120, 120);
const Y_COLOR_ACTIVE: egui::Color32 = egui::Color32::from_rgb(120, 255, 120);
const Z_COLOR_ACTIVE: egui::Color32 = egui::Color32::from_rgb(120, 160, 255);

// ──────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────

/// Which axis the user is interacting with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

/// Result of a single gizmo interaction frame.
///
/// All deltas are zero unless the corresponding axis was dragged. The
/// consumer (usually the viewport) decides how to apply these values to
/// entity components.
#[derive(Debug, Clone, Copy, Default)]
pub struct GizmoResult {
    /// Translation delta along world X.
    pub delta_x: f32,
    /// Translation delta along world Y.
    pub delta_y: f32,
    /// Translation delta along world Z.
    pub delta_z: f32,
    /// Rotation delta in radians (applied to the active axis).
    pub rotation_delta: f32,
    /// Uniform scale delta.
    pub scale_delta: f32,
}

// ──────────────────────────────────────────────
// GizmoSystem
// ──────────────────────────────────────────────

/// Manages rendering and input for a transform gizmo in the editor viewport.
///
/// The gizmo is drawn at a given screen-space centre point (typically the
/// projected position of the selected entity). It supports three modes —
/// [`GizmoMode::Translate`], [`GizmoMode::Rotate`], [`GizmoMode::Scale`] —    /// and produces a `GizmoResult` each frame while the user drags an axis.
pub struct GizmoSystem {
    /// Active gizmo mode (translate / rotate / scale).
    pub mode: GizmoMode,
    /// Axis currently being dragged, if any.
    pub active_axis: Option<GizmoAxis>,
    /// Screen-space position where the drag started (`[x, y]`).
    pub drag_start: Option<[f32; 2]>,
    /// Visual size of the gizmo in screen pixels.
    pub gizmo_size: f32,
    /// Length of each axis in world units (affects visual extent).
    pub axis_length: f32,
}

impl GizmoSystem {
    /// Create a new gizmo system with default settings.
    pub fn new() -> Self {
        Self {
            mode: GizmoMode::Translate,
            active_axis: None,
            drag_start: None,
            gizmo_size: DEFAULT_GIZMO_SIZE,
            axis_length: DEFAULT_AXIS_LENGTH,
        }
    }

    // ── Axis endpoint helpers ──

    /// Screen-space endpoint for the X axis (rightward).
    fn axis_end_x(center: egui::Pos2, length: f32) -> egui::Pos2 {
        egui::pos2(center.x + length, center.y)
    }

    /// Screen-space endpoint for the Y axis (upward, so negative in egui coords).
    fn axis_end_y(center: egui::Pos2, length: f32) -> egui::Pos2 {
        egui::pos2(center.x, center.y - length)
    }

    /// Screen-space endpoint for the Z axis (diagonal upper-right).
    fn axis_end_z(center: egui::Pos2, length: f32) -> egui::Pos2 {
        let dx = length * Z_AXIS_ANGLE.cos();
        let dy = length * Z_AXIS_ANGLE.sin();
        egui::pos2(center.x + dx, center.y + dy)
    }

    /// Return the endpoint and colour for a given axis.
    fn axis_info(axis: GizmoAxis, center: egui::Pos2, length: f32) -> (egui::Pos2, egui::Color32) {
        match axis {
            GizmoAxis::X => (Self::axis_end_x(center, length), X_COLOR),
            GizmoAxis::Y => (Self::axis_end_y(center, length), Y_COLOR),
            GizmoAxis::Z => (Self::axis_end_z(center, length), Z_COLOR),
        }
    }

    /// Return the active (brighter) colour for a given axis.
    fn active_color(axis: GizmoAxis) -> egui::Color32 {
        match axis {
            GizmoAxis::X => X_COLOR_ACTIVE,
            GizmoAxis::Y => Y_COLOR_ACTIVE,
            GizmoAxis::Z => Z_COLOR_ACTIVE,
        }
    }

    // ── Rendering ──

    /// Render the gizmo into the viewport.
    ///
    /// `painter` is the egui painter for the viewport rect.
    /// `center` is the screen-space pivot of the gizmo.
    pub fn render(&self, painter: &egui::Painter, center: egui::Pos2) {
        let length = self.gizmo_size;

        match self.mode {
            GizmoMode::Translate => self.render_translate(painter, center, length),
            GizmoMode::Rotate => self.render_rotate(painter, center, length),
            GizmoMode::Scale => self.render_scale(painter, center, length),
        }

        // Small centre dot.
        painter.circle_filled(center, 3.0, egui::Color32::from_rgb(200, 200, 200));
    }

    /// Draw translate gizmo — three axis lines with arrow-head caps.
    fn render_translate(&self, painter: &egui::Painter, center: egui::Pos2, length: f32) {
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let (end, base_color) = Self::axis_info(axis, center, length);
            let is_active = self.active_axis == Some(axis);
            let color = if is_active {
                Self::active_color(axis)
            } else {
                base_color
            };
            let thickness = if is_active {
                ACTIVE_AXIS_THICKNESS
            } else {
                AXIS_THICKNESS
            };
            draw_arrow(painter, center, end, color, thickness);
        }
    }

    /// Draw rotate gizmo — circular arcs for each axis.
    fn render_rotate(&self, painter: &egui::Painter, center: egui::Pos2, length: f32) {
        let radius = length * 0.65;
        let segments = [
            (GizmoAxis::X, 0.0, std::f32::consts::PI * 2.0),
            (
                GizmoAxis::Y,
                std::f32::consts::FRAC_PI_3,
                std::f32::consts::PI * 2.0 + std::f32::consts::FRAC_PI_3,
            ),
            (
                GizmoAxis::Z,
                std::f32::consts::FRAC_PI_6,
                std::f32::consts::PI * 2.0 + std::f32::consts::FRAC_PI_6,
            ),
        ];

        for (axis, start, end) in segments {
            let is_active = self.active_axis == Some(axis);
            let base_color = match axis {
                GizmoAxis::X => X_COLOR,
                GizmoAxis::Y => Y_COLOR,
                GizmoAxis::Z => Z_COLOR,
            };
            let color = if is_active {
                Self::active_color(axis)
            } else {
                base_color
            };
            let thickness = if is_active {
                ACTIVE_AXIS_THICKNESS
            } else {
                AXIS_THICKNESS
            };
            draw_arc(painter, center, radius, start, end, color, thickness);

            // Small dot at the "handle" position (end of arc).
            let handle_angle = end;
            let hx = center.x + radius * handle_angle.cos();
            let hy = center.y + radius * handle_angle.sin();
            painter.circle_filled(egui::pos2(hx, hy), 4.0, color);
        }
    }

    /// Draw scale gizmo — axis lines with small box caps.
    fn render_scale(&self, painter: &egui::Painter, center: egui::Pos2, length: f32) {
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let (end, base_color) = Self::axis_info(axis, center, length);
            let is_active = self.active_axis == Some(axis);
            let color = if is_active {
                Self::active_color(axis)
            } else {
                base_color
            };
            let thickness = if is_active {
                ACTIVE_AXIS_THICKNESS
            } else {
                AXIS_THICKNESS
            };

            // Axis line (no arrow — the box cap replaces the arrowhead).
            painter.line_segment([center, end], egui::Stroke::new(thickness, color));
            draw_box_cap(painter, end, BOX_CAP_HALF, color);
        }
    }

    // ── Input handling ──

    /// Process input for one frame and return a `GizmoResult` if the user
    /// is actively dragging an axis.
    ///
    /// * Returns `None` when no drag is in progress.
    /// * Returns `Some(GizmoResult)` each frame while dragging.
    /// * Clears state automatically on mouse release.
    pub fn handle_input(&mut self, ui: &mut egui::Ui, center: egui::Pos2) -> Option<GizmoResult> {
        let mouse_pos = ui.input(|i| i.pointer.interact_pos());
        let primary_down = ui.input(|i| i.pointer.primary_down());
        let pointer_delta = ui.input(|i| i.pointer.delta());

        let mouse = mouse_pos?;

        // ── If not yet dragging, try to pick an axis ──
        if self.active_axis.is_none() {
            if primary_down {
                if let Some(axis) = self.hit_test(mouse, center) {
                    self.active_axis = Some(axis);
                    self.drag_start = Some([mouse.x, mouse.y]);
                }
            }
            return None;
        }

        // ── Currently dragging ──
        if !primary_down {
            // Released — clear drag state.
            self.active_axis = None;
            self.drag_start = None;
            return None;
        }

        let axis = self.active_axis.expect("active_axis should be Some after early return on None");
        let start = self.drag_start.unwrap_or([mouse.x, mouse.y]);

        // Compute deltas based on mode.
        let result = match self.mode {
            GizmoMode::Translate => {
                let dx = pointer_delta.x;
                let dy = -pointer_delta.y; // Flip Y: egui Y is downward.
                let (mut out_x, mut out_y, mut out_z) = (0.0f32, 0.0f32, 0.0);
                match axis {
                    GizmoAxis::X => out_x = dx,
                    GizmoAxis::Y => out_y = dy,
                    GizmoAxis::Z => {
                        // Project screen delta onto the Z axis direction.
                        let z_dir_x = Z_AXIS_ANGLE.cos();
                        let z_dir_y = Z_AXIS_ANGLE.sin();
                        // egui delta Y is positive downward, negate to get upward.
                        let proj = dx * z_dir_x + (-pointer_delta.y) * z_dir_y;
                        out_z = proj;
                    }
                }
                GizmoResult {
                    delta_x: out_x,
                    delta_y: out_y,
                    delta_z: out_z,
                    rotation_delta: 0.0,
                    scale_delta: 0.0,
                }
            }
            GizmoMode::Rotate => {
                // Angle delta = change in angle from centre to mouse.
                let prev_angle = (start[1] - center.y).atan2(start[0] - center.x);
                let curr_angle = (mouse.y - center.y).atan2(mouse.x - center.x);
                let angle_delta = curr_angle - prev_angle;
                GizmoResult {
                    delta_x: 0.0,
                    delta_y: 0.0,
                    delta_z: 0.0,
                    rotation_delta: angle_delta,
                    scale_delta: 0.0,
                }
            }
            GizmoMode::Scale => {
                // Distance delta from drag start to current mouse, projected
                // onto the active axis direction.
                let (dir_x, dir_y) = match axis {
                    GizmoAxis::X => (1.0f32, 0.0f32),
                    GizmoAxis::Y => (0.0f32, -1.0f32), // upward in screen
                    GizmoAxis::Z => (Z_AXIS_ANGLE.cos(), Z_AXIS_ANGLE.sin()),
                };
                let rel_x = mouse.x - start[0];
                let rel_y = mouse.y - start[1];
                let projected = rel_x * dir_x + rel_y * dir_y;
                GizmoResult {
                    delta_x: 0.0,
                    delta_y: 0.0,
                    delta_z: 0.0,
                    rotation_delta: 0.0,
                    scale_delta: projected,
                }
            }
        };

        // Update drag start so next frame computes incremental delta.
        self.drag_start = Some([mouse.x, mouse.y]);

        Some(result)
    }

    /// Check whether the mouse is within the gizmo's bounding region.
    ///
    /// The bounding region is a circle centred on `center` with radius
    /// `gizmo_size`.
    pub fn is_hovering(&self, mouse_pos: egui::Pos2, center: egui::Pos2) -> bool {
        let dx = mouse_pos.x - center.x;
        let dy = mouse_pos.y - center.y;
        let dist_sq = dx * dx + dy * dy;
        let radius = self.gizmo_size;
        dist_sq <= radius * radius
    }

    /// Determine which axis the mouse is closest to, if any.
    ///
    /// Returns `Some(GizmoAxis)` when the mouse is within `HIT_THRESHOLD`
    /// pixels of an axis line segment. If multiple axes qualify, the closest
    /// one wins.
    pub fn hit_test(&self, mouse_pos: egui::Pos2, center: egui::Pos2) -> Option<GizmoAxis> {
        let length = self.gizmo_size;

        let candidates = [
            (GizmoAxis::X, Self::axis_end_x(center, length)),
            (GizmoAxis::Y, Self::axis_end_y(center, length)),
            (GizmoAxis::Z, Self::axis_end_z(center, length)),
        ];

        let mut best: Option<(GizmoAxis, f32)> = None;

        for (axis, end) in &candidates {
            let dist = point_to_segment_dist(mouse_pos, center, *end);
            if dist <= HIT_THRESHOLD {
                match best {
                    Some((_, best_dist)) if dist >= best_dist => {}
                    _ => best = Some((*axis, dist)),
                }
            }
        }

        best.map(|(axis, _)| axis)
    }
}

impl Default for GizmoSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Drawing helpers
// ──────────────────────────────────────────────

/// Draw an arrow from `start` to `end` with a triangular arrowhead.
fn draw_arrow(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    color: egui::Color32,
    thickness: f32,
) {
    // Shaft.
    painter.line_segment([start, end], egui::Stroke::new(thickness, color));

    // Arrowhead triangle.
    let dir = (end - start).normalized();
    let perp = egui::Vec2::new(-dir.y, dir.x);

    let tip = end;
    let base_center = end - dir * ARROW_HEAD_SIZE;
    let left = base_center + perp * (ARROW_HEAD_SIZE * 0.5);
    let right = base_center - perp * (ARROW_HEAD_SIZE * 0.5);

    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        color,
        egui::Stroke::new(1.0, color),
    ));
}

/// Draw a circular arc (approximated with line segments).
fn draw_arc(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: egui::Color32,
    thickness: f32,
) {
    let steps = 48;
    let angle_range = end_angle - start_angle;
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle + t * angle_range;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        points.push(egui::pos2(x, y));
    }

    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], egui::Stroke::new(thickness, color));
    }
}

/// Draw a filled box cap at `pos` (small square marker).
fn draw_box_cap(painter: &egui::Painter, pos: egui::Pos2, half_size: f32, color: egui::Color32) {
    let rect = egui::Rect::from_center_size(pos, egui::Vec2::splat(half_size * 2.0));
    painter.rect_filled(rect, 0.0, color);
}

// ──────────────────────────────────────────────
// Geometry utilities
// ──────────────────────────────────────────────

/// Minimum distance from a point to a line segment (A → B).
fn point_to_segment_dist(point: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let ab_len_sq = ab.length_sq();

    if ab_len_sq < f32::EPSILON {
        return ap.length();
    }

    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (point - projection).length()
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor defaults ──

    #[test]
    fn new_has_correct_defaults() {
        let gizmo = GizmoSystem::new();
        assert_eq!(gizmo.mode, GizmoMode::Translate);
        assert!(gizmo.active_axis.is_none());
        assert!(gizmo.drag_start.is_none());
        assert!((gizmo.gizmo_size - DEFAULT_GIZMO_SIZE).abs() < f32::EPSILON);
        assert!((gizmo.axis_length - DEFAULT_AXIS_LENGTH).abs() < f32::EPSILON);
    }

    // ── Default trait ──

    #[test]
    fn default_matches_new() {
        let from_new = GizmoSystem::new();
        let from_default = GizmoSystem::default();
        assert_eq!(from_new.mode, from_default.mode);
        assert_eq!(from_new.active_axis, from_default.active_axis);
        assert!((from_new.gizmo_size - from_default.gizmo_size).abs() < f32::EPSILON);
    }

    // ── Mode setting ──

    #[test]
    fn mode_can_be_changed() {
        let mut gizmo = GizmoSystem::new();
        assert_eq!(gizmo.mode, GizmoMode::Translate);

        gizmo.mode = GizmoMode::Rotate;
        assert_eq!(gizmo.mode, GizmoMode::Rotate);

        gizmo.mode = GizmoMode::Scale;
        assert_eq!(gizmo.mode, GizmoMode::Scale);

        gizmo.mode = GizmoMode::Translate;
        assert_eq!(gizmo.mode, GizmoMode::Translate);
    }

    // ── Hit test: X axis ──

    #[test]
    fn hit_test_x_axis() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        // Mouse slightly above the X axis line (which goes right from centre).
        let mouse = egui::pos2(240.0, 200.0);
        assert_eq!(gizmo.hit_test(mouse, center), Some(GizmoAxis::X));
    }

    // ── Hit test: Y axis ──

    #[test]
    fn hit_test_y_axis() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        // Mouse on the Y axis line (which goes upward from centre).
        let mouse = egui::pos2(200.0, 160.0);
        assert_eq!(gizmo.hit_test(mouse, center), Some(GizmoAxis::Y));
    }

    // ── Hit test: Z axis ──

    #[test]
    fn hit_test_z_axis() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        let end_z = GizmoSystem::axis_end_z(center, gizmo.gizmo_size);
        // Place mouse at the midpoint of the Z axis.
        let mid = egui::pos2((center.x + end_z.x) / 2.0, (center.y + end_z.y) / 2.0);
        assert_eq!(gizmo.hit_test(mid, center), Some(GizmoAxis::Z));
    }

    // ── Hit test: miss (too far) ──

    #[test]
    fn hit_test_miss() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        // Mouse far away from all axes.
        let mouse = egui::pos2(400.0, 400.0);
        assert_eq!(gizmo.hit_test(mouse, center), None);
    }

    // ── is_hovering: within bounds ──

    #[test]
    fn is_hovering_true_inside_radius() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        let mouse = egui::pos2(230.0, 200.0);
        assert!(gizmo.is_hovering(mouse, center));
    }

    // ── is_hovering: outside bounds ──

    #[test]
    fn is_hovering_false_outside_radius() {
        let gizmo = GizmoSystem::new();
        let center = egui::pos2(200.0, 200.0);
        let mouse = egui::pos2(500.0, 500.0);
        assert!(!gizmo.is_hovering(mouse, center));
    }

    // ── GizmoResult fields ──

    #[test]
    fn gizmo_result_default_is_zero() {
        let result = GizmoResult::default();
        assert!((result.delta_x - 0.0).abs() < f32::EPSILON);
        assert!((result.delta_y - 0.0).abs() < f32::EPSILON);
        assert!((result.delta_z - 0.0).abs() < f32::EPSILON);
        assert!((result.rotation_delta - 0.0).abs() < f32::EPSILON);
        assert!((result.scale_delta - 0.0).abs() < f32::EPSILON);
    }

    // ── gizmo_size accessor ──

    #[test]
    fn gizmo_size_accessor() {
        let gizmo = GizmoSystem::new();
        assert!((gizmo.gizmo_size - 80.0).abs() < f32::EPSILON);
    }

    // ── point_to_segment_dist ──

    #[test]
    fn point_to_segment_dist_on_line() {
        let a = egui::pos2(0.0, 0.0);
        let b = egui::pos2(100.0, 0.0);
        let p = egui::pos2(50.0, 0.0);
        assert!((point_to_segment_dist(p, a, b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn point_to_segment_dist_perpendicular() {
        let a = egui::pos2(0.0, 0.0);
        let b = egui::pos2(100.0, 0.0);
        let p = egui::pos2(50.0, 5.0);
        assert!((point_to_segment_dist(p, a, b) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn point_to_segment_dist_past_end() {
        let a = egui::pos2(0.0, 0.0);
        let b = egui::pos2(100.0, 0.0);
        let p = egui::pos2(150.0, 0.0);
        assert!((point_to_segment_dist(p, a, b) - 50.0).abs() < f32::EPSILON);
    }

    // ── active_axis round-trip ──

    #[test]
    fn active_axis_set_and_clear() {
        let mut gizmo = GizmoSystem::new();
        assert!(gizmo.active_axis.is_none());
        gizmo.active_axis = Some(GizmoAxis::X);
        assert_eq!(gizmo.active_axis, Some(GizmoAxis::X));
        gizmo.active_axis = None;
        assert!(gizmo.active_axis.is_none());
    }

    // ── drag_start round-trip ──

    #[test]
    fn drag_start_set_and_clear() {
        let mut gizmo = GizmoSystem::new();
        assert!(gizmo.drag_start.is_none());
        gizmo.drag_start = Some([100.0, 200.0]);
        assert_eq!(gizmo.drag_start, Some([100.0, 200.0]));
        gizmo.drag_start = None;
        assert!(gizmo.drag_start.is_none());
    }
}

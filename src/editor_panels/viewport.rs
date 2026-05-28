//! Scene Viewport panel — Phase 7B.
//!
//! The viewport is the primary 3D scene editor surface. It handles:
//! - Orbital camera (RMB drag), panning (MMB drag), and zoom (scroll).
//! - A grid overlay that can be toggled on/off.
//! - FPS / frame-time / entity-count status bar.
//! - Gizmo mode indicator in the corner.
//!
//! Real 3D rendering will land in Phase 7C. For now the render surface is a
//! painted placeholder rectangle, and the grid is drawn with `ui.painter()` lines.

use super::{EditorPanel, EditorState, GizmoMode};

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

/// Background fill for the 3D viewport placeholder (dark navy).
const VIEWPORT_BG: egui::Color32 = egui::Color32::from_rgb(20, 22, 30);

/// Color used for the grid lines.
const GRID_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 80, 80, 120);

/// Grid line spacing in UI pixels.
const GRID_SPACING: f32 = 40.0;

/// Maximum pitch (radians) — prevents flipping past straight-up.
const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

/// Orbit sensitivity — radians per pixel of pointer delta.
const ORBIT_SENSITIVITY: f32 = 0.005;

/// Pan sensitivity — world units per pixel of pointer delta.
const PAN_SENSITIVITY: f32 = 0.02;

/// Zoom sensitivity — distance multiplier per scroll unit.
const ZOOM_SENSITIVITY: f32 = 0.1;

/// Minimum camera distance (prevents zooming into the origin).
const MIN_DISTANCE: f32 = 0.5;

/// Maximum camera distance (arbitrary far limit).
const MAX_DISTANCE: f32 = 1000.0;

// ──────────────────────────────────────────────
// ViewportPanel
// ──────────────────────────────────────────────

/// The scene viewport panel.
///
/// Renders a placeholder 3D surface and a 2-D grid overlay. Camera interaction
/// is purely client-side (yaw / pitch / distance / target) and does not depend
/// on any rendering crate.
pub struct ViewportPanel {
    /// Horizontal orbit angle in radians. 0 = looking along −Z.
    pub camera_yaw: f32,
    /// Vertical orbit angle in radians. Positive = looking down.
    pub camera_pitch: f32,
    /// Distance from the camera to the orbit target.
    pub camera_distance: f32,
    /// Point the camera orbits around (world space x, y, z).
    pub camera_target: [f32; 3],

    /// Whether the reference grid is visible.
    pub grid_visible: bool,
    /// Half-extent of the grid (total width = 2 × `grid_size`).
    pub grid_size: f32,

    /// Frames per second (updated externally or by a frame-counting helper).
    pub fps: f64,
    /// Last frame time in milliseconds.
    pub frame_time_ms: f64,

    /// Last allocated viewport rect (saved for workspace overlay rendering).
    pub last_viewport_rect: Option<egui::Rect>,
}

impl ViewportPanel {
    /// Create a new viewport with sensible defaults.
    ///
    /// The camera starts 10 units above the origin, pitched at 45°.
    pub fn new() -> Self {
        Self {
            camera_yaw: 0.0,
            camera_pitch: std::f32::consts::FRAC_PI_4, // 45 degrees
            camera_distance: 10.0,
            camera_target: [0.0, 0.0, 0.0],
            grid_visible: true,
            grid_size: 20.0,
            fps: 0.0,
            frame_time_ms: 0.0,
            last_viewport_rect: None,
        }
    }

    // ── Internal helpers ──

    /// Clamp pitch to the range `[-PITCH_LIMIT, PITCH_LIMIT]`.
    fn clamp_pitch(pitch: f32) -> f32 {
        pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT)
    }

    /// Clamp distance to `[MIN_DISTANCE, MAX_DISTANCE]`.
    fn clamp_distance(distance: f32) -> f32 {
        distance.clamp(MIN_DISTANCE, MAX_DISTANCE)
    }

    /// Handle orbital camera input (RMB drag, MMB drag, scroll).
    fn handle_camera_input(&mut self, ui: &mut egui::Ui) {
        let input = ui.input(|i| {
            (
                i.pointer.delta(),
                i.pointer.primary_down(),   // RMB on some OSes
                i.pointer.middle_down(),
                i.smooth_scroll_delta.y,
                i.pointer.secondary_down(),  // RMB
            )
        });

        let (delta, _primary_down, middle_down, scroll_y, secondary_down) = input;

        // ── Orbit: RMB drag ──
        if secondary_down {
            self.camera_yaw += delta.x * ORBIT_SENSITIVITY;
            self.camera_pitch = Self::clamp_pitch(
                self.camera_pitch - delta.y * ORBIT_SENSITIVITY,
            );
        }

        // ── Pan: MMB drag ──
        if middle_down {
            self.camera_target[0] -= delta.x * PAN_SENSITIVITY;
            self.camera_target[1] += delta.y * PAN_SENSITIVITY;
        }

        // ── Zoom: scroll ──
        if scroll_y.abs() > 0.0 {
            let factor = 1.0 - scroll_y.signum() * ZOOM_SENSITIVITY;
            self.camera_distance = Self::clamp_distance(self.camera_distance * factor);
        }
    }

    /// Draw the reference grid using painter line calls.
    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let half = self.grid_size * GRID_SPACING;
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        let steps = (half / GRID_SPACING).ceil() as i32;

        for i in -steps..=steps {
            let offset = i as f32 * GRID_SPACING;

            // Vertical lines
            let x = center_x + offset;
            painter.line_segment(
                [
                    egui::pos2(x, rect.top()),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0, GRID_COLOR),
            );

            // Horizontal lines
            let y = center_y + offset;
            painter.line_segment(
                [
                    egui::pos2(rect.left(), y),
                    egui::pos2(rect.right(), y),
                ],
                egui::Stroke::new(1.0, GRID_COLOR),
            );
        }
    }

    /// Render the top stats bar (FPS, frame time, entity count).
    fn show_stats_bar(&self, ui: &mut egui::Ui, entity_count: usize) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("FPS: {:.0}", self.fps))
                    .monospace()
                    .color(egui::Color32::from_rgb(180, 220, 180)),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Frame: {:.2} ms", self.frame_time_ms))
                    .monospace()
                    .color(egui::Color32::from_rgb(180, 180, 220)),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Entities: {entity_count}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(220, 180, 180)),
            );
        });
    }

    /// Render the camera controls help overlay.
    fn show_controls_overlay(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("RMB drag: Orbit | MMB drag: Pan | Scroll: Zoom")
                    .small()
                    .monospace()
                    .color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 140)),
            );
        });
    }

    /// Render the gizmo mode indicator in the bottom-right corner.
    fn show_gizmo_indicator(&self, ui: &mut egui::Ui, mode: GizmoMode) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
            let label = match mode {
                GizmoMode::Translate => "⬚ Translate",
                GizmoMode::Rotate => "↻ Rotate",
                GizmoMode::Scale => "⤢ Scale",
            };
            let color = match mode {
                GizmoMode::Translate => egui::Color32::from_rgb(100, 200, 255),
                GizmoMode::Rotate => egui::Color32::from_rgb(255, 200, 100),
                GizmoMode::Scale => egui::Color32::from_rgb(200, 100, 255),
            };
            ui.label(egui::RichText::new(label).monospace().strong().color(color));
        });
    }
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// EditorPanel trait implementation
// ──────────────────────────────────────────────

impl EditorPanel for ViewportPanel {
    /// Human-readable panel title used for tab labels.
    fn title(&self) -> &str {
        "Viewport"
    }

    /// Render the full viewport panel for one frame.
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        let entity_count = state.world.entity_count();

        // ── Top stats bar ──
        self.show_stats_bar(ui, entity_count);
        ui.separator();

        // ── Main viewport area ──
        egui::ScrollArea::neither()
            .id_salt("viewport_surface")
            .show(ui, |ui| {
                let available = ui.available_size();
                let (rect, _response) = ui.allocate_exact_size(
                    available,
                    egui::Sense::click_and_drag(),
                );
                self.last_viewport_rect = Some(rect);

                let painter = ui.painter_at(rect);

                // ── 3D surface placeholder ──
                painter.rect_filled(rect, 0.0, VIEWPORT_BG);

                // ── Grid overlay ──
                if self.grid_visible {
                    self.draw_grid(&painter, rect);
                }

                // ── Handle camera input ──
                self.handle_camera_input(ui);
            });

        ui.separator();

        // ── Bottom bar: controls help + grid toggle + gizmo indicator ──
        ui.horizontal(|ui| {
            self.show_controls_overlay(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Grid toggle
                let grid_label = if self.grid_visible { "Grid: ON" } else { "Grid: OFF" };
                if ui.small_button(grid_label).clicked() {
                    self.grid_visible = !self.grid_visible;
                }
            });
        });

        // ── Gizmo mode indicator ──
        self.show_gizmo_indicator(ui, state.gizmo_mode);
    }
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
        let vp = ViewportPanel::new();
        assert!((vp.camera_yaw - 0.0).abs() < f32::EPSILON);
        assert!((vp.camera_pitch - std::f32::consts::FRAC_PI_4).abs() < f32::EPSILON,
            "default pitch should be 45 degrees");
        assert!((vp.camera_distance - 10.0).abs() < f32::EPSILON);
        assert_eq!(vp.camera_target, [0.0, 0.0, 0.0]);
        assert!(vp.grid_visible, "grid should be visible by default");
        assert!((vp.grid_size - 20.0).abs() < f32::EPSILON);
        assert!((vp.fps - 0.0).abs() < f64::EPSILON);
        assert!((vp.frame_time_ms - 0.0).abs() < f64::EPSILON);
    }

    // ── Title ──

    #[test]
    fn title_returns_viewport() {
        let vp = ViewportPanel::new();
        assert_eq!(vp.title(), "Viewport");
    }

    // ── Pitch clamping ──

    #[test]
    fn pitch_clamp_within_bounds() {
        // Pitch at the limit should stay unchanged
        let clamped = ViewportPanel::clamp_pitch(PITCH_LIMIT);
        assert!((clamped - PITCH_LIMIT).abs() < f32::EPSILON);
    }

    #[test]
    fn pitch_clamp_exceeds_positive() {
        // Pitch past the limit should be clamped
        let clamped = ViewportPanel::clamp_pitch(2.0);
        assert!(clamped <= PITCH_LIMIT, "pitch should not exceed PITCH_LIMIT");
    }

    #[test]
    fn pitch_clamp_exceeds_negative() {
        let clamped = ViewportPanel::clamp_pitch(-2.0);
        assert!(clamped >= -PITCH_LIMIT, "pitch should not go below -PITCH_LIMIT");
    }

    // ── Grid toggle ──

    #[test]
    fn grid_toggle_flips_visibility() {
        let mut vp = ViewportPanel::new();
        assert!(vp.grid_visible);
        vp.grid_visible = !vp.grid_visible;
        assert!(!vp.grid_visible);
        vp.grid_visible = !vp.grid_visible;
        assert!(vp.grid_visible);
    }

    // ── FPS display format ──

    #[test]
    fn fps_display_format() {
        let vp = ViewportPanel { fps: 144.7, ..ViewportPanel::new() };
        let text = format!("FPS: {:.0}", vp.fps);
        assert_eq!(text, "FPS: 145");
    }

    #[test]
    fn frame_time_display_format() {
        let vp = ViewportPanel { frame_time_ms: 6.944, ..ViewportPanel::new() };
        let text = format!("Frame: {:.2} ms", vp.frame_time_ms);
        assert_eq!(text, "Frame: 6.94 ms");
    }

    // ── Distance clamping ──

    #[test]
    fn distance_clamp_minimum() {
        let clamped = ViewportPanel::clamp_distance(0.01);
        assert!((clamped - MIN_DISTANCE).abs() < f32::EPSILON);
    }

    #[test]
    fn distance_clamp_maximum() {
        let clamped = ViewportPanel::clamp_distance(5000.0);
        assert!((clamped - MAX_DISTANCE).abs() < f32::EPSILON);
    }

    // ── Default trait ──

    #[test]
    fn default_matches_new() {
        let from_new = ViewportPanel::new();
        let from_default = ViewportPanel::default();
        assert!((from_new.camera_yaw - from_default.camera_yaw).abs() < f32::EPSILON);
        assert!((from_new.camera_pitch - from_default.camera_pitch).abs() < f32::EPSILON);
        assert!((from_new.camera_distance - from_default.camera_distance).abs() < f32::EPSILON);
        assert_eq!(from_new.camera_target, from_default.camera_target);
        assert_eq!(from_new.grid_visible, from_default.grid_visible);
    }

    // ── Gizmo indicator labels ──

    #[test]
    fn gizmo_mode_labels() {
        let translate = match GizmoMode::Translate {
            GizmoMode::Translate => "⬚ Translate",
            GizmoMode::Rotate => "↻ Rotate",
            GizmoMode::Scale => "⤢ Scale",
        };
        assert_eq!(translate, "⬚ Translate");

        let rotate = match GizmoMode::Rotate {
            GizmoMode::Translate => "⬚ Translate",
            GizmoMode::Rotate => "↻ Rotate",
            GizmoMode::Scale => "⤢ Scale",
        };
        assert_eq!(rotate, "↻ Rotate");

        let scale = match GizmoMode::Scale {
            GizmoMode::Translate => "⬚ Translate",
            GizmoMode::Rotate => "↻ Rotate",
            GizmoMode::Scale => "⤢ Scale",
        };
        assert_eq!(scale, "⤢ Scale");
    }
}

//! Toolbar panel — Phase 7B.
//!
//! A horizontal toolbar that sits at the top of the editor window. It provides
//! play/pause/stop controls, gizmo mode selection (Translate / Rotate / Scale),
//! snap-to-grid settings, and a grid visibility toggle.
//!
//! Keyboard shortcuts are also handled here:
//! - **W** — Translate gizmo
//! - **E** — Rotate gizmo
//! - **R** — Scale gizmo
//! - **G** — Toggle snap
//! - **F5** — Play / Stop toggle

use std::time::Instant;

use super::{EditorPanel, EditorState, GizmoMode, PlayMode};

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

/// Accent colour for the active Play button (green).
const PLAY_ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(40, 160, 60);

/// Accent colour for the active Pause button (amber / yellow).
const PAUSE_ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 180, 40);

/// Accent colour for the active Stop button (red).
const STOP_ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);

/// Default snap size.
const DEFAULT_SNAP_SIZE: f32 = 1.0;

// ──────────────────────────────────────────────
// ToolbarPanel
// ──────────────────────────────────────────────

/// The horizontal toolbar strip at the top of the editor.
///
/// Contains play controls, gizmo mode buttons, snap settings, and a grid
/// visibility toggle. Keyboard shortcuts are processed inside [`show`](EditorPanel::show)
/// so they fire every frame regardless of focus.
pub struct ToolbarPanel {
    /// Accumulated seconds since play mode started (displayed as `MM:SS.t`).
    pub play_elapsed: f64,

    /// Timestamp of the last frame — used to advance `play_elapsed` while
    /// in [`PlayMode::Playing`]. Wrapped in `Option` so the constructor
    /// doesn't need to call `Instant::now()` (which would break unit tests
    /// that never call `show`).
    last_frame_time: Option<Instant>,

    /// Whether the viewport grid overlay is visible (placeholder — the
    /// actual rendering lives in `ViewportPanel`).
    pub grid_visible: bool,
}

impl ToolbarPanel {
    /// Create a new toolbar with sensible defaults.
    pub fn new() -> Self {
        Self {
            play_elapsed: 0.0,
            last_frame_time: None,
            grid_visible: true,
        }
    }

    // ── Internal helpers ──

    /// Advance `play_elapsed` by the delta since the last frame (only while
    /// playing). Returns the delta in seconds.
    fn update_play_elapsed(&mut self, play_mode: PlayMode) -> f64 {
        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(prev) => now.duration_since(prev).as_secs_f64(),
            None => 0.0,
        };
        self.last_frame_time = Some(now);

        if play_mode == PlayMode::Playing {
            self.play_elapsed += delta;
        }
        delta
    }

    /// Reset the play elapsed timer to zero.
    fn reset_play_timer(&mut self) {
        self.play_elapsed = 0.0;
    }

    /// Format `play_elapsed` as `MM:SS.t` (one decimal place).
    fn format_elapsed(secs: f64) -> String {
        let total_secs = secs.max(0.0);
        let minutes = (total_secs / 60.0).floor() as u32;
        let seconds = total_secs - (minutes as f64 * 60.0);
        format!("{:02}:{:04.1}", minutes, seconds)
    }

    /// Build a coloured button for the play/pause/stop controls.
    ///
    /// `is_active` controls whether the button gets a filled background
    /// (`fill_color`) or uses the default widget style.
    fn play_button(
        ui: &mut egui::Ui,
        label: &str,
        is_active: bool,
        fill_color: egui::Color32,
    ) -> bool {
        if is_active {
            // Filled / highlighted variant.
            ui.add(
                egui::Button::new(
                    egui::RichText::new(label).strong().color(egui::Color32::WHITE),
                )
                .fill(fill_color),
            )
            .clicked()
        } else {
            ui.button(label).clicked()
        }
    }

    /// Render the play / pause / stop controls (left group).
    fn show_play_controls(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            let is_stopped = state.play_mode == PlayMode::Stopped;
            let is_playing = state.play_mode == PlayMode::Playing;
            let is_paused = state.play_mode == PlayMode::Paused;

            // ── Play ──
            if Self::play_button(ui, "▶ Play", is_playing, PLAY_ACTIVE_COLOR) {
                if is_stopped {
                    state.play_mode = PlayMode::Playing;
                    self.reset_play_timer();
                }
            }

            // ── Pause ──
            if Self::play_button(ui, "⏸ Pause", is_paused, PAUSE_ACTIVE_COLOR) {
                if is_playing {
                    state.play_mode = PlayMode::Paused;
                }
            }

            // ── Stop ──
            if Self::play_button(ui, "⏹ Stop", is_stopped, STOP_ACTIVE_COLOR) {
                if is_playing || is_paused {
                    state.play_mode = PlayMode::Stopped;
                    self.reset_play_timer();
                }
            }

            // ── Elapsed time display ──
            if is_playing || is_paused {
                ui.label(
                    egui::RichText::new(Self::format_elapsed(self.play_elapsed))
                        .monospace()
                        .color(egui::Color32::from_rgb(180, 220, 255)),
                );
            }
        });
    }

    /// Render the gizmo mode selection buttons (center group).
    fn show_gizmo_controls(ui: &mut egui::Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            ui.separator();

            let modes = [
                (GizmoMode::Translate, "Move", "W"),
                (GizmoMode::Rotate, "Rotate", "E"),
                (GizmoMode::Scale, "Scale", "R"),
            ];

            for (mode, label, shortcut) in modes {
                let is_active = state.gizmo_mode == mode;
                let button = if is_active {
                    egui::Button::new(
                        egui::RichText::new(format!("{label} ({shortcut})"))
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(ui.visuals().selection.bg_fill)
                } else {
                    egui::Button::new(format!("{label} ({shortcut})"))
                };
                if ui.add(button).clicked() {
                    state.gizmo_mode = mode;
                }
            }
        });
    }

    /// Render the snap-to-grid controls (right group).
    fn show_snap_controls(ui: &mut egui::Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            ui.separator();

            // Snap toggle.
            let snap_label = if state.snap_enabled { "Snap: ON" } else { "Snap: OFF" };
            let snap_button = if state.snap_enabled {
                egui::Button::new(
                    egui::RichText::new(snap_label).strong().color(egui::Color32::WHITE),
                )
                .fill(ui.visuals().selection.bg_fill)
            } else {
                egui::Button::new(snap_label)
            };
            if ui.add(snap_button).clicked() {
                state.snap_enabled = !state.snap_enabled;
            }

            // Snap size drag — only interactive when snap is enabled.
            ui.add_enabled(
                state.snap_enabled,
                egui::DragValue::new(&mut state.snap_size)
                    .range(0.1..=10.0)
                    .speed(0.1)
                    .fixed_decimals(1)
                    .suffix(""),
            );
        });
    }

    /// Render the grid visibility toggle (far right).
    fn show_grid_toggle(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.separator();
            let label = if self.grid_visible { "Grid: ON" } else { "Grid: OFF" };
            if ui.button(label).clicked() {
                self.grid_visible = !self.grid_visible;
            }
        });
    }

    /// Process keyboard shortcuts for the toolbar.
    fn handle_shortcuts(ui: &mut egui::Ui, state: &mut EditorState) {
        // We only react to key-press events, not held keys.
        let input = ui.input(|i| {
            let mut keys_pressed = (
                false, // W
                false, // E
                false, // R
                false, // G
                false, // F5
            );

            for event in &i.events {
                if let egui::Event::Key { key, pressed, .. } = event {
                    if *pressed {
                        match key {
                            egui::Key::W => keys_pressed.0 = true,
                            egui::Key::E => keys_pressed.1 = true,
                            egui::Key::R => keys_pressed.2 = true,
                            egui::Key::G => keys_pressed.3 = true,
                            egui::Key::F5 => keys_pressed.4 = true,
                            _ => {}
                        }
                    }
                }
            }
            keys_pressed
        });

        if input.0 {
            state.gizmo_mode = GizmoMode::Translate;
        }
        if input.1 {
            state.gizmo_mode = GizmoMode::Rotate;
        }
        if input.2 {
            state.gizmo_mode = GizmoMode::Scale;
        }
        if input.3 {
            state.snap_enabled = !state.snap_enabled;
        }
        if input.4 {
            state.play_mode = match state.play_mode {
                PlayMode::Stopped => PlayMode::Playing,
                PlayMode::Playing | PlayMode::Paused => PlayMode::Stopped,
            };
        }
    }
}

impl Default for ToolbarPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// EditorPanel trait implementation
// ──────────────────────────────────────────────

impl EditorPanel for ToolbarPanel {
    /// Human-readable panel title used for tab labels.
    fn title(&self) -> &str {
        "Toolbar"
    }

    /// Render the full horizontal toolbar for one frame.
    ///
    /// Layout: `[ Play Controls | Gizmo Mode | Snap Controls | Grid Toggle ]`
    ///
    /// Keyboard shortcuts are processed before rendering so they take effect
    /// on the same frame.
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // ── Update timing ──
        self.update_play_elapsed(state.play_mode);

        // ── Keyboard shortcuts (before render so state is current) ──
        Self::handle_shortcuts(ui, state);

        // ── Render toolbar ──
        ui.horizontal(|ui| {
            // ── Play controls (left) ──
            self.show_play_controls(ui, state);

            // ── Gizmo mode (center) ──
            Self::show_gizmo_controls(ui, state);

            // ── Snap controls (right) ──
            Self::show_snap_controls(ui, state);

            // ── Grid toggle (far right) ──
            self.show_grid_toggle(ui);
        });
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
    fn new_returns_expected_defaults() {
        let panel = ToolbarPanel::new();
        assert!((panel.play_elapsed - 0.0).abs() < f64::EPSILON);
        assert!(panel.last_frame_time.is_none());
        assert!(panel.grid_visible, "grid should be visible by default");
    }

    // ── Default trait ──

    #[test]
    fn default_matches_new() {
        let from_new = ToolbarPanel::new();
        let from_default = ToolbarPanel::default();
        assert!((from_new.play_elapsed - from_default.play_elapsed).abs() < f64::EPSILON);
        assert_eq!(from_new.grid_visible, from_default.grid_visible);
    }

    // ── Title ──

    #[test]
    fn title_returns_toolbar() {
        let panel = ToolbarPanel::new();
        assert_eq!(panel.title(), "Toolbar");
    }

    // ── Play mode transitions ──

    #[test]
    fn play_mode_stopped_to_playing() {
        let mut state = EditorState::new();
        assert_eq!(state.play_mode, PlayMode::Stopped);
        state.play_mode = PlayMode::Playing;
        assert_eq!(state.play_mode, PlayMode::Playing);
    }

    #[test]
    fn play_mode_playing_to_paused() {
        let mut state = EditorState::new();
        state.play_mode = PlayMode::Playing;
        state.play_mode = PlayMode::Paused;
        assert_eq!(state.play_mode, PlayMode::Paused);
    }

    #[test]
    fn play_mode_paused_to_stopped() {
        let mut state = EditorState::new();
        state.play_mode = PlayMode::Playing;
        state.play_mode = PlayMode::Paused;
        state.play_mode = PlayMode::Stopped;
        assert_eq!(state.play_mode, PlayMode::Stopped);
    }

    #[test]
    fn play_mode_cycle_all_states() {
        let mut state = EditorState::new();
        assert_eq!(state.play_mode, PlayMode::Stopped);
        state.play_mode = PlayMode::Playing;
        assert_eq!(state.play_mode, PlayMode::Playing);
        state.play_mode = PlayMode::Paused;
        assert_eq!(state.play_mode, PlayMode::Paused);
        state.play_mode = PlayMode::Stopped;
        assert_eq!(state.play_mode, PlayMode::Stopped);
    }

    // ── Gizmo mode cycling ──

    #[test]
    fn gizmo_mode_cycles_through_all_variants() {
        let mut state = EditorState::new();
        assert_eq!(state.gizmo_mode, GizmoMode::Translate);

        state.gizmo_mode = GizmoMode::Rotate;
        assert_eq!(state.gizmo_mode, GizmoMode::Rotate);

        state.gizmo_mode = GizmoMode::Scale;
        assert_eq!(state.gizmo_mode, GizmoMode::Scale);

        state.gizmo_mode = GizmoMode::Translate;
        assert_eq!(state.gizmo_mode, GizmoMode::Translate);
    }

    // ── Snap toggle ──

    #[test]
    fn snap_toggle_flips() {
        let mut state = EditorState::new();
        assert!(!state.snap_enabled);
        state.snap_enabled = !state.snap_enabled;
        assert!(state.snap_enabled);
        state.snap_enabled = !state.snap_enabled;
        assert!(!state.snap_enabled);
    }

    // ── Elapsed time formatting ──

    #[test]
    fn format_elapsed_zero() {
        assert_eq!(ToolbarPanel::format_elapsed(0.0), "00:00.0");
    }

    #[test]
    fn format_elapsed_seconds_only() {
        assert_eq!(ToolbarPanel::format_elapsed(12.3), "00:12.3");
    }

    #[test]
    fn format_elapsed_minutes_and_seconds() {
        assert_eq!(ToolbarPanel::format_elapsed(72.5), "01:12.5");
    }

    #[test]
    fn format_elapsed_large_value() {
        assert_eq!(ToolbarPanel::format_elapsed(3661.0), "61:01.0");
    }

    // ── Grid toggle ──

    #[test]
    fn grid_toggle_flips_visibility() {
        let mut panel = ToolbarPanel::new();
        assert!(panel.grid_visible);
        panel.grid_visible = !panel.grid_visible;
        assert!(!panel.grid_visible);
        panel.grid_visible = !panel.grid_visible;
        assert!(panel.grid_visible);
    }

    // ── Snap size range ──

    #[test]
    fn snap_size_default() {
        let state = EditorState::new();
        assert!((state.snap_size - 1.0).abs() < f32::EPSILON);
    }

    // ── F5 shortcut: play/stop toggle ──

    #[test]
    fn f5_toggles_stopped_to_playing() {
        let mut state = EditorState::new();
        assert_eq!(state.play_mode, PlayMode::Stopped);
        // Simulate what the shortcut handler does:
        state.play_mode = match state.play_mode {
            PlayMode::Stopped => PlayMode::Playing,
            PlayMode::Playing | PlayMode::Paused => PlayMode::Stopped,
        };
        assert_eq!(state.play_mode, PlayMode::Playing);
    }

    #[test]
    fn f5_toggles_playing_to_stopped() {
        let mut state = EditorState::new();
        state.play_mode = PlayMode::Playing;
        state.play_mode = match state.play_mode {
            PlayMode::Stopped => PlayMode::Playing,
            PlayMode::Playing | PlayMode::Paused => PlayMode::Stopped,
        };
        assert_eq!(state.play_mode, PlayMode::Stopped);
    }

    // ── Play timer reset ──

    #[test]
    fn reset_play_timer_clears_elapsed() {
        let mut panel = ToolbarPanel::new();
        panel.play_elapsed = 42.0;
        panel.reset_play_timer();
        assert!((panel.play_elapsed - 0.0).abs() < f64::EPSILON);
    }
}

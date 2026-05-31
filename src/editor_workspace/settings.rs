//! Settings dialog for the editor.
//!
//! Provides an `EditorSettings` struct holding every configurable value and a
//! `SettingsDialog` that renders a tabbed egui window (Rendering / Editor /
//! Shortcuts). Values are clamped on apply and can be reset to defaults.

use std::f32::consts::FRAC_PI_4;

// ──────────────────────────────────────────────
// EditorSettings
// ──────────────────────────────────────────────

/// All configurable editor state. Every field is `pub` so the dialog (and any
/// consuming system) can read freely. Use [`apply_clamp`] before reading to
/// guarantee valid ranges.
#[derive(Debug, Clone)]
pub struct EditorSettings {
    // ── Rendering ────────────────────────────────
    pub vsync: bool,
    pub max_fps: u32,
    pub background_color: [f32; 3],
    pub grid_visible: bool,
    pub grid_spacing: f32,
    pub grid_major_every: u32,

    // ── Editor ───────────────────────────────────
    pub undo_history_size: usize,
    pub auto_save_interval_secs: u32,
    pub snap_enabled: bool,
    pub snap_size: f32,
    pub gizmo_size: f32,
    pub pick_radius: f32,

    // ── Viewport ─────────────────────────────────
    pub default_camera_distance: f32,
    pub default_camera_pitch_rad: f32,
    pub camera_speed: f32,
    pub zoom_speed: f32,
}

impl EditorSettings {
    /// Sensible defaults that match a Blender-like editor experience.
    pub fn new() -> Self {
        Self {
            vsync: true,
            max_fps: 60,
            background_color: [0.067, 0.067, 0.067],
            grid_visible: true,
            grid_spacing: 1.0,
            grid_major_every: 5,
            undo_history_size: 100,
            auto_save_interval_secs: 300,
            snap_enabled: false,
            snap_size: 1.0,
            gizmo_size: 80.0,
            pick_radius: 5.0,
            default_camera_distance: 10.0,
            default_camera_pitch_rad: FRAC_PI_4,
            camera_speed: 1.0,
            zoom_speed: 1.0,
        }
    }

    /// Clamp every numeric field to its valid range so downstream consumers
    /// never see garbage.
    pub fn apply_clamp(&mut self) {
        self.max_fps = self.max_fps.clamp(30, 240);
        for c in &mut self.background_color {
            *c = c.clamp(0.0, 1.0);
        }
        self.grid_spacing = self.grid_spacing.clamp(0.1, 100.0);
        self.grid_major_every = self.grid_major_every.clamp(2, 20);
        self.undo_history_size = self.undo_history_size.clamp(10, 1000);
        self.snap_size = self.snap_size.clamp(0.01, 100.0);
        self.gizmo_size = self.gizmo_size.clamp(20.0, 200.0);
        self.pick_radius = self.pick_radius.clamp(1.0, 50.0);
        self.default_camera_distance = self.default_camera_distance.clamp(1.0, 1000.0);
        self.default_camera_pitch_rad = self
            .default_camera_pitch_rad
            .clamp(0.0, std::f32::consts::FRAC_PI_2);
        self.camera_speed = self.camera_speed.clamp(0.1, 10.0);
        self.zoom_speed = self.zoom_speed.clamp(0.1, 10.0);
    }
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// SettingsTab
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Rendering,
    Editor,
    Shortcuts,
}

// ──────────────────────────────────────────────
// SettingsDialog
// ──────────────────────────────────────────────

/// Three-tab settings window rendered via egui.
pub struct SettingsDialog {
    pub visible: bool,
    pub settings: EditorSettings,
    pub active_tab: SettingsTab,
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            settings: EditorSettings::new(),
            active_tab: SettingsTab::Rendering,
        }
    }

    /// Render the dialog (no-op when `visible == false`).
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let open_flag = std::cell::Cell::new(self.visible);
        let mut was_open = open_flag.get();
        egui::Window::new("Settings")
            .open(&mut was_open)
            .resizable(true)
            .default_size([420.0, 520.0])
            .show(ctx, |ui| {
                // ── Tab bar ────────────────────────────
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Rendering, "Rendering");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Editor, "Editor");
                    ui.selectable_value(&mut self.active_tab, SettingsTab::Shortcuts, "Shortcuts");
                });
                ui.separator();

                // ── Tab body (scrollable) ──────────────
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| match self.active_tab {
                        SettingsTab::Rendering => self.show_rendering_tab(ui),
                        SettingsTab::Editor => self.show_editor_tab(ui),
                        SettingsTab::Shortcuts => self.show_shortcuts_tab(ui),
                    });

                // ── Action bar ─────────────────────────
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        self.settings.apply_clamp();
                    }
                    if ui.button("Reset to Defaults").clicked() {
                        self.settings = EditorSettings::new();
                    }
                    if ui.button("Close").clicked() {
                        open_flag.set(false);
                    }
                });
            });

        self.visible = open_flag.get() && was_open;
    }

    // ── Rendering tab ─────────────────────────────

    fn show_rendering_tab(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;

        ui.heading("Rendering");
        ui.add_space(4.0);

        ui.checkbox(&mut s.vsync, "VSync");
        ui.add(egui::Slider::new(&mut s.max_fps, 30..=240).text("Max FPS"));

        ui.add_space(8.0);
        ui.heading("Background");
        ui.horizontal(|ui| {
            ui.color_edit_button_rgb(&mut s.background_color);
            ui.label("Background Color");
        });

        ui.add_space(8.0);
        ui.heading("Grid");
        ui.checkbox(&mut s.grid_visible, "Show Grid");
        ui.add_enabled_ui(s.grid_visible, |ui| {
            ui.add(egui::Slider::new(&mut s.grid_spacing, 0.1..=100.0).text("Grid Spacing"));
            ui.add(egui::Slider::new(&mut s.grid_major_every, 2..=20).text("Major Every N"));
        });
    }

    // ── Editor tab ────────────────────────────────

    fn show_editor_tab(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;

        ui.heading("General");
        ui.add_space(4.0);

        ui.add(egui::Slider::new(&mut s.undo_history_size, 10..=1000).text("Undo History Size"));

        // Auto-save
        ui.add_space(4.0);
        let auto_save_enabled = s.auto_save_interval_secs > 0;
        let mut toggle = auto_save_enabled;
        ui.checkbox(&mut toggle, "Auto-Save");
        if toggle != auto_save_enabled {
            s.auto_save_interval_secs = if toggle { 300 } else { 0 };
        }
        ui.add_enabled_ui(toggle, |ui| {
            ui.add(
                egui::Slider::new(&mut s.auto_save_interval_secs, 30..=3600).text("Interval (s)"),
            );
        });

        ui.add_space(8.0);
        ui.heading("Snap");
        ui.checkbox(&mut s.snap_enabled, "Enable Snap");
        ui.add_enabled_ui(s.snap_enabled, |ui| {
            ui.add(egui::Slider::new(&mut s.snap_size, 0.01..=100.0).text("Snap Size"));
        });

        ui.add_space(8.0);
        ui.heading("Tools");
        ui.add(egui::Slider::new(&mut s.gizmo_size, 20.0..=200.0).text("Gizmo Size (px)"));
        ui.add(egui::Slider::new(&mut s.pick_radius, 1.0..=50.0).text("Pick Radius (px)"));

        ui.add_space(8.0);
        ui.heading("Viewport Camera");
        ui.add(
            egui::Slider::new(&mut s.default_camera_distance, 1.0..=1000.0)
                .text("Default Distance"),
        );
        ui.add(
            egui::Slider::new(&mut s.default_camera_pitch_rad, 0.0..=1.5707964)
                .text("Default Pitch (rad)"),
        );
        ui.add(egui::Slider::new(&mut s.camera_speed, 0.1..=10.0).text("Camera Speed"));
        ui.add(egui::Slider::new(&mut s.zoom_speed, 0.1..=10.0).text("Zoom Speed"));
    }

    // ── Shortcuts tab (placeholder) ───────────────

    fn show_shortcuts_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(egui::RichText::new("Shortcut configuration coming soon").strong());
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Use the default Blender-style keybindings for now.\n\
                     A full keybinding editor with search and conflict \
                     detection will be added in a future release.",
                )
                .small()
                .weak(),
            );
        });
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    // ── EditorSettings ─────────────────────────

    #[test]
    fn editor_settings_new_defaults() {
        let s = EditorSettings::new();
        assert!(s.vsync);
        assert_eq!(s.max_fps, 60);
        assert_eq!(s.background_color, [0.067, 0.067, 0.067]);
        assert!(s.grid_visible);
        assert!((s.grid_spacing - 1.0).abs() < f32::EPSILON);
        assert_eq!(s.grid_major_every, 5);
        assert_eq!(s.undo_history_size, 100);
        assert_eq!(s.auto_save_interval_secs, 300);
        assert!(!s.snap_enabled);
        assert!((s.snap_size - 1.0).abs() < f32::EPSILON);
        assert!((s.gizmo_size - 80.0).abs() < f32::EPSILON);
        assert!((s.pick_radius - 5.0).abs() < f32::EPSILON);
        assert!((s.default_camera_distance - 10.0).abs() < f32::EPSILON);
        assert!((s.default_camera_pitch_rad - FRAC_PI_4).abs() < f32::EPSILON);
        assert!((s.camera_speed - 1.0).abs() < f32::EPSILON);
        assert!((s.zoom_speed - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_clamp_clamps_low_values() {
        let mut s = EditorSettings::new();
        s.max_fps = 1;
        s.grid_spacing = -5.0;
        s.grid_major_every = 0;
        s.undo_history_size = 1;
        s.snap_size = 0.0;
        s.gizmo_size = 1.0;
        s.pick_radius = 0.0;
        s.default_camera_distance = 0.0;
        s.default_camera_pitch_rad = -1.0;
        s.camera_speed = 0.0;
        s.zoom_speed = 0.0;
        s.background_color = [-0.5, 2.0, -0.1];
        s.apply_clamp();

        assert_eq!(s.max_fps, 30);
        assert!((s.grid_spacing - 0.1).abs() < 0.01);
        assert_eq!(s.grid_major_every, 2);
        assert_eq!(s.undo_history_size, 10);
        assert!((s.snap_size - 0.01).abs() < 0.001);
        assert!((s.gizmo_size - 20.0).abs() < 0.01);
        assert!((s.pick_radius - 1.0).abs() < 0.01);
        assert!((s.default_camera_distance - 1.0).abs() < 0.01);
        assert!((s.default_camera_pitch_rad).abs() < 0.001);
        assert!((s.camera_speed - 0.1).abs() < 0.01);
        assert!((s.zoom_speed - 0.1).abs() < 0.01);
        assert_eq!(s.background_color, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn apply_clamp_clamps_high_values() {
        let mut s = EditorSettings::new();
        s.max_fps = 9999;
        s.grid_spacing = 500.0;
        s.grid_major_every = 100;
        s.undo_history_size = 99999;
        s.snap_size = 999.0;
        s.gizmo_size = 9999.0;
        s.pick_radius = 999.0;
        s.default_camera_distance = 99999.0;
        s.default_camera_pitch_rad = 10.0;
        s.camera_speed = 100.0;
        s.zoom_speed = 100.0;
        s.apply_clamp();

        assert_eq!(s.max_fps, 240);
        assert!((s.grid_spacing - 100.0).abs() < 0.01);
        assert_eq!(s.grid_major_every, 20);
        assert_eq!(s.undo_history_size, 1000);
        assert!((s.snap_size - 100.0).abs() < 0.01);
        assert!((s.gizmo_size - 200.0).abs() < 0.01);
        assert!((s.pick_radius - 50.0).abs() < 0.01);
        assert!((s.default_camera_distance - 1000.0).abs() < 0.01);
        assert!((s.default_camera_pitch_rad - FRAC_PI_4 * 2.0).abs() < 0.01);
        assert!((s.camera_speed - 10.0).abs() < 0.01);
        assert!((s.zoom_speed - 10.0).abs() < 0.01);
    }

    #[test]
    fn apply_clamp_idempotent_on_valid_values() {
        let s1 = EditorSettings::new();
        let mut s2 = s1.clone();
        s2.apply_clamp();
        assert_eq!(s1.max_fps, s2.max_fps);
        assert_eq!(s1.grid_spacing.to_bits(), s2.grid_spacing.to_bits());
    }

    // ── SettingsDialog ─────────────────────────

    #[test]
    fn dialog_new_defaults() {
        let d = SettingsDialog::new();
        assert!(!d.visible);
        assert_eq!(d.active_tab, SettingsTab::Rendering);
        assert_eq!(d.settings.max_fps, 60);
    }

    #[test]
    fn tab_switching() {
        let mut d = SettingsDialog::new();
        assert_eq!(d.active_tab, SettingsTab::Rendering);

        d.active_tab = SettingsTab::Editor;
        assert_eq!(d.active_tab, SettingsTab::Editor);

        d.active_tab = SettingsTab::Shortcuts;
        assert_eq!(d.active_tab, SettingsTab::Shortcuts);

        d.active_tab = SettingsTab::Rendering;
        assert_eq!(d.active_tab, SettingsTab::Rendering);
    }

    #[test]
    fn visibility_toggle() {
        let mut d = SettingsDialog::new();
        assert!(!d.visible);
        d.visible = true;
        assert!(d.visible);
        d.visible = false;
        assert!(!d.visible);
    }

    #[test]
    fn reset_to_defaults() {
        let mut d = SettingsDialog::new();
        d.settings.max_fps = 240;
        d.settings.vsync = false;
        d.settings.snap_enabled = true;
        d.settings = EditorSettings::new();
        assert_eq!(d.settings.max_fps, 60);
        assert!(d.settings.vsync);
        assert!(!d.settings.snap_enabled);
    }

    #[test]
    fn default_trait_matches_new() {
        let via_new = EditorSettings::new();
        let via_default = EditorSettings::default();
        assert_eq!(via_new.max_fps, via_default.max_fps);
        assert_eq!(via_new.vsync, via_default.vsync);
        assert_eq!(via_new.undo_history_size, via_default.undo_history_size);
    }
}

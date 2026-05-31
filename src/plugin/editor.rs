//! Editor Plugin Hooks — Phase 13.
//!
//! Plugins that expose editor hooks can inject custom UI into the
//! Chronos Editor. This module defines the extension points.
//!
//! # Design
//!
//! Editor hooks are optional. A plugin implements [`EditorPluginHooks`]
//! and returns it from [`Plugin::editor_hooks`](super::Plugin::editor_hooks).
//!
//! When the editor is active, it queries each plugin for hooks and
//! renders them in the appropriate panel areas.
//!
//! # Future: egui Integration
//!
//! When the `editor` feature is enabled, these hooks will receive an
//! `egui::Ui` context and can draw widgets. For now the trait is
//! feature-agnostic, using string-based descriptors that the editor
//! can render as placeholder panels.

/// A descriptor for a custom editor panel contributed by a plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorPanel {
    /// Unique panel identifier (scoped to the plugin).
    pub id: String,
    /// Human-readable panel title.
    pub title: String,
    /// Default width in points.
    pub default_width: f32,
    /// Default height in points.
    pub default_height: f32,
    /// Which dock zone to place the panel in by default.
    pub default_zone: DockZone,
}

/// Where a plugin panel should dock by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockZone {
    Left,
    Right,
    Bottom,
    Central,
}

/// A descriptor for a custom inspector contributed by a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorHook {
    /// Component type name this inspector can edit.
    pub component_name: String,
    /// Human-readable inspector title.
    pub title: String,
}

/// A descriptor for a toolbar button contributed by a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarButton {
    /// Button identifier.
    pub id: String,
    /// Tooltip text.
    pub tooltip: String,
    /// Keyboard shortcut (e.g., "Ctrl+Shift+E").
    pub shortcut: Option<String>,
}

/// Editor extension interface for plugins.
///
/// Plugins implement this trait (in addition to [`Plugin`](super::Plugin))
/// to contribute custom UI to the Chronos Editor.
pub trait EditorPluginHooks: Send + Sync {
    /// Return descriptors for all custom panels this plugin provides.
    fn panels(&mut self) -> Vec<EditorPanel> {
        Vec::new()
    }

    /// Return descriptors for all inspector hooks this plugin provides.
    fn inspectors(&mut self) -> Vec<InspectorHook> {
        Vec::new()
    }

    /// Return descriptors for all toolbar buttons this plugin provides.
    fn toolbar_buttons(&mut self) -> Vec<ToolbarButton> {
        Vec::new()
    }

    /// Called every editor frame for each panel. The plugin should
    /// produce a text representation of the panel contents.
    ///
    /// In a full egui integration, this receives `&mut egui::Ui`.
    fn render_panel(&mut self, panel_id: &str) -> String {
        format!("Panel '{}' content", panel_id)
    }

    /// Called when the user clicks a toolbar button contributed by this plugin.
    fn on_toolbar_click(&mut self, button_id: &str) {
        let _ = button_id;
    }

    /// Called when an entity with a hooked component is selected.
    fn on_inspect(&mut self, component_name: &str, entity_id: u32) -> String {
        format!("Inspecting {} on entity {}", component_name, entity_id)
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyEditorPlugin;

    impl EditorPluginHooks for DummyEditorPlugin {
        fn panels(&mut self) -> Vec<EditorPanel> {
            vec![EditorPanel {
                id: "stats".into(),
                title: "Statistics".into(),
                default_width: 300.0,
                default_height: 400.0,
                default_zone: DockZone::Right,
            }]
        }

        fn toolbar_buttons(&mut self) -> Vec<ToolbarButton> {
            vec![ToolbarButton {
                id: "export".into(),
                tooltip: "Export scene".into(),
                shortcut: Some("Ctrl+E".into()),
            }]
        }

        fn render_panel(&mut self, panel_id: &str) -> String {
            format!("Rendering panel: {}", panel_id)
        }
    }

    #[test]
    fn panel_descriptor() {
        let mut plugin = DummyEditorPlugin;
        let panels = plugin.panels();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].id, "stats");
        assert_eq!(panels[0].default_zone, DockZone::Right);
    }

    #[test]
    fn toolbar_buttons() {
        let mut plugin = DummyEditorPlugin;
        let buttons = plugin.toolbar_buttons();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0].id, "export");
        assert_eq!(buttons[0].shortcut, Some("Ctrl+E".to_string()));
    }

    #[test]
    fn render_panel_output() {
        let mut plugin = DummyEditorPlugin;
        let out = plugin.render_panel("stats");
        assert_eq!(out, "Rendering panel: stats");
    }

    #[test]
    fn default_hooks_are_empty() {
        struct EmptyPlugin;
        impl EditorPluginHooks for EmptyPlugin {}

        let mut p = EmptyPlugin;
        assert!(p.panels().is_empty());
        assert!(p.inspectors().is_empty());
        assert!(p.toolbar_buttons().is_empty());
    }

    #[test]
    fn dock_zone_equality() {
        assert_eq!(DockZone::Left, DockZone::Left);
        assert_ne!(DockZone::Left, DockZone::Right);
    }
}

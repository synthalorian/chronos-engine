//! Keyboard shortcut system for the Chronos Engine editor.
//!
//! Provides a `ShortcutMap` with configurable keybindings and Blender-style
//! defaults. Actions are mapped to `KeyBinding`s (key + modifier combo) and
//! can be queried, rebound, or unbound at runtime.

use std::collections::HashMap;
use std::fmt;

// ──────────────────────────────────────────────
// ShortcutAction
// ──────────────────────────────────────────────

/// Every user-triggerable editor action that can be bound to a keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    // ── File ───────────────────────────────────
    NewProject,
    OpenProject,
    Save,
    SaveAs,
    Quit,
    // ── Edit ───────────────────────────────────
    Undo,
    Redo,
    Delete,
    Duplicate,
    SelectAll,
    Deselect,
    // ── View ───────────────────────────────────
    ToggleFullscreen,
    ResetLayout,
    FocusSelected,
    // ── Tools ──────────────────────────────────
    GizmoTranslate,
    GizmoRotate,
    GizmoScale,
    ToggleSnap,
    ToggleGrid,
    PlayStop,
    // ── Navigation ─────────────────────────────
    FrameAll,
    FrameSelected,
}

impl fmt::Display for ShortcutAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::label_for_action(self))
    }
}

impl ShortcutAction {
    /// Human-readable label for this action (e.g. "New Project", "Toggle Grid").
    pub fn label_for_action(action: &ShortcutAction) -> &'static str {
        match action {
            ShortcutAction::NewProject => "New Project",
            ShortcutAction::OpenProject => "Open Project",
            ShortcutAction::Save => "Save",
            ShortcutAction::SaveAs => "Save As",
            ShortcutAction::Quit => "Quit",
            ShortcutAction::Undo => "Undo",
            ShortcutAction::Redo => "Redo",
            ShortcutAction::Delete => "Delete",
            ShortcutAction::Duplicate => "Duplicate",
            ShortcutAction::SelectAll => "Select All",
            ShortcutAction::Deselect => "Deselect",
            ShortcutAction::ToggleFullscreen => "Toggle Fullscreen",
            ShortcutAction::ResetLayout => "Reset Layout",
            ShortcutAction::FocusSelected => "Focus Selected",
            ShortcutAction::GizmoTranslate => "Gizmo Translate",
            ShortcutAction::GizmoRotate => "Gizmo Rotate",
            ShortcutAction::GizmoScale => "Gizmo Scale",
            ShortcutAction::ToggleSnap => "Toggle Snap",
            ShortcutAction::ToggleGrid => "Toggle Grid",
            ShortcutAction::PlayStop => "Play / Stop",
            ShortcutAction::FrameAll => "Frame All",
            ShortcutAction::FrameSelected => "Frame Selected",
        }
    }
}

// ──────────────────────────────────────────────
// KeyBinding
// ──────────────────────────────────────────────

/// A key + modifier combination that can trigger a [`ShortcutAction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: egui::Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    /// Plain key with no modifiers.
    pub fn key(key: egui::Key) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    /// Ctrl + key.
    pub fn ctrl(key: egui::Key) -> Self {
        Self {
            key,
            ctrl: true,
            shift: false,
            alt: false,
        }
    }

    /// Ctrl + Shift + key.
    pub fn ctrl_shift(key: egui::Key) -> Self {
        Self {
            key,
            ctrl: true,
            shift: true,
            alt: false,
        }
    }

    /// Shift + key (no Ctrl).
    pub fn shift(key: egui::Key) -> Self {
        Self {
            key,
            ctrl: false,
            shift: true,
            alt: false,
        }
    }

    /// Alt + key.
    pub fn alt(key: egui::Key) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: true,
        }
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&Self::label_for_binding(self))
    }
}

impl KeyBinding {
    /// Format a binding as a human-readable string (e.g. "Ctrl+S", "Shift+A", "W").
    pub fn label_for_binding(binding: &KeyBinding) -> String {
        let mut parts: Vec<&'static str> = Vec::with_capacity(4);
        if binding.ctrl {
            parts.push("Ctrl");
        }
        if binding.alt {
            parts.push("Alt");
        }
        if binding.shift {
            parts.push("Shift");
        }
        parts.push(key_name(binding.key));
        parts.join("+")
    }
}

/// Map an `egui::Key` to a stable display name.
fn key_name(key: egui::Key) -> &'static str {
    match key {
        egui::Key::A => "A",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::F => "F",
        egui::Key::G => "G",
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::Q => "Q",
        egui::Key::R => "R",
        egui::Key::S => "S",
        egui::Key::W => "W",
        egui::Key::Y => "Y",
        egui::Key::Z => "Z",
        egui::Key::Escape => "Esc",
        egui::Key::Delete => "Del",
        egui::Key::Home => "Home",
        egui::Key::F1 => "F1",
        egui::Key::F2 => "F2",
        egui::Key::F3 => "F3",
        egui::Key::F4 => "F4",
        egui::Key::F5 => "F5",
        egui::Key::F6 => "F6",
        egui::Key::F7 => "F7",
        egui::Key::F8 => "F8",
        egui::Key::F9 => "F9",
        egui::Key::F10 => "F10",
        egui::Key::F11 => "F11",
        egui::Key::F12 => "F12",
        _ => "???",
    }
}

// ──────────────────────────────────────────────
// ShortcutMap
// ──────────────────────────────────────────────

/// Bidirectional lookup between [`ShortcutAction`] and [`KeyBinding`].
///
/// Defaults to Blender-style keybindings. Bindings can be rebound or removed
/// at runtime for user-customisable shortcut schemes.
pub struct ShortcutMap {
    bindings: HashMap<ShortcutAction, KeyBinding>,
}

impl ShortcutMap {
    /// Create a new map populated with Blender-style defaults.
    pub fn new() -> Self {
        Self::blender_defaults()
    }

    /// Build a `ShortcutMap` with Blender-style default keybindings.
    pub fn blender_defaults() -> Self {
        use egui::Key as K;
        use ShortcutAction as A;

        let mut map = ShortcutMap {
            bindings: HashMap::new(),
        };

        // ── File ───────────────────────────────
        map.bind(A::NewProject, KeyBinding::ctrl(K::N));
        map.bind(A::OpenProject, KeyBinding::ctrl(K::O));
        map.bind(A::Save, KeyBinding::ctrl(K::S));
        map.bind(A::SaveAs, KeyBinding::ctrl_shift(K::S));
        map.bind(A::Quit, KeyBinding::ctrl(K::Q));

        // ── Edit ───────────────────────────────
        map.bind(A::Undo, KeyBinding::ctrl(K::Z));
        map.bind(A::Redo, KeyBinding::ctrl(K::Y));
        map.bind(A::Delete, KeyBinding::key(K::Delete));
        map.bind(A::Duplicate, KeyBinding::ctrl(K::D));
        map.bind(A::SelectAll, KeyBinding::ctrl(K::A));
        map.bind(A::Deselect, KeyBinding::key(K::Escape));

        // ── View ───────────────────────────────
        map.bind(A::ToggleFullscreen, KeyBinding::key(K::F11));
        map.bind(A::ResetLayout, KeyBinding::key(K::Home));
        map.bind(A::FocusSelected, KeyBinding::key(K::F));

        // ── Tools ──────────────────────────────
        map.bind(A::GizmoTranslate, KeyBinding::key(K::W));
        map.bind(A::GizmoRotate, KeyBinding::key(K::E));
        map.bind(A::GizmoScale, KeyBinding::key(K::R));
        map.bind(A::ToggleSnap, KeyBinding::key(K::G));
        map.bind(A::ToggleGrid, KeyBinding::alt(K::G));
        map.bind(A::PlayStop, KeyBinding::key(K::F5));

        // ── Navigation ─────────────────────────
        map.bind(A::FrameAll, KeyBinding::key(K::A));
        map.bind(A::FrameSelected, KeyBinding::shift(K::F));

        map
    }

    /// Bind `action` to `binding`, replacing any existing binding for that action.
    pub fn bind(&mut self, action: ShortcutAction, binding: KeyBinding) {
        self.bindings.insert(action, binding);
    }

    /// Remove the binding for `action` (if any).
    pub fn unbind(&mut self, action: ShortcutAction) {
        self.bindings.remove(&action);
    }

    /// Look up the binding for `action`.
    pub fn get(&self, action: &ShortcutAction) -> Option<&KeyBinding> {
        self.bindings.get(action)
    }

    /// Reverse lookup: given a raw key press state, return the matching action.
    pub fn find_action(
        &self,
        key: egui::Key,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> Option<ShortcutAction> {
        self.bindings
            .iter()
            .find(|(_, b)| b.key == key && b.ctrl == ctrl && b.shift == shift && b.alt == alt)
            .map(|(action, _)| *action)
    }

    /// Read access to the full bindings map (for UI rendering / serialisation).
    pub fn all_bindings(&self) -> &HashMap<ShortcutAction, KeyBinding> {
        &self.bindings
    }
}

impl Default for ShortcutMap {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key as K;
    use ShortcutAction as A;

    #[test]
    fn new_has_blender_defaults() {
        let map = ShortcutMap::new();
        // Spot-check a few critical bindings.
        assert_eq!(
            map.get(&A::Save),
            Some(&KeyBinding::ctrl(K::S))
        );
        assert_eq!(
            map.get(&A::Undo),
            Some(&KeyBinding::ctrl(K::Z))
        );
        assert_eq!(
            map.get(&A::GizmoTranslate),
            Some(&KeyBinding::key(K::W))
        );
    }

    #[test]
    fn bind_and_get_roundtrip() {
        let mut map = ShortcutMap::new();
        map.bind(A::PlayStop, KeyBinding::ctrl(K::P));
        assert_eq!(map.get(&A::PlayStop), Some(&KeyBinding::ctrl(K::P)));
    }

    #[test]
    fn unbind_removes_binding() {
        let mut map = ShortcutMap::new();
        assert!(map.get(&A::Save).is_some());
        map.unbind(A::Save);
        assert!(map.get(&A::Save).is_none());
    }

    #[test]
    fn find_action_with_modifiers() {
        let map = ShortcutMap::new();
        // Ctrl+Shift+S → SaveAs
        assert_eq!(map.find_action(K::S, true, true, false), Some(A::SaveAs));
        // Ctrl+S → Save (not SaveAs)
        assert_eq!(map.find_action(K::S, true, false, false), Some(A::Save));
        // Ctrl+D → Duplicate
        assert_eq!(map.find_action(K::D, true, false, false), Some(A::Duplicate));
    }

    #[test]
    fn find_action_returns_none_for_unmapped() {
        let map = ShortcutMap::new();
        // Ctrl+K is not bound to anything by default.
        assert_eq!(map.find_action(K::K, true, false, false), None);
        // Ctrl+Alt+X is not bound.
        assert_eq!(map.find_action(K::X, true, false, true), None);
    }

    #[test]
    fn label_for_action_format() {
        assert_eq!(ShortcutAction::label_for_action(&A::NewProject), "New Project");
        assert_eq!(ShortcutAction::label_for_action(&A::ToggleGrid), "Toggle Grid");
        assert_eq!(ShortcutAction::label_for_action(&A::PlayStop), "Play / Stop");
    }

    #[test]
    fn label_for_binding_format() {
        assert_eq!(
            KeyBinding::label_for_binding(&KeyBinding::ctrl(K::S)),
            "Ctrl+S"
        );
        assert_eq!(
            KeyBinding::label_for_binding(&KeyBinding::shift(K::A)),
            "Shift+A"
        );
        assert_eq!(
            KeyBinding::label_for_binding(&KeyBinding::key(K::W)),
            "W"
        );
        assert_eq!(
            KeyBinding::label_for_binding(&KeyBinding::ctrl_shift(K::S)),
            "Ctrl+Shift+S"
        );
        assert_eq!(
            KeyBinding::label_for_binding(&KeyBinding::alt(K::G)),
            "Alt+G"
        );
    }

    #[test]
    fn blender_defaults_all_actions_have_bindings() {
        let map = ShortcutMap::blender_defaults();
        let all_actions: &[ShortcutAction] = &[
            A::NewProject,
            A::OpenProject,
            A::Save,
            A::SaveAs,
            A::Quit,
            A::Undo,
            A::Redo,
            A::Delete,
            A::Duplicate,
            A::SelectAll,
            A::Deselect,
            A::ToggleFullscreen,
            A::ResetLayout,
            A::FocusSelected,
            A::GizmoTranslate,
            A::GizmoRotate,
            A::GizmoScale,
            A::ToggleSnap,
            A::ToggleGrid,
            A::PlayStop,
            A::FrameAll,
            A::FrameSelected,
        ];
        for action in all_actions {
            assert!(
                map.get(action).is_some(),
                "Blender defaults missing binding for: {action}"
            );
        }
    }

    #[test]
    fn display_trait_delegates_correctly() {
        assert_eq!(format!("{}", A::Save), "Save");
        assert_eq!(format!("{}", KeyBinding::ctrl(K::S)), "Ctrl+S");
        assert_eq!(format!("{}", KeyBinding::key(K::F5)), "F5");
    }
}

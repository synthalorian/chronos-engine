//! Editor panels — Phase 7B.
//!
//! All editor panels share a common [`EditorPanel`] trait and operate on
//! shared [`EditorState`]. Each panel lives in its own module.

#![allow(dead_code)]

mod viewport;
mod hierarchy;
mod inspector;
mod asset_browser;
mod console;
mod toolbar;
mod menu_bar;
mod welcome;

pub use viewport::ViewportPanel;
pub use hierarchy::HierarchyPanel;
pub use inspector::InspectorPanel;
pub use asset_browser::AssetBrowserPanel;
pub use console::ConsolePanel;
pub use toolbar::ToolbarPanel;
pub use menu_bar::MenuBarPanel;
pub use welcome::WelcomeScreen;

use crate::entity::Entity;
use crate::world::World;
use std::path::PathBuf;

// ──────────────────────────────────────────────
// Shared types
// ──────────────────────────────────────────────

/// Play mode for the editor toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    /// Editor is stopped — scene is editable.
    Stopped,
    /// Editor is playing — game loop runs.
    Playing,
    /// Editor is paused — game loop frozen.
    Paused,
}

/// Transform gizmo mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// Log severity for the console panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLogLevel {
    Info,
    Warn,
    Error,
}

/// Single console log entry.
#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub level: ConsoleLogLevel,
    pub message: String,
    pub timestamp_secs: f64,
}

/// Shared state accessible by all panels.
///
/// This is the single source of truth for editor-wide data. Panels read from
/// and write to this struct during each frame.
pub struct EditorState {
    /// The ECS world being edited.
    pub world: World,

    /// Currently selected entities (empty = none selected).
    pub selected_entities: Vec<Entity>,

    /// Current play mode.
    pub play_mode: PlayMode,

    /// Current gizmo mode (translate / rotate / scale).
    pub gizmo_mode: GizmoMode,

    /// Whether snap-to-grid is enabled.
    pub snap_enabled: bool,

    /// Grid size for snapping.
    pub snap_size: f32,

    /// Console log entries.
    pub console_log: Vec<ConsoleEntry>,

    /// Console command history (most recent last).
    pub console_history: Vec<String>,

    /// Root directory for asset browsing (None = not project loaded).
    pub project_path: Option<PathBuf>,

    /// Whether the editor should quit.
    pub should_quit: bool,

    /// Whether to show the settings dialog.
    pub show_settings: bool,

    /// Whether to show the about dialog.
    pub show_about: bool,

    /// Bottom panel tab: true = Console, false = Asset Browser.
    pub show_console_tab: bool,

    /// Project manager — owns current project, recent list, dialog flags.
    pub project_manager: crate::editor_project::ProjectManager,
}

impl EditorState {
    /// Create a default editor state with an empty world.
    pub fn new() -> Self {
        Self {
            world: World::new(),
            selected_entities: Vec::new(),
            play_mode: PlayMode::Stopped,
            gizmo_mode: GizmoMode::Translate,
            snap_enabled: false,
            snap_size: 1.0,
            console_log: Vec::new(),
            console_history: Vec::new(),
            project_path: None,
            should_quit: false,
            show_settings: false,
            show_about: false,
            show_console_tab: true,
            project_manager: crate::editor_project::ProjectManager::new(),
        }
    }

    /// Append a console log entry with the current time.
    pub fn log(&mut self, level: ConsoleLogLevel, message: impl Into<String>) {
        self.console_log.push(ConsoleEntry {
            level,
            message: message.into(),
            // Use a simple counter-based timestamp for headless tests.
            // In the editor binary, this gets overwritten with real time.
            timestamp_secs: self.console_log.len() as f64,
        });
    }

    /// Select a single entity (replaces current selection).
    pub fn select(&mut self, entity: Entity) {
        self.selected_entities.clear();
        self.selected_entities.push(entity);
    }

    /// Add an entity to the selection (multi-select).
    pub fn select_add(&mut self, entity: Entity) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
        }
    }

    /// Deselect an entity.
    pub fn deselect(&mut self, entity: Entity) {
        self.selected_entities.retain(|e| *e != entity);
    }

    /// Clear all selection.
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    /// Whether an entity is currently selected.
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }
}

// ──────────────────────────────────────────────
// Panel trait
// ──────────────────────────────────────────────

/// Trait for editor panels.
///
/// Each panel implements this trait. The editor calls `show` every frame
/// with the current `egui::Ui` and mutable reference to [`EditorState`].
pub trait EditorPanel {
    /// Human-readable panel title (used for tab labels, window titles).
    fn title(&self) -> &str;

    /// Render the panel into the given `egui::Ui`.
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState);
}

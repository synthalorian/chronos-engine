//! Editor workspace modules — Phase 7C + 7E.
//!
//! Workspace tools: undo/redo, grid, gizmos, selection, shortcuts, settings, docking.

#![allow(dead_code)]

mod docking;
mod gizmo;
mod grid;
mod selection;
mod settings;
mod shortcuts;
mod undo;

pub use docking::{
    DockError, DockLayout, DockNode, DockState, DockZone, DragState, SplitDirection,
};
pub use gizmo::GizmoSystem;
pub use grid::GridRenderer;
pub use selection::ViewportSelector;
pub use settings::SettingsDialog;
pub use shortcuts::{KeyBinding, ShortcutAction, ShortcutMap};
pub use undo::{EditorCommand, UndoAction, UndoStack};

use crate::entity::Entity;

/// Snap a world-space value to the nearest grid point.
pub fn snap_to_grid(value: f32, grid_size: f32) -> f32 {
    if grid_size <= 0.0 {
        return value;
    }
    (value / grid_size).round() * grid_size
}

/// Result of a viewport pick operation.
#[derive(Debug, Clone)]
pub struct PickResult {
    pub entity: Entity,
    pub distance: f32,
    pub world_pos: [f32; 3],
}

/// Rectangle for box selection (screen-space).
#[derive(Debug, Clone, Copy)]
pub struct SelectionRect {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl SelectionRect {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x_min: x1.min(x2),
            y_min: y1.min(y2),
            x_max: x1.max(x2),
            y_max: y1.max(y2),
        }
    }

    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }
}

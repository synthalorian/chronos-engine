//! Dockable panel system for the Chronos Engine editor.
//!
//! Implements a tree-based docking layout that replaces the fixed panel
//! arrangement with a flexible, user-customizable system. Panels (hierarchy,
//! viewport, inspector, console, asset browser) live inside a binary tree of
//! splits and tabs that can be rearranged at runtime.
//!
//! # Architecture
//!
//! ```text
//! [DockLayout]
//!   └── root: DockNode            ← binary tree
//!         ├── Empty               ← placeholder leaf
//!         ├── Leaf { panel_id }   ← single panel
//!         ├── Split { direction, ratio, children }
//!         │     ├── left/top child
//!         │     └── right/bottom child
//!         └── Tabbed { tabs, active }  ← multiple panels sharing space
//!
//! [DockState]
//!   ├── layout: DockLayout
//!   ├── panel_sizes: HashMap<String, f32>
//!   └── drag_state: Option<DragState>
//! ```
//!
//! The default layout mirrors the current fixed layout:
//! left hierarchy | center viewport | right inspector, with a tabbed
//! bottom panel for console and asset browser.
//!
//! # Integration
//!
//! This module provides the data model and egui rendering infrastructure.
//! Integration with `editor_app.rs` happens separately — the `render`
//! method produces the UI regions that panel content can fill.

use std::collections::HashMap;
use std::fmt;

// ── SplitDirection ───────────────────────────────────────────────

/// Direction of a split node in the dock tree.
///
/// `Horizontal` places children side by side (left | right).
/// `Vertical` stacks children top-to-bottom (top / bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitDirection {
    /// Children arranged left-to-right.
    Horizontal,
    /// Children arranged top-to-bottom.
    Vertical,
}

impl SplitDirection {
    /// Returns the opposite direction.
    pub fn invert(self) -> Self {
        match self {
            SplitDirection::Horizontal => SplitDirection::Vertical,
            SplitDirection::Vertical => SplitDirection::Horizontal,
        }
    }
}

impl fmt::Display for SplitDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitDirection::Horizontal => write!(f, "horizontal"),
            SplitDirection::Vertical => write!(f, "vertical"),
        }
    }
}

// ── DockNode ─────────────────────────────────────────────────────

/// A node in the dock layout tree.
///
/// The tree is binary: splits have exactly two children. Leaf nodes hold a
/// single panel. Tabbed nodes hold multiple panels sharing the same space
/// with a tab bar to switch between them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum DockNode {
    /// Empty placeholder — occupies space but shows nothing.
    Empty,
    /// Leaf node containing a single panel.
    Leaf {
        /// Unique identifier for the panel (e.g. "hierarchy", "viewport").
        panel_id: String,
        /// Proportion of the parent split this node should occupy (0.0–1.0).
        size_ratio: f32,
    },
    /// Split node dividing space between two children.
    Split {
        /// Direction of the split.
        direction: SplitDirection,
        /// Ratio of space allocated to the first child (0.0–1.0).
        ratio: f32,
        /// The two children: (first, second).
        children: Box<(DockNode, DockNode)>,
    },
    /// Tabbed container — multiple panels sharing one region.
    Tabbed {
        /// Panel IDs of all tabs in this container.
        tabs: Vec<String>,
        /// Index of the currently active (visible) tab.
        active: usize,
        /// Proportion of the parent split this node should occupy (0.0–1.0).
        size_ratio: f32,
    },
}

impl DockNode {
    /// Create a new leaf node with the given panel ID and equal sizing.
    pub fn leaf(panel_id: impl Into<String>) -> Self {
        DockNode::Leaf {
            panel_id: panel_id.into(),
            size_ratio: 1.0,
        }
    }

    /// Create a horizontal split with the given ratio (left, right).
    pub fn h_split(ratio: f32, left: DockNode, right: DockNode) -> Self {
        DockNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: ratio.clamp(0.01, 0.99),
            children: Box::new((left, right)),
        }
    }

    /// Create a vertical split with the given ratio (top, bottom).
    pub fn v_split(ratio: f32, top: DockNode, bottom: DockNode) -> Self {
        DockNode::Split {
            direction: SplitDirection::Vertical,
            ratio: ratio.clamp(0.01, 0.99),
            children: Box::new((top, bottom)),
        }
    }

    /// Create a tabbed container with the given panel IDs.
    pub fn tabbed(tabs: Vec<String>) -> Self {
        let active = 0;
        DockNode::Tabbed {
            tabs,
            active,
            size_ratio: 1.0,
        }
    }

    /// Collect all panel IDs contained within this node (depth-first).
    pub fn panel_ids(&self) -> Vec<String> {
        match self {
            DockNode::Empty => Vec::new(),
            DockNode::Leaf { panel_id, .. } => vec![panel_id.clone()],
            DockNode::Split { children, .. } => {
                let mut ids = children.0.panel_ids();
                ids.extend(children.1.panel_ids());
                ids
            }
            DockNode::Tabbed { tabs, .. } => tabs.clone(),
        }
    }

    /// Returns true if this node contains the given panel ID.
    pub fn contains_panel(&self, panel_id: &str) -> bool {
        match self {
            DockNode::Empty => false,
            DockNode::Leaf { panel_id: id, .. } => id == panel_id,
            DockNode::Split { children, .. } => {
                children.0.contains_panel(panel_id) || children.1.contains_panel(panel_id)
            }
            DockNode::Tabbed { tabs, .. } => tabs.iter().any(|t| t == panel_id),
        }
    }

    /// Count the total number of panels in this subtree.
    pub fn panel_count(&self) -> usize {
        match self {
            DockNode::Empty => 0,
            DockNode::Leaf { .. } => 1,
            DockNode::Split { children, .. } => {
                children.0.panel_count() + children.1.panel_count()
            }
            DockNode::Tabbed { tabs, .. } => tabs.len(),
        }
    }

    /// Normalize size_ratio fields to the [0.0, 1.0] range in-place.
    pub fn normalize_ratios(&mut self) {
        match self {
            DockNode::Empty => {}
            DockNode::Leaf { size_ratio, .. } => {
                *size_ratio = size_ratio.clamp(0.0, 1.0);
            }
            DockNode::Split {
                ratio,
                children,
                ..
            } => {
                *ratio = ratio.clamp(0.01, 0.99);
                children.0.normalize_ratios();
                children.1.normalize_ratios();
            }
            DockNode::Tabbed { size_ratio, .. } => {
                *size_ratio = size_ratio.clamp(0.0, 1.0);
            }
        }
    }
}

// ── DockNodePath ─────────────────────────────────────────────────

/// Path from the root of the dock tree to a specific node.
///
/// Each `usize` selects a child: `0` for the first child of a split,
/// `1` for the second child, or an index into a tabbed container's tabs.
pub type DockNodePath = Vec<usize>;

// ── DockError ────────────────────────────────────────────────────

/// Errors that can occur during dock layout operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DockError {
    /// The layout structure is invalid (e.g. empty tree, orphaned nodes).
    InvalidLayout(String),
    /// Failed to serialize or deserialize the layout.
    SerializationError(String),
    /// The specified panel ID was not found in the layout.
    PanelNotFound(String),
}

impl fmt::Display for DockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockError::InvalidLayout(msg) => write!(f, "invalid layout: {}", msg),
            DockError::SerializationError(msg) => write!(f, "serialization error: {}", msg),
            DockError::PanelNotFound(id) => write!(f, "panel not found: {}", id),
        }
    }
}

impl std::error::Error for DockError {}

// ── DockZone ─────────────────────────────────────────────────────

/// Zone within a dock region where a dragged panel can be dropped.
///
/// When the user drags a panel over an existing panel, five drop zones
/// appear around the edges and center. Dropping on `Center` tabs the
/// panel alongside the existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockZone {
    /// Dock to the left of the target region.
    Left,
    /// Dock to the right of the target region.
    Right,
    /// Dock above the target region.
    Top,
    /// Dock below the target region.
    Bottom,
    /// Tab into the same region as the target.
    Center,
}

impl DockZone {
    /// Returns the split direction for this zone.
    ///
    /// `Left`/`Right` → `Horizontal`, `Top`/`Bottom` → `Vertical`.
    /// `Center` returns `None` (it tabs, not splits).
    pub fn split_direction(self) -> Option<SplitDirection> {
        match self {
            DockZone::Left | DockZone::Right => Some(SplitDirection::Horizontal),
            DockZone::Top | DockZone::Bottom => Some(SplitDirection::Vertical),
            DockZone::Center => None,
        }
    }

    /// Returns true if this zone places the new panel as the first child.
    pub fn is_first_child(self) -> bool {
        matches!(self, DockZone::Left | DockZone::Top)
    }
}

// ── DragState ────────────────────────────────────────────────────

/// Tracks an in-progress panel drag operation.
///
/// Created when the user begins dragging a panel tab or title bar.
/// Updated as the cursor moves over potential drop targets. Consumed
/// when the panel is dropped.
#[derive(Debug, Clone)]
pub struct DragState {
    /// The panel being dragged.
    pub panel_id: String,
    /// Path to the node where the drag originated.
    pub source_path: DockNodePath,
    /// The drop zone currently under the cursor, if any.
    pub preview_zone: Option<DockZone>,
}

impl DragState {
    /// Create a new drag state for the given panel and source path.
    pub fn new(panel_id: String, source_path: DockNodePath) -> Self {
        Self {
            panel_id,
            source_path,
            preview_zone: None,
        }
    }

    /// Update the preview zone based on cursor position.
    pub fn set_preview_zone(&mut self, zone: Option<DockZone>) {
        self.preview_zone = zone;
    }

    /// Returns true if a drop zone preview is active.
    pub fn has_preview(&self) -> bool {
        self.preview_zone.is_some()
    }
}

// ── DockLayout ───────────────────────────────────────────────────

/// The tree structure describing how panels are arranged.
///
/// The root node contains the entire layout. Operations like finding,
/// adding, removing, and moving panels walk the tree recursively.
/// Layouts can be serialized to JSON for persistence across sessions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct DockLayout {
    /// Root of the dock tree.
    pub root: DockNode,
}

// ── Free functions for tree mutation ─────────────────────────────

/// Remove a panel from a node subtree, simplifying the tree afterward.
///
/// Returns `true` if the panel was found and removed. After removal,
/// splits with an `Empty` child are collapsed (the non-empty child
/// replaces the split).
fn remove_panel_from_node(node: &mut DockNode, panel_id: &str) -> bool {
    match node {
        DockNode::Empty => false,
        DockNode::Leaf { panel_id: id, .. } => {
            if id == panel_id {
                *node = DockNode::Empty;
                true
            } else {
                false
            }
        }
        DockNode::Split { children, .. } => {
            if remove_panel_from_node(&mut children.0, panel_id) {
                simplify_split_node(node);
                return true;
            }
            if remove_panel_from_node(&mut children.1, panel_id) {
                simplify_split_node(node);
                return true;
            }
            false
        }
        DockNode::Tabbed { tabs, active, .. } => {
            let idx = tabs.iter().position(|t| t == panel_id);
            if let Some(i) = idx {
                tabs.remove(i);
                if tabs.is_empty() {
                    *node = DockNode::Empty;
                } else if *active >= tabs.len() {
                    *active = tabs.len().saturating_sub(1);
                } else if i < *active {
                    *active = (*active).saturating_sub(1);
                }
                true
            } else {
                false
            }
        }
    }
}

/// After a removal, simplify a split node that has an `Empty` child.
///
/// If both children are empty, the split becomes `Empty`. If one child
/// is empty, the other child replaces the split entirely.
fn simplify_split_node(node: &mut DockNode) {
    if let DockNode::Split { children, .. } = node {
        let first_empty = matches!(children.0, DockNode::Empty);
        let second_empty = matches!(children.1, DockNode::Empty);
        if first_empty && second_empty {
            *node = DockNode::Empty;
        } else if first_empty {
            let replacement = std::mem::replace(&mut children.1, DockNode::Empty);
            *node = replacement;
        } else if second_empty {
            let replacement = std::mem::replace(&mut children.0, DockNode::Empty);
            *node = replacement;
        }
    }
}

// ── DockLayout ───────────────────────────────────────────────────

impl DockLayout {
    /// Create a new layout with the default editor arrangement.
    ///
    /// The default layout matches the current fixed panel layout:
    /// ```text
    /// ┌────────────┬──────────────────────┬───────────────┐
    /// │            │                      │               │
    /// │ hierarchy  │      viewport        │  inspector    │
    /// │            │                      │               │
    /// ├────────────┴──────────────────────┴───────────────┤
    /// │  [Console] [Asset Browser]                        │
    /// │  (tabbed)                                         │
    /// └───────────────────────────────────────────────────┘
    /// ```
    pub fn new() -> Self {
        Self::default_layout()
    }

    /// Returns the standard editor layout.
    pub fn default_layout() -> Self {
        // Top row: hierarchy | viewport | inspector (horizontal 3-way split)
        let top = DockNode::h_split(
            0.20,
            DockNode::leaf("hierarchy"),
            DockNode::h_split(
                0.75,
                DockNode::leaf("viewport"),
                DockNode::leaf("inspector"),
            ),
        );

        // Bottom row: tabbed console / asset browser
        let bottom = DockNode::tabbed(vec![
            "console".into(),
            "asset_browser".into(),
        ]);

        // Root: vertical split — top area 75%, bottom 25%
        let root = DockNode::v_split(0.75, top, bottom);

        Self { root }
    }

    /// Find the path to a panel by its ID.
    ///
    /// Returns the `DockNodePath` (sequence of child indices from root)
    /// leading to the node that contains the panel. For `Tabbed` nodes,
    /// the last index in the path is the tab index.
    pub fn find_panel(&self, panel_id: &str) -> Option<DockNodePath> {
        Self::find_panel_in(&self.root, panel_id, &mut Vec::new())
    }

    /// Recursive helper for `find_panel`.
    fn find_panel_in(
        node: &DockNode,
        panel_id: &str,
        path: &mut DockNodePath,
    ) -> Option<DockNodePath> {
        match node {
            DockNode::Empty => None,
            DockNode::Leaf { panel_id: id, .. } => {
                if id == panel_id {
                    Some(path.clone())
                } else {
                    None
                }
            }
            DockNode::Split { children, .. } => {
                path.push(0);
                if let Some(p) = Self::find_panel_in(&children.0, panel_id, path) {
                    path.pop();
                    return Some(p);
                }
                path.pop();
                path.push(1);
                if let Some(p) = Self::find_panel_in(&children.1, panel_id, path) {
                    path.pop();
                    return Some(p);
                }
                path.pop();
                None
            }
            DockNode::Tabbed { tabs, .. } => {
                for (i, tab) in tabs.iter().enumerate() {
                    if tab == panel_id {
                        let mut result = path.clone();
                        result.push(i);
                        return Some(result);
                    }
                }
                None
            }
        }
    }

    /// Remove a panel from the layout by its ID.
    ///
    /// Returns `true` if the panel was found and removed. The tree is
    /// simplified after removal: if a split has only one remaining child,
    /// the split is replaced by that child.
    pub fn remove_panel(&mut self, panel_id: &str) -> bool {
        remove_panel_from_node(&mut self.root, panel_id)
    }

    /// Add a panel as a new split at the given parent path.
    ///
    /// The existing node at `parent_path` becomes one child of a new
    /// split, and the new panel becomes the other. `slot` determines
    /// the split direction. The new panel is placed as the second child.
    pub fn add_panel(
        &mut self,
        parent_path: &[usize],
        slot: SplitDirection,
        panel_id: String,
    ) {
        if let Some(target) = self.get_node_mut(parent_path) {
            let old = std::mem::replace(target, DockNode::Empty);
            *target = DockNode::Split {
                direction: slot,
                ratio: 0.5,
                children: Box::new((old, DockNode::leaf(panel_id))),
            };
        }
    }

    /// Move a panel from its current location to a new position.
    ///
    /// The panel is removed from its current node and inserted as a new
    /// split at `to_path` with the given target slot direction.
    pub fn move_panel(
        &mut self,
        from: &str,
        to_path: &[usize],
        target_slot: SplitDirection,
    ) {
        // Remove from current location.
        if !self.remove_panel(from) {
            return;
        }
        // Add at target.
        self.add_panel(to_path, target_slot, from.to_string());
    }

    /// Get a mutable reference to the node at the given path.
    fn get_node_mut(&mut self, path: &[usize]) -> Option<&mut DockNode> {
        let mut current = &mut self.root;
        for &idx in path {
            match current {
                DockNode::Split { children, .. } => {
                    current = if idx == 0 {
                        &mut children.0
                    } else {
                        &mut children.1
                    };
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Get a reference to the node at the given path.
    fn get_node(&self, path: &[usize]) -> Option<&DockNode> {
        let mut current = &self.root;
        for &idx in path {
            match current {
                DockNode::Split { children, .. } => {
                    current = if idx == 0 {
                        &children.0
                    } else {
                        &children.1
                    };
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Returns the active panel in a tabbed node identified by any tab ID.
    ///
    /// If the panel lives in a `Tabbed` node, returns the currently active
    /// tab's panel ID. If the panel is in a `Leaf`, returns that panel's ID.
    pub fn active_panel_in_tab(&self, tab_id: &str) -> Option<String> {
        self.find_active_panel_in_node(&self.root, tab_id)
    }

    /// Recursive helper: walk the tree to find which Tabbed or Leaf
    /// container holds `tab_id`, then return the active panel.
    fn find_active_panel_in_node(&self, node: &DockNode, tab_id: &str) -> Option<String> {
        match node {
            DockNode::Empty => None,
            DockNode::Leaf { panel_id, .. } => {
                if panel_id == tab_id {
                    Some(panel_id.clone())
                } else {
                    None
                }
            }
            DockNode::Split { children, .. } => {
                self.find_active_panel_in_node(&children.0, tab_id)
                    .or_else(|| self.find_active_panel_in_node(&children.1, tab_id))
            }
            DockNode::Tabbed { tabs, active, .. } => {
                if tabs.iter().any(|t| t == tab_id) {
                    tabs.get(*active).cloned()
                } else {
                    None
                }
            }
        }
    }

    /// Serialize the layout to a JSON string.
    #[cfg(feature = "serialize")]
    pub fn serialize(&self) -> Result<String, DockError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| DockError::SerializationError(e.to_string()))
    }

    /// Serialize the layout to a JSON string (no-op without serialize feature).
    #[cfg(not(feature = "serialize"))]
    pub fn serialize(&self) -> Result<String, DockError> {
        Err(DockError::SerializationError(
            "serialize feature not enabled".into(),
        ))
    }

    /// Deserialize a layout from a JSON string.
    #[cfg(feature = "serialize")]
    pub fn deserialize(json: &str) -> Result<Self, DockError> {
        serde_json::from_str(json)
            .map_err(|e| DockError::SerializationError(e.to_string()))
    }

    /// Deserialize a layout from a JSON string (no-op without serialize feature).
    #[cfg(not(feature = "serialize"))]
    pub fn deserialize(_json: &str) -> Result<Self, DockError> {
        Err(DockError::SerializationError(
            "serialize feature not enabled".into(),
        ))
    }

    /// Collect all panel IDs in the layout.
    pub fn all_panel_ids(&self) -> Vec<String> {
        self.root.panel_ids()
    }

    /// Check if a panel exists in the layout.
    pub fn has_panel(&self, panel_id: &str) -> bool {
        self.root.contains_panel(panel_id)
    }
}

impl Default for DockLayout {
    fn default() -> Self {
        Self::default_layout()
    }
}

// ── DockState ────────────────────────────────────────────────────

/// Runtime state manager for the docking system.
///
/// Owns the layout tree, tracks panel sizes, and manages drag-and-drop
/// operations. The `render` method walks the tree and produces egui
/// regions for each panel.
pub struct DockState {
    /// The current dock layout tree.
    pub layout: DockLayout,
    /// Cached pixel sizes for each panel (updated during render).
    pub panel_sizes: HashMap<String, (f32, f32)>,
    /// Active drag operation, if any.
    pub drag_state: Option<DragState>,
    /// Which panels are currently visible.
    panel_visibility: HashMap<String, bool>,
}

impl DockState {
    /// Create a new dock state with the default layout.
    pub fn new() -> Self {
        let layout = DockLayout::default_layout();
        let mut visibility = HashMap::new();
        for id in layout.all_panel_ids() {
            visibility.insert(id, true);
        }
        Self {
            layout,
            panel_sizes: HashMap::new(),
            drag_state: None,
            panel_visibility: visibility,
        }
    }

    /// Create a dock state with a custom layout.
    pub fn with_layout(layout: DockLayout) -> Self {
        let mut visibility = HashMap::new();
        for id in layout.all_panel_ids() {
            visibility.insert(id, true);
        }
        Self {
            layout,
            panel_sizes: HashMap::new(),
            drag_state: None,
            panel_visibility: visibility,
        }
    }

    /// Render the dock layout using egui.
    ///
    /// Walks the tree and allocates rectangles for each panel. Panel
    /// content is not rendered here — this only sets up the regions.
    /// Returns a map of panel_id → allocated rect for external rendering.
    pub fn render(&mut self, ctx: &egui::Context) -> HashMap<String, egui::Rect> {
        let mut rects = HashMap::new();
        let screen = ctx.available_rect();
        if screen.width() <= 0.0 || screen.height() <= 0.0 {
            return rects;
        }
        self.render_node(ctx, &self.layout.root.clone(), screen, &mut rects);
        rects
    }

    /// Recursive node renderer.
    fn render_node(
        &mut self,
        ctx: &egui::Context,
        node: &DockNode,
        rect: egui::Rect,
        rects: &mut HashMap<String, egui::Rect>,
    ) {
        match node {
            DockNode::Empty => {
                // Draw empty area with subtle background.
                let painter = ctx.layer_painter(egui::LayerId::background());
                painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
            }
            DockNode::Leaf { panel_id, .. } => {
                if self.is_panel_visible(panel_id) {
                    rects.insert(panel_id.clone(), rect);
                    self.panel_sizes
                        .insert(panel_id.clone(), (rect.width(), rect.height()));
                } else {
                    // Hidden panel renders as empty space.
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
                }
            }
            DockNode::Split {
                direction,
                ratio,
                children,
            } => {
                let r = ratio.clamp(0.05, 0.95);
                let (first_rect, second_rect) = match direction {
                    SplitDirection::Horizontal => {
                        let split_x = rect.left() + rect.width() * r;
                        let sep = 2.0;
                        let first = egui::Rect::from_min_max(
                            rect.left_top(),
                            egui::pos2(split_x - sep / 2.0, rect.bottom()),
                        );
                        let second = egui::Rect::from_min_max(
                            egui::pos2(split_x + sep / 2.0, rect.top()),
                            rect.right_bottom(),
                        );
                        (first, second)
                    }
                    SplitDirection::Vertical => {
                        let split_y = rect.top() + rect.height() * r;
                        let sep = 2.0;
                        let first = egui::Rect::from_min_max(
                            rect.left_top(),
                            egui::pos2(rect.right(), split_y - sep / 2.0),
                        );
                        let second = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), split_y + sep / 2.0),
                            rect.right_bottom(),
                        );
                        (first, second)
                    }
                };

                // Draw separator line.
                let painter = ctx.layer_painter(egui::LayerId::background());
                match direction {
                    SplitDirection::Horizontal => {
                        let split_x = rect.left() + rect.width() * r;
                        painter.line_segment(
                            [
                                egui::pos2(split_x, rect.top()),
                                egui::pos2(split_x, rect.bottom()),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 60, 60)),
                        );
                    }
                    SplitDirection::Vertical => {
                        let split_y = rect.top() + rect.height() * r;
                        painter.line_segment(
                            [
                                egui::pos2(rect.left(), split_y),
                                egui::pos2(rect.right(), split_y),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 60, 60)),
                        );
                    }
                }

                self.render_node(ctx, &children.0, first_rect, rects);
                self.render_node(ctx, &children.1, second_rect, rects);
            }
            DockNode::Tabbed { tabs, active, .. } => {
                if tabs.is_empty() {
                    return;
                }

                // Tab bar height.
                let tab_height = 24.0;
                let tab_bar_rect = egui::Rect::from_min_max(
                    rect.left_top(),
                    egui::pos2(rect.right(), rect.top() + tab_height),
                );
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.top() + tab_height),
                    rect.right_bottom(),
                );

                // Draw tab bar background.
                let painter = ctx.layer_painter(egui::LayerId::background());
                painter.rect_filled(
                    tab_bar_rect,
                    0.0,
                    egui::Color32::from_rgb(40, 40, 40),
                );

                // Draw tabs.
                let mut tab_x = rect.left();
                for (i, tab_id) in tabs.iter().enumerate() {
                    let tab_label = tab_id.replace('_', " ");
                    let galley = ctx.fonts(|f| {
                        f.layout_no_wrap(
                            tab_label,
                            egui::TextStyle::Small.resolve(&ctx.style()),
                            egui::Color32::WHITE,
                        )
                    });
                    let tab_width = galley.size().x + 16.0;
                    let tab_rect = egui::Rect::from_min_max(
                        egui::pos2(tab_x, rect.top()),
                        egui::pos2(tab_x + tab_width, rect.top() + tab_height),
                    );

                    let color = if i == *active {
                        egui::Color32::from_rgb(60, 60, 70)
                    } else {
                        egui::Color32::from_rgb(45, 45, 50)
                    };
                    painter.rect_filled(tab_rect, 0.0, color);

                    // Tab label.
                    let text_pos = egui::pos2(
                        tab_x + 8.0,
                        rect.top() + (tab_height - galley.size().y) / 2.0,
                    );
                    painter.galley(text_pos, galley, egui::Color32::WHITE);

                    tab_x += tab_width;
                }

                // Render active tab's content area.
                if let Some(active_id) = tabs.get(*active) {
                    if self.is_panel_visible(active_id) {
                        rects.insert(active_id.clone(), content_rect);
                        self.panel_sizes
                            .insert(active_id.clone(), (content_rect.width(), content_rect.height()));
                    }
                }
            }
        }
    }

    /// Show or hide a panel.
    ///
    /// Hidden panels still occupy space in the tree but render as empty
    /// areas. Use `remove_panel` to fully remove a panel.
    pub fn show_panel(&mut self, panel_id: &str, visible: bool) {
        self.panel_visibility.insert(panel_id.to_string(), visible);
    }

    /// Check if a panel is currently visible.
    pub fn is_panel_visible(&self, panel_id: &str) -> bool {
        self.panel_visibility
            .get(panel_id)
            .copied()
            .unwrap_or(true)
    }

    /// Save the current layout to a JSON string.
    pub fn save_layout(&self) -> Result<String, DockError> {
        self.layout.serialize()
    }

    /// Load a layout from a JSON string, replacing the current layout.
    pub fn load_layout(&mut self, json: &str) -> Result<(), DockError> {
        let new_layout = DockLayout::deserialize(json)?;
        self.layout = new_layout;
        // Rebuild visibility map.
        for id in self.layout.all_panel_ids() {
            self.panel_visibility.entry(id).or_insert(true);
        }
        Ok(())
    }

    /// Reset to the default editor layout.
    pub fn reset_to_default(&mut self) {
        self.layout = DockLayout::default_layout();
        self.panel_sizes.clear();
        self.drag_state = None;
        let mut visibility = HashMap::new();
        for id in self.layout.all_panel_ids() {
            visibility.insert(id, true);
        }
        self.panel_visibility = visibility;
    }

    /// Begin dragging a panel.
    pub fn begin_drag(&mut self, panel_id: String, source_path: DockNodePath) {
        self.drag_state = Some(DragState::new(panel_id, source_path));
    }

    /// End the current drag operation, returning the drag state if active.
    pub fn end_drag(&mut self) -> Option<DragState> {
        self.drag_state.take()
    }

    /// Returns true if a drag operation is in progress.
    pub fn is_dragging(&self) -> bool {
        self.drag_state.is_some()
    }
}

impl Default for DockState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. DockNode creation ──────────────────────

    #[test]
    fn test_dock_node_leaf_creation() {
        let node = DockNode::leaf("viewport");
        match &node {
            DockNode::Leaf { panel_id, size_ratio } => {
                assert_eq!(panel_id, "viewport");
                assert!((size_ratio - 1.0).abs() < f32::EPSILON);
            }
            _ => panic!("expected Leaf node"),
        }
    }

    #[test]
    fn test_dock_node_split_creation() {
        let node = DockNode::h_split(0.3, DockNode::leaf("a"), DockNode::leaf("b"));
        match &node {
            DockNode::Split { direction, ratio, children } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert!((ratio - 0.3).abs() < 0.01);
                assert!(matches!(&children.0, DockNode::Leaf { panel_id, .. } if panel_id == "a"));
                assert!(matches!(&children.1, DockNode::Leaf { panel_id, .. } if panel_id == "b"));
            }
            _ => panic!("expected Split node"),
        }
    }

    #[test]
    fn test_dock_node_tabbed_creation() {
        let node = DockNode::tabbed(vec!["console".into(), "log".into()]);
        match &node {
            DockNode::Tabbed { tabs, active, .. } => {
                assert_eq!(tabs.len(), 2);
                assert_eq!(*active, 0);
            }
            _ => panic!("expected Tabbed node"),
        }
    }

    // ── 2. Default layout structure ───────────────

    #[test]
    fn test_default_layout_has_five_panels() {
        let layout = DockLayout::default_layout();
        let ids = layout.all_panel_ids();
        assert!(ids.contains(&"hierarchy".to_string()));
        assert!(ids.contains(&"viewport".to_string()));
        assert!(ids.contains(&"inspector".to_string()));
        assert!(ids.contains(&"console".to_string()));
        assert!(ids.contains(&"asset_browser".to_string()));
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn test_default_layout_root_is_vertical_split() {
        let layout = DockLayout::default_layout();
        match &layout.root {
            DockNode::Split { direction, ratio, .. } => {
                assert_eq!(*direction, SplitDirection::Vertical);
                assert!((ratio - 0.75).abs() < 0.01);
            }
            _ => panic!("root should be a vertical split"),
        }
    }

    // ── 3. Find panel by ID ───────────────────────

    #[test]
    fn test_find_panel_leaf() {
        let layout = DockLayout::default_layout();
        let path = layout.find_panel("viewport").expect("should find viewport");
        assert!(!path.is_empty());
    }

    #[test]
    fn test_find_panel_in_tabs() {
        let layout = DockLayout::default_layout();
        let path = layout.find_panel("console").expect("should find console");
        assert!(!path.is_empty());
        // Last element is the tab index.
        assert_eq!(*path.last().unwrap(), 0);
    }

    #[test]
    fn test_find_missing_panel_returns_none() {
        let layout = DockLayout::default_layout();
        assert!(layout.find_panel("nonexistent").is_none());
    }

    // ── 4. Remove panel ───────────────────────────

    #[test]
    fn test_remove_panel_from_leaf() {
        let mut layout = DockLayout::new();
        assert!(layout.remove_panel("inspector"));
        assert!(!layout.has_panel("inspector"));
        // The tree should have simplified — viewport is still there.
        assert!(layout.has_panel("viewport"));
    }

    #[test]
    fn test_remove_panel_from_tabbed() {
        let mut layout = DockLayout::new();
        assert!(layout.remove_panel("console"));
        assert!(!layout.has_panel("console"));
        // asset_browser should still exist.
        assert!(layout.has_panel("asset_browser"));
    }

    #[test]
    fn test_remove_nonexistent_panel() {
        let mut layout = DockLayout::new();
        assert!(!layout.remove_panel("nonexistent"));
        assert_eq!(layout.all_panel_ids().len(), 5);
    }

    // ── 5. Add panel as split ─────────────────────

    #[test]
    fn test_add_panel_creates_split() {
        let mut layout = DockLayout::new();
        // Add "timeline" panel next to viewport.
        let viewport_path = layout.find_panel("viewport").expect("viewport should exist");
        // Navigate to the split parent — path goes to a split child.
        let parent_path = &viewport_path[..viewport_path.len().saturating_sub(1)];
        layout.add_panel(parent_path, SplitDirection::Vertical, "timeline".into());
        assert!(layout.has_panel("timeline"));
        assert!(layout.has_panel("viewport"));
    }

    // ── 6. Move panel between nodes ───────────────

    #[test]
    fn test_move_panel() {
        let mut layout = DockLayout::new();
        // Move inspector next to the bottom panel.
        let bottom_path = vec![1]; // second child of root (bottom area)
        layout.move_panel("inspector", &bottom_path, SplitDirection::Horizontal);
        assert!(layout.has_panel("inspector"));
        // Verify it's no longer in its original position.
        let ids = layout.all_panel_ids();
        assert!(ids.contains(&"inspector".to_string()));
    }

    // ── 7. Serialize / deserialize roundtrip ──────

    #[test]
    #[cfg(feature = "serialize")]
    fn test_serialize_deserialize_roundtrip() {
        let original = DockLayout::default_layout();
        let json = original.serialize().expect("should serialize");
        assert!(!json.is_empty());
        assert!(json.contains("viewport"));
        assert!(json.contains("hierarchy"));

        let restored = DockLayout::deserialize(&json).expect("should deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    #[cfg(feature = "serialize")]
    fn test_deserialize_invalid_json() {
        let result = DockLayout::deserialize("{invalid}");
        assert!(result.is_err());
        match result.unwrap_err() {
            DockError::SerializationError(_) => {} // expected
            other => panic!("expected SerializationError, got {:?}", other),
        }
    }

    // ── 8. Tabbed node active panel tracking ──────

    #[test]
    fn test_active_panel_in_tab() {
        let layout = DockLayout::default_layout();
        // "console" is the first tab (active=0).
        let active = layout.active_panel_in_tab("console").expect("should find");
        assert_eq!(active, "console");

        // "asset_browser" is the second tab — active tab is still console.
        let active = layout.active_panel_in_tab("asset_browser").expect("should find");
        assert_eq!(active, "console");
    }

    #[test]
    fn test_active_panel_in_leaf() {
        let layout = DockLayout::default_layout();
        let active = layout.active_panel_in_tab("viewport").expect("should find");
        assert_eq!(active, "viewport");
    }

    #[test]
    fn test_active_panel_not_found() {
        let layout = DockLayout::default_layout();
        assert!(layout.active_panel_in_tab("nonexistent").is_none());
    }

    // ── 9. DragState transitions ──────────────────

    #[test]
    fn test_drag_state_lifecycle() {
        let mut state = DockState::new();
        assert!(!state.is_dragging());

        state.begin_drag("viewport".into(), vec![0, 0, 0]);
        assert!(state.is_dragging());

        let drag = state.drag_state.as_ref().unwrap();
        assert_eq!(drag.panel_id, "viewport");
        assert!(drag.preview_zone.is_none());
        assert!(!drag.has_preview());

        state.drag_state.as_mut().unwrap().set_preview_zone(Some(DockZone::Right));
        assert!(state.drag_state.as_ref().unwrap().has_preview());

        let ended = state.end_drag().expect("should end drag");
        assert_eq!(ended.panel_id, "viewport");
        assert!(!state.is_dragging());
    }

    // ── 10. Error cases ───────────────────────────

    #[test]
    fn test_dock_error_display() {
        let err = DockError::PanelNotFound("test".into());
        assert!(err.to_string().contains("test"));

        let err = DockError::InvalidLayout("broken".into());
        assert!(err.to_string().contains("broken"));

        let err = DockError::SerializationError("bad json".into());
        assert!(err.to_string().contains("bad json"));
    }

    // ── 11. Reset to default ──────────────────────

    #[test]
    fn test_reset_to_default() {
        let mut state = DockState::new();
        // Modify the layout.
        state.layout.remove_panel("inspector");
        assert!(!state.layout.has_panel("inspector"));

        // Reset.
        state.reset_to_default();
        assert!(state.layout.has_panel("inspector"));
        assert!(state.layout.has_panel("viewport"));
        assert!(state.layout.has_panel("hierarchy"));
        assert!(state.layout.has_panel("console"));
        assert!(state.layout.has_panel("asset_browser"));
        assert!(!state.is_dragging());
    }

    // ── 12. SplitDirection methods ─────────────────

    #[test]
    fn test_split_direction_invert() {
        assert_eq!(SplitDirection::Horizontal.invert(), SplitDirection::Vertical);
        assert_eq!(SplitDirection::Vertical.invert(), SplitDirection::Horizontal);
    }

    #[test]
    fn test_split_direction_display() {
        assert_eq!(SplitDirection::Horizontal.to_string(), "horizontal");
        assert_eq!(SplitDirection::Vertical.to_string(), "vertical");
    }

    // ── 13. Size ratio normalization ──────────────

    #[test]
    fn test_normalize_ratios_clamps_values() {
        let mut node = DockNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 1.5,
            children: Box::new((
                DockNode::Leaf {
                    panel_id: "a".into(),
                    size_ratio: 2.0,
                },
                DockNode::Leaf {
                    panel_id: "b".into(),
                    size_ratio: -0.5,
                },
            )),
        };
        node.normalize_ratios();

        match &node {
            DockNode::Split { ratio, children, .. } => {
                assert!(*ratio <= 0.99 && *ratio >= 0.01);
                match &children.0 {
                    DockNode::Leaf { size_ratio, .. } => {
                        assert!(*size_ratio <= 1.0 && *size_ratio >= 0.0);
                    }
                    _ => {}
                }
                match &children.1 {
                    DockNode::Leaf { size_ratio, .. } => {
                        assert!(*size_ratio <= 1.0 && *size_ratio >= 0.0);
                    }
                    _ => {}
                }
            }
            _ => panic!("expected Split"),
        }
    }

    // ── 14. DockZone methods ──────────────────────

    #[test]
    fn test_dock_zone_split_direction() {
        assert_eq!(DockZone::Left.split_direction(), Some(SplitDirection::Horizontal));
        assert_eq!(DockZone::Right.split_direction(), Some(SplitDirection::Horizontal));
        assert_eq!(DockZone::Top.split_direction(), Some(SplitDirection::Vertical));
        assert_eq!(DockZone::Bottom.split_direction(), Some(SplitDirection::Vertical));
        assert_eq!(DockZone::Center.split_direction(), None);
    }

    #[test]
    fn test_dock_zone_is_first_child() {
        assert!(DockZone::Left.is_first_child());
        assert!(DockZone::Top.is_first_child());
        assert!(!DockZone::Right.is_first_child());
        assert!(!DockZone::Bottom.is_first_child());
        assert!(!DockZone::Center.is_first_child());
    }

    // ── 15. DockState panel visibility ────────────

    #[test]
    fn test_panel_visibility() {
        let mut state = DockState::new();
        assert!(state.is_panel_visible("viewport"));
        state.show_panel("viewport", false);
        assert!(!state.is_panel_visible("viewport"));
        state.show_panel("viewport", true);
        assert!(state.is_panel_visible("viewport"));
    }

    // ── 16. Panel count ───────────────────────────

    #[test]
    fn test_panel_count() {
        let layout = DockLayout::default_layout();
        assert_eq!(layout.root.panel_count(), 5);
    }

    // ── 17. Remove all panels from tabbed ─────────

    #[test]
    fn test_remove_all_tabs_yields_empty() {
        let mut layout = DockLayout::new();
        layout.remove_panel("console");
        layout.remove_panel("asset_browser");
        // The tabbed node should have become Empty, and the tree
        // should have simplified.
        assert!(!layout.has_panel("console"));
        assert!(!layout.has_panel("asset_browser"));
        // Remaining panels should still exist.
        assert!(layout.has_panel("viewport"));
    }

    // ── 18. contains_panel ─────────────────────────

    #[test]
    fn test_contains_panel() {
        let layout = DockLayout::default_layout();
        assert!(layout.root.contains_panel("viewport"));
        assert!(layout.root.contains_panel("console"));
        assert!(!layout.root.contains_panel("nonexistent"));
    }

    // ── 19. DockState load/save layout ────────────

    #[test]
    #[cfg(feature = "serialize")]
    fn test_save_load_layout() {
        let mut state = DockState::new();
        let json = state.save_layout().expect("should save");
        state.layout.remove_panel("inspector");
        assert!(!state.layout.has_panel("inspector"));

        state.load_layout(&json).expect("should load");
        assert!(state.layout.has_panel("inspector"));
    }

    // ── 20. DockState with custom layout ──────────

    #[test]
    fn test_dock_state_with_custom_layout() {
        let custom = DockLayout {
            root: DockNode::leaf("custom_panel"),
        };
        let state = DockState::with_layout(custom);
        assert!(state.is_panel_visible("custom_panel"));
        assert!(state.layout.has_panel("custom_panel"));
    }
}

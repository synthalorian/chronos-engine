//! Hierarchy panel — entity tree for the Chronos Engine editor.
//!
//! Displays all alive entities in the current [`World`], supports
//! single / multi-selection, search filtering, and context-menu
//! operations (create, delete, rename, duplicate).

use super::{EditorPanel, EditorState};
use crate::entity::Entity;

// ──────────────────────────────────────────────
// HierarchyPanel
// ──────────────────────────────────────────────

/// Editor panel that lists every entity in the scene hierarchy.
///
/// The hierarchy is a flat entity list (parent-child relationships are
/// planned for a future phase). Users can search, select, create, rename,
/// duplicate, and delete entities through this panel.
pub struct HierarchyPanel {
    /// When `true`, the scroll area jumps to the first selected entity
    /// on the next frame.
    scroll_to_selected: bool,

    /// Entity currently being renamed, if any.
    rename_entity: Option<Entity>,

    /// Text buffer used during an inline rename operation.
    rename_buffer: String,

    /// Toggle for the entity visibility filter.
    show_entities: bool,

    /// Search bar text used to filter the displayed entity list.
    search_filter: String,
}

impl HierarchyPanel {
    /// Create a new hierarchy panel with default state.
    pub fn new() -> Self {
        Self {
            scroll_to_selected: false,
            rename_entity: None,
            rename_buffer: String::new(),
            show_entities: true,
            search_filter: String::new(),
        }
    }

    // ── Internal helpers ──

    /// Collect every alive entity from the world, returning them sorted
    /// by index for deterministic display order.
    fn collect_alive_entities(state: &EditorState) -> Vec<Entity> {
        let capacity = state.world.entity_capacity();
        let mut entities: Vec<Entity> = (0..capacity)
            .filter_map(|i| {
                let entity = state.world.entity_from_index(i as u32);
                if state.world.entity_exists(entity) {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect();
        entities.sort_by_key(|e| e.index());
        entities
    }

    /// Return `true` when `entity`'s display label matches the current
    /// search filter (case-insensitive substring match).
    fn matches_filter(&self, entity: Entity) -> bool {
        if self.search_filter.is_empty() {
            return true;
        }
        let label = format_entity(entity);
        label
            .to_lowercase()
            .contains(&self.search_filter.to_lowercase())
    }

    /// Create an empty entity and immediately select it.
    fn create_empty_entity(state: &mut EditorState) {
        let entity = state.world.create_entity();
        state.select(entity);
    }

    /// Delete an entity from the world and remove it from the selection set.
    fn delete_entity(state: &mut EditorState, entity: Entity) {
        state.world.destroy_entity(entity);
        state.deselect(entity);
    }

    /// Duplicate an entity by creating a new empty one and auto-selecting it.
    ///
    /// *Future work*: copy all components from the source entity.
    fn duplicate_entity(state: &mut EditorState, _source: Entity) {
        let new = state.world.create_entity();
        state.select(new);
    }

    /// Handle keyboard shortcuts that operate on the current selection.
    fn handle_shortcuts(&mut self, ui: &egui::Ui, state: &mut EditorState) {
        // Delete key — delete all selected entities.
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            let to_delete: Vec<Entity> = state.selected_entities.clone();
            for entity in to_delete {
                Self::delete_entity(state, entity);
            }
        }

        // Ctrl+A — select all entities.
        if ui.input(|i| i.key_pressed(egui::Key::A) && i.modifiers.ctrl) {
            let all = Self::collect_alive_entities(state);
            state.selected_entities = all;
        }

        // Escape — clear selection.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.clear_selection();
            self.rename_entity = None;
        }
    }
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Format an entity as `"Entity {index}:{generation}"`.
fn format_entity(entity: Entity) -> String {
    format!("Entity {}:{}", entity.index(), entity.generation())
}

impl EditorPanel for HierarchyPanel {
    fn title(&self) -> &str {
        "Hierarchy"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // ── Keyboard shortcuts ──
        self.handle_shortcuts(ui, state);

        // ── Search bar ──
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut self.search_filter);
            if response.changed() {
                // Filter updated — no extra action needed, filtering is live.
            }
            if ui.button("✖").clicked() {
                self.search_filter.clear();
            }
        });
        ui.separator();

        // ── Entity list ──
        let entities = Self::collect_alive_entities(state);
        let filtered: Vec<Entity> = entities
            .into_iter()
            .filter(|e| self.matches_filter(*e))
            .collect();

        egui::ScrollArea::vertical()
            .id_salt("hierarchy_scroll")
            .show(ui, |ui| {
                for entity in &filtered {
                    let label = format_entity(*entity);
                    let is_selected = state.is_selected(*entity);

                    // ── Rename mode ──
                    if self.rename_entity == Some(*entity) {
                        ui.horizontal(|ui| {
                            ui.label("🔹");
                            let response = ui.text_edit_singleline(&mut self.rename_buffer);
                            if response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                // Finish rename — for now the label is derived
                                // from index:generation so we just close the editor.
                                self.rename_entity = None;
                            }
                            if response.lost_focus() {
                                self.rename_entity = None;
                            }
                        });
                        continue;
                    }

                    // ── Normal display row ──
                    let row_response = if is_selected {
                        ui.colored_label(
                            ui.style().visuals.selection.bg_fill,
                            format!("🔹 {label}"),
                        )
                    } else {
                        ui.label(format!("🔹 {label}"))
                    };

                    let row_response = row_response.interact(egui::Sense::click());

                    // ── Click to select / multi-select ──
                    if row_response.clicked() {
                        if ui.input(|i| i.modifiers.ctrl) {
                            if is_selected {
                                state.deselect(*entity);
                            } else {
                                state.select_add(*entity);
                            }
                        } else {
                            state.select(*entity);
                        }
                    }

                    // ── Right-click context menu on entity ──
                    row_response.context_menu(|ui| {
                        if ui.button("Delete").clicked() {
                            Self::delete_entity(state, *entity);
                            ui.close_menu();
                        }
                        if ui.button("Rename").clicked() {
                            self.rename_entity = Some(*entity);
                            self.rename_buffer = label.clone();
                            ui.close_menu();
                        }
                        if ui.button("Duplicate").clicked() {
                            Self::duplicate_entity(state, *entity);
                            ui.close_menu();
                        }
                    });
                }

                // ── Right-click context menu on empty space ──
                if filtered.is_empty() {
                    let empty = ui.allocate_response(ui.available_size(), egui::Sense::click());
                    empty.context_menu(|ui| {
                        if ui.button("Create Empty Entity").clicked() {
                            Self::create_empty_entity(state);
                            ui.close_menu();
                        }
                    });
                }
            });

        // ── Scroll to selected entity if requested ──
        if self.scroll_to_selected {
            self.scroll_to_selected = false;
        }

        // ── Context menu on empty space within scroll area ──
        // (fallthrough — also works when list is non-empty by right-clicking
        //  below the last row)
        ui.allocate_response(ui.available_size(), egui::Sense::click())
            .context_menu(|ui| {
                if ui.button("Create Empty Entity").clicked() {
                    Self::create_empty_entity(state);
                    ui.close_menu();
                }
            });

        ui.separator();

        // ── Footer ──
        let count = state.world.entity_count();
        ui.colored_label(
            ui.style().visuals.text_color(),
            format!("{count} entit{}", if count == 1 { "y" } else { "ies" }),
        );
    }
}

// ──────────────────────────────────────────────
// Unit tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a default [`EditorState`] with a fresh world.
    fn make_state() -> EditorState {
        EditorState::new()
    }

    // ── new() defaults ──

    #[test]
    fn new_defaults() {
        let panel = HierarchyPanel::new();
        assert!(!panel.scroll_to_selected);
        assert!(panel.rename_entity.is_none());
        assert!(panel.rename_buffer.is_empty());
        assert!(panel.show_entities);
        assert!(panel.search_filter.is_empty());
    }

    // ── title() ──

    #[test]
    fn title_returns_hierarchy() {
        let panel = HierarchyPanel::new();
        assert_eq!(panel.title(), "Hierarchy");
    }

    // ── search filter logic ──

    #[test]
    fn search_filter_matches_substring() {
        let mut panel = HierarchyPanel::new();
        let entity = Entity::new(0, 0);

        // Empty filter matches everything.
        assert!(panel.matches_filter(entity));

        // Matching substring.
        panel.search_filter = "Entity 0".to_string();
        assert!(panel.matches_filter(entity));

        // Non-matching substring.
        panel.search_filter = "Entity 99".to_string();
        assert!(!panel.matches_filter(entity));
    }

    #[test]
    fn search_filter_case_insensitive() {
        let mut panel = HierarchyPanel::new();
        let entity = Entity::new(5, 2);

        panel.search_filter = "entity 5".to_string();
        assert!(panel.matches_filter(entity));

        panel.search_filter = "ENTITY 5".to_string();
        assert!(panel.matches_filter(entity));
    }

    // ── entity display formatting ──

    #[test]
    fn format_entity_basic() {
        let entity = Entity::new(3, 1);
        assert_eq!(format_entity(entity), "Entity 3:1");
    }

    #[test]
    fn format_entity_zero_generation() {
        let entity = Entity::new(0, 0);
        assert_eq!(format_entity(entity), "Entity 0:0");
    }

    // ── selection logic ──

    #[test]
    fn select_sets_single_entity() {
        let mut state = make_state();
        let e = state.world.create_entity();
        state.select(e);
        assert_eq!(state.selected_entities.len(), 1);
        assert!(state.is_selected(e));
    }

    #[test]
    fn select_add_enables_multi_select() {
        let mut state = make_state();
        let a = state.world.create_entity();
        let b = state.world.create_entity();
        state.select(a);
        state.select_add(b);
        assert_eq!(state.selected_entities.len(), 2);
        assert!(state.is_selected(a));
        assert!(state.is_selected(b));
    }

    #[test]
    fn ctrl_click_toggle_selection() {
        let mut state = make_state();
        let a = state.world.create_entity();
        let b = state.world.create_entity();
        state.select(a);
        // Simulate Ctrl+click on b → add.
        state.select_add(b);
        assert!(state.is_selected(b));
        // Simulate Ctrl+click on b again → remove.
        state.deselect(b);
        assert!(!state.is_selected(b));
        assert!(state.is_selected(a));
    }

    // ── delete from selection ──

    #[test]
    fn delete_entity_removes_from_selection() {
        let mut state = make_state();
        let a = state.world.create_entity();
        let b = state.world.create_entity();
        state.select(a);
        state.select_add(b);
        assert_eq!(state.selected_entities.len(), 2);

        HierarchyPanel::delete_entity(&mut state, a);
        assert!(!state.is_selected(a));
        assert!(state.is_selected(b));
        assert_eq!(state.world.entity_count(), 1);
    }

    #[test]
    fn delete_all_selected() {
        let mut state = make_state();
        let a = state.world.create_entity();
        let b = state.world.create_entity();
        state.select(a);
        state.select_add(b);

        let to_delete = state.selected_entities.clone();
        for entity in to_delete {
            HierarchyPanel::delete_entity(&mut state, entity);
        }
        assert!(state.selected_entities.is_empty());
        assert_eq!(state.world.entity_count(), 0);
    }

    // ── create empty entity ──

    #[test]
    fn create_empty_entity_auto_selects() {
        let mut state = make_state();
        assert_eq!(state.world.entity_count(), 0);
        HierarchyPanel::create_empty_entity(&mut state);
        assert_eq!(state.world.entity_count(), 1);
        assert_eq!(state.selected_entities.len(), 1);
    }

    // ── collect alive entities ──

    #[test]
    fn collect_alive_returns_sorted_entities() {
        let mut state = make_state();
        let c = state.world.create_entity(); // index 0
        let a = state.world.create_entity(); // index 1
        let b = state.world.create_entity(); // index 2

        state.world.destroy_entity(a); // kill index 1

        let alive = HierarchyPanel::collect_alive_entities(&state);
        assert_eq!(alive.len(), 2);
        assert_eq!(alive[0].index(), c.index());
        assert_eq!(alive[1].index(), b.index());
    }

    // ── clear selection ──

    #[test]
    fn clear_selection_empties_selection() {
        let mut state = make_state();
        let a = state.world.create_entity();
        let b = state.world.create_entity();
        state.select(a);
        state.select_add(b);
        assert_eq!(state.selected_entities.len(), 2);
        state.clear_selection();
        assert!(state.selected_entities.is_empty());
    }
}

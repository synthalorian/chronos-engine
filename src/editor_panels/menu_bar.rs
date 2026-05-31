//! Menu bar panel — Phase 7B.
//!
//! Top-level menu bar for the Chronos Engine editor. Provides File, Edit,
//! View, and Help menus with keyboard-shortcut hints, a shortcuts reference
//! dialog, and an about dialog. Uses `egui::menu::bar` for native-feeling
//! dropdown menus.

use super::{ConsoleLogLevel, EditorPanel, EditorState};

// ── Menu Bar Panel ──────────────────────────────────────────────────────────

/// Top-of-window menu bar with File / Edit / View / Help menus.
///
/// Also owns the state for popup dialogs (shortcuts reference, about) which
/// are rendered as separate `egui::Window`s each frame.
pub struct MenuBarPanel {
    /// Whether the keyboard-shortcuts reference dialog is visible.
    show_shortcuts_dialog: bool,
}

impl MenuBarPanel {
    /// Create a new menu bar panel.
    pub fn new() -> Self {
        Self {
            show_shortcuts_dialog: false,
        }
    }

    // ── Menu Builders ────────────────────────────────────────────────────

    /// Render the **File** menu.
    fn show_file_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.menu_button("File", |ui| {
            ui.set_min_width(220.0);

            if menu_item(ui, "New Project", "Ctrl+N").clicked() {
                state.project_manager.show_new_wizard = true;
                state.log(ConsoleLogLevel::Info, "New project wizard opened");
                ui.close_menu();
            }

            if menu_item(ui, "Open Project...", "Ctrl+O").clicked() {
                state.project_manager.show_open_dialog = true;
                state.log(ConsoleLogLevel::Info, "Open project dialog opened");
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Save", "Ctrl+S").clicked() {
                if state.project_manager.is_loaded() {
                    match state.project_manager.save_current() {
                        Ok(()) => state.log(ConsoleLogLevel::Info, "Project saved"),
                        Err(e) => state.log(ConsoleLogLevel::Error, format!("Save failed: {e}")),
                    }
                } else {
                    state.log(ConsoleLogLevel::Warn, "No project loaded to save");
                }
                ui.close_menu();
            }

            if menu_item(ui, "Save As...", "Ctrl+Shift+S").clicked() {
                state.save_as_requested = true;
                state.log(ConsoleLogLevel::Info, "Save As requested");
                ui.close_menu();
            }

            ui.separator();

            // ── Recent projects submenu ──────────────────────────────
            let recents: Vec<_> = state.project_manager.recent_projects.clone();
            ui.menu_button("Recent Projects", |ui| {
                if recents.is_empty() {
                    ui.weak("No recent projects");
                } else {
                    for recent in &recents {
                        let label = format!("{}  {}", recent.template.icon(), recent.name);
                        if ui.button(label).clicked() {
                            let path = std::path::PathBuf::from(&recent.path);
                            match crate::editor_project::ProjectManager::open_project(&path) {
                                Ok(mgr) => {
                                    let name = mgr.project_name().to_string();
                                    let template = mgr
                                        .current_project
                                        .as_ref()
                                        .map(|m| m.template)
                                        .unwrap_or(recent.template);
                                    let old_recents =
                                        std::mem::take(&mut state.project_manager.recent_projects);
                                    state.project_manager = mgr;
                                    state.project_manager.recent_projects = old_recents;
                                    state.project_manager.add_recent(
                                        &name,
                                        &path.to_string_lossy(),
                                        template,
                                    );
                                    state.project_path = Some(path.clone());
                                    state.recent_dirty = true;
                                    state.log(
                                        ConsoleLogLevel::Info,
                                        format!("Opened recent project: {name}"),
                                    );
                                }
                                Err(e) => {
                                    state.log(
                                        ConsoleLogLevel::Error,
                                        format!(
                                            "Failed to open recent project '{}': {e}",
                                            recent.name
                                        ),
                                    );
                                }
                            }
                            ui.close_menu();
                        }
                    }
                }
            });

            ui.separator();

            if menu_item(ui, "Quit", "Ctrl+Q").clicked() {
                state.should_quit = true;
                ui.close_menu();
            }
        });
    }

    /// Render the **Edit** menu.
    fn show_edit_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.menu_button("Edit", |ui| {
            ui.set_min_width(200.0);

            if menu_item(ui, "Undo", "Ctrl+Z").clicked() {
                state.undo_requested = true;
                ui.close_menu();
            }

            if menu_item(ui, "Redo", "Ctrl+Y").clicked() {
                state.redo_requested = true;
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Select All", "Ctrl+A").clicked() {
                state.select_all_requested = true;
                ui.close_menu();
            }

            if menu_item(ui, "Deselect", "Escape").clicked() {
                state.clear_selection();
                state.log(ConsoleLogLevel::Info, "Deselected all");
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Delete", "Delete").clicked() {
                state.delete_selected_requested = true;
                ui.close_menu();
            }

            if menu_item(ui, "Duplicate", "Ctrl+D").clicked() {
                state.duplicate_selected_requested = true;
                ui.close_menu();
            }
        });
    }

    /// Render the **View** menu.
    fn show_view_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.menu_button("View", |ui| {
            ui.set_min_width(180.0);

            let _ = check_menu_item(ui, "Hierarchy", true);
            let _ = check_menu_item(ui, "Inspector", true);
            let _ = check_menu_item(ui, "Asset Browser", true);
            let _ = check_menu_item(ui, "Console", true);

            ui.separator();

            if menu_item(ui, "Fullscreen", "F11").clicked() {
                state.fullscreen_requested = true;
                state.log(ConsoleLogLevel::Info, "Toggle fullscreen requested");
                ui.close_menu();
            }

            if menu_item_no_shortcut(ui, "Reset Layout").clicked() {
                state.reset_layout_requested = true;
                state.log(ConsoleLogLevel::Info, "Reset layout requested");
                ui.close_menu();
            }
        });
    }

    /// Render the **Help** menu.
    fn show_help_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.menu_button("Help", |ui| {
            ui.set_min_width(200.0);

            if menu_item_no_shortcut(ui, "Keyboard Shortcuts").clicked() {
                self.show_shortcuts_dialog = true;
                ui.close_menu();
            }

            if menu_item_no_shortcut(ui, "Documentation").clicked() {
                state.log(ConsoleLogLevel::Info, "Open docs requested");
                ui.close_menu();
            }

            ui.separator();

            if menu_item_no_shortcut(ui, "About Chronos Engine").clicked() {
                state.show_about = true;
                ui.close_menu();
            }
        });
    }

    // ── Dialog Renderers ─────────────────────────────────────────────────

    /// Render the keyboard-shortcuts reference dialog.
    fn render_shortcuts_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts_dialog {
            return;
        }

        let open = std::cell::Cell::new(self.show_shortcuts_dialog);
        let mut was_open = open.get();
        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut was_open)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        shortcut_row(ui, "Ctrl+N", "New Project");
                        shortcut_row(ui, "Ctrl+O", "Open Project");
                        shortcut_row(ui, "Ctrl+S", "Save");
                        shortcut_row(ui, "Ctrl+Shift+S", "Save As");
                        shortcut_row(ui, "Ctrl+Q", "Quit");
                        ui.end_row();

                        shortcut_row(ui, "Ctrl+Z", "Undo");
                        shortcut_row(ui, "Ctrl+Y", "Redo");
                        shortcut_row(ui, "Ctrl+A", "Select All");
                        shortcut_row(ui, "Escape", "Deselect");
                        shortcut_row(ui, "Delete", "Delete Selected");
                        shortcut_row(ui, "Ctrl+D", "Duplicate");
                        ui.end_row();

                        shortcut_row(ui, "F11", "Toggle Fullscreen");
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if ui.button("Close").clicked() {
                        open.set(false);
                    }
                });
            });
        self.show_shortcuts_dialog = open.get() && was_open;
    }

    /// Render the about dialog.
    fn render_about_dialog(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        if !state.show_about {
            return;
        }

        let open = std::cell::Cell::new(state.show_about);
        let mut was_open = open.get();
        egui::Window::new("About Chronos Engine")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut was_open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new("Chronos Engine")
                            .text_style(egui::TextStyle::Heading)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("v1.0.0")
                            .text_style(egui::TextStyle::Body)
                            .weak(),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label("Built with 🎹🦈 by synth");

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "A time-bending game engine and visual editor.\n\
                             Write the future in the present while preserving the past.",
                        )
                        .text_style(egui::TextStyle::Body)
                        .weak(),
                    );

                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        open.set(false);
                    }
                });
            });
        state.show_about = open.get() && was_open;
    }
}

impl Default for MenuBarPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ── EditorPanel Implementation ─────────────────────────────────────────────

impl EditorPanel for MenuBarPanel {
    fn title(&self) -> &str {
        "Menu Bar"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        egui::menu::bar(ui, |ui| {
            self.show_file_menu(ui, state);
            self.show_edit_menu(ui, state);
            self.show_view_menu(ui, state);
            self.show_help_menu(ui, state);
        });

        let ctx = ui.ctx().clone();
        self.render_shortcuts_dialog(&ctx);
        self.render_about_dialog(&ctx, state);
    }
}

// ── Menu Item Helpers ────────────────────────────────────────────────

/// Render a clickable menu item with a right-aligned keyboard shortcut.
///
/// Uses egui's standard button inside the menu's vertical layout.
/// The shortcut is displayed as weak text on the right side.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui.button(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(16.0);
            ui.label(egui::RichText::new(shortcut).italics().weak().small());
        });
        response
    })
    .inner
}

/// Render a clickable menu item without a shortcut hint.
fn menu_item_no_shortcut(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.button(label)
}

/// Render a checkable menu item (always-checked placeholder).
fn check_menu_item(ui: &mut egui::Ui, label: &str, checked: bool) -> egui::Response {
    let mut dummy = checked;
    ui.checkbox(&mut dummy, label)
}

/// Render a single row in the shortcuts grid.
fn shortcut_row(ui: &mut egui::Ui, key: &str, action: &str) {
    ui.label(egui::RichText::new(key).strong());
    ui.label(action);
    ui.end_row();
}

// ── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let panel = MenuBarPanel::new();
        assert!(!panel.show_shortcuts_dialog);
    }

    #[test]
    fn title_returns_menu_bar() {
        let panel = MenuBarPanel::new();
        assert_eq!(panel.title(), "Menu Bar");
    }

    #[test]
    fn quit_flag_set() {
        let mut state = EditorState::new();
        assert!(!state.should_quit);
        state.should_quit = true;
        assert!(state.should_quit);
    }

    #[test]
    fn about_dialog_toggle() {
        let mut state = EditorState::new();
        assert!(!state.show_about);
        state.show_about = true;
        assert!(state.show_about);
        state.show_about = false;
        assert!(!state.show_about);
    }

    #[test]
    fn shortcuts_dialog_toggle() {
        let mut panel = MenuBarPanel::new();
        assert!(!panel.show_shortcuts_dialog);
        panel.show_shortcuts_dialog = true;
        assert!(panel.show_shortcuts_dialog);
        panel.show_shortcuts_dialog = false;
        assert!(!panel.show_shortcuts_dialog);
    }

    #[test]
    fn clear_selection_from_edit_menu() {
        let mut state = EditorState::new();
        let entity = state.world.create_entity();
        state.select(entity);
        assert!(state.is_selected(entity));
        state.clear_selection();
        assert!(state.selected_entities.is_empty());
        assert!(!state.is_selected(entity));
    }

    #[test]
    fn recent_projects_from_project_manager() {
        let mut state = EditorState::new();
        state.project_manager.add_recent(
            "platformer",
            "/tmp/platformer",
            crate::editor_project::ProjectTemplate::Platformer2D,
        );
        state.project_manager.add_recent(
            "rpg_demo",
            "/tmp/rpg_demo",
            crate::editor_project::ProjectTemplate::RPG,
        );
        assert_eq!(state.project_manager.recent_projects.len(), 2);
        assert_eq!(state.project_manager.recent_projects[0].name, "rpg_demo");
        assert_eq!(state.project_manager.recent_projects[1].name, "platformer");
    }

    #[test]
    fn log_on_new_project() {
        let mut state = EditorState::new();
        state.log(ConsoleLogLevel::Info, "New project");
        assert_eq!(state.console_log.len(), 1);
        assert_eq!(state.console_log[0].message, "New project");
        assert_eq!(state.console_log[0].level, ConsoleLogLevel::Info);
    }

    #[test]
    fn log_on_save() {
        let mut state = EditorState::new();
        state.log(ConsoleLogLevel::Info, "Save");
        assert_eq!(state.console_log.len(), 1);
        assert_eq!(state.console_log[0].message, "Save");
    }
}

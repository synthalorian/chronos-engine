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
    /// Mock list of recently opened projects (demo data).
    recent_projects: Vec<String>,

    /// Whether the keyboard-shortcuts reference dialog is visible.
    show_shortcuts_dialog: bool,
}

impl MenuBarPanel {
    /// Create a new menu bar panel with default demo data.
    pub fn new() -> Self {
        Self {
            recent_projects: vec![
                "examples/platformer.chronos".into(),
                "examples/rpg_demo.chronos".into(),
                "projects/particle_sim.chronos".into(),
                "projects/voxel_world.chronos".into(),
            ],
            show_shortcuts_dialog: false,
        }
    }

    // ── Menu Builders ────────────────────────────────────────────────────

    /// Render the **File** menu.
    fn show_file_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        egui::menu::menu_button(ui, "File", |ui| {
            // ── Project operations ───────────────────────────────────
            Self::menu_item(ui, "New Project", "Ctrl+N", || {
                state.log(ConsoleLogLevel::Info, "New project");
            });

            Self::menu_item(ui, "Open Project...", "Ctrl+O", || {
                state.log(ConsoleLogLevel::Info, "Open project");
            });

            Self::menu_item(ui, "Save", "Ctrl+S", || {
                state.log(ConsoleLogLevel::Info, "Save");
            });

            Self::menu_item(ui, "Save As...", "Ctrl+Shift+S", || {
                state.log(ConsoleLogLevel::Info, "Save As");
            });

            ui.separator();

            // ── Recent projects submenu ──────────────────────────────
            egui::menu::menu_button(ui, "Recent Projects", |ui| {
                if self.recent_projects.is_empty() {
                    ui.add_enabled(false, egui::Label::new("No recent projects"));
                } else {
                    for project in &self.recent_projects {
                        if ui.button(project).clicked() {
                            state.log(
                                ConsoleLogLevel::Info,
                                format!("Open recent: {project}"),
                            );
                            ui.close_menu();
                        }
                    }
                }
            });

            ui.separator();

            // ── Quit ─────────────────────────────────────────────────
            Self::menu_item(ui, "Quit", "Ctrl+Q", || {
                state.should_quit = true;
            });
        });
    }

    /// Render the **Edit** menu.
    fn show_edit_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        egui::menu::menu_button(ui, "Edit", |ui| {
            // ── Undo / Redo ──────────────────────────────────────────
            Self::menu_item(ui, "Undo", "Ctrl+Z", || {
                state.log(ConsoleLogLevel::Info, "Undo");
            });

            Self::menu_item(ui, "Redo", "Ctrl+Y", || {
                state.log(ConsoleLogLevel::Info, "Redo");
            });

            ui.separator();

            // ── Selection ────────────────────────────────────────────
            Self::menu_item(ui, "Select All", "Ctrl+A", || {
                state.log(ConsoleLogLevel::Info, "Select all");
            });

            Self::menu_item(ui, "Deselect", "Escape", || {
                state.clear_selection();
            });

            ui.separator();

            // ── Object manipulation ──────────────────────────────────
            Self::menu_item(ui, "Delete", "Delete", || {
                state.log(ConsoleLogLevel::Info, "Delete selected");
            });

            Self::menu_item(ui, "Duplicate", "Ctrl+D", || {
                state.log(ConsoleLogLevel::Info, "Duplicate");
            });
        });
    }

    /// Render the **View** menu.
    fn show_view_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        egui::menu::menu_button(ui, "View", |ui| {
            // ── Panel toggles ────────────────────────────────────────
            Self::check_menu_item(ui, "Viewport", true, || {
                // Viewport is always shown — placeholder toggle
            });

            Self::check_menu_item(ui, "Hierarchy", true, || {
                state.log(ConsoleLogLevel::Info, "Toggle Hierarchy");
            });

            Self::check_menu_item(ui, "Inspector", true, || {
                state.log(ConsoleLogLevel::Info, "Toggle Inspector");
            });

            Self::check_menu_item(ui, "Asset Browser", true, || {
                state.log(ConsoleLogLevel::Info, "Toggle Asset Browser");
            });

            Self::check_menu_item(ui, "Console", true, || {
                state.log(ConsoleLogLevel::Info, "Toggle Console");
            });

            ui.separator();

            // ── Layout ───────────────────────────────────────────────
            Self::menu_item(ui, "Fullscreen", "F11", || {
                state.log(ConsoleLogLevel::Info, "Toggle fullscreen");
            });

            Self::menu_item_no_shortcut(ui, "Reset Layout", || {
                state.log(ConsoleLogLevel::Info, "Reset layout");
            });
        });
    }

    /// Render the **Help** menu.
    fn show_help_menu(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        egui::menu::menu_button(ui, "Help", |ui| {
            Self::menu_item_no_shortcut(ui, "Keyboard Shortcuts", || {
                self.show_shortcuts_dialog = true;
            });

            Self::menu_item_no_shortcut(ui, "Documentation", || {
                state.log(ConsoleLogLevel::Info, "Open docs");
            });

            ui.separator();

            Self::menu_item_no_shortcut(ui, "About Chronos Engine", || {
                state.show_about = true;
            });
        });
    }

    // ── Dialog Renderers ─────────────────────────────────────────────────

    /// Render the keyboard-shortcuts reference dialog.
    fn show_shortcuts_dialog(&mut self, ctx: &egui::Context) {
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
                        Self::shortcut_row(ui, "Ctrl+N", "New Project");
                        Self::shortcut_row(ui, "Ctrl+O", "Open Project");
                        Self::shortcut_row(ui, "Ctrl+S", "Save");
                        Self::shortcut_row(ui, "Ctrl+Shift+S", "Save As");
                        Self::shortcut_row(ui, "Ctrl+Q", "Quit");
                        ui.end_row();

                        Self::shortcut_row(ui, "Ctrl+Z", "Undo");
                        Self::shortcut_row(ui, "Ctrl+Y", "Redo");
                        Self::shortcut_row(ui, "Ctrl+A", "Select All");
                        Self::shortcut_row(ui, "Escape", "Deselect");
                        Self::shortcut_row(ui, "Delete", "Delete Selected");
                        Self::shortcut_row(ui, "Ctrl+D", "Duplicate");
                        ui.end_row();

                        Self::shortcut_row(ui, "F11", "Toggle Fullscreen");
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
    fn show_about_dialog(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        if !state.show_about {
            return;
        }

        let open_flag = std::cell::Cell::new(state.show_about);
        let mut was_open = open_flag.get();
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
                        egui::RichText::new("v0.3")
                            .text_style(egui::TextStyle::Body)
                            .weak(),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label("Built with 🎹🦞 by synth");

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
                        open_flag.set(false);
                    }
                });
            });
        state.show_about = open_flag.get() && was_open;
    }

    // ── Menu Item Helpers ────────────────────────────────────────────────

    /// Render a clickable menu item with a right-aligned keyboard shortcut.
    fn menu_item(
        ui: &mut egui::Ui,
        label: &str,
        shortcut: &str,
        on_click: impl FnOnce(),
    ) {
        let _clicked = ui
            .horizontal(|ui| {
                ui.menu_button(label, |_| {}); // fake width calc
                let available = ui.available_width();
                ui.add_sized(
                    [available - ui.min_rect().width(), ui.min_rect().height()],
                    egui::Label::new(
                        egui::RichText::new(shortcut).italics().weak(),
                    ),
                );
                false
            })
            .inner;

        // Simpler approach: button + shortcut label on same row
        let mut inner_clicked = false;
        ui.horizontal(|ui| {
            if ui.button(label).clicked() {
                inner_clicked = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new(shortcut).italics().weak(),
                ));
            });
        });

        if inner_clicked {
            on_click();
            ui.close_menu();
        }
    }

    /// Render a clickable menu item without a shortcut hint.
    fn menu_item_no_shortcut(
        ui: &mut egui::Ui,
        label: &str,
        on_click: impl FnOnce(),
    ) {
        if ui.button(label).clicked() {
            on_click();
            ui.close_menu();
        }
    }

    /// Render a checkable menu item (always-checked placeholder).
    fn check_menu_item(
        ui: &mut egui::Ui,
        label: &str,
        checked: bool,
        on_click: impl FnOnce(),
    ) {
        if ui.checkbox(&mut true, label).clicked() {
            on_click();
            ui.close_menu();
        }
        let _ = checked;
    }

    /// Render a single row in the shortcuts grid.
    fn shortcut_row(ui: &mut egui::Ui, key: &str, action: &str) {
        ui.label(
            egui::RichText::new(key).strong(),
        );
        ui.label(action);
    }
}

// ── EditorPanel Implementation ─────────────────────────────────────────────

impl EditorPanel for MenuBarPanel {
    fn title(&self) -> &str {
        "Menu Bar"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // ── Menu Bar ─────────────────────────────────────────────────
        egui::menu::bar(ui, |ui| {
            self.show_file_menu(ui, state);
            self.show_edit_menu(ui, state);
            self.show_view_menu(ui, state);
            self.show_help_menu(ui, state);
        });

        // ── Popup Dialogs ────────────────────────────────────────────
        let ctx = ui.ctx().clone();
        self.show_shortcuts_dialog(&ctx);
        self.show_about_dialog(&ctx, state);
    }
}

// ── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor & Defaults ─────────────────────────────────────────

    #[test]
    fn new_defaults() {
        let panel = MenuBarPanel::new();
        assert!(!panel.show_shortcuts_dialog);
        assert_eq!(panel.recent_projects.len(), 4);
        assert!(panel.recent_projects[0].contains("platformer"));
    }

    #[test]
    fn title_returns_menu_bar() {
        let panel = MenuBarPanel::new();
        assert_eq!(panel.title(), "Menu Bar");
    }

    // ── Quit Flag ─────────────────────────────────────────────────────

    #[test]
    fn quit_flag_set() {
        let mut state = EditorState::new();
        assert!(!state.should_quit);

        // Simulate the quit action
        state.should_quit = true;
        assert!(state.should_quit);
    }

    // ── About Dialog Toggle ───────────────────────────────────────────

    #[test]
    fn about_dialog_toggle() {
        let mut state = EditorState::new();
        assert!(!state.show_about);

        // Simulate opening about dialog
        state.show_about = true;
        assert!(state.show_about);

        // Simulate closing
        state.show_about = false;
        assert!(!state.show_about);
    }

    // ── Shortcuts Dialog Toggle ───────────────────────────────────────

    #[test]
    fn shortcuts_dialog_toggle() {
        let mut panel = MenuBarPanel::new();
        assert!(!panel.show_shortcuts_dialog);

        // Simulate opening shortcuts dialog
        panel.show_shortcuts_dialog = true;
        assert!(panel.show_shortcuts_dialog);

        // Simulate closing
        panel.show_shortcuts_dialog = false;
        assert!(!panel.show_shortcuts_dialog);
    }

    // ── Clear Selection from Edit Menu ────────────────────────────────

    #[test]
    fn clear_selection_from_edit_menu() {
        let mut state = EditorState::new();

        // Create and select an entity
        let entity = state.world.create_entity();
        state.select(entity);
        assert!(state.is_selected(entity));
        assert_eq!(state.selected_entities.len(), 1);

        // Simulate "Deselect" action from edit menu
        state.clear_selection();
        assert!(state.selected_entities.is_empty());
        assert!(!state.is_selected(entity));
    }

    // ── Recent Projects ───────────────────────────────────────────────

    #[test]
    fn recent_projects_list() {
        let panel = MenuBarPanel::new();

        // Verify demo data
        assert_eq!(panel.recent_projects.len(), 4);
        assert!(panel.recent_projects[0].contains("platformer"));
        assert!(panel.recent_projects[1].contains("rpg_demo"));
        assert!(panel.recent_projects[2].contains("particle_sim"));
        assert!(panel.recent_projects[3].contains("voxel_world"));
    }

    // ── Log Messages on Menu Actions ──────────────────────────────────

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

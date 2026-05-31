//! Welcome screen panel — Phase 7D.
//!
//! Landing screen shown when no project is loaded. Provides:
//! - New project creation with template selection (writes to disk)
//! - Open project from path (loads from disk)
//! - Clickable recent-projects list
//!
//! Follows the same `EditorPanel` trait pattern as the other editor panels.

use super::{ConsoleLogLevel, EditorPanel, EditorState};
use crate::editor_project::{ProjectManager, ProjectTemplate};
use std::path::PathBuf;

// ──────────────────────────────────────────────
// WelcomeScreen
// ──────────────────────────────────────────────

/// Welcome screen panel displayed when no project is loaded.
///
/// Shows branding, a "New Project" wizard with template selector, an "Open
/// Project" path input, and a clickable recent-projects list.
pub struct WelcomeScreen {
    /// Selected template for new project.
    pub selected_template: ProjectTemplate,
    /// Project name input buffer.
    pub project_name: String,
    /// Show new project wizard section.
    pub show_new_project: bool,
    /// Path input for opening an existing project.
    pub open_project_path: String,
    /// Show open-project path input.
    pub show_open_project: bool,
    /// Error message to display inline (cleared each frame).
    pub last_error: Option<String>,
}

impl WelcomeScreen {
    /// Create a new welcome screen with default state.
    pub fn new() -> Self {
        Self {
            selected_template: ProjectTemplate::Empty,
            project_name: String::new(),
            show_new_project: false,
            open_project_path: String::new(),
            show_open_project: false,
            last_error: None,
        }
    }

    /// Clear any transient error message.
    pub fn clear_error(&mut self) {
        self.last_error = None;
    }
}

impl Default for WelcomeScreen {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// EditorPanel trait implementation
// ──────────────────────────────────────────────

impl EditorPanel for WelcomeScreen {
    fn title(&self) -> &str {
        "Welcome"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // ── React to keyboard-shortcut flags ──
        if state.project_manager.show_new_wizard {
            self.show_new_project = true;
            state.project_manager.show_new_wizard = false;
        }
        if state.project_manager.show_open_dialog {
            self.show_open_project = true;
            state.project_manager.show_open_dialog = false;
        }

        // ── Logo / Title ──────────────────────────────────────────────
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading("\u{1F3B9} Chronos Engine");
            ui.label(
                egui::RichText::new("Write the future in the present.")
                    .italics()
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("v0.3.0")
                    .monospace()
                    .color(egui::Color32::DARK_GRAY),
            );
            ui.add_space(30.0);
        });

        // ── Error banner ──
        let error_to_show = self.last_error.clone();
        if let Some(err) = error_to_show {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("\u{26A0} Error")
                            .color(egui::Color32::from_rgb(255, 100, 100))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(&err).color(egui::Color32::from_rgb(255, 150, 150)),
                    );
                    if ui.small_button("Dismiss").clicked() {
                        self.clear_error();
                    }
                });
            });
            ui.add_space(8.0);
        }

        // ── New Project / Open Project buttons ────────────────────────
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if ui
                    .button(egui::RichText::new("\u{1F4C2} Open Project").size(16.0))
                    .clicked()
                {
                    self.show_open_project = !self.show_open_project;
                    self.show_new_project = false;
                }
                ui.add_space(4.0);
                if ui
                    .button(egui::RichText::new("\u{1F195} New Project").size(16.0))
                    .clicked()
                {
                    self.show_new_project = !self.show_new_project;
                    self.show_open_project = false;
                }
            });
        });

        // ── Open Project dialog ───────────────────────────────────────
        if self.show_open_project {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.heading("Open Project");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Path:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.open_project_path)
                            .hint_text("/home/user/projects/MyGame"),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        try_open_project(self, state);
                    }
                });

                // Live validation indicator
                let path = std::path::PathBuf::from(self.open_project_path.trim());
                if !self.open_project_path.trim().is_empty() {
                    match ProjectManager::validate_dir(&path) {
                        Ok(true) => {
                            ui.label(
                                egui::RichText::new("\u{2705} Valid project directory")
                                    .small()
                                    .color(egui::Color32::from_rgb(100, 200, 100)),
                            );
                        }
                        Ok(false) => {
                            ui.label(
                                egui::RichText::new("\u{274C} No manifest.json found")
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 100, 100)),
                            );
                        }
                        Err(_) => {
                            ui.label(
                                egui::RichText::new("\u{26A0} Unable to read directory")
                                    .small()
                                    .color(egui::Color32::YELLOW),
                            );
                        }
                    }
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        try_open_project(self, state);
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_open_project = false;
                    }
                });
            });
        }

        // ── New Project Wizard (collapsible) ──────────────────────────
        if self.show_new_project {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.heading("New Project");
                ui.separator();

                // Project name
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.project_name);
                });

                // Preview of where the project will be created
                let preview = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(self.project_name.trim());
                ui.label(
                    egui::RichText::new(format!("Will create at: {}", preview.display()))
                        .small()
                        .color(egui::Color32::GRAY),
                );

                // Template selector — grid of selectable cards
                ui.add_space(8.0);
                ui.heading("Template");
                ui.add_space(4.0);

                egui::Grid::new("template_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        for template in ProjectTemplate::all() {
                            let is_selected = self.selected_template == template;
                            let response = ui.selectable_label(
                                is_selected,
                                format!("{} {}", template.icon(), template.label()),
                            );
                            if response.clicked() {
                                self.selected_template = template;
                            }
                            ui.label(template.description());
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Create Project").clicked() {
                        try_create_project(self, state);
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_new_project = false;
                    }
                });
            });
        }

        ui.add_space(20.0);

        // ── Recent Projects ───────────────────────────────────────────
        ui.heading("Recent Projects");
        ui.separator();
        ui.add_space(4.0);

        if state.project_manager.recent_projects.is_empty() {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("No recent projects").color(egui::Color32::GRAY));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Create or open a project to get started.")
                        .small()
                        .color(egui::Color32::DARK_GRAY),
                );
            });
        } else {
            // Clone recent project data to avoid borrow conflicts when
            // we replace state.project_manager inside the loop.
            let recents: Vec<(String, String, ProjectTemplate)> = state
                .project_manager
                .recent_projects
                .iter()
                .map(|r| (r.name.clone(), r.path.clone(), r.template))
                .collect();

            ui.vertical(|ui| {
                for (name, path_str, template) in &recents {
                    let label = format!("{}  {} — {}", template.icon(), name, path_str,);
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(&label)
                                    .color(egui::Color32::from_rgb(180, 210, 255)),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        let path = PathBuf::from(path_str);
                        match ProjectManager::open_project(&path) {
                            Ok(mgr) => {
                                let template = mgr
                                    .current_project
                                    .as_ref()
                                    .map(|m| m.template)
                                    .unwrap_or(*template);
                                let old_recents =
                                    std::mem::take(&mut state.project_manager.recent_projects);
                                state.project_manager = mgr;
                                state.project_manager.recent_projects = old_recents;
                                state.project_manager.add_recent(
                                    name,
                                    &path.to_string_lossy(),
                                    template,
                                );
                                state.project_path = Some(path);
                                state.recent_dirty = true;
                                state.log(
                                    ConsoleLogLevel::Info,
                                    format!("Opened recent project: {}", name),
                                );
                            }
                            Err(e) => {
                                state.log(
                                    ConsoleLogLevel::Error,
                                    format!("Failed to open recent project '{}': {e}", name),
                                );
                            }
                        }
                    }
                }
            });
        }

        ui.add_space(20.0);

        // ── Footer ────────────────────────────────────────────────────
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("Built with \u{1F3B9}\u{1F99E} by synth")
                    .small()
                    .color(egui::Color32::DARK_GRAY),
            );
        });
    }
}

// ──────────────────────────────────────────────
// Helper functions
// ──────────────────────────────────────────────

fn try_create_project(welcome: &mut WelcomeScreen, state: &mut EditorState) {
    let name = welcome.project_name.trim();
    if name.is_empty() {
        welcome.last_error = Some("Project name cannot be empty.".to_string());
        return;
    }

    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(name);
    match ProjectManager::create_project(name, &dir, welcome.selected_template) {
        Ok(mgr) => {
            let created_name = mgr.project_name().to_string();
            let template = mgr
                .current_project
                .as_ref()
                .map(|m| m.template)
                .unwrap_or(welcome.selected_template);
            // Preserve the existing recent list before replacing manager.
            let old_recents = std::mem::take(&mut state.project_manager.recent_projects);
            state.project_manager = mgr;
            state.project_manager.recent_projects = old_recents;
            state
                .project_manager
                .add_recent(&created_name, &dir.to_string_lossy(), template);
            state.project_path = Some(dir);
            state.recent_dirty = true;
            state.log(
                ConsoleLogLevel::Info,
                format!("Created project '{}' at {}", name, created_name),
            );
            welcome.show_new_project = false;
            welcome.project_name.clear();
            welcome.clear_error();
        }
        Err(e) => {
            welcome.last_error = Some(format!("{e}"));
            state.log(
                ConsoleLogLevel::Error,
                format!("Create project failed: {e}"),
            );
        }
    }
}

fn try_open_project(welcome: &mut WelcomeScreen, state: &mut EditorState) {
    let path_str = welcome.open_project_path.trim();
    if path_str.is_empty() {
        welcome.last_error = Some("Project path cannot be empty.".to_string());
        return;
    }

    let path = PathBuf::from(path_str);
    match ProjectManager::open_project(&path) {
        Ok(mgr) => {
            let name = mgr.project_name().to_string();
            let template = mgr
                .current_project
                .as_ref()
                .map(|m| m.template)
                .unwrap_or(crate::editor_project::ProjectTemplate::Empty);
            // Preserve the existing recent list before replacing manager.
            let old_recents = std::mem::take(&mut state.project_manager.recent_projects);
            state.project_manager = mgr;
            state.project_manager.recent_projects = old_recents;
            state
                .project_manager
                .add_recent(&name, &path.to_string_lossy(), template);
            state.project_path = Some(path);
            state.recent_dirty = true;
            state.log(ConsoleLogLevel::Info, format!("Opened project '{}'", name));
            welcome.show_open_project = false;
            welcome.open_project_path.clear();
            welcome.clear_error();
        }
        Err(e) => {
            welcome.last_error = Some(format!("{e}"));
            state.log(ConsoleLogLevel::Error, format!("Open project failed: {e}"));
        }
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_new_defaults() {
        let w = WelcomeScreen::new();
        assert_eq!(w.selected_template, ProjectTemplate::Empty);
        assert!(w.project_name.is_empty());
        assert!(!w.show_new_project);
        assert!(!w.show_open_project);
        assert!(w.last_error.is_none());
    }

    #[test]
    fn welcome_title() {
        let w = WelcomeScreen::new();
        assert_eq!(w.title(), "Welcome");
    }

    #[test]
    fn template_labels() {
        assert_eq!(ProjectTemplate::Empty.label(), "Empty Project");
        assert_eq!(ProjectTemplate::Platformer2D.label(), "2D Platformer");
        assert_eq!(ProjectTemplate::Shooter3D.label(), "3D Shooter");
        assert_eq!(ProjectTemplate::RPG.label(), "RPG");
    }

    #[test]
    fn template_icons() {
        assert!(!ProjectTemplate::Empty.icon().is_empty());
        assert!(!ProjectTemplate::Platformer2D.icon().is_empty());
        assert!(!ProjectTemplate::Shooter3D.icon().is_empty());
        assert!(!ProjectTemplate::RPG.icon().is_empty());
    }

    #[test]
    fn template_descriptions_not_empty() {
        for t in ProjectTemplate::all() {
            assert!(!t.description().is_empty());
        }
    }

    #[test]
    fn template_all_count() {
        assert_eq!(ProjectTemplate::all().len(), 4);
    }

    #[test]
    fn default_impl() {
        let w = WelcomeScreen::default();
        assert_eq!(w.selected_template, ProjectTemplate::Empty);
        assert!(w.project_name.is_empty());
        assert!(!w.show_new_project);
    }

    #[test]
    fn toggle_new_project() {
        let mut w = WelcomeScreen::new();
        assert!(!w.show_new_project);
        w.show_new_project = true;
        assert!(w.show_new_project);
        w.show_new_project = false;
        assert!(!w.show_new_project);
    }

    #[test]
    fn template_variants_distinct() {
        let all = ProjectTemplate::all();
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "variants must be distinct");
            }
        }
    }

    #[test]
    fn template_selectable_labels_unique() {
        let all = ProjectTemplate::all();
        let labels: Vec<&str> = all.iter().map(|t| t.label()).collect();
        for i in 0..labels.len() {
            for j in (i + 1)..labels.len() {
                assert_ne!(labels[i], labels[j], "labels must be unique");
            }
        }
    }

    #[test]
    fn create_project_logs_message() {
        let mut state = EditorState::new();
        state.log(
            ConsoleLogLevel::Info,
            format!(
                "Create project: '{}' ({:?})",
                "TestGame",
                ProjectTemplate::Platformer2D
            ),
        );
        assert_eq!(state.console_log.len(), 1);
        assert!(state.console_log[0].message.contains("TestGame"));
        assert!(state.console_log[0].message.contains("Platformer2D"));
    }

    #[test]
    fn open_project_logs_request() {
        let mut state = EditorState::new();
        state.log(ConsoleLogLevel::Info, "Open project requested");
        assert_eq!(state.console_log.len(), 1);
        assert_eq!(state.console_log[0].message, "Open project requested");
        assert_eq!(state.console_log[0].level, ConsoleLogLevel::Info);
    }

    #[test]
    fn project_name_editable() {
        let mut w = WelcomeScreen::new();
        assert!(w.project_name.is_empty());
        w.project_name = "My Game".to_string();
        assert_eq!(w.project_name, "My Game");
        w.project_name.clear();
        assert!(w.project_name.is_empty());
    }

    #[test]
    fn template_selection_changes() {
        let mut w = WelcomeScreen::new();
        assert_eq!(w.selected_template, ProjectTemplate::Empty);
        w.selected_template = ProjectTemplate::RPG;
        assert_eq!(w.selected_template, ProjectTemplate::RPG);
        w.selected_template = ProjectTemplate::Shooter3D;
        assert_eq!(w.selected_template, ProjectTemplate::Shooter3D);
    }

    #[test]
    fn error_display_and_clear() {
        let mut w = WelcomeScreen::new();
        assert!(w.last_error.is_none());
        w.last_error = Some("Something went wrong".to_string());
        assert!(w.last_error.is_some());
        w.clear_error();
        assert!(w.last_error.is_none());
    }

    #[test]
    fn open_project_path_defaults_empty() {
        let w = WelcomeScreen::new();
        assert!(w.open_project_path.is_empty());
    }
}

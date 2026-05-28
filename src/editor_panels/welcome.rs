//! Welcome screen panel — Phase 7D.
//!
//! Landing screen shown when no project is loaded. Provides:
//! - New project creation with template selection
//! - Open project action
//! - Recent projects placeholder
//!
//! Follows the same `EditorPanel` trait pattern as the other editor panels.

use super::{ConsoleLogLevel, EditorPanel, EditorState};

// ──────────────────────────────────────────────
// ProjectTemplate
// ──────────────────────────────────────────────

/// Template type for new projects (mirrors editor_project::ProjectTemplate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    /// Blank project — no pre-built systems.
    Empty,
    /// 2D side-scrolling platformer scaffold.
    Platformer2D,
    /// First-person 3D shooter scaffold.
    Shooter3D,
    /// Role-playing game scaffold with stats / inventory.
    RPG,
}

impl ProjectTemplate {
    /// Human-readable label.
    pub fn label(&self) -> &str {
        match self {
            Self::Empty => "Empty Project",
            Self::Platformer2D => "2D Platformer",
            Self::Shooter3D => "3D Shooter",
            Self::RPG => "RPG",
        }
    }

    /// One-line description of what the template provides.
    pub fn description(&self) -> &str {
        match self {
            Self::Empty => "A clean slate — no pre-built systems.",
            Self::Platformer2D => "2D side-scroller with physics and input.",
            Self::Shooter3D => "First-person 3D shooter with weapon system.",
            Self::RPG => "RPG with stats, inventory, and dialogue.",
        }
    }

    /// Emoji icon for the template card.
    pub fn icon(&self) -> &str {
        match self {
            Self::Empty => "\u{1F4C2}",         // 📂
            Self::Platformer2D => "\u{1F3AE}",   // 🎮
            Self::Shooter3D => "\u{1F52B}",      // 🔫
            Self::RPG => "\u{2694}\u{FE0F}",     // ⚔️
        }
    }

    /// All available template variants.
    pub fn all() -> [ProjectTemplate; 4] {
        [Self::Empty, Self::Platformer2D, Self::Shooter3D, Self::RPG]
    }
}

// ──────────────────────────────────────────────
// WelcomeScreen
// ──────────────────────────────────────────────

/// Welcome screen panel displayed when no project is loaded.
///
/// Shows branding, a "New Project" wizard with template selector, an "Open
/// Project" button, and a placeholder recent-projects list.
pub struct WelcomeScreen {
    /// Selected template for new project.
    pub selected_template: ProjectTemplate,
    /// Project name input buffer.
    pub project_name: String,
    /// Show new project wizard section.
    pub show_new_project: bool,
}

impl WelcomeScreen {
    /// Create a new welcome screen with default state.
    pub fn new() -> Self {
        Self {
            selected_template: ProjectTemplate::Empty,
            project_name: String::new(),
            show_new_project: false,
        }
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

        // ── New Project / Open Project buttons ────────────────────────
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if ui
                    .button(egui::RichText::new("\u{1F4C2} Open Project").size(16.0))
                    .clicked()
                {
                    state.log(ConsoleLogLevel::Info, "Open project requested");
                }
                ui.add_space(4.0);
                if ui
                    .button(egui::RichText::new("\u{1F195} New Project").size(16.0))
                    .clicked()
                {
                    self.show_new_project = !self.show_new_project;
                }
            });
        });

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
                    if ui.button("Create Project").clicked()
                        && !self.project_name.trim().is_empty()
                    {
                        state.log(
                            ConsoleLogLevel::Info,
                            format!(
                                "Create project: '{}' ({:?})",
                                self.project_name, self.selected_template
                            ),
                        );
                        self.show_new_project = false;
                        self.project_name.clear();
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

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("No recent projects").color(egui::Color32::GRAY),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Create or open a project to get started.")
                    .small()
                    .color(egui::Color32::DARK_GRAY),
            );
        });

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
        assert_eq!(ProjectTemplate::Empty.icon(), "\u{1F4C2}");
        assert_eq!(ProjectTemplate::Platformer2D.icon(), "\u{1F3AE}");
        assert_eq!(ProjectTemplate::Shooter3D.icon(), "\u{1F52B}");
        assert_eq!(ProjectTemplate::RPG.icon(), "\u{2694}\u{FE0F}");
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
        // Ensure each variant is unique
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
                "TestGame", ProjectTemplate::Platformer2D
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
}

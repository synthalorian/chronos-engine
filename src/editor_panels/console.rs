//! Console panel — Phase 7B.
//!
//! Interactive developer console with command input, log-level filtering,
//! scrollable output, and command history. Built-in commands cover entity
//! inspection, console management, and general debugging.

use super::{ConsoleEntry, ConsoleLogLevel, EditorPanel, EditorState};

// ── Console Panel ──────────────────────────────────────────────────────────

/// Developer console panel.
///
/// Displays timestamped log entries with severity-based filtering and accepts
/// typed commands via an input bar. Supports command history navigation with
/// arrow keys.
pub struct ConsolePanel {
    /// Text buffer for the command input field.
    command_input: String,
    /// Current vertical scroll position in the log area.
    scrollback_position: f32,
    /// Whether to auto-scroll to the latest entry when new logs arrive.
    auto_scroll: bool,
    /// Whether to display `[INFO]` entries.
    filter_info: bool,
    /// Whether to display `[WARN]` entries.
    filter_warn: bool,
    /// Whether to display `[ERROR]` entries.
    filter_error: bool,
    /// Current position in command history (`None` = typing new input).
    history_index: Option<usize>,
    /// Maximum number of log entries before oldest are trimmed.
    max_entries: usize,
}

impl ConsolePanel {
    /// Create a new console panel with default settings.
    pub fn new() -> Self {
        Self {
            command_input: String::new(),
            scrollback_position: 0.0,
            auto_scroll: true,
            filter_info: true,
            filter_warn: true,
            filter_error: true,
            history_index: None,
            max_entries: 10_000,
        }
    }

    // ── Command Processing ─────────────────────────────────────────────────

    /// Process a console command string.
    ///
    /// Parses the command, executes the corresponding action, and writes
    /// output back to `state.console_log`. Does **not** modify command
    /// history — the caller is responsible for that.
    fn process_command(&self, cmd: &str, state: &mut EditorState) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let command = parts[0];
        let args = parts.get(1).copied().unwrap_or("");

        match command {
            "help" => {
                state.log(ConsoleLogLevel::Info, "Available commands:");
                state.log(ConsoleLogLevel::Info, "  help        — Show this message");
                state.log(ConsoleLogLevel::Info, "  clear       — Clear console output");
                state.log(ConsoleLogLevel::Info, "  entities    — Show entity count");
                state.log(ConsoleLogLevel::Info, "  fps         — Show FPS hint");
                state.log(ConsoleLogLevel::Info, "  echo <text> — Print text to console");
                state.log(ConsoleLogLevel::Info, "  select <id> — Select entity by index");
            }
            "clear" => {
                state.console_log.clear();
            }
            "entities" => {
                let count = state.world.entity_count();
                state.log(ConsoleLogLevel::Info, format!("Entity count: {count}"));
            }
            "fps" => {
                state.log(ConsoleLogLevel::Info, "FPS: (not available in headless mode)");
            }
            "echo" => {
                if args.is_empty() {
                    state.log(ConsoleLogLevel::Warn, "Usage: echo <text>");
                } else {
                    state.log(ConsoleLogLevel::Info, args.to_string());
                }
            }
            "select" => {
                if args.is_empty() {
                    state.log(ConsoleLogLevel::Warn, "Usage: select <entity_index>");
                } else {
                    match args.parse::<u32>() {
                        Ok(index) => {
                            let entity = state.world.entity_from_index(index);
                            if state.world.entity_exists(entity) {
                                state.select(entity);
                                state.log(
                                    ConsoleLogLevel::Info,
                                    format!("Selected entity {index}"),
                                );
                            } else {
                                state.log(
                                    ConsoleLogLevel::Warn,
                                    format!("No alive entity at index {index}"),
                                );
                            }
                        }
                        Err(_) => {
                            state.log(ConsoleLogLevel::Warn, format!("Invalid index: {args}"));
                        }
                    }
                }
            }
            _ => {
                state.log(
                    ConsoleLogLevel::Warn,
                    format!(
                        "Unknown command: {command}. Type 'help' for available commands."
                    ),
                );
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    /// Trim the console log to `max_entries`, removing oldest entries first.
    fn trim_log(&self, state: &mut EditorState) {
        if state.console_log.len() > self.max_entries {
            let excess = state.console_log.len() - self.max_entries;
            state.console_log.drain(0..excess);
        }
    }

    /// Format a timestamp (in seconds) as `HH:MM:SS`.
    fn format_timestamp(secs: f64) -> String {
        let total = secs as u64;
        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        let seconds = total % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }

    /// Returns the egui color for a given log level.
    fn level_color(level: ConsoleLogLevel) -> egui::Color32 {
        match level {
            ConsoleLogLevel::Info => egui::Color32::WHITE,
            ConsoleLogLevel::Warn => egui::Color32::YELLOW,
            ConsoleLogLevel::Error => egui::Color32::from_rgb(255, 80, 80),
        }
    }

    /// Returns the bracket tag string for a log level.
    fn level_tag(level: ConsoleLogLevel) -> &'static str {
        match level {
            ConsoleLogLevel::Info => "[INFO]",
            ConsoleLogLevel::Warn => "[WARN]",
            ConsoleLogLevel::Error => "[ERROR]",
        }
    }

    /// Whether a given entry passes the current filter settings.
    fn entry_visible(&self, entry: &ConsoleEntry) -> bool {
        match entry.level {
            ConsoleLogLevel::Info => self.filter_info,
            ConsoleLogLevel::Warn => self.filter_warn,
            ConsoleLogLevel::Error => self.filter_error,
        }
    }
}

// ── EditorPanel Implementation ─────────────────────────────────────────────

impl EditorPanel for ConsolePanel {
    fn title(&self) -> &str {
        "Console"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Trim old entries if over limit.
        self.trim_log(state);

        // ── Toolbar ────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Filter:");

            // Info filter toggle
            let info_color = if self.filter_info {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(80, 80, 80)
            };
            if ui
                .button(egui::RichText::new("Info").color(info_color))
                .clicked()
            {
                self.filter_info = !self.filter_info;
            }

            // Warn filter toggle
            let warn_color = if self.filter_warn {
                egui::Color32::YELLOW
            } else {
                egui::Color32::from_rgb(80, 80, 0)
            };
            if ui
                .button(egui::RichText::new("Warn").color(warn_color))
                .clicked()
            {
                self.filter_warn = !self.filter_warn;
            }

            // Error filter toggle
            let err_color = if self.filter_error {
                egui::Color32::from_rgb(255, 80, 80)
            } else {
                egui::Color32::from_rgb(80, 30, 30)
            };
            if ui
                .button(egui::RichText::new("Error").color(err_color))
                .clicked()
            {
                self.filter_error = !self.filter_error;
            }

            ui.separator();

            // Clear button
            if ui.button("Clear").clicked() {
                state.console_log.clear();
            }

            ui.separator();

            // Entry count (visible / total)
            let visible = state
                .console_log
                .iter()
                .filter(|e| self.entry_visible(e))
                .count();
            let total = state.console_log.len();
            ui.label(format!("{visible}/{total} entries"));
        });

        ui.separator();

        // ── Log Area ───────────────────────────────────────────────────
        let available_height = ui.available_height() - 30.0; // reserve for input bar
        egui::ScrollArea::vertical()
            .stick_to_bottom(self.auto_scroll)
            .id_salt("console_log_scroll")
            .max_height(available_height)
            .show(ui, |ui| {
                let dim = egui::Color32::from_rgb(120, 120, 120);
                for entry in &state.console_log {
                    if !self.entry_visible(entry) {
                        continue;
                    }

                    let color = Self::level_color(entry.level);
                    let tag = Self::level_tag(entry.level);
                    let timestamp = Self::format_timestamp(entry.timestamp_secs);

                    ui.horizontal(|ui| {
                        ui.colored_label(dim, format!("{timestamp} "));
                        ui.colored_label(color, format!("{tag} "));
                        ui.colored_label(color, &entry.message);
                    });
                }
            });

        ui.separator();

        // ── Command Input ──────────────────────────────────────────────
        let mut submit = false;

        ui.horizontal(|ui| {
            let text_response = ui.add(
                egui::TextEdit::singleline(&mut self.command_input)
                    .hint_text("Enter command...")
                    .desired_width(ui.available_width() - 40.0)
                    .interactive(true),
            );

            // Arrow-up: walk backward through command history
            if text_response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::ArrowUp))
            {
                if !state.console_history.is_empty() {
                    self.history_index = Some(match self.history_index {
                        None => state.console_history.len() - 1,
                        Some(i) if i > 0 => i - 1,
                        Some(i) => i,
                    });
                    if let Some(idx) = self.history_index {
                        self.command_input = state.console_history[idx].clone();
                    }
                }
            }

            // Arrow-down: walk forward through command history
            if text_response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
            {
                if let Some(idx) = self.history_index {
                    if idx + 1 < state.console_history.len() {
                        self.history_index = Some(idx + 1);
                        self.command_input = state.console_history[idx + 1].clone();
                    } else {
                        self.history_index = None;
                        self.command_input.clear();
                    }
                }
            }

            // Submit on Enter key
            if text_response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                submit = true;
            }

            // Submit button (▶)
            if ui.button("▶").clicked() {
                submit = true;
            }
        });

        // Execute the command if requested.
        if submit {
            let cmd = self.command_input.trim().to_string();
            if !cmd.is_empty() {
                state.console_history.push(cmd.clone());
                self.history_index = None;
                self.process_command(&cmd, state);
                self.command_input.clear();
                self.auto_scroll = true;
            }
        }
    }
}

// ── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    

    // ── Constructor & Defaults ─────────────────────────────────────────

    #[test]
    fn new_defaults() {
        let panel = ConsolePanel::new();
        assert!(panel.command_input.is_empty());
        assert_eq!(panel.scrollback_position, 0.0);
        assert!(panel.auto_scroll);
        assert!(panel.filter_info);
        assert!(panel.filter_warn);
        assert!(panel.filter_error);
        assert!(panel.history_index.is_none());
        assert_eq!(panel.max_entries, 10_000);
    }

    #[test]
    fn title_returns_console() {
        let panel = ConsolePanel::new();
        assert_eq!(panel.title(), "Console");
    }

    // ── Filter Toggles ────────────────────────────────────────────────

    #[test]
    fn filter_toggles() {
        let mut panel = ConsolePanel::new();
        let _state = EditorState::new();

        // All visible by default
        let info_entry = ConsoleEntry {
            level: ConsoleLogLevel::Info,
            message: "hi".into(),
            timestamp_secs: 0.0,
        };
        let warn_entry = ConsoleEntry {
            level: ConsoleLogLevel::Warn,
            message: "careful".into(),
            timestamp_secs: 1.0,
        };
        let err_entry = ConsoleEntry {
            level: ConsoleLogLevel::Error,
            message: "boom".into(),
            timestamp_secs: 2.0,
        };

        assert!(panel.entry_visible(&info_entry));
        assert!(panel.entry_visible(&warn_entry));
        assert!(panel.entry_visible(&err_entry));

        // Toggle info off
        panel.filter_info = false;
        assert!(!panel.entry_visible(&info_entry));
        assert!(panel.entry_visible(&warn_entry));

        // Toggle warn off
        panel.filter_warn = false;
        assert!(!panel.entry_visible(&warn_entry));

        // Toggle error off
        panel.filter_error = false;
        assert!(!panel.entry_visible(&err_entry));

        // Re-enable all
        panel.filter_info = true;
        panel.filter_warn = true;
        panel.filter_error = true;
        assert!(panel.entry_visible(&info_entry));
        assert!(panel.entry_visible(&warn_entry));
        assert!(panel.entry_visible(&err_entry));
    }

    // ── Command Processing ────────────────────────────────────────────

    #[test]
    fn command_help() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();
        panel.process_command("help", &mut state);

        // Should produce at least the header + 6 command descriptions
        assert!(state.console_log.len() >= 7);
        assert_eq!(state.console_log[0].level, ConsoleLogLevel::Info);
        assert!(state.console_log[0].message.contains("Available commands"));
    }

    #[test]
    fn command_clear() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        state.log(ConsoleLogLevel::Info, "before clear");
        assert_eq!(state.console_log.len(), 1);

        panel.process_command("clear", &mut state);
        assert!(state.console_log.is_empty());
    }

    #[test]
    fn command_echo() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        panel.process_command("echo hello world", &mut state);
        assert_eq!(state.console_log.len(), 1);
        assert_eq!(state.console_log[0].message, "hello world");
        assert_eq!(state.console_log[0].level, ConsoleLogLevel::Info);

        // echo with no args warns
        let mut state2 = EditorState::new();
        panel.process_command("echo", &mut state2);
        assert_eq!(state2.console_log[0].level, ConsoleLogLevel::Warn);
    }

    #[test]
    fn command_entities() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        // Empty world
        panel.process_command("entities", &mut state);
        assert!(state.console_log[0].message.contains("0"));

        // With entities
        state.world.create_entity();
        state.world.create_entity();
        state.console_log.clear();
        panel.process_command("entities", &mut state);
        assert!(state.console_log[0].message.contains("2"));
    }

    #[test]
    fn command_unknown() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        panel.process_command("blarghle", &mut state);
        assert_eq!(state.console_log.len(), 1);
        assert_eq!(state.console_log[0].level, ConsoleLogLevel::Warn);
        assert!(state.console_log[0].message.contains("Unknown command: blarghle"));
        assert!(state.console_log[0].message.contains("Type 'help'"));
    }

    #[test]
    fn command_select() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        // Create an entity at index 0
        let e = state.world.create_entity();
        panel.process_command("select 0", &mut state);

        assert!(state.is_selected(e));
        // Should log a success info
        let log = &state.console_log;
        assert!(log.iter().any(|l| l.message.contains("Selected entity 0")));

        // Select non-existent index
        let mut state2 = EditorState::new();
        panel.process_command("select 99", &mut state2);
        assert!(state2
            .console_log
            .iter()
            .any(|l| l.message.contains("No alive entity")));

        // Select with invalid arg
        let mut state3 = EditorState::new();
        panel.process_command("select abc", &mut state3);
        assert!(state3
            .console_log
            .iter()
            .any(|l| l.message.contains("Invalid index")));
    }

    // ── History Navigation ────────────────────────────────────────────

    #[test]
    fn history_index_navigation() {
        let mut panel = ConsolePanel::new();
        let mut state = EditorState::new();

        // Simulate submitting several commands
        state.console_history.push("cmd_one".into());
        state.console_history.push("cmd_two".into());
        state.console_history.push("cmd_three".into());

        // Start at None (new input)
        assert!(panel.history_index.is_none());

        // Navigate up: should land on last entry (index 2)
        panel.history_index = Some(
            match panel.history_index {
                None => state.console_history.len() - 1,
                Some(i) if i > 0 => i - 1,
                Some(i) => i,
            },
        );
        assert_eq!(panel.history_index, Some(2));

        // Navigate up again: index 1
        panel.history_index = Some(
            match panel.history_index {
                None => state.console_history.len() - 1,
                Some(i) if i > 0 => i - 1,
                Some(i) => i,
            },
        );
        assert_eq!(panel.history_index, Some(1));

        // Navigate up again: index 0
        panel.history_index = Some(
            match panel.history_index {
                None => state.console_history.len() - 1,
                Some(i) if i > 0 => i - 1,
                Some(i) => i,
            },
        );
        assert_eq!(panel.history_index, Some(0));

        // Navigate down: back to index 1
        if let Some(idx) = panel.history_index {
            if idx + 1 < state.console_history.len() {
                panel.history_index = Some(idx + 1);
            }
        }
        assert_eq!(panel.history_index, Some(1));

        // Navigate down past end: reset to None
        if let Some(idx) = panel.history_index {
            if idx + 1 < state.console_history.len() {
                panel.history_index = Some(idx + 1);
            } else {
                panel.history_index = None;
            }
        }
        assert_eq!(panel.history_index, Some(2));

        // One more down — now past the end
        if let Some(idx) = panel.history_index {
            if idx + 1 < state.console_history.len() {
                panel.history_index = Some(idx + 1);
            } else {
                panel.history_index = None;
            }
        }
        assert!(panel.history_index.is_none());
    }

    // ── Max Entry Trimming ────────────────────────────────────────────

    #[test]
    fn max_entry_trimming() {
        let mut panel = ConsolePanel::new();
        panel.max_entries = 5;
        let mut state = EditorState::new();

        // Add 8 entries — should trim to 5
        for i in 0..8 {
            state.log(ConsoleLogLevel::Info, format!("entry {i}"));
        }
        panel.trim_log(&mut state);
        assert_eq!(state.console_log.len(), 5);

        // Oldest entries should be gone; we keep entries 3–7
        assert!(state.console_log[0].message.contains("entry 3"));
        assert!(state.console_log[4].message.contains("entry 7"));
    }

    #[test]
    fn trim_does_nothing_below_limit() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        state.log(ConsoleLogLevel::Info, "only one");
        panel.trim_log(&mut state);
        assert_eq!(state.console_log.len(), 1);
    }

    // ── Timestamp Formatting ──────────────────────────────────────────

    #[test]
    fn format_timestamp_basic() {
        assert_eq!(ConsolePanel::format_timestamp(0.0), "00:00:00");
        assert_eq!(ConsolePanel::format_timestamp(65.0), "00:01:05");
        assert_eq!(ConsolePanel::format_timestamp(3661.0), "01:01:01");
        assert_eq!(ConsolePanel::format_timestamp(86399.0), "23:59:59");
    }

    // ── Empty Command ─────────────────────────────────────────────────

    #[test]
    fn empty_command_is_noop() {
        let panel = ConsolePanel::new();
        let mut state = EditorState::new();

        panel.process_command("", &mut state);
        panel.process_command("   ", &mut state);
        assert!(state.console_log.is_empty());
    }
}

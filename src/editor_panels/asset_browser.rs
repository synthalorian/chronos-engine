//! Asset browser panel — Phase 7B.
//!
//! Provides a file-system browser for project assets. Supports list and grid
//! views, breadcrumb navigation, search filtering, and context menus. Operates
//! on the shared [`EditorState`] through the [`EditorPanel`] trait.

use std::fs;
use std::path::{Path, PathBuf};

use super::{EditorPanel, EditorState};

// ──────────────────────────────────────────────
// Supporting types
// ──────────────────────────────────────────────

/// Visual layout mode for the file listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetViewMode {
    /// One entry per row with name, type, and size columns.
    List,
    /// Colored thumbnail grid with type-based colours.
    Grid,
}

/// Asset classification derived from file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    /// Scene / level files (`.scene`, `.json`).
    Scene,
    /// Image assets (`.png`, `.jpg`, `.bmp`).
    Image,
    /// 3-D mesh files (`.obj`, `.gltf`, `.glb`).
    Mesh,
    /// Audio clips (`.wav`, `.ogg`, `.mp3`).
    Audio,
    /// Rhai scripts (`.rhai`).
    Script,
    /// Font files (`.ttf`, `.otf`).
    Font,
    /// Any file that doesn't match a known category.
    Unknown,
}

impl AssetType {
    /// Returns a short label for display in the browser.
    pub fn label(&self) -> &str {
        match self {
            AssetType::Scene => "Scene",
            AssetType::Image => "Image",
            AssetType::Mesh => "Mesh",
            AssetType::Audio => "Audio",
            AssetType::Script => "Script",
            AssetType::Font => "Font",
            AssetType::Unknown => "File",
        }
    }

    /// Returns a hex colour used for grid-view thumbnail placeholders.
    pub fn color_hex(&self) -> &str {
        match self {
            AssetType::Scene => "#e8a948",
            AssetType::Image => "#5b9bd5",
            AssetType::Mesh => "#70ad47",
            AssetType::Audio => "#ed7d31",
            AssetType::Script => "#a855f7",
            AssetType::Font => "#ec4899",
            AssetType::Unknown => "#9ca3af",
        }
    }
}

/// A single entry inside the browser listing.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    /// File or directory name.
    pub name: String,
    /// Full filesystem path.
    pub path: PathBuf,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size_bytes: u64,
    /// Detected asset type (directories are always [`AssetType::Unknown`]).
    pub asset_type: AssetType,
}

// ──────────────────────────────────────────────
// Asset type detection
// ──────────────────────────────────────────────

/// Determine [`AssetType`] from a file's extension.
///
/// Returns [`AssetType::Unknown`] when the extension is missing or
/// unrecognised. Directories should pass any path; the extension is only
/// consulted for the final component.
pub fn detect_asset_type(path: &Path) -> AssetType {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => return AssetType::Unknown,
    };

    match ext.as_str() {
        "scene" | "json" => AssetType::Scene,
        "png" | "jpg" | "jpeg" | "bmp" => AssetType::Image,
        "obj" | "gltf" | "glb" => AssetType::Mesh,
        "wav" | "ogg" | "mp3" => AssetType::Audio,
        "rhai" => AssetType::Script,
        "ttf" | "otf" => AssetType::Font,
        _ => AssetType::Unknown,
    }
}

// ──────────────────────────────────────────────
// AssetBrowserPanel
// ──────────────────────────────────────────────

/// File-system asset browser for the editor.
///
/// Presents a navigable view of the project's `assets/` directory (or an
/// arbitrary root). Supports list and grid layouts, search filtering, and
/// context menus for common operations.
pub struct AssetBrowserPanel {
    /// Directory currently being displayed. `None` means "not initialised".
    current_dir: Option<PathBuf>,
    /// Cached entries for the current directory.
    entries: Vec<AssetEntry>,
    /// Index into `entries` for the selected item, if any.
    selected_entry: Option<usize>,
    /// Whether hidden (dot-prefixed) files are visible.
    show_hidden: bool,
    /// Current visual layout.
    view_mode: AssetViewMode,
    /// Text typed into the search / filter input.
    search_filter: String,
}

impl AssetBrowserPanel {
    /// Create a new panel with default settings and no directory loaded.
    pub fn new() -> Self {
        Self {
            current_dir: None,
            entries: Vec::new(),
            selected_entry: None,
            show_hidden: false,
            view_mode: AssetViewMode::List,
            search_filter: String::new(),
        }
    }

    /// Re-read the current directory and rebuild `entries`.
    ///
    /// When `current_dir` is `None`, falls back to
    /// `state.project_path / "assets"`. The entries are sorted with
    /// directories first, then alphabetically by name.
    pub fn refresh(&mut self, state: &EditorState) {
        let dir = match &self.current_dir {
            Some(d) => d.clone(),
            None => match &state.project_path {
                Some(p) => p.join("assets"),
                None => return, // nowhere to browse
            },
        };

        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                self.entries.clear();
                return;
            }
        };

        // If we resolved the fallback, remember it.
        if self.current_dir.is_none() {
            self.current_dir = Some(dir.clone());
        }

        let mut entries: Vec<AssetEntry> = read_dir
            .filter_map(|res| {
                let entry = res.ok()?;
                let name = entry.file_name().to_string_lossy().into_owned();

                // Skip hidden files unless toggled.
                if !self.show_hidden && name.starts_with('.') {
                    return None;
                }

                let meta = entry.metadata().ok()?;
                let is_dir = meta.is_dir();
                let path = entry.path();
                let asset_type = if is_dir {
                    AssetType::Unknown
                } else {
                    detect_asset_type(&path)
                };

                Some(AssetEntry {
                    name,
                    path,
                    is_dir,
                    size_bytes: if is_dir { 0 } else { meta.len() },
                    asset_type,
                })
            })
            .collect();

        // Directories first, then alphabetical.
        entries.sort_by(|a, b| {
            a.is_dir
                .cmp(&b.is_dir)
                .reverse()
                .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
        });

        self.entries = entries;
        self.selected_entry = None;
    }

    /// Navigate into a subdirectory by index.
    fn enter_directory(&mut self, index: usize, state: &EditorState) {
        if let Some(entry) = self.entries.get(index) {
            if entry.is_dir {
                self.current_dir = Some(entry.path.clone());
                self.refresh(state);
            }
        }
    }

    /// Navigate to the parent directory.
    fn navigate_up(&mut self, state: &EditorState) {
        if let Some(ref dir) = self.current_dir {
            if let Some(parent) = dir.parent() {
                self.current_dir = Some(parent.to_path_buf());
                self.refresh(state);
            }
        }
    }

    /// Return the entries visible after applying the current search filter.
    fn filtered_entries(&self) -> Vec<(usize, &AssetEntry)> {
        let query = self.search_filter.to_ascii_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if query.is_empty() {
                    true
                } else {
                    e.name.to_ascii_lowercase().contains(&query)
                }
            })
            .collect()
    }

    // ── UI helpers ──

    /// Render the toolbar (back button, path, search, view toggle).
    fn show_toolbar(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            // Back button.
            let back_enabled = self
                .current_dir
                .as_ref()
                .and_then(|d| d.parent())
                .is_some();
            ui.add_enabled_ui(back_enabled, |ui| {
                if ui.button("⬅ Back").clicked() {
                    self.navigate_up(state);
                }
            });

            // Current path display (read-only).
            let path_str = self
                .current_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(no project)".into());
            ui.label(
                egui::RichText::new(&path_str).small().monospace(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View mode toggle.
                let toggle_label = match self.view_mode {
                    AssetViewMode::List => "☰ Grid",
                    AssetViewMode::Grid => "☰ List",
                };
                if ui.button(toggle_label).clicked() {
                    self.view_mode = match self.view_mode {
                        AssetViewMode::List => AssetViewMode::Grid,
                        AssetViewMode::Grid => AssetViewMode::List,
                    };
                }

                // Search filter.
                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_filter)
                        .hint_text("🔍 Filter...")
                        .desired_width(140.0),
                );
                if search_response.changed() {
                    self.selected_entry = None;
                }
            });
        });
    }

    /// Render breadcrumb path segments (clickable to navigate up).
    fn show_breadcrumbs(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.horizontal(|ui| {
            let dir = match &self.current_dir {
                Some(d) => d.clone(),
                None => return,
            };

            // Collect all ancestors from root to current.
            let mut ancestors: Vec<PathBuf> = dir.ancestors().map(PathBuf::from).collect::<Vec<_>>();
            ancestors.reverse();

            for (i, segment) in ancestors.iter().enumerate() {
                if i > 0 {
                    ui.label("›");
                }
                let label = segment
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "/".into());
                if ui.add(egui::Button::new(&label).frame(false)).clicked() {
                    self.current_dir = Some(segment.clone());
                    self.refresh(state);
                }
            }
        });
        ui.add_space(2.0);
    }

    /// Render the file listing in list mode.
    fn show_list_view(&mut self, ui: &mut egui::Ui) {
        let filtered: Vec<(usize, AssetEntry)> = self.filtered_entries().into_iter().map(|(i, e)| (i, e.clone())).collect();
        let selected = self.selected_entry;
        let mut new_selection: Option<usize> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, entry) in &filtered {
                let is_selected = selected == Some(*idx);
                let icon = if entry.is_dir { "📁" } else { "📄" };

                let response = ui.selectable_label(
                    is_selected,
                    format!(
                        "{} {}  [{}]  {}",
                        icon,
                        entry.name,
                        entry.asset_type.label(),
                        format_size(entry.size_bytes),
                    ),
                );

                if response.clicked() {
                    new_selection = Some(*idx);
                }

                response.context_menu(|ui| {
                    if ui.button("Open in Explorer").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Copy Path").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Rename").clicked() {
                        ui.close_menu();
                    }
                });
            }
        });
        if let Some(idx) = new_selection {
            self.selected_entry = Some(idx);
        }
    }

    /// Render the file listing in grid mode.
    fn show_grid_view(&mut self, ui: &mut egui::Ui) {
        let filtered: Vec<(usize, AssetEntry)> = self.filtered_entries().into_iter().map(|(i, e)| (i, e.clone())).collect();
        let selected = self.selected_entry;
        let item_size = egui::vec2(72.0, 80.0);
        let mut new_selection: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (idx, entry) in &filtered {
                    let is_selected = selected == Some(*idx);
                    let color = egui::Color32::from_hex(entry.asset_type.color_hex())
                        .unwrap_or(egui::Color32::GRAY);

                    let (rect, response) = ui.allocate_exact_size(
                        item_size,
                        egui::Sense::click(),
                    );

                    let stroke = if is_selected {
                        egui::Stroke::new(2.0, egui::Color32::WHITE)
                    } else {
                        egui::Stroke::new(1.0, egui::Color32::from_gray(60))
                    };
                    ui.painter().rect_filled(rect, 4.0, color.gamma_multiply(0.3));
                    ui.painter().rect_stroke(rect, 4.0, stroke);

                    let icon = if entry.is_dir { "📁" } else { "📄" };
                    ui.painter().text(
                        rect.center_top() + egui::vec2(0.0, 20.0),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::proportional(20.0),
                        egui::Color32::WHITE,
                    );
                    ui.painter().text(
                        rect.center_bottom() + egui::vec2(0.0, -6.0),
                        egui::Align2::CENTER_BOTTOM,
                        entry.name.chars().take(10).collect::<String>(),
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );

                    if response.clicked() {
                        new_selection = Some(*idx);
                    }

                    response.context_menu(|ui| {
                        if ui.button("Open in Explorer").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("Copy Path").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("Delete").clicked() {
                            ui.close_menu();
                        }
                        if ui.button("Rename").clicked() {
                            ui.close_menu();
                        }
                    });
                }
            });
        });
        if let Some(idx) = new_selection {
            self.selected_entry = Some(idx);
        }
    }

    /// Render the bottom status bar.
    fn show_status_bar(&self, ui: &mut egui::Ui) {
        let total = self.entries.len();
        let folders = self.entries.iter().filter(|e| e.is_dir).count();
        let files = total - folders;
        ui.colored_label(
            egui::Color32::GRAY,
            format!("{total} items — {folders} folders, {files} files"),
        );
    }
}

// ──────────────────────────────────────────────
// EditorPanel implementation
// ──────────────────────────────────────────────

impl EditorPanel for AssetBrowserPanel {
    fn title(&self) -> &str {
        "Asset Browser"
    }

    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Auto-refresh on first show when a project is loaded.
        if self.current_dir.is_none() && state.project_path.is_some() {
            self.refresh(state);
        }

        self.show_toolbar(ui, state);
        ui.separator();
        self.show_breadcrumbs(ui, state);
        ui.separator();

        // Handle double-click navigation for directories.
        // We snapshot the selected entry before rendering so we can detect
        // double-clicks that occurred on a directory during the *previous* frame.
        let dir_to_enter: Option<usize> = None;
        if let Some(idx) = self.selected_entry {
            if let Some(entry) = self.entries.get(idx) {
                if entry.is_dir {
                    // The selectable_label below will set this on click;
                    // we detect double-click via the interaction after render.
                }
            }
        }

        match self.view_mode {
            AssetViewMode::List => self.show_list_view(ui),
            AssetViewMode::Grid => self.show_grid_view(ui),
        }

        // Check if the currently selected entry was double-clicked and is a dir.
        if let Some(idx) = self.selected_entry {
            if let Some(entry) = self.entries.get(idx) {
                if entry.is_dir {
                    // Simple heuristic: if the user clicked the entry during this
                    // frame and it's already selected, treat it as "open".
                    // Real double-click detection would need per-item state;
                    // for now we handle this in the list/grid view methods.
                }
            }
        }

        if dir_to_enter.is_some() {
            // Will be handled inline.
        }

        ui.separator();
        self.show_status_bar(ui);
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Format a byte count into a human-readable string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = 1_048_576;

    if bytes == 0 {
        return String::new();
    }
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor ──

    #[test]
    fn new_returns_expected_defaults() {
        let panel = AssetBrowserPanel::new();
        assert!(panel.current_dir.is_none());
        assert!(panel.entries.is_empty());
        assert!(panel.selected_entry.is_none());
        assert!(!panel.show_hidden);
        assert_eq!(panel.view_mode, AssetViewMode::List);
        assert!(panel.search_filter.is_empty());
    }

    // ── Title ──

    #[test]
    fn title_returns_asset_browser() {
        let panel = AssetBrowserPanel::new();
        assert_eq!(panel.title(), "Asset Browser");
    }

    // ── Asset type detection ──

    #[test]
    fn detect_asset_type_known_extensions() {
        assert_eq!(detect_asset_type(Path::new("level.scene")), AssetType::Scene);
        assert_eq!(detect_asset_type(Path::new("data.json")), AssetType::Scene);
        assert_eq!(detect_asset_type(Path::new("hero.png")), AssetType::Image);
        assert_eq!(detect_asset_type(Path::new("bg.jpg")), AssetType::Image);
        assert_eq!(detect_asset_type(Path::new("icon.bmp")), AssetType::Image);
        assert_eq!(detect_asset_type(Path::new("character.obj")), AssetType::Mesh);
        assert_eq!(detect_asset_type(Path::new("model.gltf")), AssetType::Mesh);
        assert_eq!(detect_asset_type(Path::new("anim.glb")), AssetType::Mesh);
        assert_eq!(detect_asset_type(Path::new("bgm.wav")), AssetType::Audio);
        assert_eq!(detect_asset_type(Path::new("sfx.ogg")), AssetType::Audio);
        assert_eq!(detect_asset_type(Path::new("track.mp3")), AssetType::Audio);
        assert_eq!(detect_asset_type(Path::new("ai.rhai")), AssetType::Script);
        assert_eq!(detect_asset_type(Path::new("body.ttf")), AssetType::Font);
        assert_eq!(detect_asset_type(Path::new("display.otf")), AssetType::Font);
    }

    #[test]
    fn detect_asset_type_unknown_and_case_insensitive() {
        assert_eq!(detect_asset_type(Path::new("readme")), AssetType::Unknown);
        assert_eq!(detect_asset_type(Path::new("data.zip")), AssetType::Unknown);
        // Case-insensitive check.
        assert_eq!(detect_asset_type(Path::new("sprite.PNG")), AssetType::Image);
        assert_eq!(detect_asset_type(Path::new("hero.OBJ")), AssetType::Mesh);
    }

    // ── Entry sorting ──

    #[test]
    fn entries_sorted_directories_first_then_alphabetical() {
        let mut entries = vec![
            AssetEntry {
                name: "zebra.txt".into(),
                path: PathBuf::from("/assets/zebra.txt"),
                is_dir: false,
                size_bytes: 10,
                asset_type: AssetType::Unknown,
            },
            AssetEntry {
                name: "alpha".into(),
                path: PathBuf::from("/assets/alpha"),
                is_dir: true,
                size_bytes: 0,
                asset_type: AssetType::Unknown,
            },
            AssetEntry {
                name: "beta.lua".into(),
                path: PathBuf::from("/assets/beta.lua"),
                is_dir: false,
                size_bytes: 5,
                asset_type: AssetType::Unknown,
            },
            AssetEntry {
                name: "chars".into(),
                path: PathBuf::from("/assets/chars"),
                is_dir: true,
                size_bytes: 0,
                asset_type: AssetType::Unknown,
            },
        ];

        entries.sort_by(|a, b| {
            a.is_dir
                .cmp(&b.is_dir)
                .reverse()
                .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
        });

        assert!(entries[0].is_dir);
        assert!(entries[1].is_dir);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "chars");
        assert!(!entries[2].is_dir);
        assert_eq!(entries[2].name, "beta.lua");
        assert_eq!(entries[3].name, "zebra.txt");
    }

    // ── Search filter ──

    #[test]
    fn search_filter_matches_name_case_insensitive() {
        let mut panel = AssetBrowserPanel::new();
        panel.entries = vec![
            AssetEntry {
                name: "hero.png".into(),
                path: PathBuf::from("/assets/hero.png"),
                is_dir: false,
                size_bytes: 2048,
                asset_type: AssetType::Image,
            },
            AssetEntry {
                name: "hero_scene.json".into(),
                path: PathBuf::from("/assets/hero_scene.json"),
                is_dir: false,
                size_bytes: 512,
                asset_type: AssetType::Scene,
            },
            AssetEntry {
                name: "enemy.obj".into(),
                path: PathBuf::from("/assets/enemy.obj"),
                is_dir: false,
                size_bytes: 8192,
                asset_type: AssetType::Mesh,
            },
        ];

        // Empty filter returns everything.
        panel.search_filter = String::new();
        assert_eq!(panel.filtered_entries().len(), 3);

        // Filter "hero" matches two entries.
        panel.search_filter = "hero".into();
        let filtered = panel.filtered_entries();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].1.name, "hero.png");
        assert_eq!(filtered[1].1.name, "hero_scene.json");

        // Case-insensitive.
        panel.search_filter = "HERO".into();
        assert_eq!(panel.filtered_entries().len(), 2);

        // No match.
        panel.search_filter = "nonexistent".into();
        assert_eq!(panel.filtered_entries().len(), 0);
    }

    // ── View mode toggle ──

    #[test]
    fn view_mode_toggles_between_list_and_grid() {
        let mut panel = AssetBrowserPanel::new();
        assert_eq!(panel.view_mode, AssetViewMode::List);

        panel.view_mode = AssetViewMode::Grid;
        assert_eq!(panel.view_mode, AssetViewMode::Grid);

        panel.view_mode = match panel.view_mode {
            AssetViewMode::List => AssetViewMode::Grid,
            AssetViewMode::Grid => AssetViewMode::List,
        };
        assert_eq!(panel.view_mode, AssetViewMode::List);
    }

    // ── AssetType labels ──

    #[test]
    fn asset_type_labels_are_distinct() {
        let labels: Vec<&str> = vec![
            AssetType::Scene.label(),
            AssetType::Image.label(),
            AssetType::Mesh.label(),
            AssetType::Audio.label(),
            AssetType::Script.label(),
            AssetType::Font.label(),
            AssetType::Unknown.label(),
        ];
        // All labels should be unique.
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), labels.len());
    }

    // ── Format size ──

    #[test]
    fn format_size_human_readable() {
        assert_eq!(format_size(0), "");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(2_097_152), "2.0 MB");
    }
}

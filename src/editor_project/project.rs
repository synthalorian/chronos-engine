//! Core project management for the Chronos Engine editor.
//!
//! Handles the `.chronos` project format: manifest, templates, save/load,
//! the [`ProjectManager`] state machine, and recent-project tracking.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

/// Errors that can occur during project operations.
#[derive(Debug)]
pub enum ProjectError {
    /// An I/O error reading or writing project files.
    IoError(io::Error),
    /// A serialization or deserialization failure.
    SerializationError(String),
    /// The manifest is present but malformed or missing required fields.
    InvalidManifest(String),
    /// No project found at the given path.
    ProjectNotFound(String),
    /// A project or resource already exists at the given path.
    AlreadyExists(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::IoError(e) => write!(f, "project I/O error: {e}"),
            ProjectError::SerializationError(msg) => {
                write!(f, "project serialization error: {msg}")
            }
            ProjectError::InvalidManifest(msg) => {
                write!(f, "invalid project manifest: {msg}")
            }
            ProjectError::ProjectNotFound(msg) => {
                write!(f, "project not found: {msg}")
            }
            ProjectError::AlreadyExists(msg) => {
                write!(f, "project already exists: {msg}")
            }
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ProjectError {
    fn from(e: io::Error) -> Self {
        ProjectError::IoError(e)
    }
}

// ──────────────────────────────────────────────
// ProjectTemplate
// ──────────────────────────────────────────────

/// Built-in project templates that pre-populate scenes and settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectTemplate {
    /// A blank project with a single empty scene.
    Empty,
    /// A 2D side-scrolling platformer starter.
    Platformer2D,
    /// A 3D arena shooter starter.
    Shooter3D,
    /// A top-down RPG starter with multiple maps.
    RPG,
}

impl ProjectTemplate {
    /// Human-readable label for display in template pickers.
    pub fn label(&self) -> &str {
        match self {
            ProjectTemplate::Empty => "Empty Project",
            ProjectTemplate::Platformer2D => "2D Platformer",
            ProjectTemplate::Shooter3D => "3D Shooter",
            ProjectTemplate::RPG => "RPG",
        }
    }

    /// Short description of what the template provides.
    pub fn description(&self) -> &str {
        match self {
            ProjectTemplate::Empty => "A blank canvas — one empty scene, no extras.",
            ProjectTemplate::Platformer2D => {
                "2D side-scrolling setup with a main scene and a sample level."
            }
            ProjectTemplate::Shooter3D => "3D arena starter with a main scene and an arena map.",
            ProjectTemplate::RPG => "Top-down RPG starter with main, overworld, and town scenes.",
        }
    }

    /// Emoji or short icon for UI display.
    pub fn icon(&self) -> &str {
        match self {
            ProjectTemplate::Empty => "📄",
            ProjectTemplate::Platformer2D => "🏃",
            ProjectTemplate::Shooter3D => "🎯",
            ProjectTemplate::RPG => "⚔️",
        }
    }

    /// Template-specific default scene file names (relative to `scenes/`).
    pub fn default_scenes(&self) -> Vec<String> {
        match self {
            ProjectTemplate::Empty => vec!["main.scene".into()],
            ProjectTemplate::Platformer2D => {
                vec!["main.scene".into(), "level_1.scene".into()]
            }
            ProjectTemplate::Shooter3D => {
                vec!["main.scene".into(), "arena.scene".into()]
            }
            ProjectTemplate::RPG => {
                vec![
                    "main.scene".into(),
                    "overworld.scene".into(),
                    "town.scene".into(),
                ]
            }
        }
    }

    /// All available template variants.
    pub fn all() -> [ProjectTemplate; 4] {
        [
            ProjectTemplate::Empty,
            ProjectTemplate::Platformer2D,
            ProjectTemplate::Shooter3D,
            ProjectTemplate::RPG,
        ]
    }
}

// ──────────────────────────────────────────────
// ProjectConfig
// ──────────────────────────────────────────────

/// Per-project editor and runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Default game window width in pixels.
    pub window_width: u32,
    /// Default game window height in pixels.
    pub window_height: u32,
    /// Whether vertical sync is enabled.
    pub vsync: bool,
    /// Target frames per second for the game loop.
    pub target_fps: u32,
    /// File path (relative to project dir) of the last scene open in the editor.
    pub last_opened_scene: Option<String>,
    /// Opaque JSON string carrying future layout configuration.
    pub editor_layout: String,
}

impl ProjectConfig {
    /// Create a config with sensible defaults.
    pub fn new() -> Self {
        Self {
            window_width: 1280,
            window_height: 720,
            vsync: true,
            target_fps: 60,
            last_opened_scene: None,
            editor_layout: String::new(),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// ProjectManifest
// ──────────────────────────────────────────────

/// The `manifest.json` at the root of every `.chronos` project directory.
///
/// Stores project metadata, scene list, template origin, and editor config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Human-readable project name.
    pub name: String,
    /// Project format version (semver).
    pub version: String,
    /// Minimum engine version required.
    pub engine_version: String,
    /// One-line project description.
    pub description: String,
    /// Unix timestamp (seconds) when the project was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the project was last modified.
    pub modified_at: u64,
    /// Scene file paths relative to the project directory.
    pub scenes: Vec<String>,
    /// Template this project was created from.
    pub template: ProjectTemplate,
    /// Per-project editor/runtime settings.
    pub settings: ProjectConfig,
}

impl ProjectManifest {
    /// Create a new manifest with the given name and template.
    ///
    /// Timestamps are initialised to `0` — call [`touch`](Self::touch) or set
    /// them manually before saving.
    pub fn new(name: &str, template: ProjectTemplate) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            engine_version: "0.3.0".to_string(),
            description: String::new(),
            created_at: 0,
            modified_at: 0,
            scenes: template.default_scenes(),
            template,
            settings: ProjectConfig::new(),
        }
    }

    /// Save the manifest as `manifest.json` inside `dir`.
    pub fn save_to_dir(&self, dir: &Path) -> Result<(), ProjectError> {
        let path = dir.join("manifest.json");
        self.save_to_file(&path)
    }

    /// Load a manifest from `manifest.json` inside `dir`.
    pub fn load_from_dir(dir: &Path) -> Result<Self, ProjectError> {
        let path = dir.join("manifest.json");
        Self::load_from_file(&path)
    }

    /// Serialize the manifest to an arbitrary file path.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ProjectError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ProjectError::SerializationError(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Deserialize a manifest from an arbitrary file path.
    pub fn load_from_file(path: &Path) -> Result<Self, ProjectError> {
        if !path.exists() {
            return Err(ProjectError::ProjectNotFound(format!(
                "manifest not found at {}",
                path.display()
            )));
        }
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| ProjectError::SerializationError(e.to_string()))
    }

    /// Update `modified_at` to the current wall-clock time.
    pub fn touch(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.modified_at = now;
    }
}

// ──────────────────────────────────────────────
// RecentProject
// ──────────────────────────────────────────────

/// A lightweight entry in the recent-projects list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    /// Project display name.
    pub name: String,
    /// Absolute path to the project directory.
    pub path: String,
    /// Unix timestamp of the last time the project was opened.
    pub last_opened: u64,
    /// Template the project was created from.
    pub template: ProjectTemplate,
}

impl RecentProject {
    /// Create a new recent-project entry with `last_opened` set to `0`.
    pub fn new(name: &str, path: &str, template: ProjectTemplate) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            last_opened: 0,
            template,
        }
    }
}

// ──────────────────────────────────────────────
// Directory structure helper
// ──────────────────────────────────────────────

/// Create the standard directory layout for a new project.
///
/// Produces:
/// - `{dir}/manifest.json`
/// - `{dir}/scenes/`
/// - `{dir}/assets/`
/// - `{dir}/scripts/`
pub fn create_project_structure(
    dir: &Path,
    manifest: &ProjectManifest,
) -> Result<(), ProjectError> {
    fs::create_dir_all(dir)?;

    manifest.save_to_dir(dir)?;

    fs::create_dir_all(dir.join("scenes"))?;
    fs::create_dir_all(dir.join("assets"))?;
    fs::create_dir_all(dir.join("scripts"))?;

    Ok(())
}

// ──────────────────────────────────────────────
// ProjectManager
// ──────────────────────────────────────────────

/// Owns the current editor session state: the loaded project, recent list,
/// and UI toggle flags for the welcome / new-project / open-project dialogs.
#[derive(Debug)]
pub struct ProjectManager {
    /// The currently loaded project manifest, if any.
    pub current_project: Option<ProjectManifest>,
    /// Absolute path to the current project directory.
    pub project_dir: Option<PathBuf>,
    /// Recently opened projects (most recent first).
    pub recent_projects: Vec<RecentProject>,
    /// Maximum number of recent-project entries kept.
    pub max_recent: usize,
    /// Whether the welcome screen should be shown.
    pub show_welcome: bool,
    /// Whether the new-project wizard dialog should be shown.
    pub show_new_wizard: bool,
    /// Whether the open-project dialog should be shown.
    pub show_open_dialog: bool,
}

impl ProjectManager {
    /// Create an empty manager (no project loaded).
    pub fn new() -> Self {
        Self {
            current_project: None,
            project_dir: None,
            recent_projects: Vec::new(),
            max_recent: 10,
            show_welcome: true,
            show_new_wizard: false,
            show_open_dialog: false,
        }
    }

    /// Create a brand-new project on disk and return a manager with it loaded.
    ///
    /// Creates the directory tree, writes `manifest.json`, and registers the
    /// project in the recent list.
    pub fn create_project(
        name: &str,
        dir: &Path,
        template: ProjectTemplate,
    ) -> Result<Self, ProjectError> {
        let manifest_path = dir.join("manifest.json");
        if manifest_path.exists() {
            return Err(ProjectError::AlreadyExists(format!(
                "a project already exists at {}",
                dir.display()
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut manifest = ProjectManifest::new(name, template);
        manifest.created_at = now;
        manifest.modified_at = now;

        create_project_structure(dir, &manifest)?;

        let mut manager = Self::new();
        manager.current_project = Some(manifest);
        manager.project_dir = Some(dir.to_path_buf());
        manager.show_welcome = false;

        manager.add_recent(name, &dir.to_string_lossy(), template);

        Ok(manager)
    }

    /// Open an existing project from disk and return a manager with it loaded.
    pub fn open_project(dir: &Path) -> Result<Self, ProjectError> {
        let manifest = ProjectManifest::load_from_dir(dir)?;

        let mut manager = Self::new();
        manager.current_project = Some(manifest.clone());
        manager.project_dir = Some(dir.to_path_buf());
        manager.show_welcome = false;

        manager.add_recent(&manifest.name, &dir.to_string_lossy(), manifest.template);

        Ok(manager)
    }

    /// Persist the current project manifest back to disk.
    pub fn save_current(&mut self) -> Result<(), ProjectError> {
        let dir = match &self.project_dir {
            Some(d) => d.clone(),
            None => {
                return Err(ProjectError::ProjectNotFound(
                    "no project is currently loaded".into(),
                ))
            }
        };

        match &mut self.current_project {
            Some(manifest) => {
                manifest.touch();
                manifest.save_to_dir(&dir)
            }
            None => Err(ProjectError::ProjectNotFound(
                "no project is currently loaded".into(),
            )),
        }
    }

    /// Save the current project under a new name and directory.
    ///
    /// Creates a copy of the manifest with the new name at the destination.
    pub fn save_as(&mut self, name: &str, dir: &Path) -> Result<(), ProjectError> {
        let mut manifest = match &self.current_project {
            Some(m) => m.clone(),
            None => {
                return Err(ProjectError::ProjectNotFound(
                    "no project is currently loaded".into(),
                ))
            }
        };

        let dest_manifest = dir.join("manifest.json");
        if dest_manifest.exists() {
            return Err(ProjectError::AlreadyExists(format!(
                "a project already exists at {}",
                dir.display()
            )));
        }

        manifest.name = name.to_string();
        manifest.touch();

        create_project_structure(dir, &manifest)?;

        self.current_project = Some(manifest);
        self.project_dir = Some(dir.to_path_buf());

        Ok(())
    }

    /// Unload the current project and return to the welcome state.
    pub fn close_project(&mut self) {
        self.current_project = None;
        self.project_dir = None;
        self.show_welcome = true;
        self.show_new_wizard = false;
        self.show_open_dialog = false;
    }

    /// Add an entry to the recent-projects list.
    ///
    /// The new entry is pushed to the front. If an entry with the same `path`
    /// already exists it is removed first (no duplicates). The list is then
    /// trimmed to `max_recent`.
    pub fn add_recent(&mut self, name: &str, path: &str, template: ProjectTemplate) {
        self.recent_projects.retain(|r| r.path != path);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut entry = RecentProject::new(name, path, template);
        entry.last_opened = now;

        self.recent_projects.insert(0, entry);
        self.recent_projects.truncate(self.max_recent);
    }

    /// Remove a recent-project entry by its directory path.
    pub fn remove_recent(&mut self, path: &str) {
        self.recent_projects.retain(|r| r.path != path);
    }

    /// Serialize the recent-projects list to a JSON file.
    pub fn save_recent_to_file(&self, path: &Path) -> Result<(), ProjectError> {
        let json = serde_json::to_string_pretty(&self.recent_projects)
            .map_err(|e| ProjectError::SerializationError(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Deserialize a recent-projects list from a JSON file.
    pub fn load_recent_from_file(path: &Path) -> Result<Vec<RecentProject>, ProjectError> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| ProjectError::SerializationError(e.to_string()))
    }

    /// Whether a project is currently loaded.
    pub fn is_loaded(&self) -> bool {
        self.current_project.is_some()
    }

    /// The display name of the current project, or `"No Project"`.
    pub fn project_name(&self) -> &str {
        match &self.current_project {
            Some(m) => &m.name,
            None => "No Project",
        }
    }

    /// Check whether `dir` contains a valid `manifest.json`.
    pub fn validate_dir(dir: &Path) -> Result<bool, ProjectError> {
        let manifest = dir.join("manifest.json");
        if !manifest.exists() {
            return Ok(false);
        }
        // Try to parse it — validates structure.
        ProjectManifest::load_from_file(&manifest)?;
        Ok(true)
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Helper: create a unique temp dir for each test.
    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join("chronos_project_tests");
        let dir = base.join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // ── 1. project_manifest_new ──

    #[test]
    fn project_manifest_new() {
        let m = ProjectManifest::new("TestGame", ProjectTemplate::Empty);
        assert_eq!(m.name, "TestGame");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.engine_version, "0.3.0");
        assert!(m.description.is_empty());
        assert_eq!(m.created_at, 0);
        assert_eq!(m.modified_at, 0);
        assert_eq!(m.scenes, vec!["main.scene"]);
        assert_eq!(m.template, ProjectTemplate::Empty);
    }

    // ── 2. project_manifest_save_load_roundtrip ──

    #[test]
    fn project_manifest_save_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let original = ProjectManifest::new("RoundTrip", ProjectTemplate::Platformer2D);
        original.save_to_dir(&dir).unwrap();

        let loaded = ProjectManifest::load_from_dir(&dir).unwrap();
        assert_eq!(loaded.name, "RoundTrip");
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.engine_version, "0.3.0");
        assert_eq!(loaded.template, ProjectTemplate::Platformer2D);
        assert_eq!(loaded.scenes, vec!["main.scene", "level_1.scene"]);
        cleanup(&dir);
    }

    // ── 3. project_manifest_save_to_file ──

    #[test]
    fn project_manifest_save_to_file() {
        let dir = temp_dir("save_file");
        let path = dir.join("custom_manifest.json");

        let m = ProjectManifest::new("FileTest", ProjectTemplate::Shooter3D);
        m.save_to_file(&path).unwrap();

        let loaded = ProjectManifest::load_from_file(&path).unwrap();
        assert_eq!(loaded.name, "FileTest");
        assert_eq!(loaded.template, ProjectTemplate::Shooter3D);
        cleanup(&dir);
    }

    // ── 4. project_config_defaults ──

    #[test]
    fn project_config_defaults() {
        let c = ProjectConfig::new();
        assert_eq!(c.window_width, 1280);
        assert_eq!(c.window_height, 720);
        assert!(c.vsync);
        assert_eq!(c.target_fps, 60);
        assert!(c.last_opened_scene.is_none());
        assert!(c.editor_layout.is_empty());
    }

    // ── 5. project_template_labels ──

    #[test]
    fn project_template_labels() {
        assert_eq!(ProjectTemplate::Empty.label(), "Empty Project");
        assert_eq!(ProjectTemplate::Platformer2D.label(), "2D Platformer");
        assert_eq!(ProjectTemplate::Shooter3D.label(), "3D Shooter");
        assert_eq!(ProjectTemplate::RPG.label(), "RPG");
    }

    // ── 6. project_template_descriptions ──

    #[test]
    fn project_template_descriptions() {
        for t in ProjectTemplate::all() {
            assert!(!t.description().is_empty(), "description empty for {:?}", t);
        }
    }

    // ── 7. project_template_default_scenes ──

    #[test]
    fn project_template_default_scenes() {
        assert_eq!(ProjectTemplate::Empty.default_scenes(), vec!["main.scene"]);
        assert_eq!(
            ProjectTemplate::Platformer2D.default_scenes(),
            vec!["main.scene", "level_1.scene"]
        );
        assert_eq!(
            ProjectTemplate::Shooter3D.default_scenes(),
            vec!["main.scene", "arena.scene"]
        );
        assert_eq!(
            ProjectTemplate::RPG.default_scenes(),
            vec!["main.scene", "overworld.scene", "town.scene"]
        );
    }

    // ── 8. project_template_all_variants ──

    #[test]
    fn project_template_all_variants() {
        let all = ProjectTemplate::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], ProjectTemplate::Empty);
        assert_eq!(all[1], ProjectTemplate::Platformer2D);
        assert_eq!(all[2], ProjectTemplate::Shooter3D);
        assert_eq!(all[3], ProjectTemplate::RPG);
    }

    // ── 9. project_error_display ──

    #[test]
    fn project_error_display() {
        let io_err = ProjectError::IoError(io::Error::new(io::ErrorKind::NotFound, "gone"));
        assert!(io_err.to_string().contains("project I/O error"));

        let ser_err = ProjectError::SerializationError("bad json".into());
        assert!(ser_err.to_string().contains("bad json"));

        let inv_err = ProjectError::InvalidManifest("missing field".into());
        assert!(inv_err.to_string().contains("missing field"));

        let nf_err = ProjectError::ProjectNotFound("nope".into());
        assert!(nf_err.to_string().contains("nope"));

        let ae_err = ProjectError::AlreadyExists("dup".into());
        assert!(ae_err.to_string().contains("dup"));
    }

    // ── 10. project_error_from_io ──

    #[test]
    fn project_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let project_err: ProjectError = io_err.into();
        match project_err {
            ProjectError::IoError(e) => {
                assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
            }
            _ => panic!("expected IoError variant"),
        }
    }

    // ── 11. recent_project_new ──

    #[test]
    fn recent_project_new() {
        let rp = RecentProject::new("MyGame", "/tmp/mygame", ProjectTemplate::RPG);
        assert_eq!(rp.name, "MyGame");
        assert_eq!(rp.path, "/tmp/mygame");
        assert_eq!(rp.last_opened, 0);
        assert_eq!(rp.template, ProjectTemplate::RPG);
    }

    // ── 12. project_manager_new ──

    #[test]
    fn project_manager_new() {
        let mgr = ProjectManager::new();
        assert!(mgr.current_project.is_none());
        assert!(mgr.project_dir.is_none());
        assert!(mgr.recent_projects.is_empty());
        assert_eq!(mgr.max_recent, 10);
        assert!(mgr.show_welcome);
        assert!(!mgr.show_new_wizard);
        assert!(!mgr.show_open_dialog);
        assert!(!mgr.is_loaded());
        assert_eq!(mgr.project_name(), "No Project");
    }

    // ── 13. project_manager_create_project ──

    #[test]
    fn project_manager_create_project() {
        let dir = temp_dir("create_proj");
        let mgr =
            ProjectManager::create_project("CoolGame", &dir, ProjectTemplate::Shooter3D).unwrap();

        assert!(mgr.is_loaded());
        assert_eq!(mgr.project_name(), "CoolGame");
        assert!(!mgr.show_welcome);
        assert!(dir.join("manifest.json").exists());
        assert!(dir.join("scenes").is_dir());
        assert!(dir.join("assets").is_dir());
        assert!(dir.join("scripts").is_dir());

        let manifest = mgr.current_project.unwrap();
        assert_eq!(manifest.scenes, vec!["main.scene", "arena.scene"]);
        assert!(manifest.created_at > 0);

        cleanup(&dir);
    }

    // ── 14. project_manager_open_project ──

    #[test]
    fn project_manager_open_project() {
        let dir = temp_dir("open_proj");

        // Create first, then open fresh.
        let created = ProjectManager::create_project("OpenMe", &dir, ProjectTemplate::RPG).unwrap();
        let manifest = created.current_project.unwrap();
        assert!(manifest.created_at > 0);

        let mgr = ProjectManager::open_project(&dir).unwrap();
        assert!(mgr.is_loaded());
        assert_eq!(mgr.project_name(), "OpenMe");
        assert!(!mgr.recent_projects.is_empty());

        cleanup(&dir);
    }

    // ── 15. project_manager_recent_projects ──

    #[test]
    fn project_manager_recent_projects() {
        let mut mgr = ProjectManager::new();

        mgr.add_recent("A", "/tmp/a", ProjectTemplate::Empty);
        mgr.add_recent("B", "/tmp/b", ProjectTemplate::Platformer2D);
        assert_eq!(mgr.recent_projects.len(), 2);
        // Most recent first.
        assert_eq!(mgr.recent_projects[0].name, "B");

        // Deduplication: re-adding A moves it to front.
        mgr.add_recent("A", "/tmp/a", ProjectTemplate::Empty);
        assert_eq!(mgr.recent_projects.len(), 2);
        assert_eq!(mgr.recent_projects[0].name, "A");

        // Remove.
        mgr.remove_recent("/tmp/b");
        assert_eq!(mgr.recent_projects.len(), 1);
        assert_eq!(mgr.recent_projects[0].name, "A");

        // Max cap.
        mgr.max_recent = 2;
        mgr.add_recent("C", "/tmp/c", ProjectTemplate::RPG);
        mgr.add_recent("D", "/tmp/d", ProjectTemplate::Shooter3D);
        assert_eq!(mgr.recent_projects.len(), 2);
        // D is newest, then C; A was pushed out.
        assert_eq!(mgr.recent_projects[0].name, "D");
        assert_eq!(mgr.recent_projects[1].name, "C");
    }

    // ── 16. project_manager_save_load_recent ──

    #[test]
    fn project_manager_save_load_recent() {
        let dir = temp_dir("recent_io");
        let path = dir.join("recent.json");

        let mut mgr = ProjectManager::new();
        mgr.add_recent("X", "/tmp/x", ProjectTemplate::Empty);
        mgr.add_recent("Y", "/tmp/y", ProjectTemplate::RPG);

        mgr.save_recent_to_file(&path).unwrap();
        let loaded = ProjectManager::load_recent_from_file(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Y");
        assert_eq!(loaded[1].name, "X");

        // Non-existent file returns empty vec.
        let empty = ProjectManager::load_recent_from_file(&dir.join("nope.json")).unwrap();
        assert!(empty.is_empty());

        cleanup(&dir);
    }

    // ── 17. project_manager_close_project ──

    #[test]
    fn project_manager_close_project() {
        let dir = temp_dir("close_proj");
        let mut mgr =
            ProjectManager::create_project("Bye", &dir, ProjectTemplate::Platformer2D).unwrap();
        assert!(mgr.is_loaded());

        mgr.close_project();
        assert!(!mgr.is_loaded());
        assert!(mgr.current_project.is_none());
        assert!(mgr.project_dir.is_none());
        assert!(mgr.show_welcome);
        assert!(!mgr.show_new_wizard);
        assert!(!mgr.show_open_dialog);
        assert_eq!(mgr.project_name(), "No Project");

        cleanup(&dir);
    }

    // ── 18. project_manager_validate_dir ──

    #[test]
    fn project_manager_validate_dir() {
        let dir = temp_dir("validate");

        // Empty dir → false.
        assert!(!ProjectManager::validate_dir(&dir).unwrap());

        // Create a project, then validate.
        ProjectManager::create_project("Valid", &dir, ProjectTemplate::Empty).unwrap();
        assert!(ProjectManager::validate_dir(&dir).unwrap());

        cleanup(&dir);
    }

    // ── Bonus: touch updates modified_at ──

    #[test]
    fn project_manifest_touch() {
        let mut m = ProjectManifest::new("Touched", ProjectTemplate::Empty);
        assert_eq!(m.modified_at, 0);
        m.touch();
        assert!(m.modified_at > 0);
    }

    // ── Bonus: create_project rejects existing ──

    #[test]
    fn project_manager_create_rejects_existing() {
        let dir = temp_dir("reject_existing");
        ProjectManager::create_project("First", &dir, ProjectTemplate::Empty).unwrap();

        let result = ProjectManager::create_project("Second", &dir, ProjectTemplate::RPG);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::AlreadyExists(_) => {}
            other => panic!("expected AlreadyExists, got {:?}", other),
        }

        cleanup(&dir);
    }

    // ── Bonus: save_current without project fails ──

    #[test]
    fn project_manager_save_current_no_project() {
        let mut mgr = ProjectManager::new();
        let result = mgr.save_current();
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ProjectNotFound(_) => {}
            other => panic!("expected ProjectNotFound, got {:?}", other),
        }
    }

    // ── Bonus: load_from_file missing file ──

    #[test]
    fn project_manifest_load_missing_file() {
        let result = ProjectManifest::load_from_file(Path::new("/no/such/manifest.json"));
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ProjectNotFound(_) => {}
            other => panic!("expected ProjectNotFound, got {:?}", other),
        }
    }

    // ── Bonus: save_as copies to new location ──

    #[test]
    fn project_manager_save_as() {
        let dir_a = temp_dir("save_as_src");
        let dir_b = temp_dir("save_as_dst");

        let mut mgr =
            ProjectManager::create_project("Original", &dir_a, ProjectTemplate::Empty).unwrap();
        mgr.save_as("Copy", &dir_b).unwrap();

        assert_eq!(mgr.project_name(), "Copy");
        assert_eq!(mgr.project_dir, Some(dir_b.clone()));
        assert!(dir_b.join("manifest.json").exists());

        let loaded = ProjectManifest::load_from_dir(&dir_b).unwrap();
        assert_eq!(loaded.name, "Copy");

        cleanup(&dir_a);
        cleanup(&dir_b);
    }

    // ── Bonus: template icons ──

    #[test]
    fn project_template_icons() {
        for t in ProjectTemplate::all() {
            assert!(!t.icon().is_empty(), "icon empty for {:?}", t);
        }
    }

    // ── Bonus: ProjectConfig Default trait ──

    #[test]
    fn project_config_default_trait() {
        let c = ProjectConfig::default();
        assert_eq!(c.window_width, 1280);
        assert_eq!(c.window_height, 720);
    }
}

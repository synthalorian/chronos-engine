//! Plugin System — Phase 13.
//!
//! Provides a safe, zero-unsafe plugin architecture for Chronos Engine.
//!
//! # Design
//!
//! Plugins are Rust crates that implement the [`Plugin`] trait and are
//! compiled into the engine binary (static linking). This avoids the
//! complexity and `unsafe` code required by dynamic library loading.
//! Future versions may support WASM-based dynamic plugins for
//! sandboxing.
//!
//! # Usage
//!
//! ```rust,ignore
//! use chronos_engine::plugin::{Plugin, PluginManifest, PluginContext};
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn manifest(&self) -> PluginManifest {
//!         PluginManifest::new("my_plugin", "1.0.0")
//!             .with_author("Alice")
//!             .with_description("Adds cool features")
//!     }
//!
//!     fn on_init(&mut self, ctx: &mut PluginContext) {
//!         ctx.log_info("MyPlugin initialized!");
//!     }
//!
//!     fn on_update(&mut self, ctx: &mut PluginContext, dt: f32) {
//!         // Do work every frame
//!     }
//!
//!     fn on_shutdown(&mut self, ctx: &mut PluginContext) {
//!         ctx.log_info("MyPlugin shutting down.");
//!     }
//! }
//! ```

use std::any::Any;

use crate::world::World;

pub mod api;
pub mod editor;

pub use api::PluginApi;
pub use editor::EditorPluginHooks;

// ──────────────────────────────────────────────────────────────
// Plugin Manifest
// ──────────────────────────────────────────────────────────────

/// Metadata describing a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse-DNS style recommended).
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Plugin author.
    pub author: String,
    /// Short description.
    pub description: String,
    /// Names of other plugins this plugin depends on.
    pub dependencies: Vec<String>,
    /// Minimum engine version required.
    pub min_engine_version: String,
    /// Whether this plugin provides editor extensions.
    pub has_editor_hooks: bool,
}

impl PluginManifest {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        let name = name.into();
        PluginManifest {
            display_name: name.clone(),
            name,
            version: version.into(),
            author: String::new(),
            description: String::new(),
            dependencies: Vec::new(),
            min_engine_version: "1.0.0".into(),
            has_editor_hooks: false,
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_min_engine_version(mut self, ver: impl Into<String>) -> Self {
        self.min_engine_version = ver.into();
        self
    }

    pub fn with_editor_hooks(mut self, has: bool) -> Self {
        self.has_editor_hooks = has;
        self
    }
}

// ──────────────────────────────────────────────────────────────
// Plugin Context
// ──────────────────────────────────────────────────────────────

/// The execution context passed to plugins on every lifecycle call.
///
/// This is intentionally a limited, safe view into the engine. Plugins
/// cannot directly access `World` mutably outside of sanctioned
/// callback windows — they must use the provided `PluginApi`.
pub struct PluginContext<'a> {
    /// API surface exposed to the plugin.
    pub api: PluginApi<'a>,
    /// Frame delta time in seconds.
    pub dt: f32,
    /// Current simulation tick.
    pub tick: u64,
    /// Accumulated log messages from this plugin.
    pub log_buffer: &'a mut Vec<String>,
}

impl<'a> PluginContext<'a> {
    /// Log an informational message from the plugin.
    pub fn log_info(&mut self, msg: impl Into<String>) {
        self.log_buffer.push(format!("[INFO] {}", msg.into()));
    }

    /// Log a warning message from the plugin.
    pub fn log_warn(&mut self, msg: impl Into<String>) {
        self.log_buffer.push(format!("[WARN] {}", msg.into()));
    }

    /// Log an error message from the plugin.
    pub fn log_error(&mut self, msg: impl Into<String>) {
        self.log_buffer.push(format!("[ERR]  {}", msg.into()));
    }
}

// ──────────────────────────────────────────────────────────────
// Plugin Trait
// ──────────────────────────────────────────────────────────────

/// The core trait that every Chronos plugin must implement.
///
/// Plugins are long-lived objects. The engine calls lifecycle methods
/// in this order:
/// 1. `manifest()` — once, at discovery time.
/// 2. `on_init()` — once, before the first update.
/// 3. `on_update()` — every frame while the plugin is active.
/// 4. `on_shutdown()` — once, on engine teardown or plugin unload.
pub trait Plugin: Send + Sync + Any {
    /// Return the plugin's metadata manifest.
    fn manifest(&self) -> PluginManifest;

    /// Called once when the plugin is loaded and initialized.
    fn on_init(&mut self, _ctx: &mut PluginContext) {}

    /// Called every simulation tick with delta time.
    fn on_update(&mut self, _ctx: &mut PluginContext, _dt: f32) {}

    /// Called once when the plugin is being shut down.
    fn on_shutdown(&mut self, _ctx: &mut PluginContext) {}

    /// If this plugin exposes editor hooks, return them.
    fn editor_hooks(&mut self) -> Option<&mut dyn EditorPluginHooks> {
        None
    }

    /// Cast to `&dyn Any` for downcasting.
    fn as_any(&self) -> &dyn Any;
    /// Cast to `&mut dyn Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ──────────────────────────────────────────────────────────────
// Plugin Registry
// ──────────────────────────────────────────────────────────────

/// Owns and dispatches to all loaded plugins.
///
/// # Usage
///
/// ```rust,ignore
/// let mut registry = PluginRegistry::new();
/// registry.register(Box::new(MyPlugin::new()));
/// registry.init_all(&mut world);
/// registry.update_all(&mut world, 0.016);
/// registry.shutdown_all(&mut world);
/// ```
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    manifests: Vec<PluginManifest>,
    /// Log output from the most recent update cycle.
    pub log_output: Vec<String>,
    /// Whether the registry has been initialized.
    initialized: bool,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry {
            plugins: Vec::new(),
            manifests: Vec::new(),
            log_output: Vec::new(),
            initialized: false,
        }
    }

    /// Register a plugin. Must be called before `init_all`.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        if self.initialized {
            return Err(PluginError::AlreadyInitialized);
        }
        let manifest = plugin.manifest();
        self.manifests.push(manifest);
        self.plugins.push(plugin);
        Ok(())
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Initialize all registered plugins.
    pub fn init_all(&mut self, world: &mut World) {
        self.log_output.clear();
        let plugins = &mut self.plugins;
        let log_output = &mut self.log_output;
        for plugin in plugins {
            let mut log_buffer = Vec::new();
            let api = PluginApi::new(world);
            let mut ctx = PluginContext {
                api,
                dt: 0.0,
                tick: 0,
                log_buffer: &mut log_buffer,
            };
            plugin.on_init(&mut ctx);
            log_output.extend(log_buffer);
        }
        self.initialized = true;
    }

    /// Update all plugins.
    pub fn update_all(&mut self, world: &mut World, dt: f32, tick: u64) {
        self.log_output.clear();
        let plugins = &mut self.plugins;
        let log_output = &mut self.log_output;
        for plugin in plugins {
            let mut log_buffer = Vec::new();
            let api = PluginApi::new(world);
            let mut ctx = PluginContext {
                api,
                dt,
                tick,
                log_buffer: &mut log_buffer,
            };
            plugin.on_update(&mut ctx, dt);
            log_output.extend(log_buffer);
        }
    }

    /// Shut down all plugins.
    pub fn shutdown_all(&mut self, world: &mut World) {
        self.log_output.clear();
        let plugins = &mut self.plugins;
        let log_output = &mut self.log_output;
        for plugin in plugins {
            let mut log_buffer = Vec::new();
            let api = PluginApi::new(world);
            let mut ctx = PluginContext {
                api,
                dt: 0.0,
                tick: 0,
                log_buffer: &mut log_buffer,
            };
            plugin.on_shutdown(&mut ctx);
            log_output.extend(log_buffer);
        }
        self.initialized = false;
    }

    /// Get a manifest by plugin index.
    pub fn manifest(&self, index: usize) -> Option<&PluginManifest> {
        self.manifests.get(index)
    }

    /// Iterate over all manifests.
    pub fn manifests(&self) -> &[PluginManifest] {
        &self.manifests
    }

    /// Get mutable access to a plugin by index.
    pub fn plugin_mut(&mut self, index: usize) -> Option<&mut dyn Plugin> {
        self.plugins.get_mut(index).map(|p| p.as_mut())
    }

    /// Find a plugin index by name.
    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.manifests.iter().position(|m| m.name == name)
    }

    /// Remove a plugin by name.
    pub fn remove_by_name(&mut self, name: &str) -> Result<(), PluginError> {
        if self.initialized {
            return Err(PluginError::AlreadyInitialized);
        }
        if let Some(idx) = self.find_by_name(name) {
            self.plugins.remove(idx);
            self.manifests.remove(idx);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.into()))
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────
// Plugin Loader
// ──────────────────────────────────────────────────────────────

/// Discovers plugin manifests from a directory tree.
///
/// In the current static-linking architecture, this scans for
/// `plugin.toml` or `plugin.json` manifest files. The actual plugin
/// objects must be registered manually via [`PluginRegistry::register`].
/// Future versions may auto-load WASM modules from these manifests.
pub struct PluginLoader;

impl PluginLoader {
    /// Scan a directory for plugin manifest files.
    pub fn scan_manifests(dir: &std::path::Path) -> Vec<PluginManifest> {
        let mut manifests = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let toml_path = path.join("plugin.toml");
                    if toml_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&toml_path) {
                            if let Some(mf) = Self::parse_toml_manifest(&content) {
                                manifests.push(mf);
                            }
                        }
                    }
                    let json_path = path.join("plugin.json");
                    if json_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&json_path) {
                            if let Some(mf) = Self::parse_json_manifest(&content) {
                                manifests.push(mf);
                            }
                        }
                    }
                }
            }
        }
        manifests
    }

    fn parse_toml_manifest(content: &str) -> Option<PluginManifest> {
        let mut manifest = None;
        let mut name = None;
        let mut version = None;
        let mut display_name = None;
        let mut author = None;
        let mut description = None;
        let mut dependencies = Vec::new();
        let mut min_engine_version = None;
        let mut has_editor_hooks = false;

        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("version = ") {
                version = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("display_name = ") {
                display_name = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("author = ") {
                author = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("description = ") {
                description = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("min_engine_version = ") {
                min_engine_version = Some(v.trim_matches('"').to_string());
            } else if line.starts_with("dependencies = [") {
                // Parse inline array: dependencies = ["foo", "bar"]
                let inner = line
                    .trim_start_matches("dependencies = [")
                    .trim_end_matches(']');
                for dep in inner.split(',') {
                    let dep = dep.trim().trim_matches('"');
                    if !dep.is_empty() {
                        dependencies.push(dep.to_string());
                    }
                }
            } else if line == "has_editor_hooks = true" {
                has_editor_hooks = true;
            }
        }

        if let (Some(n), Some(v)) = (name, version) {
            let mut m = PluginManifest::new(n, v);
            if let Some(dn) = display_name {
                m.display_name = dn;
            }
            if let Some(a) = author {
                m.author = a;
            }
            if let Some(d) = description {
                m.description = d;
            }
            if !dependencies.is_empty() {
                m.dependencies = dependencies;
            }
            if let Some(ver) = min_engine_version {
                m.min_engine_version = ver;
            }
            m.has_editor_hooks = has_editor_hooks;
            manifest = Some(m);
        }

        manifest
    }

    fn parse_json_manifest(content: &str) -> Option<PluginManifest> {
        // Simplified JSON parsing — look for key-value pairs
        let mut name = None;
        let mut version = None;
        let mut display_name = None;
        let mut author = None;
        let mut description = None;
        let mut min_engine_version = None;
        let mut has_editor_hooks = false;

        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("\"name\"") {
                if let Some(q) = v.split(':').nth(1) {
                    name = Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if let Some(v) = line.strip_prefix("\"version\"") {
                if let Some(q) = v.split(':').nth(1) {
                    version = Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if let Some(v) = line.strip_prefix("\"display_name\"") {
                if let Some(q) = v.split(':').nth(1) {
                    display_name = Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if let Some(v) = line.strip_prefix("\"author\"") {
                if let Some(q) = v.split(':').nth(1) {
                    author = Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if let Some(v) = line.strip_prefix("\"description\"") {
                if let Some(q) = v.split(':').nth(1) {
                    description = Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if let Some(v) = line.strip_prefix("\"min_engine_version\"") {
                if let Some(q) = v.split(':').nth(1) {
                    min_engine_version =
                        Some(q.trim().trim_matches(',').trim_matches('"').to_string());
                }
            } else if line.contains("\"has_editor_hooks\"") && line.contains("true") {
                has_editor_hooks = true;
            }
        }

        if let (Some(n), Some(v)) = (name, version) {
            let mut m = PluginManifest::new(n, v);
            if let Some(dn) = display_name {
                m.display_name = dn;
            }
            if let Some(a) = author {
                m.author = a;
            }
            if let Some(d) = description {
                m.description = d;
            }
            if let Some(ver) = min_engine_version {
                m.min_engine_version = ver;
            }
            m.has_editor_hooks = has_editor_hooks;
            Some(m)
        } else {
            None
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Plugin Errors
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    AlreadyInitialized,
    NotFound(String),
    DependencyMissing {
        plugin: String,
        dependency: String,
    },
    VersionMismatch {
        plugin: String,
        expected: String,
        found: String,
    },
    InvalidManifest(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::AlreadyInitialized => write!(f, "plugin registry already initialized"),
            PluginError::NotFound(name) => write!(f, "plugin '{}' not found", name),
            PluginError::DependencyMissing { plugin, dependency } => {
                write!(
                    f,
                    "plugin '{}' requires missing dependency '{}'",
                    plugin, dependency
                )
            }
            PluginError::VersionMismatch {
                plugin,
                expected,
                found,
            } => {
                write!(
                    f,
                    "plugin '{}' version mismatch: expected {}, found {}",
                    plugin, expected, found
                )
            }
            PluginError::InvalidManifest(msg) => write!(f, "invalid manifest: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        init_called: bool,
        update_count: u32,
        shutdown_called: bool,
    }

    impl TestPlugin {
        fn new() -> Self {
            TestPlugin {
                init_called: false,
                update_count: 0,
                shutdown_called: false,
            }
        }
    }

    impl Plugin for TestPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest::new("test_plugin", "0.1.0")
                .with_author("Test")
                .with_description("A test plugin")
        }

        fn on_init(&mut self, ctx: &mut PluginContext) {
            self.init_called = true;
            ctx.log_info("initialized");
        }

        fn on_update(&mut self, _ctx: &mut PluginContext, _dt: f32) {
            self.update_count += 1;
        }

        fn on_shutdown(&mut self, ctx: &mut PluginContext) {
            self.shutdown_called = true;
            ctx.log_warn("shutting down");
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    // Test 1: Register and manifest retrieval.
    #[test]
    fn registry_register_and_manifest() {
        let mut reg = PluginRegistry::new();
        let plugin = TestPlugin::new();
        let manifest = plugin.manifest();
        reg.register(Box::new(plugin)).unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.manifest(0).unwrap().name, "test_plugin");
    }

    // Test 2: Init lifecycle.
    #[test]
    fn registry_init_lifecycle() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();

        let mut world = World::new();
        reg.init_all(&mut world);
        assert!(reg.log_output.iter().any(|l| l.contains("initialized")));
    }

    // Test 3: Update lifecycle.
    #[test]
    fn registry_update_lifecycle() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();

        let mut world = World::new();
        reg.init_all(&mut world);
        reg.update_all(&mut world, 0.016, 1);
        reg.update_all(&mut world, 0.016, 2);

        let plugin = reg.plugin_mut(0).unwrap();
        let downcast = plugin.as_any_mut().downcast_mut::<TestPlugin>().unwrap();
        assert_eq!(downcast.update_count, 2);
    }

    // Test 4: Shutdown lifecycle.
    #[test]
    fn registry_shutdown_lifecycle() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();

        let mut world = World::new();
        reg.init_all(&mut world);
        reg.shutdown_all(&mut world);
        assert!(reg.log_output.iter().any(|l| l.contains("shutting down")));
    }

    // Test 5: Cannot register after init.
    #[test]
    fn registry_no_register_after_init() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();

        let mut world = World::new();
        reg.init_all(&mut world);
        let result = reg.register(Box::new(TestPlugin::new()));
        assert!(matches!(result, Err(PluginError::AlreadyInitialized)));
    }

    // Test 6: Find by name.
    #[test]
    fn registry_find_by_name() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();
        assert_eq!(reg.find_by_name("test_plugin"), Some(0));
        assert_eq!(reg.find_by_name("missing"), None);
    }

    // Test 7: Manifest builder.
    #[test]
    fn manifest_builder() {
        let m = PluginManifest::new("my.plugin", "2.0.0")
            .with_display_name("My Plugin")
            .with_author("Alice")
            .with_description("Does things")
            .with_dependencies(vec!["dep1".into(), "dep2".into()])
            .with_min_engine_version("1.5.0")
            .with_editor_hooks(true);

        assert_eq!(m.name, "my.plugin");
        assert_eq!(m.version, "2.0.0");
        assert_eq!(m.display_name, "My Plugin");
        assert_eq!(m.author, "Alice");
        assert_eq!(m.description, "Does things");
        assert_eq!(m.dependencies, vec!["dep1", "dep2"]);
        assert_eq!(m.min_engine_version, "1.5.0");
        assert!(m.has_editor_hooks);
    }

    // Test 8: TOML manifest parsing.
    #[test]
    fn toml_manifest_parsing() {
        let toml = r#"
name = "cool_plugin"
version = "1.2.3"
display_name = "Cool Plugin"
author = "Bob"
description = "Very cool"
min_engine_version = "1.0.0"
dependencies = ["dep_a", "dep_b"]
has_editor_hooks = true
"#;
        let m = PluginLoader::parse_toml_manifest(toml).unwrap();
        assert_eq!(m.name, "cool_plugin");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.display_name, "Cool Plugin");
        assert_eq!(m.author, "Bob");
        assert_eq!(m.dependencies, vec!["dep_a", "dep_b"]);
        assert!(m.has_editor_hooks);
    }

    // Test 9: JSON manifest parsing.
    #[test]
    fn json_manifest_parsing() {
        let json = r#"
{
    "name": "json_plugin",
    "version": "3.0.0",
    "display_name": "JSON Plugin",
    "author": "Carol",
    "description": "Loaded from JSON",
    "min_engine_version": "2.0.0",
    "has_editor_hooks": true
}
"#;
        let m = PluginLoader::parse_json_manifest(json).unwrap();
        assert_eq!(m.name, "json_plugin");
        assert_eq!(m.version, "3.0.0");
        assert_eq!(m.display_name, "JSON Plugin");
        assert_eq!(m.author, "Carol");
        assert_eq!(m.min_engine_version, "2.0.0");
        assert!(m.has_editor_hooks);
    }

    // Test 10: PluginError display.
    #[test]
    fn plugin_error_display() {
        let e = PluginError::NotFound("foo".into());
        assert!(e.to_string().contains("foo"));
        let e = PluginError::AlreadyInitialized;
        assert!(e.to_string().contains("already initialized"));
    }

    // Test 11: Remove by name before init.
    #[test]
    fn registry_remove_before_init() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();
        reg.remove_by_name("test_plugin").unwrap();
        assert!(reg.is_empty());
    }

    // Test 12: Cannot remove after init.
    #[test]
    fn registry_no_remove_after_init() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(TestPlugin::new())).unwrap();
        let mut world = World::new();
        reg.init_all(&mut world);
        assert!(matches!(
            reg.remove_by_name("test_plugin"),
            Err(PluginError::AlreadyInitialized)
        ));
    }

    // Test 13: Empty registry is harmless.
    #[test]
    fn empty_registry_lifecycle() {
        let mut reg = PluginRegistry::new();
        let mut world = World::new();
        reg.init_all(&mut world);
        reg.update_all(&mut world, 0.016, 1);
        reg.shutdown_all(&mut world);
        assert!(reg.log_output.is_empty());
    }
}

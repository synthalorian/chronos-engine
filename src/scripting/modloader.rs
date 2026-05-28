//! Mod loading system for Chronos Engine — Phase 9.
//!
//! Provides programmatic mod registration, metadata management,
//! dependency resolution (topological sort), and lifecycle control.
//! No filesystem I/O — mods are created in code and registered via
//! [`ModLoader`].

use std::collections::{HashMap, HashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// ModError
// ---------------------------------------------------------------------------

/// Errors produced by the mod loading system.
#[derive(Debug, Clone)]
pub enum ModError {
    /// A requested mod was not found.
    NotFound(String),
    /// A mod with the same name is already loaded.
    AlreadyLoaded(String),
    /// The mod requires a different engine version.
    IncompatibleVersion {
        mod_name: String,
        required: String,
        engine: String,
    },
    /// A dependency referenced by the mod is not loaded.
    MissingDependency {
        mod_name: String,
        missing: String,
    },
    /// A dependency cycle was detected.
    CircularDependency(Vec<String>),
    /// Failed to parse mod data (e.g. malformed JSON).
    ParseError(String),
    /// Metadata validation failed.
    InvalidMetadata(String),
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModError::NotFound(name) => write!(f, "mod not found: {}", name),
            ModError::AlreadyLoaded(name) => write!(f, "mod already loaded: {}", name),
            ModError::IncompatibleVersion {
                mod_name,
                required,
                engine,
            } => write!(
                f,
                "mod '{}' requires engine version {}, but running {}",
                mod_name, required, engine
            ),
            ModError::MissingDependency {
                mod_name,
                missing,
            } => write!(
                f,
                "mod '{}' is missing dependency: {}",
                mod_name, missing
            ),
            ModError::CircularDependency(cycle) => {
                write!(f, "circular dependency detected: {}", cycle.join(" -> "))
            }
            ModError::ParseError(msg) => write!(f, "parse error: {}", msg),
            ModError::InvalidMetadata(msg) => write!(f, "invalid metadata: {}", msg),
        }
    }
}

impl std::error::Error for ModError {}

// ---------------------------------------------------------------------------
// ModMetadata
// ---------------------------------------------------------------------------

/// Metadata describing a mod — name, version, author, dependencies, etc.
#[derive(Debug, Clone)]
pub struct ModMetadata {
    /// Unique mod identifier.
    pub name: String,
    /// Semantic version string (e.g. "1.0.0").
    pub version: String,
    /// Author name.
    pub author: String,
    /// Human-readable description.
    pub description: String,
    /// Minimum compatible engine version.
    pub game_version: String,
    /// Names of mods this mod depends on.
    pub dependencies: Vec<String>,
    /// Main script filename (e.g. "main.rhai").
    pub entry_point: String,
}

impl ModMetadata {
    /// Create minimal metadata with a name and version.
    pub fn new(name: &str, version: &str) -> Self {
        ModMetadata {
            name: name.to_string(),
            version: version.to_string(),
            author: String::new(),
            description: String::new(),
            game_version: String::new(),
            dependencies: Vec::new(),
            entry_point: "main.rhai".to_string(),
        }
    }

    /// Builder: set the author.
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }

    /// Builder: set the description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Builder: set the compatible engine version.
    pub fn with_game_version(mut self, ver: &str) -> Self {
        self.game_version = ver.to_string();
        self
    }

    /// Builder: add a dependency.
    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.dependencies.push(dep.to_string());
        self
    }

    /// Builder: set the entry point script filename.
    pub fn with_entry_point(mut self, entry: &str) -> Self {
        self.entry_point = entry.to_string();
        self
    }

    /// Check compatibility using a simple version-prefix check.
    ///
    /// Returns `true` when the mod's `game_version` is empty (no constraint),
    /// or when `engine_version` starts with `game_version`.
    pub fn is_compatible(&self, engine_version: &str) -> bool {
        if self.game_version.is_empty() {
            return true;
        }
        engine_version.starts_with(&self.game_version)
    }

    /// Serialize metadata to a JSON string.
    ///
    /// Hand-rolled because the `serialize` feature is separate from
    /// `scripting`.
    pub fn to_json(&self) -> String {
        let deps: Vec<String> = self
            .dependencies
            .iter()
            .map(|d| format!("\"{}\"", escape_json(d)))
            .collect();
        format!(
            concat!(
                "{{\n",
                "  \"name\": \"{}\",\n",
                "  \"version\": \"{}\",\n",
                "  \"author\": \"{}\",\n",
                "  \"description\": \"{}\",\n",
                "  \"game_version\": \"{}\",\n",
                "  \"dependencies\": [{}],\n",
                "  \"entry_point\": \"{}\"\n",
                "}}"
            ),
            escape_json(&self.name),
            escape_json(&self.version),
            escape_json(&self.author),
            escape_json(&self.description),
            escape_json(&self.game_version),
            deps.join(", "),
            escape_json(&self.entry_point),
        )
    }

    /// Deserialize metadata from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, ModError> {
        let extract = |key: &str| -> Result<String, ModError> {
            let pattern = format!("\"{}\"", key);
            let start = json
                .find(&pattern)
                .ok_or_else(|| ModError::ParseError(format!("missing key: {}", key)))?;
            let after_key = &json[start + pattern.len()..];
            // skip colon and whitespace
            let rest = after_key.trim_start();
            if !rest.starts_with(':') {
                return Err(ModError::ParseError(format!("expected ':' after key {}", key)));
            }
            let rest = rest[1..].trim_start();

            if key == "dependencies" {
                // parse array — not a string
                return Ok(String::new()); // placeholder, handled below
            }

            // string value
            if !rest.starts_with('"') {
                return Err(ModError::ParseError(format!(
                    "expected string value for {}",
                    key
                )));
            }
            let val_start = 1;
            let val_end = rest[1..]
                .find('"')
                .ok_or_else(|| ModError::ParseError(format!("unterminated string for {}", key)))?
                + 1;
            Ok(rest[val_start..val_end].to_string())
        };

        let name = extract("name")?;
        let version = extract("version")?;
        let author = extract("author").unwrap_or_default();
        let description = extract("description").unwrap_or_default();
        let game_version = extract("game_version").unwrap_or_default();
        let entry_point = extract("entry_point").unwrap_or_else(|_| "main.rhai".to_string());

        // Parse dependencies array
        let dependencies = parse_json_string_array(json, "dependencies")?;

        Ok(ModMetadata {
            name,
            version,
            author,
            description,
            game_version,
            dependencies,
            entry_point,
        })
    }
}

// ---------------------------------------------------------------------------
// ModEntry
// ---------------------------------------------------------------------------

/// A single script file within a mod.
#[derive(Debug, Clone)]
pub struct ModEntry {
    /// Relative path within the mod archive.
    pub path: String,
    /// Script source code.
    pub source: String,
}

impl ModEntry {
    /// Create a new script entry.
    pub fn new(path: &str, source: &str) -> Self {
        ModEntry {
            path: path.to_string(),
            source: source.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Mod
// ---------------------------------------------------------------------------

/// A loaded mod containing metadata and all its scripts.
#[derive(Debug, Clone)]
pub struct Mod {
    /// Mod metadata.
    pub metadata: ModMetadata,
    /// Script files bundled with this mod.
    pub scripts: Vec<ModEntry>,
    /// Whether the mod is currently enabled.
    pub enabled: bool,
    /// Load-order priority (lower = loaded first).
    pub load_order: i32,
}

impl Mod {
    /// Create a new mod from metadata.
    pub fn new(metadata: ModMetadata) -> Self {
        Mod {
            metadata,
            scripts: Vec::new(),
            enabled: true,
            load_order: 0,
        }
    }

    /// Add a script file to the mod.
    pub fn add_script(&mut self, path: &str, source: &str) {
        self.scripts.push(ModEntry::new(path, source));
    }

    /// Look up a script by its relative path.
    pub fn get_script(&self, path: &str) -> Option<&ModEntry> {
        self.scripts.iter().find(|s| s.path == path)
    }

    /// List all script paths in this mod.
    pub fn list_scripts(&self) -> Vec<String> {
        self.scripts.iter().map(|s| s.path.clone()).collect()
    }

    /// Number of scripts in this mod.
    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    /// Get the source code of the entry-point script, if present.
    pub fn entry_point_source(&self) -> Option<&str> {
        self.get_script(&self.metadata.entry_point).map(|e| e.source.as_str())
    }

    /// Enable the mod.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the mod.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check whether the mod is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ---------------------------------------------------------------------------
// ModBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for constructing [`Mod`] instances programmatically.
pub struct ModBuilder {
    metadata: ModMetadata,
    scripts: Vec<ModEntry>,
}

impl ModBuilder {
    /// Start building a mod with a name and version.
    pub fn new(name: &str, version: &str) -> Self {
        ModBuilder {
            metadata: ModMetadata::new(name, version),
            scripts: Vec::new(),
        }
    }

    /// Set the author.
    pub fn author(mut self, author: &str) -> Self {
        self.metadata.author = author.to_string();
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: &str) -> Self {
        self.metadata.description = desc.to_string();
        self
    }

    /// Add a script with an arbitrary path.
    pub fn script(mut self, path: &str, source: &str) -> Self {
        self.scripts.push(ModEntry::new(path, source));
        self
    }

    /// Convenience: add the entry-point script (`main.rhai`).
    pub fn entry(mut self, source: &str) -> Self {
        self.scripts
            .push(ModEntry::new(&self.metadata.entry_point.clone(), source));
        self
    }

    /// Add a dependency.
    pub fn dependency(mut self, dep: &str) -> Self {
        self.metadata.dependencies.push(dep.to_string());
        self
    }

    /// Build the final [`Mod`].
    pub fn build(self) -> Mod {
        Mod {
            metadata: self.metadata,
            scripts: self.scripts,
            enabled: true,
            load_order: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// ModLoader
// ---------------------------------------------------------------------------

/// Manages loading, querying, and lifecycle of mods.
pub struct ModLoader {
    /// Registered mods keyed by name.
    mods: HashMap<String, Mod>,
    /// Ordered list of mod names (load order).
    load_order: Vec<String>,
    /// Current engine version for compatibility checks.
    engine_version: String,
}

impl ModLoader {
    /// Create a new mod loader for the given engine version.
    pub fn new(engine_version: &str) -> Self {
        ModLoader {
            mods: HashMap::new(),
            load_order: Vec::new(),
            engine_version: engine_version.to_string(),
        }
    }

    /// Register a mod by its metadata.
    ///
    /// Returns `Err(ModError::AlreadyLoaded)` if a mod with the same name
    /// already exists. Returns `Err(ModError::IncompatibleVersion)` if the
    /// mod's required engine version does not match.
    pub fn load_mod(&mut self, metadata: ModMetadata) -> Result<(), ModError> {
        if self.mods.contains_key(&metadata.name) {
            return Err(ModError::AlreadyLoaded(metadata.name.clone()));
        }
        if !metadata.is_compatible(&self.engine_version) {
            return Err(ModError::IncompatibleVersion {
                mod_name: metadata.name.clone(),
                required: metadata.game_version.clone(),
                engine: self.engine_version.clone(),
            });
        }
        let name = metadata.name.clone();
        let m = Mod::new(metadata);
        self.mods.insert(name.clone(), m);
        self.load_order.push(name);
        Ok(())
    }

    /// Add a script file to an already-loaded mod.
    pub fn add_script_to_mod(
        &mut self,
        mod_name: &str,
        path: &str,
        source: &str,
    ) -> Result<(), ModError> {
        let m = self
            .mods
            .get_mut(mod_name)
            .ok_or_else(|| ModError::NotFound(mod_name.to_string()))?;
        m.add_script(path, source);
        Ok(())
    }

    /// Remove a mod. Returns `true` if the mod existed and was removed.
    pub fn unload_mod(&mut self, name: &str) -> bool {
        if self.mods.remove(name).is_some() {
            self.load_order.retain(|n| n != name);
            true
        } else {
            false
        }
    }

    /// Get an immutable reference to a loaded mod.
    pub fn get_mod(&self, name: &str) -> Option<&Mod> {
        self.mods.get(name)
    }

    /// Get a mutable reference to a loaded mod.
    pub fn get_mod_mut(&mut self, name: &str) -> Option<&mut Mod> {
        self.mods.get_mut(name)
    }

    /// List all registered mod names in load order.
    pub fn list_mods(&self) -> Vec<String> {
        self.load_order.clone()
    }

    /// Get all enabled mods in load order.
    pub fn enabled_mods(&self) -> Vec<&Mod> {
        self.load_order
            .iter()
            .filter_map(|name| {
                self.mods.get(name).filter(|m| m.enabled)
            })
            .collect()
    }

    /// Collect all scripts from all enabled mods.
    ///
    /// Returns `(mod_name, script_path, source)` tuples.
    pub fn all_scripts(&self) -> Vec<(String, String, &str)> {
        let mut out = Vec::new();
        for name in &self.load_order {
            if let Some(m) = self.mods.get(name) {
                if m.enabled {
                    for entry in &m.scripts {
                        out.push((name.clone(), entry.path.clone(), entry.source.as_str()));
                    }
                }
            }
        }
        out
    }

    /// Number of registered mods.
    pub fn mod_count(&self) -> usize {
        self.mods.len()
    }

    /// Resolve dependency order via topological sort (Kahn's algorithm).
    ///
    /// Returns an ordered list of mod names such that every mod appears
    /// after all of its dependencies.
    pub fn resolve_dependencies(&self) -> Result<Vec<String>, ModError> {
        let names: HashSet<&str> = self.mods.keys().map(|s| s.as_str()).collect();

        // Build adjacency: dep → mod (dep must load before mod)
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for name in &names {
            in_degree.entry(name).or_insert(0);
            adj.entry(name).or_insert_with(Vec::new);
        }

        for (name, m) in &self.mods {
            for dep in &m.metadata.dependencies {
                if !names.contains(dep.as_str()) {
                    return Err(ModError::MissingDependency {
                        mod_name: name.clone(),
                        missing: dep.clone(),
                    });
                }
                adj.entry(dep.as_str()).or_default().push(name.as_str());
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
            }
        }

        // Seed with zero-degree nodes, respecting existing load order for stability
        let mut queue: Vec<&str> = names
            .iter()
            .filter(|n| *in_degree.get(*n).unwrap_or(&0) == 0)
            .copied()
            .collect();

        // Sort seed by load_order position for deterministic output
        let order_index: HashMap<&str, usize> = self
            .load_order
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        queue.sort_by_key(|n| order_index.get(*n).unwrap_or(&usize::MAX));

        let mut result: Vec<String> = Vec::new();

        while let Some(node) = queue.first().copied() {
            queue.remove(0);
            result.push(node.to_string());

            let neighbors = adj.get(node).cloned().unwrap_or_default();
            for &neighbor in &neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
            // Keep queue sorted by load order for stability
            queue.sort_by_key(|n| order_index.get(*n).unwrap_or(&usize::MAX));
        }

        if result.len() != names.len() {
            // Nodes remaining have cycles
            let remaining: Vec<String> = names
                .iter()
                .filter(|n| !result.contains(&n.to_string()))
                .map(|n| n.to_string())
                .collect();
            return Err(ModError::CircularDependency(remaining));
        }

        Ok(result)
    }

    /// Manually override the load order.
    pub fn set_load_order(&mut self, order: Vec<String>) {
        self.load_order = order;
    }

    /// Validate all mods: check dependencies exist, versions are compatible,
    /// and entry-point scripts are present.
    pub fn validate_all(&self) -> Result<(), Vec<ModError>> {
        let mut errors: Vec<ModError> = Vec::new();

        for (name, m) in &self.mods {
            // Check compatibility
            if !m.metadata.is_compatible(&self.engine_version) {
                errors.push(ModError::IncompatibleVersion {
                    mod_name: name.clone(),
                    required: m.metadata.game_version.clone(),
                    engine: self.engine_version.clone(),
                });
            }

            // Check dependencies
            for dep in &m.metadata.dependencies {
                if !self.mods.contains_key(dep) {
                    errors.push(ModError::MissingDependency {
                        mod_name: name.clone(),
                        missing: dep.clone(),
                    });
                }
            }

            // Check entry-point script exists
            if m.get_script(&m.metadata.entry_point).is_none() {
                errors.push(ModError::InvalidMetadata(format!(
                    "mod '{}' is missing entry-point script: {}",
                    name, m.metadata.entry_point
                )));
            }
        }

        // Check for circular dependencies
        if let Err(e) = self.resolve_dependencies() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ---------------------------------------------------------------------------
// JSON helpers (no serde dependency)
// ---------------------------------------------------------------------------

/// Escape a string for embedding in a JSON value.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// Parse a JSON string array value for a given key.
fn parse_json_string_array(json: &str, key: &str) -> Result<Vec<String>, ModError> {
    let pattern = format!("\"{}\"", key);
    let start = json
        .find(&pattern)
        .ok_or_else(|| ModError::ParseError(format!("missing key: {}", key)))?;
    let rest = &json[start + pattern.len()..];
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':').ok_or_else(|| {
        ModError::ParseError(format!("expected ':' after key {}", key))
    })?;
    let rest = rest.trim_start();

    if !rest.starts_with('[') {
        return Err(ModError::ParseError(format!(
            "expected array for key {}",
            key
        )));
    }

    let bracket_end = rest
        .find(']')
        .ok_or_else(|| ModError::ParseError("unterminated array".to_string()))?;
    let inner = rest[1..bracket_end].trim();

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        // skip whitespace / comma
        while let Some(&c) = chars.peek() {
            if c == ' ' || c == ',' || c == '\n' || c == '\r' || c == '\t' {
                chars.next();
            } else {
                break;
            }
        }
        if chars.peek().is_none() {
            break;
        }
        // expect opening quote
        if chars.next() != Some('"') {
            return Err(ModError::ParseError("expected '\"' in array".to_string()));
        }
        let mut val = String::new();
        loop {
            match chars.next() {
                Some('\\') => {
                    if let Some(c) = chars.next() {
                        match c {
                            '"' => val.push('"'),
                            '\\' => val.push('\\'),
                            'n' => val.push('\n'),
                            'r' => val.push('\r'),
                            't' => val.push('\t'),
                            _ => {
                                val.push('\\');
                                val.push(c);
                            }
                        }
                    }
                }
                Some('"') => break,
                Some(c) => val.push(c),
                None => {
                    return Err(ModError::ParseError(
                        "unterminated string in array".to_string(),
                    ))
                }
            }
        }
        items.push(val);
    }

    Ok(items)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ModMetadata ----

    #[test]
    fn metadata_creation_and_builder() {
        let meta = ModMetadata::new("test_mod", "1.0.0")
            .with_author("synth")
            .with_description("A test mod")
            .with_game_version("0.3")
            .with_dependency("base_mod")
            .with_entry_point("init.rhai");

        assert_eq!(meta.name, "test_mod");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.author, "synth");
        assert_eq!(meta.description, "A test mod");
        assert_eq!(meta.game_version, "0.3");
        assert_eq!(meta.dependencies, vec!["base_mod"]);
        assert_eq!(meta.entry_point, "init.rhai");
    }

    #[test]
    fn metadata_json_roundtrip() {
        let original = ModMetadata::new("roundtrip", "2.1.0")
            .with_author("author")
            .with_description("desc")
            .with_game_version("0.3")
            .with_dependency("dep_a")
            .with_dependency("dep_b")
            .with_entry_point("start.rhai");

        let json = original.to_json();
        let parsed = ModMetadata::from_json(&json).expect("parse should succeed");

        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.author, original.author);
        assert_eq!(parsed.description, original.description);
        assert_eq!(parsed.game_version, original.game_version);
        assert_eq!(parsed.dependencies, original.dependencies);
        assert_eq!(parsed.entry_point, original.entry_point);
    }

    #[test]
    fn metadata_is_compatible() {
        let meta = ModMetadata::new("m", "1.0.0").with_game_version("0.3");
        assert!(meta.is_compatible("0.3.0"));
        assert!(meta.is_compatible("0.3"));
        assert!(!meta.is_compatible("0.4.0"));

        // Empty game_version = always compatible
        let unrestricted = ModMetadata::new("m2", "1.0.0");
        assert!(unrestricted.is_compatible("99.99"));
    }

    // ---- ModEntry ----

    #[test]
    fn mod_entry_creation() {
        let entry = ModEntry::new("scripts/main.rhai", "print(\"hello\")");
        assert_eq!(entry.path, "scripts/main.rhai");
        assert_eq!(entry.source, "print(\"hello\")");
    }

    // ---- Mod ----

    #[test]
    fn mod_creation_and_add_script() {
        let meta = ModMetadata::new("weapons", "1.0.0");
        let mut m = Mod::new(meta);

        m.add_script("main.rhai", "// entry");
        m.add_script("utils.rhai", "// util");

        assert_eq!(m.script_count(), 2);
        assert_eq!(m.list_scripts(), vec!["main.rhai", "utils.rhai"]);

        let fetched = m.get_script("utils.rhai").expect("should exist");
        assert_eq!(fetched.source, "// util");

        assert!(m.get_script("nonexistent.rhai").is_none());
    }

    #[test]
    fn mod_entry_point_source() {
        let meta = ModMetadata::new("ep", "1.0.0").with_entry_point("main.rhai");
        let mut m = Mod::new(meta);
        assert!(m.entry_point_source().is_none());

        m.add_script("main.rhai", "fn main() {}");
        assert_eq!(m.entry_point_source(), Some("fn main() {}"));
    }

    #[test]
    fn mod_enable_disable() {
        let meta = ModMetadata::new("toggle", "1.0.0");
        let mut m = Mod::new(meta);

        assert!(m.is_enabled());
        m.disable();
        assert!(!m.is_enabled());
        m.enable();
        assert!(m.is_enabled());
    }

    // ---- ModBuilder ----

    #[test]
    fn mod_builder_fluent() {
        let m = ModBuilder::new("built", "3.0.0")
            .author("builder")
            .description("built mod")
            .dependency("core")
            .entry("fn init() {}")
            .script("extra.rhai", "fn extra() {}")
            .build();

        assert_eq!(m.metadata.name, "built");
        assert_eq!(m.metadata.version, "3.0.0");
        assert_eq!(m.metadata.author, "builder");
        assert_eq!(m.metadata.description, "built mod");
        assert_eq!(m.metadata.dependencies, vec!["core"]);
        assert_eq!(m.script_count(), 2);
        assert_eq!(m.entry_point_source(), Some("fn init() {}"));
        assert!(m.is_enabled());
    }

    // ---- ModLoader: load / get / unload ----

    #[test]
    fn loader_load_and_get() {
        let mut loader = ModLoader::new("0.3.0");

        let meta = ModMetadata::new("alpha", "1.0.0");
        assert!(loader.load_mod(meta).is_ok());
        assert_eq!(loader.mod_count(), 1);

        let m = loader.get_mod("alpha").expect("should exist");
        assert_eq!(m.metadata.name, "alpha");

        // Duplicate
        let dup = ModMetadata::new("alpha", "2.0.0");
        assert!(matches!(
            loader.load_mod(dup),
            Err(ModError::AlreadyLoaded(_))
        ));
    }

    #[test]
    fn loader_unload() {
        let mut loader = ModLoader::new("0.3.0");
        loader.load_mod(ModMetadata::new("tmp", "1.0.0")).unwrap();

        assert!(loader.unload_mod("tmp"));
        assert!(!loader.unload_mod("tmp")); // already gone
        assert_eq!(loader.mod_count(), 0);
        assert!(loader.get_mod("tmp").is_none());
    }

    // ---- ModLoader: list / enabled ----

    #[test]
    fn loader_list_and_enabled() {
        let mut loader = ModLoader::new("0.3.0");
        loader
            .load_mod(ModMetadata::new("a", "1.0.0"))
            .unwrap();
        loader
            .load_mod(ModMetadata::new("b", "1.0.0"))
            .unwrap();

        assert_eq!(loader.list_mods(), vec!["a", "b"]);
        assert_eq!(loader.enabled_mods().len(), 2);

        loader.get_mod_mut("a").unwrap().disable();
        let enabled = loader.enabled_mods();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].metadata.name, "b");
    }

    // ---- ModLoader: all_scripts ----

    #[test]
    fn loader_all_scripts() {
        let mut loader = ModLoader::new("0.3.0");
        loader.load_mod(ModMetadata::new("s1", "1.0.0")).unwrap();
        loader.add_script_to_mod("s1", "main.rhai", "code_a").unwrap();

        loader.load_mod(ModMetadata::new("s2", "1.0.0")).unwrap();
        loader.add_script_to_mod("s2", "main.rhai", "code_b").unwrap();
        loader.add_script_to_mod("s2", "util.rhai", "code_c").unwrap();

        // Disable s2 — only s1 scripts should appear
        loader.get_mod_mut("s2").unwrap().disable();
        let scripts = loader.all_scripts();
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0], ("s1".to_string(), "main.rhai".to_string(), "code_a"));
    }

    // ---- ModLoader: resolve_dependencies ----

    #[test]
    fn loader_resolve_dependencies_chain() {
        let mut loader = ModLoader::new("0.3.0");

        // base <- mid <- top
        loader
            .load_mod(ModMetadata::new("base", "1.0.0"))
            .unwrap();
        loader
            .load_mod(
                ModMetadata::new("mid", "1.0.0").with_dependency("base"),
            )
            .unwrap();
        loader
            .load_mod(
                ModMetadata::new("top", "1.0.0").with_dependency("mid"),
            )
            .unwrap();

        let order = loader.resolve_dependencies().unwrap();
        assert_eq!(order, vec!["base", "mid", "top"]);
    }

    #[test]
    fn loader_circular_dependency_detection() {
        let mut loader = ModLoader::new("0.3.0");

        loader
            .load_mod(
                ModMetadata::new("x", "1.0.0").with_dependency("y"),
            )
            .unwrap();
        loader
            .load_mod(
                ModMetadata::new("y", "1.0.0").with_dependency("x"),
            )
            .unwrap();

        let result = loader.resolve_dependencies();
        assert!(matches!(result, Err(ModError::CircularDependency(_))));
        if let Err(ModError::CircularDependency(cycle)) = result {
            assert!(cycle.contains(&"x".to_string()));
            assert!(cycle.contains(&"y".to_string()));
        }
    }

    // ---- ModError display ----

    #[test]
    fn error_display_variants() {
        assert_eq!(
            ModError::NotFound("foo".into()).to_string(),
            "mod not found: foo"
        );
        assert_eq!(
            ModError::AlreadyLoaded("bar".into()).to_string(),
            "mod already loaded: bar"
        );
        assert_eq!(
            ModError::IncompatibleVersion {
                mod_name: "m".into(),
                required: "0.2".into(),
                engine: "0.3".into(),
            }
            .to_string(),
            "mod 'm' requires engine version 0.2, but running 0.3"
        );
        assert_eq!(
            ModError::MissingDependency {
                mod_name: "m".into(),
                missing: "dep".into(),
            }
            .to_string(),
            "mod 'm' is missing dependency: dep"
        );
        assert_eq!(
            ModError::CircularDependency(vec!["a".into(), "b".into()]).to_string(),
            "circular dependency detected: a -> b"
        );
        assert_eq!(
            ModError::ParseError("bad".into()).to_string(),
            "parse error: bad"
        );
        assert_eq!(
            ModError::InvalidMetadata("nope".into()).to_string(),
            "invalid metadata: nope"
        );
    }

    // ---- ModLoader: validate_all ----

    #[test]
    fn loader_validate_all() {
        let mut loader = ModLoader::new("0.3.0");

        // Good mod
        loader.load_mod(ModMetadata::new("good", "1.0.0")).unwrap();
        loader.add_script_to_mod("good", "main.rhai", "ok").unwrap();

        // Mod missing entry point
        loader.load_mod(ModMetadata::new("no_entry", "1.0.0")).unwrap();

        // Mod with missing dependency
        loader
            .load_mod(
                ModMetadata::new("orphan", "1.0.0").with_dependency("nonexistent"),
            )
            .unwrap();

        let result = loader.validate_all();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // no_entry → InvalidMetadata, orphan → MissingDependency, plus circular (orphan dep missing)
        assert!(errors.len() >= 2);
    }

    // ---- Incompatible version rejection ----

    #[test]
    fn loader_rejects_incompatible_version() {
        let mut loader = ModLoader::new("0.3.0");
        let meta = ModMetadata::new("old", "1.0.0").with_game_version("0.2");
        let result = loader.load_mod(meta);
        assert!(matches!(
            result,
            Err(ModError::IncompatibleVersion { .. })
        ));
    }

    // ---- add_script_to_mod errors ----

    #[test]
    fn loader_add_script_missing_mod() {
        let mut loader = ModLoader::new("0.3.0");
        let result = loader.add_script_to_mod("ghost", "main.rhai", "code");
        assert!(matches!(result, Err(ModError::NotFound(_))));
    }
}

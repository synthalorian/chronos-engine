//! Script component, handle, and registry.
//!
//! [`ScriptHandle`] wraps source code and an optional compiled Rhai AST.
//! [`ScriptComponent`] attaches a handle to an entity with an enabled flag
//! and execution priority. [`ScriptRegistry`] manages named handles.

use std::collections::HashMap;

use super::bridge::{ScriptEngine, ScriptError};

// ──────────────────────────────────────────────
// ScriptHandle
// ──────────────────────────────────────────────

/// Reference to a loaded script — stores source and optional compiled AST.
pub struct ScriptHandle {
    name: String,
    ast: Option<rhai::AST>,
    source: String,
}

impl ScriptHandle {
    pub fn new(name: &str, source: &str) -> Self {
        ScriptHandle {
            name: name.to_string(),
            ast: None,
            source: source.to_string(),
        }
    }

    pub fn compile(&mut self, engine: &mut ScriptEngine) -> Result<(), ScriptError> {
        let compiled = engine.compile(&self.source)?;
        self.ast = Some(compiled);
        Ok(())
    }

    pub fn is_compiled(&self) -> bool {
        self.ast.is_some()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn ast(&self) -> Option<&rhai::AST> {
        self.ast.as_ref()
    }
}

// ──────────────────────────────────────────────
// ScriptComponent
// ──────────────────────────────────────────────

/// Attachable script component with enable/disable and priority ordering.
pub struct ScriptComponent {
    handle: ScriptHandle,
    enabled: bool,
    priority: i32,
}

impl ScriptComponent {
    pub fn new(name: &str, source: &str) -> Self {
        ScriptComponent {
            handle: ScriptHandle::new(name, source),
            enabled: true,
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn compile(&mut self, engine: &mut ScriptEngine) -> Result<(), ScriptError> {
        self.handle.compile(engine)
    }

    pub fn is_compiled(&self) -> bool {
        self.handle.is_compiled()
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn name(&self) -> &str {
        self.handle.name()
    }

    pub fn handle(&self) -> &ScriptHandle {
        &self.handle
    }

    pub fn handle_mut(&mut self) -> &mut ScriptHandle {
        &mut self.handle
    }
}

// ──────────────────────────────────────────────
// ScriptRegistry
// ──────────────────────────────────────────────

/// Manages all loaded script handles by name.
pub struct ScriptRegistry {
    scripts: HashMap<String, ScriptHandle>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        ScriptRegistry {
            scripts: HashMap::new(),
        }
    }

    pub fn load(
        &mut self,
        name: &str,
        source: &str,
        engine: &mut ScriptEngine,
    ) -> Result<(), ScriptError> {
        let mut handle = ScriptHandle::new(name, source);
        handle.compile(engine)?;
        self.scripts.insert(name.to_string(), handle);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ScriptHandle> {
        self.scripts.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ScriptHandle> {
        self.scripts.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.scripts.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn count(&self) -> usize {
        self.scripts.len()
    }

    pub fn reload(
        &mut self,
        name: &str,
        source: &str,
        engine: &mut ScriptEngine,
    ) -> Result<(), ScriptError> {
        let mut handle = ScriptHandle::new(name, source);
        handle.compile(engine)?;
        self.scripts.insert(name.to_string(), handle);
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }
}

impl Default for ScriptRegistry {
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

    fn make_engine() -> ScriptEngine {
        ScriptEngine::new()
    }

    // ── ScriptHandle ───────────────────────────

    #[test]
    fn test_script_handle_creation() {
        let h = ScriptHandle::new("test", "let x = 1;");
        assert_eq!(h.name(), "test");
        assert_eq!(h.source(), "let x = 1;");
        assert!(!h.is_compiled());
    }

    #[test]
    fn test_script_handle_compile_success() {
        let mut engine = make_engine();
        let mut h = ScriptHandle::new("ok", "let x = 1 + 2;");
        assert!(h.compile(&mut engine).is_ok());
        assert!(h.is_compiled());
        assert!(h.ast().is_some());
    }

    #[test]
    fn test_script_handle_compile_failure() {
        let mut engine = make_engine();
        let mut h = ScriptHandle::new("bad", "let x = ;");
        let result = h.compile(&mut engine);
        assert!(result.is_err());
        assert!(!h.is_compiled());
        match result.unwrap_err() {
            ScriptError::CompilationError(msg) => assert!(!msg.is_empty()),
            other => panic!("expected CompilationError, got {:?}", other),
        }
    }

    #[test]
    fn test_script_handle_is_compiled_states() {
        let mut engine = make_engine();
        let mut h = ScriptHandle::new("states", "42");
        assert!(!h.is_compiled());
        h.compile(&mut engine).unwrap();
        assert!(h.is_compiled());
    }

    // ── ScriptComponent ────────────────────────

    #[test]
    fn test_script_component_creation() {
        let c = ScriptComponent::new("comp", "let a = 1;");
        assert!(c.enabled());
        assert_eq!(c.priority(), 0);
        assert_eq!(c.name(), "comp");
        assert!(!c.is_compiled());
    }

    #[test]
    fn test_script_component_enable_disable() {
        let mut c = ScriptComponent::new("toggle", "1");
        assert!(c.enabled());
        c.disable();
        assert!(!c.enabled());
        c.enable();
        assert!(c.enabled());
    }

    #[test]
    fn test_script_component_with_priority() {
        let c = ScriptComponent::new("prio", "1").with_priority(10);
        assert_eq!(c.priority(), 10);

        let c2 = ScriptComponent::new("prio2", "1").with_priority(-5);
        assert_eq!(c2.priority(), -5);
    }

    #[test]
    fn test_script_component_disabled_builder() {
        let c = ScriptComponent::new("off", "1").disabled();
        assert!(!c.enabled());
    }

    #[test]
    fn test_script_component_compile() {
        let mut engine = make_engine();
        let mut c = ScriptComponent::new("run", "let x = 42;");
        assert!(c.compile(&mut engine).is_ok());
        assert!(c.is_compiled());
    }

    // ── ScriptRegistry ─────────────────────────

    #[test]
    fn test_script_registry_load_and_get() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        reg.load("player", "let hp = 100;", &mut engine).unwrap();
        assert!(reg.get("player").is_some());
        assert!(reg.get("player").unwrap().is_compiled());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn test_script_registry_remove() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        reg.load("temp", "1", &mut engine).unwrap();
        assert_eq!(reg.count(), 1);

        assert!(reg.remove("temp"));
        assert!(!reg.remove("temp")); // already gone
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_script_registry_list_and_count() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();
        assert_eq!(reg.count(), 0);

        reg.load("z_script", "1", &mut engine).unwrap();
        reg.load("a_script", "2", &mut engine).unwrap();
        reg.load("m_script", "3", &mut engine).unwrap();

        assert_eq!(reg.count(), 3);
        assert_eq!(reg.list(), vec!["a_script", "m_script", "z_script"]);
    }

    #[test]
    fn test_script_registry_reload() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        reg.load("mod", "let v = 1;", &mut engine).unwrap();
        assert!(reg.get("mod").unwrap().is_compiled());

        reg.reload("mod", "let v = 2;", &mut engine).unwrap();
        assert!(reg.get("mod").unwrap().is_compiled());
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn test_script_registry_load_failure() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        let result = reg.load("bad", "let x = ;", &mut engine);
        assert!(result.is_err());
        assert_eq!(reg.count(), 0);
        assert!(!reg.contains("bad"));
    }

    #[test]
    fn test_script_registry_get_mut() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        reg.load("editable", "let x = 1;", &mut engine).unwrap();
        let handle = reg.get_mut("editable").unwrap();
        assert!(handle.is_compiled());
    }

    #[test]
    fn test_script_registry_default() {
        let reg = ScriptRegistry::default();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_script_component_combined_workflow() {
        let mut engine = make_engine();
        let mut reg = ScriptRegistry::new();

        reg.load(
            "enemy_ai",
            "fn on_update(dt) { let speed = dt * 2.0; }",
            &mut engine,
        )
        .unwrap();

        let mut comp = ScriptComponent::new("enemy_ai", "let x = 1;")
            .with_priority(5)
            .disabled();

        comp.compile(&mut engine).unwrap();
        assert!(comp.is_compiled());
        assert!(!comp.enabled());
        assert_eq!(comp.priority(), 5);

        comp.enable();
        assert!(comp.enabled());
    }
}

//! Script lifecycle and execution system.
//!
//! Manages script hooks (`on_start`, `on_update`, `on_destroy`, etc.)
//! and provides the context passed to script execution. Scripts define
//! plain Rhai functions and the lifecycle system calls them at the right time.

use std::collections::HashSet;
use std::fmt;

// ──────────────────────────────────────────────
// ScriptHook
// ──────────────────────────────────────────────

/// Named lifecycle hooks that scripts can implement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptHook {
    /// Called when a script is first loaded / attached.
    OnStart,
    /// Called every frame with delta time.
    OnUpdate,
    /// Called when the owning entity is destroyed.
    OnDestroy,
    /// Called when a collision is detected.
    OnCollision,
    /// Called when a trigger zone is entered.
    OnTrigger,
    /// Called on input events (carries the event type string).
    OnInput(String),
}

impl ScriptHook {
    /// Returns the Rhai function name for this hook.
    pub fn function_name(&self) -> &str {
        match self {
            ScriptHook::OnStart => "on_start",
            ScriptHook::OnUpdate => "on_update",
            ScriptHook::OnDestroy => "on_destroy",
            ScriptHook::OnCollision => "on_collision",
            ScriptHook::OnTrigger => "on_trigger",
            ScriptHook::OnInput(_) => "on_input",
        }
    }
}

// ──────────────────────────────────────────────
// ScriptContext
// ──────────────────────────────────────────────

/// Context passed to script execution, carrying per-frame and per-entity data.
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// Entity index this script is attached to.
    pub entity_id: u32,
    /// Generational counter for the entity slot.
    pub entity_generation: u32,
    /// Seconds since the last frame.
    pub delta_time: f32,
    /// Monotonically increasing frame counter.
    pub frame_count: u64,
}

impl ScriptContext {
    /// Create a new context for the given entity slot.
    pub fn new(entity_id: u32, generation: u32) -> Self {
        ScriptContext {
            entity_id,
            entity_generation: generation,
            delta_time: 0.0,
            frame_count: 0,
        }
    }

    /// Attach delta-time to the context (builder style).
    pub fn with_delta(mut self, dt: f32) -> Self {
        self.delta_time = dt;
        self
    }

    /// Attach the current frame number (builder style).
    pub fn with_frame(mut self, frame: u64) -> Self {
        self.frame_count = frame;
        self
    }
}

// ──────────────────────────────────────────────
// CollisionData
// ──────────────────────────────────────────────

/// Payload delivered to the `on_collision` hook.
#[derive(Debug, Clone)]
pub struct CollisionData {
    /// Entity ID of the *other* entity involved.
    pub other_entity_id: u32,
    /// World-space X coordinate of the contact point.
    pub contact_x: f32,
    /// World-space Y coordinate of the contact point.
    pub contact_y: f32,
    /// Collision normal X component.
    pub normal_x: f32,
    /// Collision normal Y component.
    pub normal_y: f32,
    /// Overlap depth in world units.
    pub penetration: f32,
}

impl CollisionData {
    /// Minimal collision data — only the other entity ID, rest defaults.
    pub fn new(other_entity_id: u32) -> Self {
        CollisionData {
            other_entity_id,
            contact_x: 0.0,
            contact_y: 0.0,
            normal_x: 0.0,
            normal_y: 1.0,
            penetration: 0.0,
        }
    }

    /// Set the contact point.
    pub fn with_contact(mut self, x: f32, y: f32) -> Self {
        self.contact_x = x;
        self.contact_y = y;
        self
    }

    /// Set the collision normal.
    pub fn with_normal(mut self, x: f32, y: f32) -> Self {
        self.normal_x = x;
        self.normal_y = y;
        self
    }

    /// Set the penetration depth.
    pub fn with_penetration(mut self, p: f32) -> Self {
        self.penetration = p;
        self
    }
}

// ──────────────────────────────────────────────
// ScriptLifecycleError
// ──────────────────────────────────────────────

/// Errors produced by the script lifecycle runner.
#[derive(Debug)]
pub enum ScriptLifecycleError {
    /// The requested hook function was not defined in the script.
    FunctionNotFound(String),
    /// The hook function was found but raised a runtime error.
    ExecutionError(String),
    /// The script AST was never compiled.
    NotCompiled,
}

impl fmt::Display for ScriptLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptLifecycleError::FunctionNotFound(name) => {
                write!(f, "Script function not found: {}", name)
            }
            ScriptLifecycleError::ExecutionError(msg) => {
                write!(f, "Script execution error: {}", msg)
            }
            ScriptLifecycleError::NotCompiled => write!(f, "Script not compiled"),
        }
    }
}

impl std::error::Error for ScriptLifecycleError {}

// ──────────────────────────────────────────────
// ScriptLifecycle
// ──────────────────────────────────────────────

/// Manages script execution over the entity lifecycle.
///
/// Tracks which entities have already received their `on_start` call
/// so the hook fires exactly once. Missing hook functions are silently
/// treated as no-ops rather than errors.
pub struct ScriptLifecycle {
    started_scripts: HashSet<u32>,
}

impl ScriptLifecycle {
    /// Create a fresh lifecycle manager.
    pub fn new() -> Self {
        ScriptLifecycle {
            started_scripts: HashSet::new(),
        }
    }

    /// Fire `on_start()` for *entity_id*'s script.
    ///
    /// The call is made at most once per entity — subsequent invocations
    /// are silent no-ops.  If the script does not define `on_start`, that
    /// also counts as "started".
    pub fn on_start(
        &mut self,
        engine: &mut rhai::Engine,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
        entity_id: u32,
    ) -> Result<(), ScriptLifecycleError> {
        // Already started — skip.
        if self.started_scripts.contains(&entity_id) {
            return Ok(());
        }

        let result: Result<rhai::Dynamic, _> = engine.call_fn(scope, ast, "on_start", ());

        match result {
            Ok(_) => {
                self.started_scripts.insert(entity_id);
                Ok(())
            }
            Err(err) => {
                if is_function_not_found(&err) {
                    // Function absent → no-op, still mark started.
                    self.started_scripts.insert(entity_id);
                    Ok(())
                } else {
                    Err(ScriptLifecycleError::ExecutionError(err.to_string()))
                }
            }
        }
    }

    /// Fire `on_update(dt)` for the entity's script.
    ///
    /// `dt` is forwarded as an `f64` (Rhai's FLOAT type).  If the
    /// script does not define `on_update`, the call is a no-op.
    pub fn on_update(
        &mut self,
        engine: &mut rhai::Engine,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
        _entity_id: u32,
        dt: f32,
    ) -> Result<(), ScriptLifecycleError> {
        let result: Result<rhai::Dynamic, _> =
            engine.call_fn(scope, ast, "on_update", (dt as f64,));

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if is_function_not_found(&err) {
                    Ok(())
                } else {
                    Err(ScriptLifecycleError::ExecutionError(err.to_string()))
                }
            }
        }
    }

    /// Fire `on_destroy()` for the entity's script.
    ///
    /// Also removes the entity from the "started" set so that if the
    /// same slot is reused later, `on_start` will fire again.
    pub fn on_destroy(
        &mut self,
        engine: &mut rhai::Engine,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
        entity_id: u32,
    ) -> Result<(), ScriptLifecycleError> {
        let result: Result<rhai::Dynamic, _> = engine.call_fn(scope, ast, "on_destroy", ());

        match result {
            Ok(_) => {
                self.started_scripts.remove(&entity_id);
                Ok(())
            }
            Err(err) => {
                if is_function_not_found(&err) {
                    self.started_scripts.remove(&entity_id);
                    Ok(())
                } else {
                    Err(ScriptLifecycleError::ExecutionError(err.to_string()))
                }
            }
        }
    }

    /// Fire `on_collision(other_id, cx, cy, nx, ny, pen)` for the entity.
    ///
    /// All six fields of [`CollisionData`] are forwarded as individual
    /// Rhai arguments (`i64` for entity ID, `f64` for the rest).
    pub fn on_collision(
        &mut self,
        engine: &mut rhai::Engine,
        ast: &rhai::AST,
        scope: &mut rhai::Scope,
        _entity_id: u32,
        data: &CollisionData,
    ) -> Result<(), ScriptLifecycleError> {
        let result: Result<rhai::Dynamic, _> = engine.call_fn(
            scope,
            ast,
            "on_collision",
            (
                data.other_entity_id as i64,
                data.contact_x as f64,
                data.contact_y as f64,
                data.normal_x as f64,
                data.normal_y as f64,
                data.penetration as f64,
            ),
        );

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if is_function_not_found(&err) {
                    Ok(())
                } else {
                    Err(ScriptLifecycleError::ExecutionError(err.to_string()))
                }
            }
        }
    }

    /// Returns `true` if `on_start` has already been called for this entity.
    pub fn has_started(&self, entity_id: u32) -> bool {
        self.started_scripts.contains(&entity_id)
    }

    /// Reset all tracking state (e.g. between scenes).
    pub fn reset(&mut self) {
        self.started_scripts.clear();
    }
}

// ──────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────

/// Determine whether a Rhai evaluation error is "function not found".
///
/// We accept the error regardless of *which* function name is missing —
/// lifecycle hooks are always optional, so a missing function is benign.
fn is_function_not_found(err: &rhai::EvalAltResult) -> bool {
    matches!(err, rhai::EvalAltResult::ErrorFunctionNotFound(_, _))
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ScriptHook ────────────────────────────

    #[test]
    fn test_script_hook_variants_and_names() {
        assert_eq!(ScriptHook::OnStart.function_name(), "on_start");
        assert_eq!(ScriptHook::OnUpdate.function_name(), "on_update");
        assert_eq!(ScriptHook::OnDestroy.function_name(), "on_destroy");
        assert_eq!(ScriptHook::OnCollision.function_name(), "on_collision");
        assert_eq!(ScriptHook::OnTrigger.function_name(), "on_trigger");
        assert_eq!(
            ScriptHook::OnInput("key_down".into()).function_name(),
            "on_input"
        );
    }

    // ── ScriptContext ─────────────────────────

    #[test]
    fn test_script_context_new() {
        let ctx = ScriptContext::new(42, 7);
        assert_eq!(ctx.entity_id, 42);
        assert_eq!(ctx.entity_generation, 7);
        assert_eq!(ctx.delta_time, 0.0);
        assert_eq!(ctx.frame_count, 0);
    }

    #[test]
    fn test_script_context_builders() {
        let ctx = ScriptContext::new(10, 0).with_delta(0.016).with_frame(100);
        assert!((ctx.delta_time - 0.016).abs() < f32::EPSILON);
        assert_eq!(ctx.frame_count, 100);
    }

    // ── CollisionData ─────────────────────────

    #[test]
    fn test_collision_data_new_defaults() {
        let data = CollisionData::new(5);
        assert_eq!(data.other_entity_id, 5);
        assert_eq!(data.contact_x, 0.0);
        assert_eq!(data.contact_y, 0.0);
        assert_eq!(data.normal_x, 0.0);
        assert_eq!(data.normal_y, 1.0);
        assert_eq!(data.penetration, 0.0);
    }

    #[test]
    fn test_collision_data_builders() {
        let data = CollisionData::new(3)
            .with_contact(10.0, 20.0)
            .with_normal(0.0, -1.0)
            .with_penetration(0.5);
        assert_eq!(data.other_entity_id, 3);
        assert!((data.contact_x - 10.0).abs() < f32::EPSILON);
        assert!((data.contact_y - 20.0).abs() < f32::EPSILON);
        assert!((data.normal_y - (-1.0)).abs() < f32::EPSILON);
        assert!((data.penetration - 0.5).abs() < f32::EPSILON);
    }

    // ── ScriptLifecycle basic ─────────────────

    #[test]
    fn test_lifecycle_new_is_empty() {
        let lc = ScriptLifecycle::new();
        assert!(!lc.has_started(0));
        assert!(!lc.has_started(1));
        assert!(!lc.has_started(u32::MAX));
    }

    #[test]
    fn test_lifecycle_on_start_calls_function() {
        let mut engine = rhai::Engine::new();
        let ast = engine.compile("fn on_start() { }").unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        let result = lc.on_start(&mut engine, &ast, &mut scope, 1);
        assert!(result.is_ok());
        assert!(lc.has_started(1));
    }

    #[test]
    fn test_lifecycle_on_start_fires_only_once() {
        let mut engine = rhai::Engine::new();
        let ast = engine.compile("fn on_start() { }").unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        lc.on_start(&mut engine, &ast, &mut scope, 1).unwrap();
        assert!(lc.has_started(1));

        // Second call must be a silent no-op, still Ok.
        let second = lc.on_start(&mut engine, &ast, &mut scope, 1);
        assert!(second.is_ok());
        assert!(lc.has_started(1));
    }

    #[test]
    fn test_lifecycle_on_update_passes_dt() {
        let mut engine = rhai::Engine::new();
        let ast = engine.compile("fn on_update(dt) { }").unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        let result = lc.on_update(&mut engine, &ast, &mut scope, 1, 0.016);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lifecycle_on_destroy_removes_from_started() {
        let mut engine = rhai::Engine::new();
        let ast = engine
            .compile("fn on_start() { } fn on_destroy() { }")
            .unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        lc.on_start(&mut engine, &ast, &mut scope, 42).unwrap();
        assert!(lc.has_started(42));

        lc.on_destroy(&mut engine, &ast, &mut scope, 42).unwrap();
        assert!(!lc.has_started(42));
    }

    #[test]
    fn test_lifecycle_missing_function_is_noop() {
        let mut engine = rhai::Engine::new();
        // Script defines nothing relevant.
        let ast = engine.compile("let x = 42;").unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        // on_start
        let r1 = lc.on_start(&mut engine, &ast, &mut scope, 1);
        assert!(r1.is_ok());
        assert!(lc.has_started(1)); // still marked started

        // on_update
        let r2 = lc.on_update(&mut engine, &ast, &mut scope, 1, 0.016);
        assert!(r2.is_ok());

        // on_destroy
        let r3 = lc.on_destroy(&mut engine, &ast, &mut scope, 1);
        assert!(r3.is_ok());
        assert!(!lc.has_started(1));
    }

    #[test]
    fn test_lifecycle_reset_clears_everything() {
        let mut engine = rhai::Engine::new();
        let ast = engine.compile("fn on_start() { }").unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        lc.on_start(&mut engine, &ast, &mut scope, 1).unwrap();
        lc.on_start(&mut engine, &ast, &mut scope, 2).unwrap();
        lc.on_start(&mut engine, &ast, &mut scope, 3).unwrap();
        assert!(lc.has_started(1) && lc.has_started(2) && lc.has_started(3));

        lc.reset();
        assert!(!lc.has_started(1));
        assert!(!lc.has_started(2));
        assert!(!lc.has_started(3));
    }

    // ── ScriptLifecycleError display ──────────

    #[test]
    fn test_lifecycle_error_display() {
        assert_eq!(
            ScriptLifecycleError::FunctionNotFound("on_start".into()).to_string(),
            "Script function not found: on_start"
        );
        assert_eq!(
            ScriptLifecycleError::ExecutionError("overflow".into()).to_string(),
            "Script execution error: overflow"
        );
        assert_eq!(
            ScriptLifecycleError::NotCompiled.to_string(),
            "Script not compiled"
        );
    }

    // ── on_collision ──────────────────────────

    #[test]
    fn test_lifecycle_on_collision() {
        let mut engine = rhai::Engine::new();
        let ast = engine
            .compile("fn on_collision(other, cx, cy, nx, ny, pen) { }")
            .unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        let data = CollisionData::new(5)
            .with_contact(10.0, 20.0)
            .with_normal(0.0, -1.0)
            .with_penetration(0.5);

        let result = lc.on_collision(&mut engine, &ast, &mut scope, 1, &data);
        assert!(result.is_ok());
    }

    // ── execution error propagation ───────────

    #[test]
    fn test_lifecycle_execution_error_propagates() {
        let mut engine = rhai::Engine::new();
        let ast = engine
            .compile("fn on_start() { throw \"boom\"; }")
            .unwrap();
        let mut scope = rhai::Scope::new();
        let mut lc = ScriptLifecycle::new();

        let result = lc.on_start(&mut engine, &ast, &mut scope, 1);
        assert!(result.is_err());
        match result {
            Err(ScriptLifecycleError::ExecutionError(msg)) => {
                assert!(msg.contains("boom"));
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }
}

//! Rhai scripting engine bridge.
//!
//! Provides [`ScriptEngine`] — a thin wrapper around Rhai's [`rhai::Engine`] —
//! along with Rhai-compatible value types ([`ScriptVec2`], [`ScriptVec3`],
//! [`ScriptColor`], [`ScriptEntity`]) that mirror the engine's native maths
//! types but are safe to expose directly to user scripts.
//!
//! # Type conversion
//!
//! Rhai's built-in numeric types are `i64` (INT) and `f64` (FLOAT).  All
//! wrapper structs store `f32` / `u32` internally (matching the rest of
//! Chronos), and the Rhai registrations in [`ScriptEngine::register_stdlib`]
//! handle the `f64 ↔ f32` and `i64 ↔ u32` conversions automatically.

use std::fmt;

// ──────────────────────────────────────────────
// ScriptVec2
// ──────────────────────────────────────────────

/// 2D vector exposed to Rhai scripts.
///
/// Wraps two `f32` components but is registered with the Rhai engine so that
/// script floats (`f64` by default) are converted on entry and exit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptVec2 {
    pub x: f32,
    pub y: f32,
}

impl ScriptVec2 {
    pub fn new(x: f32, y: f32) -> Self {
        ScriptVec2 { x, y }
    }

    pub fn zero() -> Self {
        ScriptVec2 { x: 0.0, y: 0.0 }
    }

    pub fn length(&mut self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&mut self) -> ScriptVec2 {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len > 1e-8 {
            ScriptVec2 {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            ScriptVec2::zero()
        }
    }

    pub fn dot(&mut self, other: ScriptVec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn distance(&mut self, other: ScriptVec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn to_string(&mut self) -> String {
        format!("Vec2({}, {})", self.x, self.y)
    }
}

// ──────────────────────────────────────────────
// ScriptVec3
// ──────────────────────────────────────────────

/// 3D vector exposed to Rhai scripts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ScriptVec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        ScriptVec3 { x, y, z }
    }

    pub fn zero() -> Self {
        ScriptVec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn length(&mut self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&mut self) -> ScriptVec3 {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 1e-8 {
            ScriptVec3 {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            ScriptVec3::zero()
        }
    }

    pub fn dot(&mut self, other: ScriptVec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn to_string(&mut self) -> String {
        format!("Vec3({}, {}, {})", self.x, self.y, self.z)
    }
}

// ──────────────────────────────────────────────
// ScriptColor
// ──────────────────────────────────────────────

/// RGBA colour exposed to Rhai scripts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ScriptColor {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        ScriptColor { r, g, b, a }
    }

    pub fn to_string(&mut self) -> String {
        format!("Color({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

// ──────────────────────────────────────────────
// ScriptEntity
// ──────────────────────────────────────────────

/// Lightweight entity handle exposed to Rhai scripts.
///
/// Mirrors [`crate::entity::Entity`] but is a plain data struct so Rhai can
/// construct and inspect it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptEntity {
    pub id: u32,
    pub generation: u32,
}

impl ScriptEntity {
    pub fn new(id: u32, generation: u32) -> Self {
        ScriptEntity { id, generation }
    }

    pub fn to_string(&mut self) -> String {
        format!("Entity(id={}, gen={})", self.id, self.generation)
    }

    /// Returns `true` when the id is not `u32::MAX` (the sentinel used by
    /// [`crate::world::World::entity_from_index`] for dead / missing entities).
    pub fn is_valid(&mut self) -> bool {
        self.id != u32::MAX
    }
}

// ──────────────────────────────────────────────
// ScriptError
// ──────────────────────────────────────────────

/// Errors that can arise when compiling or executing Rhai scripts.
#[derive(Debug, Clone)]
pub enum ScriptError {
    CompilationError(String),
    RuntimeError(String),
    TypeError(String),
    FunctionNotFound(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::CompilationError(msg) => {
                write!(f, "Script compilation error: {}", msg)
            }
            ScriptError::RuntimeError(msg) => {
                write!(f, "Script runtime error: {}", msg)
            }
            ScriptError::TypeError(msg) => {
                write!(f, "Script type error: {}", msg)
            }
            ScriptError::FunctionNotFound(msg) => {
                write!(f, "Script function not found: {}", msg)
            }
        }
    }
}

impl std::error::Error for ScriptError {}

impl From<Box<rhai::EvalAltResult>> for ScriptError {
    fn from(err: Box<rhai::EvalAltResult>) -> Self {
        match *err {
            rhai::EvalAltResult::ErrorParsing(..) => {
                ScriptError::CompilationError(err.to_string())
            }
            rhai::EvalAltResult::ErrorFunctionNotFound(..) => {
                ScriptError::FunctionNotFound(err.to_string())
            }
            rhai::EvalAltResult::ErrorMismatchDataType(..) => {
                ScriptError::TypeError(err.to_string())
            }
            _ => ScriptError::RuntimeError(err.to_string()),
        }
    }
}

// ──────────────────────────────────────────────
// ScriptEngine
// ──────────────────────────────────────────────

/// The central scripting engine wrapper.
///
/// Owns a Rhai [`rhai::Engine`] with all Chronos-specific types and functions
/// pre-registered.  Create with [`ScriptEngine::new`] and use
/// [`ScriptEngine::eval`] / [`ScriptEngine::compile`] / [`ScriptEngine::call_fn`]
/// to execute scripts.
pub struct ScriptEngine {
    engine: rhai::Engine,
}

impl ScriptEngine {
    /// Create a new engine with the Chronos standard library registered.
    pub fn new() -> Self {
        let mut se = ScriptEngine {
            engine: rhai::Engine::new(),
        };
        se.register_stdlib();
        se
    }

    /// Evaluate a script fragment and return the result.
    pub fn eval<T: Clone + Send + Sync + 'static>(
        &mut self,
        script: &str,
    ) -> Result<T, ScriptError> {
        self.engine.eval::<T>(script).map_err(ScriptError::from)
    }

    /// Evaluate a script fragment inside a mutable [`rhai::Scope`].
    pub fn eval_scope<T: Clone + Send + Sync + 'static>(
        &mut self,
        scope: &mut rhai::Scope,
        script: &str,
    ) -> Result<T, ScriptError> {
        self.engine
            .eval_with_scope::<T>(scope, script)
            .map_err(ScriptError::from)
    }

    /// Compile a script to an AST without executing it.
    pub fn compile(&mut self, script: &str) -> Result<rhai::AST, ScriptError> {
        self.engine
            .compile(script)
            .map_err(|e| ScriptError::CompilationError(e.to_string()))
    }

    /// Call a named function stored in a pre-compiled AST.
    pub fn call_fn<T: Clone + Send + Sync + 'static>(
        &mut self,
        scope: &mut rhai::Scope,
        ast: &rhai::AST,
        name: &str,
        args: Vec<rhai::Dynamic>,
    ) -> Result<T, ScriptError> {
        self.engine
            .call_fn::<T>(scope, ast, name, args)
            .map_err(ScriptError::from)
    }

    /// Register all built-in Chronos types and functions with the Rhai engine.
    pub fn register_stdlib(&mut self) {
        self.register_vec2();
        self.register_vec3();
        self.register_color();
        self.register_entity();
        self.register_math_helpers();
    }

    /// Returns a shared reference to the underlying Rhai engine.
    pub fn raw_engine(&self) -> &rhai::Engine {
        &self.engine
    }

    /// Returns a mutable reference to the underlying Rhai engine.
    pub fn raw_engine_mut(&mut self) -> &mut rhai::Engine {
        &mut self.engine
    }

    // ── private helpers ────────────────────────

    fn register_vec2(&mut self) {
        self.engine
            .register_type_with_name::<ScriptVec2>("Vec2")
            .register_fn("Vec2", |x: f64, y: f64| {
                ScriptVec2::new(x as f32, y as f32)
            })
            .register_fn("length", |v: &mut ScriptVec2| -> f64 {
                v.length() as f64
            })
            .register_fn("normalize", |v: &mut ScriptVec2| -> ScriptVec2 {
                v.normalize()
            })
            .register_fn("dot", |v: &mut ScriptVec2, o: ScriptVec2| -> f64 {
                v.dot(o) as f64
            })
            .register_fn("distance", |v: &mut ScriptVec2, o: ScriptVec2| -> f64 {
                v.distance(o) as f64
            })
            .register_fn("to_string", |v: &mut ScriptVec2| v.to_string())
            .register_fn(
                "+",
                |a: &mut ScriptVec2, b: ScriptVec2| -> ScriptVec2 {
                    ScriptVec2::new(a.x + b.x, a.y + b.y)
                },
            )
            .register_fn(
                "-",
                |a: &mut ScriptVec2, b: ScriptVec2| -> ScriptVec2 {
                    ScriptVec2::new(a.x - b.x, a.y - b.y)
                },
            )
            .register_get("x", |v: &mut ScriptVec2| -> f64 { v.x as f64 })
            .register_set("x", |v: &mut ScriptVec2, val: f64| v.x = val as f32)
            .register_get("y", |v: &mut ScriptVec2| -> f64 { v.y as f64 })
            .register_set("y", |v: &mut ScriptVec2, val: f64| v.y = val as f32);
    }

    fn register_vec3(&mut self) {
        self.engine
            .register_type_with_name::<ScriptVec3>("Vec3")
            .register_fn("Vec3", |x: f64, y: f64, z: f64| {
                ScriptVec3::new(x as f32, y as f32, z as f32)
            })
            .register_fn("length", |v: &mut ScriptVec3| -> f64 {
                v.length() as f64
            })
            .register_fn("normalize", |v: &mut ScriptVec3| -> ScriptVec3 {
                v.normalize()
            })
            .register_fn("dot", |v: &mut ScriptVec3, o: ScriptVec3| -> f64 {
                v.dot(o) as f64
            })
            .register_fn("to_string", |v: &mut ScriptVec3| v.to_string())
            .register_get("x", |v: &mut ScriptVec3| -> f64 { v.x as f64 })
            .register_set("x", |v: &mut ScriptVec3, val: f64| v.x = val as f32)
            .register_get("y", |v: &mut ScriptVec3| -> f64 { v.y as f64 })
            .register_set("y", |v: &mut ScriptVec3, val: f64| v.y = val as f32)
            .register_get("z", |v: &mut ScriptVec3| -> f64 { v.z as f64 })
            .register_set("z", |v: &mut ScriptVec3, val: f64| v.z = val as f32);
    }

    fn register_color(&mut self) {
        self.engine
            .register_type_with_name::<ScriptColor>("Color")
            .register_fn("Color", |r: f64, g: f64, b: f64, a: f64| {
                ScriptColor::new(r as f32, g as f32, b as f32, a as f32)
            })
            .register_fn("to_string", |v: &mut ScriptColor| v.to_string())
            .register_get("r", |v: &mut ScriptColor| -> f64 { v.r as f64 })
            .register_set("r", |v: &mut ScriptColor, val: f64| v.r = val as f32)
            .register_get("g", |v: &mut ScriptColor| -> f64 { v.g as f64 })
            .register_set("g", |v: &mut ScriptColor, val: f64| v.g = val as f32)
            .register_get("b", |v: &mut ScriptColor| -> f64 { v.b as f64 })
            .register_set("b", |v: &mut ScriptColor, val: f64| v.b = val as f32)
            .register_get("a", |v: &mut ScriptColor| -> f64 { v.a as f64 })
            .register_set("a", |v: &mut ScriptColor, val: f64| v.a = val as f32);
    }

    fn register_entity(&mut self) {
        self.engine
            .register_type_with_name::<ScriptEntity>("Entity")
            .register_fn("Entity", |id: i64, gen: i64| {
                ScriptEntity::new(id as u32, gen as u32)
            })
            .register_fn("is_valid", |v: &mut ScriptEntity| -> bool { v.is_valid() })
            .register_fn("to_string", |v: &mut ScriptEntity| v.to_string())
            .register_get("id", |v: &mut ScriptEntity| -> i64 { v.id as i64 })
            .register_get(
                "generation",
                |v: &mut ScriptEntity| -> i64 { v.generation as i64 },
            );
    }

    fn register_math_helpers(&mut self) {
        self.engine
            .register_fn("sqrt_f", |v: f64| -> f64 { v.sqrt() })
            .register_fn("abs_f", |v: f64| -> f64 { v.abs() })
            .register_fn("min_f", |a: f64, b: f64| -> f64 { a.min(b) })
            .register_fn("max_f", |a: f64, b: f64| -> f64 { a.max(b) })
            .register_fn("clamp_f", |v: f64, lo: f64, hi: f64| -> f64 {
                v.clamp(lo, hi)
            })
            .register_fn("lerp", |a: f64, b: f64, t: f64| -> f64 {
                a + (b - a) * t
            })
            .register_fn("deg_to_rad", |deg: f64| -> f64 {
                deg * std::f64::consts::PI / 180.0
            })
            .register_fn("rad_to_deg", |rad: f64| -> f64 {
                rad * 180.0 / std::f64::consts::PI
            });
    }
}

impl Default for ScriptEngine {
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

    // ── ScriptVec2 ─────────────────────────────

    #[test]
    fn test_script_vec2_creation_and_fields() {
        let v = ScriptVec2::new(3.0, 4.0);
        assert_eq!(v.x, 3.0);
        assert_eq!(v.y, 4.0);
        assert_eq!(v, ScriptVec2::new(3.0, 4.0));
    }

    #[test]
    fn test_script_vec2_length_and_normalize() {
        let mut v = ScriptVec2::new(3.0, 4.0);
        let len = v.length();
        assert!((len - 5.0).abs() < 1e-6, "expected 5.0, got {}", len);

        let mut n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);

        let mut z = ScriptVec2::zero();
        assert_eq!(z.normalize(), ScriptVec2::zero());
    }

    #[test]
    fn test_script_vec2_dot_and_distance() {
        let mut a = ScriptVec2::new(1.0, 0.0);
        let b = ScriptVec2::new(0.0, 1.0);
        assert!(a.dot(b).abs() < 1e-6, "perpendicular dot should be ~0");
        assert!((a.dot(ScriptVec2::new(1.0, 0.0)) - 1.0).abs() < 1e-6);

        let mut origin = ScriptVec2::new(0.0, 0.0);
        let p = ScriptVec2::new(3.0, 4.0);
        assert!((origin.distance(p) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_script_vec2_to_string() {
        let mut v = ScriptVec2::new(1.5, 2.5);
        assert_eq!(v.to_string(), "Vec2(1.5, 2.5)");
    }

    // ── ScriptVec3 ─────────────────────────────

    #[test]
    fn test_script_vec3_creation_and_methods() {
        let mut v = ScriptVec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);

        let len = v.length();
        assert!((len - 14.0_f32.sqrt()).abs() < 1e-5);

        let mut n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);

        let d = v.dot(ScriptVec3::new(1.0, 0.0, 0.0));
        assert!((d - 1.0).abs() < 1e-6);
    }

    // ── ScriptColor ────────────────────────────

    #[test]
    fn test_script_color_creation() {
        let mut c = ScriptColor::new(0.5, 0.25, 0.75, 1.0);
        assert!((c.r - 0.5).abs() < 1e-6);
        assert!((c.g - 0.25).abs() < 1e-6);
        assert!((c.b - 0.75).abs() < 1e-6);
        assert!((c.a - 1.0).abs() < 1e-6);
        assert_eq!(c.to_string(), "Color(0.5, 0.25, 0.75, 1)");
    }

    // ── ScriptEntity ───────────────────────────

    #[test]
    fn test_script_entity_creation_and_validity() {
        let mut e = ScriptEntity::new(0, 0);
        assert!(e.is_valid());
        assert_eq!(e.to_string(), "Entity(id=0, gen=0)");

        let mut invalid = ScriptEntity::new(u32::MAX, 0);
        assert!(!invalid.is_valid());
    }

    // ── ScriptEngine ───────────────────────────

    #[test]
    fn test_script_engine_creation() {
        let engine = ScriptEngine::new();
        let _ = engine.raw_engine();
    }

    #[test]
    fn test_script_engine_eval_simple_expression() {
        let mut engine = ScriptEngine::new();
        let result: i64 = engine.eval("1 + 2").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_script_engine_eval_custom_types() {
        let mut engine = ScriptEngine::new();

        let result: f64 = engine
            .eval(
                "let a = Vec2(3.0, 4.0); \
                 let b = Vec2(1.0, 2.0); \
                 let c = a + b; \
                 c.length()",
            )
            .unwrap();

        // c = Vec2(4.0, 6.0), length = sqrt(16 + 36) = sqrt(52)
        let expected = (16.0_f64 + 36.0).sqrt();
        assert!((result - expected).abs() < 1e-4);
    }

    #[test]
    fn test_script_engine_compile_to_ast() {
        let mut engine = ScriptEngine::new();
        let ast = engine.compile("fn get_answer() { 42 }").unwrap();
        let mut scope = rhai::Scope::new();
        let result: i64 = engine
            .call_fn(&mut scope, &ast, "get_answer", vec![])
            .unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_script_engine_compile_error() {
        let mut engine = ScriptEngine::new();
        let result = engine.compile("let x = ;");
        assert!(result.is_err());
        match result.unwrap_err() {
            ScriptError::CompilationError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("expected CompilationError, got {:?}", other),
        }
    }

    // ── ScriptError ────────────────────────────

    #[test]
    fn test_script_error_display() {
        let err = ScriptError::CompilationError("unexpected token".into());
        assert_eq!(
            format!("{}", err),
            "Script compilation error: unexpected token"
        );

        let err = ScriptError::RuntimeError("division by zero".into());
        assert_eq!(
            format!("{}", err),
            "Script runtime error: division by zero"
        );

        let err = ScriptError::TypeError("expected int".into());
        assert_eq!(format!("{}", err), "Script type error: expected int");

        let err = ScriptError::FunctionNotFound("foo".into());
        assert_eq!(format!("{}", err), "Script function not found: foo");
    }

    #[test]
    fn test_script_engine_eval_vec3_script() {
        let mut engine = ScriptEngine::new();
        let result: f64 = engine
            .eval(
                "let v = Vec3(1.0, 0.0, 0.0); \
                 let len = v.length(); \
                 let n = v.normalize(); \
                 n.dot(n)",
            )
            .unwrap();
        assert!((result - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_script_engine_eval_entity_script() {
        let mut engine = ScriptEngine::new();
        let result: bool = engine
            .eval("let e = Entity(5, 1); e.is_valid()")
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_script_engine_eval_scope() {
        let mut engine = ScriptEngine::new();
        let mut scope = rhai::Scope::new();
        let _: () = engine
            .eval_scope(&mut scope, "let c = Color(1.0, 0.5, 0.0, 1.0);")
            .unwrap();
        assert!(scope.contains("c"));
    }

    #[test]
    fn test_script_engine_math_helpers() {
        let mut engine = ScriptEngine::new();
        let result: f64 = engine.eval("sqrt_f(9.0)").unwrap();
        assert!((result - 3.0).abs() < 1e-4);

        let result: f64 = engine.eval("lerp(0.0, 10.0, 0.5)").unwrap();
        assert!((result - 5.0).abs() < 1e-4);

        let result: f64 = engine.eval("clamp_f(15.0, 0.0, 10.0)").unwrap();
        assert!((result - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_script_engine_vec2_arithmetic_via_script() {
        let mut engine = ScriptEngine::new();
        let result: f64 = engine
            .eval(
                "let a = Vec2(5.0, 3.0); \
                 let b = Vec2(2.0, 1.0); \
                 let diff = a - b; \
                 diff.x + diff.y",
            )
            .unwrap();
        // diff = Vec2(3.0, 2.0), diff.x + diff.y = 3.0 + 2.0 = 5.0
        assert!((result - 5.0).abs() < 1e-4);
    }
}

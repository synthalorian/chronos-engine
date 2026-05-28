//! Full script API — engine functions exposed to Rhai scripts.
//!
//! Provides math, entity, transform, input, audio, physics, debug,
//! and time APIs that scripts can call directly. Each API category
//! registers plain Rhai functions against a `rhai::Engine`.
//!
//! State (time, input, entity IDs) is kept in thread-local cells so
//! the registered closures remain `Send + Sync + 'static`. The host
//! engine updates this state each frame via the public helper
//! functions (`set_delta_time`, `press_key`, etc.).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

// ──────────────────────────────────────────────
// Thread-local API state
// ──────────────────────────────────────────────

thread_local! {
    static DELTA_TIME: Cell<f64> = Cell::new(0.016);
    static FRAME_COUNT: Cell<i64> = Cell::new(0);
    static TIME_SINCE_START: Cell<f64> = Cell::new(0.0);
    static ENTITY_COUNTER: Cell<i64> = Cell::new(0);
    static ENTITY_EXISTS: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    static INPUT_KEYS: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    static MOUSE_X: Cell<f64> = Cell::new(0.0);
    static MOUSE_Y: Cell<f64> = Cell::new(0.0);
}

// ──────────────────────────────────────────────
// ScriptApi
// ──────────────────────────────────────────────

/// Registers all engine API functions with a Rhai `Engine`.
///
/// Construct with [`ScriptApi::new()`], then call
/// [`register_all`](ScriptApi::register_all) to attach every API
/// category, or call individual `register_*` methods for finer
/// control.
pub struct ScriptApi;

impl ScriptApi {
    /// Create a new API registrar.
    pub fn new() -> Self {
        ScriptApi
    }

    /// Register **all** API categories at once.
    pub fn register_all(&self, engine: &mut rhai::Engine) {
        Self::register_math(engine);
        Self::register_entity(engine);
        Self::register_transform(engine);
        Self::register_input(engine);
        Self::register_audio(engine);
        Self::register_physics(engine);
        Self::register_debug(engine);
        Self::register_time(engine);
    }

    // ──────────────────────────────────────────
    // Math
    // ──────────────────────────────────────────

    /// Register math helpers: `abs`, `min`, `max`, `sqrt`, `sin`, `cos`,
    /// `clamp`, `lerp`, `move_toward`, `smoothstep`, `floor`, `ceil`, `round`.
    pub fn register_math(engine: &mut rhai::Engine) {
        engine.register_fn("abs", |x: f64| x.abs());
        engine.register_fn("min", |a: f64, b: f64| a.min(b));
        engine.register_fn("max", |a: f64, b: f64| a.max(b));
        engine.register_fn("sqrt", |x: f64| x.sqrt());
        engine.register_fn("sin", |x: f64| x.sin());
        engine.register_fn("cos", |x: f64| x.cos());
        engine.register_fn("floor", |x: f64| x.floor());
        engine.register_fn("ceil", |x: f64| x.ceil());
        engine.register_fn("round", |x: f64| x.round());

        engine.register_fn("clamp", |value: f64, min: f64, max: f64| {
            if value < min {
                min
            } else if value > max {
                max
            } else {
                value
            }
        });

        engine.register_fn("lerp", |a: f64, b: f64, t: f64| a + (b - a) * t);

        engine.register_fn(
            "move_toward",
            |current: f64, target: f64, max_delta: f64| {
                if (target - current).abs() <= max_delta {
                    target
                } else {
                    current + max_delta * (target - current).signum()
                }
            },
        );

        engine.register_fn("smoothstep", |edge0: f64, edge1: f64, x: f64| {
            let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        });
    }

    // ──────────────────────────────────────────
    // Entity
    // ──────────────────────────────────────────

    /// Register entity management stubs: `create_entity`, `destroy_entity`,
    /// `entity_is_valid`.
    pub fn register_entity(engine: &mut rhai::Engine) {
        engine.register_fn("create_entity", || {
            ENTITY_COUNTER.with(|counter| {
                let id = counter.get() + 1;
                counter.set(id);
                ENTITY_EXISTS.with(|set| {
                    set.borrow_mut().insert(id);
                });
                id
            })
        });

        engine.register_fn("destroy_entity", |id: i64| {
            ENTITY_EXISTS.with(|set| {
                set.borrow_mut().remove(&id);
            });
        });

        engine.register_fn("entity_is_valid", |id: i64| {
            ENTITY_EXISTS.with(|set| set.borrow().contains(&id))
        });
    }

    // ──────────────────────────────────────────
    // Transform
    // ──────────────────────────────────────────

    /// Register transform stubs: `get_position_x/y`, `set_position`,
    /// `get_velocity_x/y`, `set_velocity`.
    pub fn register_transform(engine: &mut rhai::Engine) {
        engine.register_fn("get_position_x", || 0.0_f64);
        engine.register_fn("get_position_y", || 0.0_f64);
        engine.register_fn("set_position", |_x: f64, _y: f64| ());
        engine.register_fn("get_velocity_x", || 0.0_f64);
        engine.register_fn("get_velocity_y", || 0.0_f64);
        engine.register_fn("set_velocity", |_x: f64, _y: f64| ());
    }

    // ──────────────────────────────────────────
    // Input
    // ──────────────────────────────────────────

    /// Register input query stubs: `is_key_pressed`, `is_mouse_clicked`,
    /// `get_mouse_x`, `get_mouse_y`.
    pub fn register_input(engine: &mut rhai::Engine) {
        engine.register_fn("is_key_pressed", |key: i64| {
            INPUT_KEYS.with(|keys| keys.borrow().contains(&key))
        });

        engine.register_fn("is_mouse_clicked", || false);

        engine.register_fn("get_mouse_x", || MOUSE_X.with(|x| x.get()));

        engine.register_fn("get_mouse_y", || MOUSE_Y.with(|y| y.get()));
    }

    // ──────────────────────────────────────────
    // Audio
    // ──────────────────────────────────────────

    /// Register audio stubs: `play_sound`, `set_music`, `set_volume`.
    pub fn register_audio(engine: &mut rhai::Engine) {
        engine.register_fn("play_sound", |_name: &str| ());
        engine.register_fn("set_music", |_name: &str| ());
        engine.register_fn("set_volume", |_vol: f64| ());
    }

    // ──────────────────────────────────────────
    // Physics
    // ──────────────────────────────────────────

    /// Register physics stubs: `raycast_2d`, `apply_force`, `apply_impulse`.
    pub fn register_physics(engine: &mut rhai::Engine) {
        engine.register_fn(
            "raycast_2d",
            |_x: f64, _y: f64, _dx: f64, _dy: f64| false,
        );
        engine.register_fn("apply_force", |_id: i64, _fx: f64, _fy: f64| ());
        engine.register_fn("apply_impulse", |_id: i64, _ix: f64, _iy: f64| ());
    }

    // ──────────────────────────────────────────
    // Debug
    // ──────────────────────────────────────────

    /// Register debug / logging functions: `print`, `log_info`, `log_warn`,
    /// `log_error`, `debug_log`.
    pub fn register_debug(engine: &mut rhai::Engine) {
        engine.register_fn("debug_print", |msg: &str| {
            println!("{}", msg);
        });

        engine.register_fn("log_info", |msg: &str| {
            println!("[INFO] {}", msg);
        });

        engine.register_fn("log_warn", |msg: &str| {
            println!("[WARN] {}", msg);
        });

        engine.register_fn("log_error", |msg: &str| {
            println!("[ERROR] {}", msg);
        });

        engine.register_fn("debug_log", |level: i64, msg: &str| {
            let prefix = match level {
                0 => "DEBUG",
                1 => "INFO",
                2 => "WARN",
                _ => "ERROR",
            };
            println!("[{}] {}", prefix, msg);
        });
    }

    // ──────────────────────────────────────────
    // Time
    // ──────────────────────────────────────────

    /// Register time queries: `delta_time`, `frame_count`, `time_since_start`.
    pub fn register_time(engine: &mut rhai::Engine) {
        engine.register_fn("delta_time", || DELTA_TIME.with(|dt| dt.get()));

        engine.register_fn("frame_count", || FRAME_COUNT.with(|f| f.get()));

        engine.register_fn("time_since_start", || TIME_SINCE_START.with(|t| t.get()));
    }
}

// ──────────────────────────────────────────────
// Public helpers — host-side state updates
// ──────────────────────────────────────────────

/// Set the current frame's delta time (call each tick).
pub fn set_delta_time(dt: f64) {
    DELTA_TIME.with(|cell| cell.set(dt));
}

/// Set the current frame counter.
pub fn set_frame_count(count: i64) {
    FRAME_COUNT.with(|cell| cell.set(count));
}

/// Set the elapsed time since start.
pub fn set_time_since_start(time: f64) {
    TIME_SINCE_START.with(|cell| cell.set(time));
}

/// Simulate a key press (testing / replay).
pub fn press_key(key: i64) {
    INPUT_KEYS.with(|keys| {
        keys.borrow_mut().insert(key);
    });
}

/// Simulate a key release.
pub fn release_key(key: i64) {
    INPUT_KEYS.with(|keys| {
        keys.borrow_mut().remove(&key);
    });
}

/// Set the virtual mouse position.
pub fn set_mouse_position(x: f64, y: f64) {
    MOUSE_X.with(|cell| cell.set(x));
    MOUSE_Y.with(|cell| cell.set(y));
}

/// Reset **all** thread-local API state to defaults.
pub fn reset_api_state() {
    DELTA_TIME.with(|c| c.set(0.016));
    FRAME_COUNT.with(|c| c.set(0));
    TIME_SINCE_START.with(|c| c.set(0.0));
    ENTITY_COUNTER.with(|c| c.set(0));
    ENTITY_EXISTS.with(|s| s.borrow_mut().clear());
    INPUT_KEYS.with(|k| k.borrow_mut().clear());
    MOUSE_X.with(|c| c.set(0.0));
    MOUSE_Y.with(|c| c.set(0.0));
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an engine with all APIs registered and state reset.
    fn setup_engine() -> rhai::Engine {
        reset_api_state();
        let mut engine = rhai::Engine::new();
        let api = ScriptApi::new();
        api.register_all(&mut engine);
        engine
    }

    // ── Construction ──────────────────────────

    #[test]
    fn test_script_api_new() {
        let _api = ScriptApi::new();
    }

    #[test]
    fn test_register_all_no_panic() {
        let mut engine = rhai::Engine::new();
        ScriptApi::new().register_all(&mut engine);
    }

    // ── Math: basics ──────────────────────────

    #[test]
    fn test_math_abs() {
        let engine = setup_engine();
        let result: f64 = engine.eval("abs(-5.0)").unwrap();
        assert!((result - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_math_sqrt_sin_cos() {
        let engine = setup_engine();

        let sq: f64 = engine.eval("sqrt(16.0)").unwrap();
        assert!((sq - 4.0).abs() < 1e-9);

        let sn: f64 = engine.eval("sin(0.0)").unwrap();
        assert!(sn.abs() < 1e-9);

        let cs: f64 = engine.eval("cos(0.0)").unwrap();
        assert!((cs - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_math_floor_ceil_round() {
        let engine = setup_engine();

        let f: f64 = engine.eval("floor(3.7)").unwrap();
        assert!((f - 3.0).abs() < 1e-9);

        let c: f64 = engine.eval("ceil(3.2)").unwrap();
        assert!((c - 4.0).abs() < 1e-9);

        let r: f64 = engine.eval("round(3.5)").unwrap();
        assert!((r - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_math_min_max() {
        let engine = setup_engine();

        let mn: f64 = engine.eval("min(3.0, 7.0)").unwrap();
        assert!((mn - 3.0).abs() < 1e-9);

        let mx: f64 = engine.eval("max(3.0, 7.0)").unwrap();
        assert!((mx - 7.0).abs() < 1e-9);
    }

    // ── Math: clamp ───────────────────────────

    #[test]
    fn test_math_clamp_boundaries() {
        let engine = setup_engine();

        let in_range: f64 = engine.eval("clamp(5.0, 0.0, 10.0)").unwrap();
        assert!((in_range - 5.0).abs() < 1e-9);

        let below: f64 = engine.eval("clamp(-1.0, 0.0, 10.0)").unwrap();
        assert!((below - 0.0).abs() < 1e-9);

        let above: f64 = engine.eval("clamp(15.0, 0.0, 10.0)").unwrap();
        assert!((above - 10.0).abs() < 1e-9);
    }

    // ── Math: lerp ────────────────────────────

    #[test]
    fn test_math_lerp_boundaries() {
        let engine = setup_engine();

        let at_zero: f64 = engine.eval("lerp(0.0, 10.0, 0.0)").unwrap();
        assert!((at_zero - 0.0).abs() < 1e-9);

        let at_one: f64 = engine.eval("lerp(0.0, 10.0, 1.0)").unwrap();
        assert!((at_one - 10.0).abs() < 1e-9);

        let at_half: f64 = engine.eval("lerp(0.0, 10.0, 0.5)").unwrap();
        assert!((at_half - 5.0).abs() < 1e-9);
    }

    // ── Math: move_toward ─────────────────────

    #[test]
    fn test_math_move_toward() {
        let engine = setup_engine();

        let partial: f64 = engine.eval("move_toward(0.0, 10.0, 3.0)").unwrap();
        assert!((partial - 3.0).abs() < 1e-9);

        // max_delta large enough to reach target
        let reached: f64 = engine.eval("move_toward(0.0, 10.0, 20.0)").unwrap();
        assert!((reached - 10.0).abs() < 1e-9);

        // negative direction
        let neg: f64 = engine.eval("move_toward(10.0, 0.0, 3.0)").unwrap();
        assert!((neg - 7.0).abs() < 1e-9);
    }

    // ── Math: smoothstep ──────────────────────

    #[test]
    fn test_math_smoothstep() {
        let engine = setup_engine();

        let below: f64 = engine.eval("smoothstep(0.0, 1.0, -0.5)").unwrap();
        assert!((below - 0.0).abs() < 1e-9);

        let above: f64 = engine.eval("smoothstep(0.0, 1.0, 1.5)").unwrap();
        assert!((above - 1.0).abs() < 1e-9);

        let mid: f64 = engine.eval("smoothstep(0.0, 1.0, 0.5)").unwrap();
        assert!((mid - 0.5).abs() < 0.01);
    }

    // ── Entity ────────────────────────────────

    #[test]
    fn test_entity_create_returns_positive() {
        let engine = setup_engine();
        let id: i64 = engine.eval("create_entity()").unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_entity_is_valid_after_create() {
        let engine = setup_engine();
        let id: i64 = engine.eval("create_entity()").unwrap();

        let valid: bool = engine.eval(&format!("entity_is_valid({})", id)).unwrap();
        assert!(valid);

        let invalid: bool = engine.eval("entity_is_valid(9999)").unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_entity_destroy_removes_validity() {
        let engine = setup_engine();
        let id: i64 = engine.eval("create_entity()").unwrap();

        engine.run(&format!("destroy_entity({})", id)).unwrap();

        let valid: bool = engine.eval(&format!("entity_is_valid({})", id)).unwrap();
        assert!(!valid);
    }

    // ── Debug ─────────────────────────────────

    #[test]
    fn test_debug_print_no_panic() {
        let engine = setup_engine();
        engine.eval::<()>("debug_print(\"hello world\")").unwrap();
    }

    #[test]
    fn test_debug_log_functions() {
        let engine = setup_engine();
        engine.eval::<()>("log_info(\"info\")").unwrap();
        engine.eval::<()>("log_warn(\"warn\")").unwrap();
        engine.eval::<()>("log_error(\"error\")").unwrap();
        engine.eval::<()>("debug_log(0, \"debug\")").unwrap();
    }

    // ── Time ──────────────────────────────────

    #[test]
    fn test_time_delta_returns_value() {
        reset_api_state();
        set_delta_time(0.033);

        let mut engine = rhai::Engine::new();
        ScriptApi::register_time(&mut engine);

        let dt: f64 = engine.eval("delta_time()").unwrap();
        assert!((dt - 0.033).abs() < 1e-9);
    }

    #[test]
    fn test_time_frame_count() {
        reset_api_state();
        set_frame_count(42);

        let mut engine = rhai::Engine::new();
        ScriptApi::register_time(&mut engine);

        let count: i64 = engine.eval("frame_count()").unwrap();
        assert_eq!(count, 42);
    }

    // ── Input ─────────────────────────────────

    #[test]
    fn test_input_key_pressed() {
        reset_api_state();
        press_key(65); // 'A' key

        let mut engine = rhai::Engine::new();
        ScriptApi::register_input(&mut engine);

        let pressed: bool = engine.eval("is_key_pressed(65)").unwrap();
        assert!(pressed);

        let not_pressed: bool = engine.eval("is_key_pressed(66)").unwrap();
        assert!(!not_pressed);
    }

    #[test]
    fn test_input_mouse_position() {
        reset_api_state();
        set_mouse_position(100.0, 200.0);

        let mut engine = rhai::Engine::new();
        ScriptApi::register_input(&mut engine);

        let mx: f64 = engine.eval("get_mouse_x()").unwrap();
        let my: f64 = engine.eval("get_mouse_y()").unwrap();
        assert!((mx - 100.0).abs() < 1e-9);
        assert!((my - 200.0).abs() < 1e-9);
    }

    // ── Integration: script calls API ─────────

    #[test]
    fn test_script_uses_api_to_compute() {
        let engine = setup_engine();
        let ast = engine
            .compile(
                r#"
                    fn compute() {
                        let x = lerp(0.0, 100.0, 0.25);
                        let clamped = clamp(x, 10.0, 20.0);
                        clamped
                    }
                "#,
            )
            .unwrap();
        let mut scope = rhai::Scope::new();
        let result: f64 = engine.call_fn(&mut scope, &ast, "compute", ()).unwrap();
        // lerp(0, 100, 0.25) = 25, clamp(25, 10, 20) = 20
        assert!((result - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_entity_lifecycle_in_script() {
        let engine = setup_engine();
        let ast = engine
            .compile(
                r#"
                    fn setup() {
                        let e1 = create_entity();
                        let e2 = create_entity();
                        destroy_entity(e1);
                        e2
                    }
                "#,
            )
            .unwrap();
        let mut scope = rhai::Scope::new();
        let surviving: i64 = engine.call_fn(&mut scope, &ast, "setup", ()).unwrap();
        assert!(surviving > 0);

        // The destroyed entity should be invalid
        let valid: bool = engine
            .eval("entity_is_valid(1)")
            .unwrap();
        assert!(!valid);

        // The surviving entity should be valid
        let valid2: bool = engine
            .eval(&format!("entity_is_valid({})", surviving))
            .unwrap();
        assert!(valid2);
    }
}

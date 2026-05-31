//! Input system for the Chronos Engine.
//!
//! Provides keyboard, mouse, and gamepad input handling with a flexible
//! action-binding system. Maps raw hardware inputs to named game actions
//! (e.g., "jump" → Space) and tracks per-frame state transitions
//! (pressed, just_pressed, just_released).
//!
//! Zero external dependencies — core module.

use std::collections::HashMap;

// ──────────────────────────────────────────────
// Key Codes
// ──────────────────────────────────────────────

/// Keyboard key codes.
///
/// Covers standard US keyboard layout. Each variant maps to a physical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Number row
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,

    // Letter row 1
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    // Letter row 2
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,

    // Modifiers
    LShift,
    RShift,
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,
    LSuper,
    RSuper,

    // Editing
    Tab,
    Return,
    Escape,
    Backspace,
    Space,

    // Brackets and punctuation
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Period,
    Slash,

    // Lock keys
    CapsLock,
    NumLock,
    ScrollLock,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
    NumpadEquals,

    // Print/SysRq/Pause
    PrintScreen,
    Pause,
    Menu,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u8),
}

/// Gamepad buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    // Face buttons (Sony layout: Cross/Circle/Square/Triangle)
    South, // Cross / A
    East,  // Circle / B
    West,  // Square / X
    North, // Triangle / Y

    // Shoulder
    LeftShoulder,  // L1 / LB
    RightShoulder, // R1 / RB
    LeftTrigger,   // L2 / LT (digital)
    RightTrigger,  // R2 / RT (digital)

    // Sticks
    LeftStick,  // L3 (press)
    RightStick, // R3 (press)

    // D-pad
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    // Center
    Start, // Options / Menu
    Back,  // Share / View
    Home,  // PS / Xbox
}

// ──────────────────────────────────────────────
// Input Source
// ──────────────────────────────────────────────

/// A physical input source — keyboard key, mouse button, or gamepad button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSource {
    Key(KeyCode),
    Mouse(MouseButton),
    Gamepad(GamepadButton),
}

// ──────────────────────────────────────────────
// Bindings
// ──────────────────────────────────────────────

/// A single input binding: maps one or more sources to a named action.
///
/// An action like "jump" can be bound to multiple keys simultaneously
/// (Space, GamepadButton::South, etc.). Any source triggering is sufficient.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The sources that activate this binding.
    pub sources: Vec<InputSource>,
}

impl Binding {
    /// Create a binding from a single source.
    pub fn single(source: InputSource) -> Self {
        Binding {
            sources: vec![source],
        }
    }

    /// Create a binding from multiple sources.
    pub fn many(sources: Vec<InputSource>) -> Self {
        Binding { sources }
    }

    /// Chain another source onto this binding.
    pub fn or(mut self, source: InputSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Check if this binding is triggered by the given source.
    pub fn contains(&self, source: &InputSource) -> bool {
        self.sources.contains(source)
    }
}

/// A named action with its binding.
#[derive(Debug, Clone)]
pub struct ActionBinding {
    pub name: String,
    pub binding: Binding,
}

impl ActionBinding {
    pub fn new(name: &str, binding: Binding) -> Self {
        ActionBinding {
            name: name.to_string(),
            binding,
        }
    }
}

// ──────────────────────────────────────────────
// Input Map / Context
// ──────────────────────────────────────────────

/// A collection of action bindings that can be activated as a group.
///
/// For example, a "gameplay" context might have WASD + jump + shoot,
/// while a "menu" context has arrow keys + enter + escape.
/// Only the active context's bindings are processed.
#[derive(Debug, Clone)]
pub struct InputContext {
    /// Unique name for this context (e.g., "gameplay", "menu", "console").
    pub name: String,
    /// Action bindings in this context.
    pub actions: Vec<ActionBinding>,
    /// Reverse lookup: InputSource → action indices.
    source_map: HashMap<InputSource, Vec<usize>>,
}

impl InputContext {
    /// Create a new empty input context.
    pub fn new(name: &str) -> Self {
        InputContext {
            name: name.to_string(),
            actions: Vec::new(),
            source_map: HashMap::new(),
        }
    }

    /// Add an action binding to this context.
    pub fn bind(mut self, name: &str, binding: Binding) -> Self {
        let idx = self.actions.len();
        let action = ActionBinding::new(name, binding);
        for source in &action.binding.sources {
            self.source_map.entry(*source).or_default().push(idx);
        }
        self.actions.push(action);
        self
    }

    /// Look up which action indices are triggered by a source.
    pub fn actions_for_source(&self, source: &InputSource) -> &[usize] {
        self.source_map
            .get(source)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ──────────────────────────────────────────────
// Per-Action State
// ──────────────────────────────────────────────

/// The state of a single action in the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionState {
    /// Not currently pressed.
    #[default]
    Released,
    /// Was pressed in a previous frame and is still held.
    Held,
    /// Just pressed this frame (transition from Released).
    JustPressed,
    /// Just released this frame (transition from Held/JustPressed).
    JustReleased,
}

impl ActionState {
    pub fn is_pressed(&self) -> bool {
        matches!(self, ActionState::Held | ActionState::JustPressed)
    }

    pub fn is_just_pressed(&self) -> bool {
        matches!(self, ActionState::JustPressed)
    }

    pub fn is_just_released(&self) -> bool {
        matches!(self, ActionState::JustReleased)
    }

    pub fn is_released(&self) -> bool {
        matches!(self, ActionState::Released)
    }
}

// ──────────────────────────────────────────────
// Axis Values
// ──────────────────────────────────────────────

/// An axis binding: maps two sources to a positive/negative axis value.
///
/// For example, "move_x" might bind A (negative) and D (positive),
/// producing -1.0, 0.0, or 1.0 depending on key state.
#[derive(Debug, Clone)]
pub struct AxisBinding {
    pub name: String,
    pub positive: InputSource,
    pub negative: InputSource,
}

impl AxisBinding {
    pub fn new(name: &str, positive: InputSource, negative: InputSource) -> Self {
        AxisBinding {
            name: name.to_string(),
            positive,
            negative,
        }
    }
}

// ──────────────────────────────────────────────
// Raw Input Events
// ──────────────────────────────────────────────

/// A raw input event from the hardware.
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// A key was pressed.
    KeyPressed(KeyCode),
    /// A key was released.
    KeyReleased(KeyCode),
    /// A mouse button was pressed.
    MousePressed(MouseButton),
    /// A mouse button was released.
    MouseReleased(MouseButton),
    /// Mouse moved to (x, y) position.
    MouseMoved { x: f64, y: f64 },
    /// Mouse moved by (dx, dy) pixels.
    MouseDelta { dx: f64, dy: f64 },
    /// Mouse scroll wheel.
    MouseScroll { delta_y: f32, delta_x: f32 },
    /// Gamepad button pressed.
    GamepadPressed(GamepadButton),
    /// Gamepad button released.
    GamepadReleased(GamepadButton),
    /// Gamepad axis moved (e.g., thumbstick). Value in [-1.0, 1.0].
    GamepadAxis { axis: GamepadAxis, value: f32 },
}

/// Gamepad analog axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,  // 0.0 to 1.0
    RightTrigger, // 0.0 to 1.0
}

// ──────────────────────────────────────────────
// Input Manager
// ──────────────────────────────────────────────

/// The central input manager. Processes raw events → action states.
///
/// Usage:
/// 1. Register contexts with `add_context()`.
/// 2. Set the active context with `set_context()`.
/// 3. Each frame, feed raw events with `process_event()`.
/// 4. Call `end_frame()` after processing all events.
/// 5. Query states with `pressed()`, `just_pressed()`, etc.
#[derive(Debug)]
pub struct InputManager {
    /// All registered contexts, indexed by name.
    contexts: HashMap<String, InputContext>,
    /// Currently active context name.
    active_context: Option<String>,
    /// Per-action state for the current frame.
    action_states: HashMap<String, ActionState>,
    /// Raw source pressed state (true if currently down).
    source_pressed: HashMap<InputSource, bool>,
    /// Axis bindings from the active context.
    axis_bindings: Vec<AxisBinding>,
    /// Current axis values.
    axis_values: HashMap<String, f32>,
    /// Mouse position.
    mouse_position: (f64, f64),
    /// Mouse delta for this frame.
    mouse_delta: (f64, f64),
    /// Scroll delta for this frame.
    scroll_delta: (f32, f32),
    /// Raw gamepad axis values.
    gamepad_axes: HashMap<GamepadAxis, f32>,
}

impl InputManager {
    /// Create a new input manager with no contexts.
    pub fn new() -> Self {
        InputManager {
            contexts: HashMap::new(),
            active_context: None,
            action_states: HashMap::new(),
            source_pressed: HashMap::new(),
            axis_bindings: Vec::new(),
            axis_values: HashMap::new(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            gamepad_axes: HashMap::new(),
        }
    }

    /// Register an input context.
    pub fn add_context(&mut self, context: InputContext) {
        self.contexts.insert(context.name.clone(), context);
    }

    /// Switch to a different input context by name.
    ///
    /// Resets all action states when switching. The previous context's
    /// state is discarded.
    pub fn set_context(&mut self, name: &str) {
        if self.active_context.as_deref() == Some(name) {
            return;
        }
        self.active_context = Some(name.to_string());
        self.action_states.clear();
        self.axis_bindings.clear();
        self.axis_values.clear();

        // Rebuild axis bindings from the new context (axes are stored separately)
        // Axes can be added via add_axis() after set_context()
    }

    /// Get the name of the currently active context.
    pub fn active_context(&self) -> Option<&str> {
        self.active_context.as_deref()
    }

    /// Add an axis binding to the active context.
    pub fn add_axis(&mut self, axis: AxisBinding) {
        self.axis_values.insert(axis.name.clone(), 0.0);
        self.axis_bindings.push(axis);
    }

    /// Process a raw input event for the current frame.
    ///
    /// Call this for every event from the windowing system before
    /// calling `end_frame()`.
    pub fn process_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed(key) => {
                let source = InputSource::Key(*key);
                self.source_pressed.insert(source, true);
                self.update_action_state(&source, true);
            }
            InputEvent::KeyReleased(key) => {
                let source = InputSource::Key(*key);
                self.source_pressed.insert(source, false);
                self.update_action_state(&source, false);
            }
            InputEvent::MousePressed(button) => {
                let source = InputSource::Mouse(*button);
                self.source_pressed.insert(source, true);
                self.update_action_state(&source, true);
            }
            InputEvent::MouseReleased(button) => {
                let source = InputSource::Mouse(*button);
                self.source_pressed.insert(source, false);
                self.update_action_state(&source, false);
            }
            InputEvent::MouseMoved { x, y } => {
                let old = self.mouse_position;
                self.mouse_position = (*x, *y);
                self.mouse_delta.0 += x - old.0;
                self.mouse_delta.1 += y - old.1;
            }
            InputEvent::MouseDelta { dx, dy } => {
                self.mouse_delta.0 += dx;
                self.mouse_delta.1 += dy;
            }
            InputEvent::MouseScroll { delta_y, delta_x } => {
                self.scroll_delta.0 += delta_x;
                self.scroll_delta.1 += delta_y;
            }
            InputEvent::GamepadPressed(button) => {
                let source = InputSource::Gamepad(*button);
                self.source_pressed.insert(source, true);
                self.update_action_state(&source, true);
            }
            InputEvent::GamepadReleased(button) => {
                let source = InputSource::Gamepad(*button);
                self.source_pressed.insert(source, false);
                self.update_action_state(&source, false);
            }
            InputEvent::GamepadAxis { axis, value } => {
                self.gamepad_axes.insert(*axis, *value);
            }
        }
    }

    /// Finalize the frame: transition JustPressed → Held, JustReleased → Released.
    ///
    /// Call this after all events have been processed for the frame.
    /// Also recomputes axis values.
    pub fn end_frame(&mut self) {
        // Transition states
        let action_names: Vec<String> = self.action_states.keys().cloned().collect();
        for name in &action_names {
            if let Some(state) = self.action_states.get_mut(name) {
                match state {
                    ActionState::JustPressed => *state = ActionState::Held,
                    ActionState::JustReleased => *state = ActionState::Released,
                    _ => {}
                }
            }
        }

        // Recompute axes
        for axis in &self.axis_bindings {
            let pos = self
                .source_pressed
                .get(&axis.positive)
                .copied()
                .unwrap_or(false);
            let neg = self
                .source_pressed
                .get(&axis.negative)
                .copied()
                .unwrap_or(false);
            let mut value = 0.0;
            if pos {
                value += 1.0;
            }
            if neg {
                value -= 1.0;
            }
            self.axis_values.insert(axis.name.clone(), value);
        }

        // Reset per-frame deltas
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = (0.0, 0.0);
    }

    /// Check if an action is currently pressed (held or just pressed).
    pub fn pressed(&self, action: &str) -> bool {
        self.action_states
            .get(action)
            .map(|s| s.is_pressed())
            .unwrap_or(false)
    }

    /// Check if an action was just pressed this frame.
    pub fn just_pressed(&self, action: &str) -> bool {
        self.action_states
            .get(action)
            .map(|s| s.is_just_pressed())
            .unwrap_or(false)
    }

    /// Check if an action was just released this frame.
    pub fn just_released(&self, action: &str) -> bool {
        self.action_states
            .get(action)
            .map(|s| s.is_just_released())
            .unwrap_or(false)
    }

    /// Get the full state of an action.
    pub fn action_state(&self, action: &str) -> ActionState {
        self.action_states
            .get(action)
            .copied()
            .unwrap_or(ActionState::Released)
    }

    /// Get the current value of an axis (e.g., "move_x").
    ///
    /// Returns 0.0 if the axis doesn't exist.
    pub fn axis(&self, name: &str) -> f32 {
        self.axis_values.get(name).copied().unwrap_or(0.0)
    }

    /// Get the current mouse position.
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    /// Get the mouse delta for this frame.
    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    /// Get the scroll delta for this frame.
    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }

    /// Get a raw gamepad axis value.
    pub fn gamepad_axis(&self, axis: GamepadAxis) -> f32 {
        self.gamepad_axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// Check if a raw input source is currently pressed.
    pub fn is_source_pressed(&self, source: &InputSource) -> bool {
        self.source_pressed.get(source).copied().unwrap_or(false)
    }

    /// Get all action names that are currently active (pressed or just pressed).
    pub fn active_actions(&self) -> Vec<&str> {
        self.action_states
            .iter()
            .filter(|(_, state)| state.is_pressed())
            .map(|(name, _)| name.as_str())
            .collect()
    }

    // ──────────────────────────────────────────
    // Internal
    // ──────────────────────────────────────────

    /// Update action state based on a source press/release.
    fn update_action_state(&mut self, source: &InputSource, pressed: bool) {
        let ctx_name = match &self.active_context {
            Some(name) => name.clone(),
            None => return,
        };

        let ctx = match self.contexts.get(&ctx_name) {
            Some(c) => c,
            None => return,
        };

        let action_indices = ctx.actions_for_source(source);
        for &idx in action_indices {
            let action_name = &ctx.actions[idx].name;
            let current = self
                .action_states
                .get(action_name.as_str())
                .copied()
                .unwrap_or(ActionState::Released);

            let new_state = match (pressed, current) {
                (true, ActionState::Released) => ActionState::JustPressed,
                (true, ActionState::JustReleased) => ActionState::JustPressed,
                (true, ActionState::JustPressed) => ActionState::JustPressed,
                (true, ActionState::Held) => ActionState::Held,
                (false, ActionState::JustPressed) => ActionState::JustReleased,
                (false, ActionState::Held) => ActionState::JustReleased,
                (false, ActionState::Released) => ActionState::Released,
                (false, ActionState::JustReleased) => ActionState::Released,
            };

            self.action_states.insert(action_name.clone(), new_state);
        }
    }
}

impl Default for InputManager {
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

    #[test]
    fn basic_key_binding() {
        let ctx = InputContext::new("gameplay")
            .bind("jump", Binding::single(InputSource::Key(KeyCode::Space)));

        let mut mgr = InputManager::new();
        mgr.add_context(ctx);
        mgr.set_context("gameplay");

        // Press space
        mgr.process_event(&InputEvent::KeyPressed(KeyCode::Space));
        assert!(mgr.just_pressed("jump"));
        assert!(mgr.pressed("jump"));

        mgr.end_frame();
        assert!(!mgr.just_pressed("jump"));
        assert!(mgr.pressed("jump"));

        // Release space
        mgr.process_event(&InputEvent::KeyReleased(KeyCode::Space));
        assert!(mgr.just_released("jump"));

        mgr.end_frame();
        assert!(!mgr.pressed("jump"));
        assert!(!mgr.just_released("jump"));
    }

    #[test]
    fn multi_source_binding() {
        let ctx = InputContext::new("gameplay").bind(
            "jump",
            Binding::many(vec![
                InputSource::Key(KeyCode::Space),
                InputSource::Gamepad(GamepadButton::South),
            ]),
        );

        let mut mgr = InputManager::new();
        mgr.add_context(ctx);
        mgr.set_context("gameplay");

        mgr.process_event(&InputEvent::GamepadPressed(GamepadButton::South));
        assert!(mgr.just_pressed("jump"));

        mgr.end_frame();
        mgr.process_event(&InputEvent::GamepadReleased(GamepadButton::South));
        assert!(mgr.just_released("jump"));
    }

    #[test]
    fn axis_binding() {
        let ctx = InputContext::new("gameplay");

        let mut mgr = InputManager::new();
        mgr.add_context(ctx);
        mgr.set_context("gameplay");
        mgr.add_axis(AxisBinding::new(
            "move_x",
            InputSource::Key(KeyCode::D),
            InputSource::Key(KeyCode::A),
        ));

        // Press D
        mgr.process_event(&InputEvent::KeyPressed(KeyCode::D));
        mgr.end_frame();
        assert!((mgr.axis("move_x") - 1.0).abs() < 0.001);

        // Press A too (cancel out)
        mgr.process_event(&InputEvent::KeyPressed(KeyCode::A));
        mgr.end_frame();
        assert!((mgr.axis("move_x") - 0.0).abs() < 0.001);

        // Release D
        mgr.process_event(&InputEvent::KeyReleased(KeyCode::D));
        mgr.end_frame();
        assert!((mgr.axis("move_x") - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn context_switching() {
        let gameplay = InputContext::new("gameplay")
            .bind("shoot", Binding::single(InputSource::Key(KeyCode::Space)));

        let menu = InputContext::new("menu")
            .bind("confirm", Binding::single(InputSource::Key(KeyCode::Space)));

        let mut mgr = InputManager::new();
        mgr.add_context(gameplay);
        mgr.add_context(menu);

        // In gameplay, Space → shoot
        mgr.set_context("gameplay");
        mgr.process_event(&InputEvent::KeyPressed(KeyCode::Space));
        assert!(mgr.pressed("shoot"));
        assert!(!mgr.pressed("confirm"));

        // Switch to menu — states reset
        mgr.set_context("menu");
        mgr.process_event(&InputEvent::KeyReleased(KeyCode::Space));
        mgr.end_frame();
        assert!(!mgr.pressed("shoot"));
    }

    #[test]
    fn mouse_tracking() {
        let mut mgr = InputManager::new();
        mgr.process_event(&InputEvent::MouseMoved { x: 100.0, y: 200.0 });
        assert_eq!(mgr.mouse_position(), (100.0, 200.0));

        mgr.process_event(&InputEvent::MouseMoved { x: 150.0, y: 250.0 });
        assert_eq!(mgr.mouse_position(), (150.0, 250.0));
        // delta accumulates within the frame

        mgr.end_frame();
        // delta resets after end_frame
        assert_eq!(mgr.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn gamepad_axis() {
        let mut mgr = InputManager::new();
        mgr.process_event(&InputEvent::GamepadAxis {
            axis: GamepadAxis::LeftStickX,
            value: 0.75,
        });
        assert!((mgr.gamepad_axis(GamepadAxis::LeftStickX) - 0.75).abs() < 0.001);
    }

    #[test]
    fn binding_or_chain() {
        let binding = Binding::single(InputSource::Key(KeyCode::Space))
            .or(InputSource::Key(KeyCode::Up))
            .or(InputSource::Gamepad(GamepadButton::South));

        assert!(binding.contains(&InputSource::Key(KeyCode::Space)));
        assert!(binding.contains(&InputSource::Key(KeyCode::Up)));
        assert!(binding.contains(&InputSource::Gamepad(GamepadButton::South)));
        assert!(!binding.contains(&InputSource::Key(KeyCode::Escape)));
    }
}

#[cfg(feature = "game")]
use std::collections::HashMap;

// ── ScreenState ──

#[cfg(feature = "game")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenState {
    MainMenu,
    Loading,
    Gameplay,
    Paused,
    CharacterSheet,
    Inventory,
    JobBoard,
    Dialogue,
    Settings,
    GameOver,
}

#[cfg(feature = "game")]
impl ScreenState {
    pub fn name(&self) -> &str {
        match self {
            Self::MainMenu => "Main Menu",
            Self::Loading => "Loading",
            Self::Gameplay => "Gameplay",
            Self::Paused => "Paused",
            Self::CharacterSheet => "Character Sheet",
            Self::Inventory => "Inventory",
            Self::JobBoard => "Job Board",
            Self::Dialogue => "Dialogue",
            Self::Settings => "Settings",
            Self::GameOver => "Game Over",
        }
    }

    pub fn pauses_gameplay(&self) -> bool {
        matches!(
            self,
            Self::Paused
                | Self::CharacterSheet
                | Self::Inventory
                | Self::JobBoard
                | Self::Settings
                | Self::Dialogue
        )
    }

    pub fn shows_hud(&self) -> bool {
        matches!(self, Self::Gameplay | Self::Paused | Self::Dialogue)
    }
}

// ── ScreenTransition ──

#[cfg(feature = "game")]
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenTransition {
    Push(ScreenState),
    Pop,
    Replace(ScreenState),
    ClearAndPush(ScreenState),
}

// ── ScreenError ──

#[cfg(feature = "game")]
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenError {
    StackOverflow,
    InvalidTransition,
}

// ── ScreenHistory ──

#[cfg(feature = "game")]
pub struct ScreenHistory {
    pub stack: Vec<ScreenState>,
    pub max_depth: usize,
}

#[cfg(feature = "game")]
impl ScreenHistory {
    pub fn new(initial: ScreenState) -> Self {
        Self {
            stack: vec![initial],
            max_depth: 10,
        }
    }

    pub fn current(&self) -> ScreenState {
        self.stack
            .last()
            .copied()
            .expect("ScreenHistory stack should never be empty")
    }

    pub fn push(&mut self, state: ScreenState) -> Result<(), ScreenError> {
        if self.stack.len() >= self.max_depth {
            return Err(ScreenError::StackOverflow);
        }
        self.stack.push(state);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<ScreenState> {
        if self.stack.len() <= 1 {
            return None;
        }
        self.stack.pop()
    }

    pub fn replace(&mut self, state: ScreenState) -> ScreenState {
        let old = self
            .stack
            .pop()
            .expect("ScreenHistory stack should never be empty");
        self.stack.push(state);
        old
    }

    pub fn clear_and_push(&mut self, state: ScreenState) {
        self.stack.clear();
        self.stack.push(state);
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn is_state(&self, state: ScreenState) -> bool {
        self.current() == state
    }

    pub fn history(&self) -> &[ScreenState] {
        &self.stack
    }
}

// ── ButtonConfig ──

#[cfg(feature = "game")]
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonConfig {
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub enabled: bool,
    pub visible: bool,
    pub action_id: u32,
}

#[cfg(feature = "game")]
impl ButtonConfig {
    pub fn new(label: &str, x: f32, y: f32, w: f32, h: f32, action_id: u32) -> Self {
        Self {
            label: label.to_string(),
            x,
            y,
            width: w,
            height: h,
            enabled: true,
            visible: true,
            action_id,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x
            && px <= self.x + self.width
            && py >= self.y
            && py <= self.y + self.height
    }
}

// ── ScreenLayout ──

#[cfg(feature = "game")]
pub struct ScreenLayout {
    pub state: ScreenState,
    pub buttons: Vec<ButtonConfig>,
    pub title: String,
    pub bg_color: [f32; 4],
}

#[cfg(feature = "game")]
impl ScreenLayout {
    pub fn new(state: ScreenState, title: &str) -> Self {
        Self {
            state,
            buttons: Vec::new(),
            title: title.to_string(),
            bg_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn add_button(&mut self, button: ButtonConfig) {
        self.buttons.push(button);
    }

    pub fn button_at(&self, x: f32, y: f32) -> Option<&ButtonConfig> {
        self.buttons
            .iter()
            .find(|b| b.visible && b.enabled && b.contains_point(x, y))
    }

    pub fn button_at_mut(&mut self, x: f32, y: f32) -> Option<&mut ButtonConfig> {
        self.buttons
            .iter_mut()
            .find(|b| b.visible && b.enabled && b.contains_point(x, y))
    }
}

// ── ScreenManager ──

#[cfg(feature = "game")]
pub struct ScreenManager {
    pub history: ScreenHistory,
    pub layouts: HashMap<ScreenState, ScreenLayout>,
    pub transition_queue: Vec<ScreenTransition>,
    pub fade_alpha: f32,
    pub transitioning: bool,
}

#[cfg(feature = "game")]
impl ScreenManager {
    pub fn new() -> Self {
        Self {
            history: ScreenHistory::new(ScreenState::MainMenu),
            layouts: HashMap::new(),
            transition_queue: Vec::new(),
            fade_alpha: 0.0,
            transitioning: false,
        }
    }

    pub fn current_state(&self) -> ScreenState {
        self.history.current()
    }

    pub fn register_layout(&mut self, layout: ScreenLayout) {
        self.layouts.insert(layout.state, layout);
    }

    pub fn current_layout(&self) -> Option<&ScreenLayout> {
        self.layouts.get(&self.current_state())
    }

    pub fn current_layout_mut(&mut self) -> Option<&mut ScreenLayout> {
        self.layouts.get_mut(&self.current_state())
    }

    pub fn queue_transition(&mut self, transition: ScreenTransition) {
        self.transition_queue.push(transition);
    }

    pub fn process_transitions(&mut self) {
        let queue = std::mem::take(&mut self.transition_queue);
        for transition in queue {
            match transition {
                ScreenTransition::Push(state) => {
                    let _ = self.history.push(state);
                }
                ScreenTransition::Pop => {
                    self.history.pop();
                }
                ScreenTransition::Replace(state) => {
                    self.history.replace(state);
                }
                ScreenTransition::ClearAndPush(state) => {
                    self.history.clear_and_push(state);
                }
            }
        }
    }

    pub fn handle_click(&mut self, x: f32, y: f32) -> Option<u32> {
        self.current_layout()
            .and_then(|layout| layout.button_at(x, y))
            .map(|button| button.action_id)
    }

    pub fn is_paused(&self) -> bool {
        self.current_state().pauses_gameplay()
    }

    pub fn shows_hud(&self) -> bool {
        self.current_state().shows_hud()
    }

    pub fn go_back(&mut self) -> Option<ScreenState> {
        self.history.pop()
    }
}

// ── LayoutPresets ──

#[cfg(feature = "game")]
pub struct LayoutPresets;

#[cfg(feature = "game")]
impl LayoutPresets {
    pub fn main_menu(screen_w: f32, screen_h: f32) -> ScreenLayout {
        let mut layout = ScreenLayout::new(ScreenState::MainMenu, "Chronos Company");
        let bw = 200.0;
        let bh = 40.0;
        let cx = screen_w / 2.0 - bw / 2.0;
        let start_y = screen_h / 2.0 - (4.0 * (bh + 10.0)) / 2.0;

        layout.add_button(ButtonConfig::new("New Game", cx, start_y, bw, bh, 1));
        layout.add_button(ButtonConfig::new(
            "Load Game",
            cx,
            start_y + bh + 10.0,
            bw,
            bh,
            2,
        ));
        layout.add_button(ButtonConfig::new(
            "Settings",
            cx,
            start_y + 2.0 * (bh + 10.0),
            bw,
            bh,
            3,
        ));
        layout.add_button(ButtonConfig::new(
            "Quit",
            cx,
            start_y + 3.0 * (bh + 10.0),
            bw,
            bh,
            4,
        ));

        layout
    }

    pub fn pause_menu(screen_w: f32, screen_h: f32) -> ScreenLayout {
        let mut layout = ScreenLayout::new(ScreenState::Paused, "Paused");
        let bw = 200.0;
        let bh = 40.0;
        let cx = screen_w / 2.0 - bw / 2.0;
        let start_y = screen_h / 2.0 - (3.0 * (bh + 10.0)) / 2.0;

        layout.add_button(ButtonConfig::new("Resume", cx, start_y, bw, bh, 10));
        layout.add_button(ButtonConfig::new(
            "Settings",
            cx,
            start_y + bh + 10.0,
            bw,
            bh,
            11,
        ));
        layout.add_button(ButtonConfig::new(
            "Quit to Menu",
            cx,
            start_y + 2.0 * (bh + 10.0),
            bw,
            bh,
            12,
        ));

        layout
    }

    pub fn game_over(screen_w: f32, screen_h: f32) -> ScreenLayout {
        let mut layout = ScreenLayout::new(ScreenState::GameOver, "Game Over");
        let bw = 200.0;
        let bh = 40.0;
        let cx = screen_w / 2.0 - bw / 2.0;
        let start_y = screen_h / 2.0 - (2.0 * (bh + 10.0)) / 2.0;

        layout.add_button(ButtonConfig::new("Load Save", cx, start_y, bw, bh, 20));
        layout.add_button(ButtonConfig::new(
            "Quit to Menu",
            cx,
            start_y + bh + 10.0,
            bw,
            bh,
            21,
        ));

        layout
    }
}

// ── Tests ──

#[cfg(feature = "game")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_state_properties() {
        // MainMenu
        assert_eq!(ScreenState::MainMenu.name(), "Main Menu");
        assert!(!ScreenState::MainMenu.pauses_gameplay());
        assert!(!ScreenState::MainMenu.shows_hud());

        // Loading
        assert_eq!(ScreenState::Loading.name(), "Loading");
        assert!(!ScreenState::Loading.pauses_gameplay());
        assert!(!ScreenState::Loading.shows_hud());

        // Gameplay
        assert_eq!(ScreenState::Gameplay.name(), "Gameplay");
        assert!(!ScreenState::Gameplay.pauses_gameplay());
        assert!(ScreenState::Gameplay.shows_hud());

        // Paused
        assert_eq!(ScreenState::Paused.name(), "Paused");
        assert!(ScreenState::Paused.pauses_gameplay());
        assert!(ScreenState::Paused.shows_hud());

        // CharacterSheet
        assert_eq!(ScreenState::CharacterSheet.name(), "Character Sheet");
        assert!(ScreenState::CharacterSheet.pauses_gameplay());
        assert!(!ScreenState::CharacterSheet.shows_hud());

        // Inventory
        assert_eq!(ScreenState::Inventory.name(), "Inventory");
        assert!(ScreenState::Inventory.pauses_gameplay());
        assert!(!ScreenState::Inventory.shows_hud());

        // JobBoard
        assert_eq!(ScreenState::JobBoard.name(), "Job Board");
        assert!(ScreenState::JobBoard.pauses_gameplay());
        assert!(!ScreenState::JobBoard.shows_hud());

        // Dialogue
        assert_eq!(ScreenState::Dialogue.name(), "Dialogue");
        assert!(ScreenState::Dialogue.pauses_gameplay());
        assert!(ScreenState::Dialogue.shows_hud());

        // Settings
        assert_eq!(ScreenState::Settings.name(), "Settings");
        assert!(ScreenState::Settings.pauses_gameplay());
        assert!(!ScreenState::Settings.shows_hud());

        // GameOver
        assert_eq!(ScreenState::GameOver.name(), "Game Over");
        assert!(!ScreenState::GameOver.pauses_gameplay());
        assert!(!ScreenState::GameOver.shows_hud());
    }

    #[test]
    fn screen_history_push_pop() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        assert_eq!(history.current(), ScreenState::MainMenu);
        assert_eq!(history.depth(), 1);

        history.push(ScreenState::Loading).unwrap();
        assert_eq!(history.current(), ScreenState::Loading);
        assert_eq!(history.depth(), 2);

        history.push(ScreenState::Gameplay).unwrap();
        assert_eq!(history.current(), ScreenState::Gameplay);
        assert_eq!(history.depth(), 3);

        let popped = history.pop();
        assert_eq!(popped, Some(ScreenState::Gameplay));
        assert_eq!(history.current(), ScreenState::Loading);

        let popped2 = history.pop();
        assert_eq!(popped2, Some(ScreenState::Loading));
        assert_eq!(history.current(), ScreenState::MainMenu);
        assert_eq!(history.depth(), 1);
    }

    #[test]
    fn screen_history_max_depth() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        // max_depth is 10, already has 1
        for _ in 0..9 {
            assert!(history.push(ScreenState::Gameplay).is_ok());
        }
        assert_eq!(history.depth(), 10);
        assert_eq!(
            history.push(ScreenState::Gameplay),
            Err(ScreenError::StackOverflow)
        );
    }

    #[test]
    fn screen_history_replace() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        history.push(ScreenState::Loading).unwrap();
        assert_eq!(history.current(), ScreenState::Loading);

        let old = history.replace(ScreenState::Gameplay);
        assert_eq!(old, ScreenState::Loading);
        assert_eq!(history.current(), ScreenState::Gameplay);
        assert_eq!(history.depth(), 2);
    }

    #[test]
    fn screen_history_clear_and_push() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        history.push(ScreenState::Loading).unwrap();
        history.push(ScreenState::Gameplay).unwrap();
        history.push(ScreenState::Paused).unwrap();
        assert_eq!(history.depth(), 4);

        history.clear_and_push(ScreenState::GameOver);
        assert_eq!(history.depth(), 1);
        assert_eq!(history.current(), ScreenState::GameOver);
        assert_eq!(history.history(), &[ScreenState::GameOver]);
    }

    #[test]
    fn screen_layout_button_at() {
        let mut layout = ScreenLayout::new(ScreenState::MainMenu, "Test");
        layout.add_button(ButtonConfig::new("Click Me", 10.0, 20.0, 100.0, 50.0, 42));
        layout.add_button(ButtonConfig::new("Other", 200.0, 200.0, 100.0, 50.0, 99));

        let btn = layout.button_at(50.0, 40.0);
        assert!(btn.is_some());
        assert_eq!(btn.unwrap().action_id, 42);

        let btn2 = layout.button_at(250.0, 225.0);
        assert!(btn2.is_some());
        assert_eq!(btn2.unwrap().action_id, 99);
    }

    #[test]
    fn screen_layout_button_at_misses() {
        let mut layout = ScreenLayout::new(ScreenState::MainMenu, "Test");
        layout.add_button(ButtonConfig::new("Click Me", 10.0, 20.0, 100.0, 50.0, 42));

        // Outside all buttons
        assert!(layout.button_at(0.0, 0.0).is_none());
        assert!(layout.button_at(500.0, 500.0).is_none());

        // Disabled button not found
        let mut layout2 = ScreenLayout::new(ScreenState::MainMenu, "Test");
        layout2.add_button(ButtonConfig::new("Nope", 10.0, 20.0, 100.0, 50.0, 1).with_enabled(false));
        assert!(layout2.button_at(50.0, 40.0).is_none());

        // Invisible button not found
        let mut layout3 = ScreenLayout::new(ScreenState::MainMenu, "Test");
        let mut btn = ButtonConfig::new("Hidden", 10.0, 20.0, 100.0, 50.0, 2);
        btn.visible = false;
        layout3.add_button(btn);
        assert!(layout3.button_at(50.0, 40.0).is_none());
    }

    #[test]
    fn screen_manager_creation() {
        let manager = ScreenManager::new();
        assert_eq!(manager.current_state(), ScreenState::MainMenu);
        assert_eq!(manager.fade_alpha, 0.0);
        assert!(!manager.transitioning);
        assert!(manager.transition_queue.is_empty());
        assert!(manager.layouts.is_empty());
    }

    #[test]
    fn screen_manager_queue_and_process() {
        let mut manager = ScreenManager::new();
        manager.queue_transition(ScreenTransition::Push(ScreenState::Loading));
        manager.queue_transition(ScreenTransition::Replace(ScreenState::Gameplay));
        manager.queue_transition(ScreenTransition::Push(ScreenState::Paused));

        assert_eq!(manager.transition_queue.len(), 3);
        manager.process_transitions();
        assert_eq!(manager.current_state(), ScreenState::Paused);
        assert!(manager.transition_queue.is_empty());
        assert_eq!(manager.history.depth(), 3);

        // ClearAndPush
        manager.queue_transition(ScreenTransition::ClearAndPush(ScreenState::GameOver));
        manager.process_transitions();
        assert_eq!(manager.current_state(), ScreenState::GameOver);
        assert_eq!(manager.history.depth(), 1);

        // Pop on single element does nothing
        manager.queue_transition(ScreenTransition::Pop);
        manager.process_transitions();
        assert_eq!(manager.current_state(), ScreenState::GameOver);
        assert_eq!(manager.history.depth(), 1);
    }

    #[test]
    fn screen_manager_handle_click() {
        let mut manager = ScreenManager::new();
        let mut layout = ScreenLayout::new(ScreenState::MainMenu, "Main");
        layout.add_button(ButtonConfig::new("New Game", 100.0, 100.0, 200.0, 50.0, 1));
        layout.add_button(ButtonConfig::new("Quit", 100.0, 160.0, 200.0, 50.0, 4));
        manager.register_layout(layout);

        assert_eq!(manager.handle_click(150.0, 120.0), Some(1));
        assert_eq!(manager.handle_click(150.0, 180.0), Some(4));
        assert_eq!(manager.handle_click(0.0, 0.0), None);
    }

    #[test]
    fn screen_manager_go_back() {
        let mut manager = ScreenManager::new();
        manager.queue_transition(ScreenTransition::Push(ScreenState::Gameplay));
        manager.process_transitions();

        let popped = manager.go_back();
        assert_eq!(popped, Some(ScreenState::Gameplay));
        assert_eq!(manager.current_state(), ScreenState::MainMenu);

        // Can't go back further
        let popped2 = manager.go_back();
        assert_eq!(popped2, None);
        assert_eq!(manager.current_state(), ScreenState::MainMenu);
    }

    #[test]
    fn layout_presets_main_menu() {
        let layout = LayoutPresets::main_menu(800.0, 600.0);
        assert_eq!(layout.title, "Chronos Company");
        assert_eq!(layout.state, ScreenState::MainMenu);
        assert_eq!(layout.buttons.len(), 4);

        assert_eq!(layout.buttons[0].label, "New Game");
        assert_eq!(layout.buttons[0].action_id, 1);
        assert_eq!(layout.buttons[1].label, "Load Game");
        assert_eq!(layout.buttons[1].action_id, 2);
        assert_eq!(layout.buttons[2].label, "Settings");
        assert_eq!(layout.buttons[2].action_id, 3);
        assert_eq!(layout.buttons[3].label, "Quit");
        assert_eq!(layout.buttons[3].action_id, 4);

        // All buttons centered horizontally
        let cx = 800.0 / 2.0 - 200.0 / 2.0;
        for btn in &layout.buttons {
            assert!((btn.x - cx).abs() < f32::EPSILON);
            assert!((btn.width - 200.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn layout_presets_pause() {
        let layout = LayoutPresets::pause_menu(800.0, 600.0);
        assert_eq!(layout.title, "Paused");
        assert_eq!(layout.state, ScreenState::Paused);
        assert_eq!(layout.buttons.len(), 3);

        assert_eq!(layout.buttons[0].label, "Resume");
        assert_eq!(layout.buttons[0].action_id, 10);
        assert_eq!(layout.buttons[1].label, "Settings");
        assert_eq!(layout.buttons[1].action_id, 11);
        assert_eq!(layout.buttons[2].label, "Quit to Menu");
        assert_eq!(layout.buttons[2].action_id, 12);
    }

    #[test]
    fn button_config_builder_and_contains() {
        let btn = ButtonConfig::new("Test", 10.0, 20.0, 100.0, 50.0, 7);
        assert!(btn.enabled);
        assert!(btn.visible);
        assert!(btn.contains_point(10.0, 20.0));
        assert!(btn.contains_point(110.0, 70.0));
        assert!(!btn.contains_point(9.9, 20.0));
        assert!(!btn.contains_point(10.0, 19.9));

        let disabled = btn.with_enabled(false);
        assert!(!disabled.enabled);
    }

    #[test]
    fn screen_manager_is_paused_and_shows_hud() {
        let mut manager = ScreenManager::new();
        assert!(!manager.is_paused());
        assert!(!manager.shows_hud());

        manager.queue_transition(ScreenTransition::Push(ScreenState::Gameplay));
        manager.process_transitions();
        assert!(!manager.is_paused());
        assert!(manager.shows_hud());

        manager.queue_transition(ScreenTransition::Push(ScreenState::Paused));
        manager.process_transitions();
        assert!(manager.is_paused());
        assert!(manager.shows_hud());

        manager.queue_transition(ScreenTransition::Replace(ScreenState::Inventory));
        manager.process_transitions();
        assert!(manager.is_paused());
        assert!(!manager.shows_hud());
    }

    #[test]
    fn screen_layout_button_at_mut() {
        let mut layout = ScreenLayout::new(ScreenState::MainMenu, "Test");
        layout.add_button(ButtonConfig::new("Click Me", 10.0, 20.0, 100.0, 50.0, 42));

        let btn = layout.button_at_mut(50.0, 40.0);
        assert!(btn.is_some());
        btn.unwrap().action_id = 99;

        assert_eq!(layout.buttons[0].action_id, 99);

        assert!(layout.button_at_mut(500.0, 500.0).is_none());
    }

    #[test]
    fn screen_history_pop_never_empties() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        assert_eq!(history.pop(), None);
        assert_eq!(history.depth(), 1);
        assert_eq!(history.current(), ScreenState::MainMenu);
    }

    #[test]
    fn screen_history_can_pop_and_is_state() {
        let mut history = ScreenHistory::new(ScreenState::MainMenu);
        assert!(!history.can_pop());
        assert!(history.is_state(ScreenState::MainMenu));

        history.push(ScreenState::Gameplay).unwrap();
        assert!(history.can_pop());
        assert!(history.is_state(ScreenState::Gameplay));
        assert!(!history.is_state(ScreenState::MainMenu));
    }
}

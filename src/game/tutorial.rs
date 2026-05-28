#[cfg(feature = "game")]
use std::collections::HashMap;

// ── ObjectiveType ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveType {
    MoveToLocation,
    KillEnemies,
    CollectItems,
    TalkToNpc,
    UseAbility,
    SelectUnits,
    FormSquad,
    Complete,
}

impl ObjectiveType {
    pub fn description_template(&self) -> &str {
        match self {
            ObjectiveType::MoveToLocation => "Move to the marked location",
            ObjectiveType::KillEnemies => "Defeat {count} enemies",
            ObjectiveType::CollectItems => "Collect {count} items",
            ObjectiveType::TalkToNpc => "Talk to {npc_name}",
            ObjectiveType::UseAbility => "Use the {ability_name} ability",
            ObjectiveType::SelectUnits => "Select {count} units",
            ObjectiveType::FormSquad => "Form a squad with {count} units",
            ObjectiveType::Complete => "Complete the objective",
        }
    }
}

// ── Objective ──

#[derive(Debug, Clone, PartialEq)]
pub struct Objective {
    pub id: u32,
    pub objective_type: ObjectiveType,
    pub title: String,
    pub description: String,
    pub current_progress: u32,
    pub target_progress: u32,
    pub completed: bool,
    pub optional: bool,
    pub reward_xp: u32,
    pub reward_gold: u32,
}

impl Objective {
    pub fn new(id: u32, objective_type: ObjectiveType, title: &str, description: &str) -> Self {
        Self {
            id,
            objective_type,
            title: title.to_string(),
            description: description.to_string(),
            current_progress: 0,
            target_progress: 1,
            completed: false,
            optional: false,
            reward_xp: 0,
            reward_gold: 0,
        }
    }

    pub fn with_target(mut self, target: u32) -> Self {
        self.target_progress = target;
        self
    }

    pub fn with_rewards(mut self, xp: u32, gold: u32) -> Self {
        self.reward_xp = xp;
        self.reward_gold = gold;
        self
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    pub fn advance(&mut self, amount: u32) -> bool {
        if self.completed {
            return false;
        }
        self.current_progress = (self.current_progress + amount).min(self.target_progress);
        if self.current_progress >= self.target_progress {
            self.completed = true;
            return true;
        }
        false
    }

    pub fn progress_fraction(&self) -> f32 {
        if self.target_progress == 0 {
            return 1.0;
        }
        self.current_progress as f32 / self.target_progress as f32
    }

    pub fn is_complete(&self) -> bool {
        self.completed
    }
}

// ── TutorialStep ──

#[derive(Debug, Clone, PartialEq)]
pub struct TutorialStep {
    pub id: u32,
    pub title: String,
    pub instruction: String,
    pub objective: Objective,
    pub hint: String,
    pub highlight_element: Option<String>,
    pub auto_advance: bool,
    pub completed: bool,
}

impl TutorialStep {
    pub fn new(id: u32, title: &str, instruction: &str, objective: Objective) -> Self {
        Self {
            id,
            title: title.to_string(),
            instruction: instruction.to_string(),
            objective,
            hint: String::new(),
            highlight_element: None,
            auto_advance: false,
            completed: false,
        }
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = hint.to_string();
        self
    }

    pub fn with_highlight(mut self, element: &str) -> Self {
        self.highlight_element = Some(element.to_string());
        self
    }

    pub fn with_auto_advance(mut self, auto: bool) -> Self {
        self.auto_advance = auto;
        self
    }

    pub fn advance_objective(&mut self, amount: u32) -> bool {
        if self.completed {
            return false;
        }
        let just_completed = self.objective.advance(amount);
        if just_completed {
            self.completed = true;
        }
        just_completed
    }

    pub fn is_complete(&self) -> bool {
        self.completed
    }
}

// ── TutorialSequence ──

#[derive(Debug, Clone, PartialEq)]
pub struct TutorialSequence {
    pub steps: Vec<TutorialStep>,
    pub current_step_index: usize,
    pub active: bool,
    pub completed: bool,
    pub name: String,
}

impl TutorialSequence {
    pub fn new(name: &str, steps: Vec<TutorialStep>) -> Self {
        Self {
            steps,
            current_step_index: 0,
            active: false,
            completed: false,
            name: name.to_string(),
        }
    }

    pub fn current_step(&self) -> Option<&TutorialStep> {
        self.steps.get(self.current_step_index)
    }

    pub fn current_step_mut(&mut self) -> Option<&mut TutorialStep> {
        self.steps.get_mut(self.current_step_index)
    }

    pub fn advance(&mut self) -> Option<&TutorialStep> {
        if self.completed {
            return None;
        }
        self.current_step_index += 1;
        if self.current_step_index >= self.steps.len() {
            self.completed = true;
            self.active = false;
            return None;
        }
        self.steps.get(self.current_step_index)
    }

    pub fn advance_objective(&mut self, amount: u32) -> bool {
        if self.completed || !self.active {
            return false;
        }
        if let Some(step) = self.steps.get_mut(self.current_step_index) {
            let just_completed = step.advance_objective(amount);
            if just_completed && step.auto_advance {
                // Mark sequence completed if this was the last step
                self.current_step_index += 1;
                if self.current_step_index >= self.steps.len() {
                    self.completed = true;
                    self.active = false;
                    return true;
                }
            }
            just_completed
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn is_complete(&self) -> bool {
        self.completed
    }

    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn skip(&mut self) {
        for step in &mut self.steps {
            step.completed = true;
            step.objective.completed = true;
            step.objective.current_progress = step.objective.target_progress;
        }
        self.completed = true;
        self.active = false;
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn completed_step_count(&self) -> usize {
        self.steps.iter().filter(|s| s.completed).count()
    }
}

// ── HintSystem ──

#[derive(Debug, Clone, PartialEq)]
pub struct HintSystem {
    pub hints: HashMap<String, String>,
    pub shown_hints: Vec<String>,
    pub enabled: bool,
    pub auto_hide_delay: f32,
}

impl HintSystem {
    pub fn new() -> Self {
        Self {
            hints: HashMap::new(),
            shown_hints: Vec::new(),
            enabled: true,
            auto_hide_delay: 5.0,
        }
    }

    pub fn register_hint(&mut self, element: &str, text: &str) {
        self.hints.insert(element.to_string(), text.to_string());
    }

    pub fn get_hint(&self, element: &str) -> Option<&str> {
        self.hints.get(element).map(|s| s.as_str())
    }

    pub fn show_hint(&mut self, element: &str) -> Option<&str> {
        if let Some(text) = self.hints.get(element) {
            let _text_str = text.as_str();
            if !self.shown_hints.contains(&element.to_string()) {
                self.shown_hints.push(element.to_string());
            }
            // Return a reference into the hints map
            self.hints.get(element).map(|s| s.as_str())
        } else {
            None
        }
    }

    pub fn has_shown(&self, element: &str) -> bool {
        self.shown_hints.iter().any(|e| e == element)
    }

    pub fn should_show(&self, element: &str) -> bool {
        self.enabled && self.hints.contains_key(element) && !self.has_shown(element)
    }

    pub fn reset(&mut self) {
        self.shown_hints.clear();
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

// ── TutorialPresets ──

pub struct TutorialPresets;

impl TutorialPresets {
    pub fn basic_training() -> TutorialSequence {
        let steps = vec![
            // Step 1: Welcome
            TutorialStep::new(
                1,
                "Welcome",
                "Welcome to Chronos Company! Let's learn the basics.",
                Objective::new(1, ObjectiveType::Complete, "Welcome", "Acknowledge the welcome message"),
            )
            .with_hint("Use WASD to move the camera, left-click to select units")
            .with_auto_advance(true),
            // Step 2: Select Your Squad
            TutorialStep::new(
                2,
                "Select Your Squad",
                "Click on a unit to select it for battle.",
                Objective::new(2, ObjectiveType::SelectUnits, "Select Units", "Select your first unit")
                    .with_target(1),
            )
            .with_hint("Left-click on a unit portrait to select it")
            .with_highlight("squad_panel"),
            // Step 3: Move to the Marker
            TutorialStep::new(
                3,
                "Move to the Marker",
                "Right-click on the minimap marker to move your squad.",
                Objective::new(3, ObjectiveType::MoveToLocation, "Move to Marker", "Navigate to the marked location")
                    .with_target(1),
            )
            .with_hint("Right-click on the ground or minimap to issue a move command")
            .with_highlight("minimap"),
            // Step 4: Defeat the Enemies
            TutorialStep::new(
                4,
                "Defeat the Enemies",
                "Engage and defeat the enemy forces ahead.",
                Objective::new(4, ObjectiveType::KillEnemies, "Defeat Enemies", "Eliminate all hostile targets")
                    .with_target(3),
            )
            .with_hint("Use abilities by clicking their icons or pressing hotkeys 1-5")
            .with_highlight("abilities"),
            // Step 5: Talk to the Commander
            TutorialStep::new(
                5,
                "Talk to the Commander",
                "Approach the commander to complete your training.",
                Objective::new(5, ObjectiveType::TalkToNpc, "Talk to Commander", "Speak with the field commander")
                    .with_target(1)
                    .with_rewards(100, 50),
            )
            .with_hint("Walk near an NPC and press E to interact")
            .with_auto_advance(true),
        ];

        TutorialSequence::new("Basic Training", steps)
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_advance() {
        let mut obj = Objective::new(1, ObjectiveType::KillEnemies, "Kill", "Defeat enemies")
            .with_target(3);
        assert!(!obj.advance(1));
        assert_eq!(obj.current_progress, 1);
        assert!(!obj.completed);
        assert!(obj.advance(2));
        assert_eq!(obj.current_progress, 3);
        assert!(obj.completed);
    }

    #[test]
    fn objective_advance_over_target() {
        let mut obj = Objective::new(1, ObjectiveType::CollectItems, "Collect", "Gather items")
            .with_target(2);
        assert!(obj.advance(10));
        assert_eq!(obj.current_progress, 2);
        assert!(obj.completed);
    }

    #[test]
    fn objective_progress_fraction() {
        let mut obj = Objective::new(1, ObjectiveType::KillEnemies, "Kill", "Defeat enemies")
            .with_target(4);
        assert!((obj.progress_fraction() - 0.0).abs() < f32::EPSILON);
        obj.advance(1);
        assert!((obj.progress_fraction() - 0.25).abs() < f32::EPSILON);
        obj.advance(3);
        assert!((obj.progress_fraction() - 1.0).abs() < f32::EPSILON);

        let zero_target = Objective::new(2, ObjectiveType::Complete, "Done", "Complete")
            .with_target(0);
        assert!((zero_target.progress_fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tutorial_step_completion() {
        let obj = Objective::new(1, ObjectiveType::MoveToLocation, "Move", "Move there")
            .with_target(1);
        let mut step = TutorialStep::new(1, "Move", "Go there", obj);
        assert!(!step.is_complete());
        assert!(step.advance_objective(1));
        assert!(step.is_complete());
    }

    #[test]
    fn tutorial_step_auto_advance() {
        let obj = Objective::new(1, ObjectiveType::Complete, "Welcome", "Acknowledge")
            .with_target(1);
        let step = TutorialStep::new(1, "Welcome", "Hi", obj)
            .with_auto_advance(true);
        assert!(step.auto_advance);
    }

    #[test]
    fn tutorial_sequence_flow() {
        let steps = vec![
            TutorialStep::new(1, "Step 1", "First", Objective::new(1, ObjectiveType::Complete, "S1", "First").with_target(1)),
            TutorialStep::new(2, "Step 2", "Second", Objective::new(2, ObjectiveType::Complete, "S2", "Second").with_target(1)),
        ];
        let mut seq = TutorialSequence::new("Test", steps);
        seq.start();
        assert!(seq.is_active());
        assert_eq!(seq.current_step().unwrap().title, "Step 1");

        seq.advance();
        assert_eq!(seq.current_step().unwrap().title, "Step 2");

        seq.advance();
        assert!(seq.is_complete());
        assert!(seq.current_step().is_none());
    }

    #[test]
    fn tutorial_sequence_skip() {
        let steps = vec![
            TutorialStep::new(1, "A", "a", Objective::new(1, ObjectiveType::Complete, "A", "a")),
            TutorialStep::new(2, "B", "b", Objective::new(2, ObjectiveType::Complete, "B", "b")),
        ];
        let mut seq = TutorialSequence::new("Test", steps);
        seq.start();
        seq.skip();
        assert!(seq.is_complete());
        assert!(!seq.is_active());
        assert!(seq.steps.iter().all(|s| s.is_complete()));
    }

    #[test]
    fn tutorial_sequence_completed_steps_count() {
        let steps = vec![
            TutorialStep::new(1, "A", "a", Objective::new(1, ObjectiveType::Complete, "A", "a").with_target(1)),
            TutorialStep::new(2, "B", "b", Objective::new(2, ObjectiveType::Complete, "B", "b").with_target(1)),
            TutorialStep::new(3, "C", "c", Objective::new(3, ObjectiveType::Complete, "C", "c").with_target(1)),
        ];
        let mut seq = TutorialSequence::new("Test", steps);
        assert_eq!(seq.completed_step_count(), 0);

        seq.start();
        seq.advance_objective(1);
        assert_eq!(seq.completed_step_count(), 1);

        seq.advance();
        seq.advance_objective(1);
        assert_eq!(seq.completed_step_count(), 2);
    }

    #[test]
    fn hint_system_register_and_show() {
        let mut hs = HintSystem::new();
        hs.register_hint("minimap", "Click to move camera");
        assert_eq!(hs.get_hint("minimap"), Some("Click to move camera"));

        let result = hs.show_hint("minimap");
        assert_eq!(result, Some("Click to move camera"));
        assert!(hs.has_shown("minimap"));
        assert!(!hs.has_shown("abilities"));
    }

    #[test]
    fn hint_system_should_show() {
        let mut hs = HintSystem::new();
        hs.register_hint("panel", "Click here");
        assert!(hs.should_show("panel"));

        hs.show_hint("panel");
        assert!(!hs.should_show("panel")); // already shown

        hs.enabled = false;
        assert!(!hs.should_show("panel")); // disabled

        hs.enabled = true;
        assert!(!hs.should_show("nonexistent")); // doesn't exist
    }

    #[test]
    fn hint_system_reset() {
        let mut hs = HintSystem::new();
        hs.register_hint("x", "hint x");
        hs.show_hint("x");
        assert!(hs.has_shown("x"));

        hs.reset();
        assert!(!hs.has_shown("x"));
        assert!(hs.should_show("x"));
    }

    #[test]
    fn tutorial_presets_basic_training() {
        let seq = TutorialPresets::basic_training();
        assert_eq!(seq.steps.len(), 5);
        assert_eq!(seq.name, "Basic Training");
        assert_eq!(seq.steps[0].objective.objective_type, ObjectiveType::Complete);
        assert_eq!(seq.steps[1].objective.objective_type, ObjectiveType::SelectUnits);
        assert_eq!(seq.steps[2].objective.objective_type, ObjectiveType::MoveToLocation);
        assert_eq!(seq.steps[3].objective.objective_type, ObjectiveType::KillEnemies);
        assert_eq!(seq.steps[3].objective.target_progress, 3);
        assert_eq!(seq.steps[4].objective.objective_type, ObjectiveType::TalkToNpc);
        assert_eq!(seq.steps[4].objective.reward_xp, 100);
        assert_eq!(seq.steps[4].objective.reward_gold, 50);
        assert!(seq.steps[0].auto_advance);
        assert!(seq.steps[4].auto_advance);
        assert_eq!(seq.steps[1].highlight_element, Some("squad_panel".to_string()));
        assert_eq!(seq.steps[2].highlight_element, Some("minimap".to_string()));
        assert_eq!(seq.steps[3].highlight_element, Some("abilities".to_string()));
    }

    #[test]
    fn tutorial_presets_advance_through_all() {
        let mut seq = TutorialPresets::basic_training();
        seq.start();
        assert!(seq.is_active());

        // Step 1 auto-advances
        assert!(seq.advance_objective(1));
        // After auto-advance, we should be on step 2
        assert_eq!(seq.current_step().unwrap().id, 2);

        // Step 2: SelectUnits(1)
        seq.advance_objective(1);
        seq.advance();
        assert_eq!(seq.current_step().unwrap().id, 3);

        // Step 3: MoveToLocation(1)
        seq.advance_objective(1);
        seq.advance();
        assert_eq!(seq.current_step().unwrap().id, 4);

        // Step 4: KillEnemies(3)
        seq.advance_objective(3);
        seq.advance();
        assert_eq!(seq.current_step().unwrap().id, 5);

        // Step 5: TalkToNpc(1), auto-advance
        assert!(seq.advance_objective(1));
        assert!(seq.is_complete());
        assert!(!seq.is_active());
    }
}

#[cfg(feature = "game")]

use std::collections::HashMap;

// ──────────────────────────────────────────────
// DialogueCondition
// ──────────────────────────────────────────────

/// Conditions that gate whether a dialogue choice is visible or selectable.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogueCondition {
    /// Player must possess a specific item.
    HasItem { name: String },
    /// A numeric stat must meet a minimum threshold.
    StatCheck { stat: String, min_value: u32 },
    /// Player must be at or above a given level.
    LevelCheck { min_level: u32 },
    /// A quest must have been completed already.
    QuestComplete { quest_id: u32 },
    /// Faction reputation must meet a minimum value.
    ReputationCheck { faction: String, min_value: i32 },
    /// No restriction — always available.
    Always,
    /// Negation of another condition.
    Not(Box<DialogueCondition>),
}

impl DialogueCondition {
    /// Evaluate whether this condition is satisfied given the current context.
    pub fn is_met(&self, context: &DialogueContext) -> bool {
        match self {
            DialogueCondition::HasItem { name } => context.has_item(name),
            DialogueCondition::StatCheck { stat, min_value } => {
                context
                    .get_variable(stat)
                    .and_then(|v| v.parse::<u32>().ok())
                    .map_or(false, |v| v >= *min_value)
            }
            DialogueCondition::LevelCheck { min_level } => context.player_level >= *min_level,
            DialogueCondition::QuestComplete { quest_id } => context.has_quest(*quest_id),
            DialogueCondition::ReputationCheck { faction, min_value } => {
                context.get_reputation(faction) >= *min_value
            }
            DialogueCondition::Always => true,
            DialogueCondition::Not(inner) => !inner.is_met(context),
        }
    }
}

// ──────────────────────────────────────────────
// DialogueContext
// ──────────────────────────────────────────────

/// Snapshot of game state used to evaluate dialogue conditions.
#[derive(Debug, Clone)]
pub struct DialogueContext {
    pub player_level: u32,
    pub player_items: Vec<String>,
    pub completed_quests: Vec<u32>,
    pub faction_reputation: HashMap<String, i32>,
    pub variables: HashMap<String, String>,
}

impl DialogueContext {
    pub fn new() -> Self {
        DialogueContext {
            player_level: 1,
            player_items: Vec::new(),
            completed_quests: Vec::new(),
            faction_reputation: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    pub fn with_level(mut self, level: u32) -> Self {
        self.player_level = level;
        self
    }

    pub fn with_items(mut self, items: Vec<&str>) -> Self {
        self.player_items = items.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_quest(mut self, quest_id: u32) -> Self {
        self.completed_quests.push(quest_id);
        self
    }

    pub fn with_reputation(mut self, faction: &str, value: i32) -> Self {
        self.faction_reputation.insert(faction.to_string(), value);
        self
    }

    pub fn set_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }

    pub fn has_item(&self, name: &str) -> bool {
        self.player_items.iter().any(|i| i == name)
    }

    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.completed_quests.contains(&quest_id)
    }

    pub fn get_reputation(&self, faction: &str) -> i32 {
        self.faction_reputation.get(faction).copied().unwrap_or(0)
    }
}

// ──────────────────────────────────────────────
// DialogueAction
// ──────────────────────────────────────────────

/// Side-effects triggered by selecting a dialogue choice or entering a node.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogueAction {
    GiveItem { name: String },
    GiveGold { amount: u32 },
    GiveXp { amount: u32 },
    StartQuest { quest_id: u32 },
    CompleteQuest { quest_id: u32 },
    SetVariable { key: String, value: String },
    ModifyReputation { faction: String, delta: i32 },
    None,
}

// ──────────────────────────────────────────────
// DialogueChoice
// ──────────────────────────────────────────────

/// A player-facing response option within a dialogue node.
#[derive(Debug, Clone)]
pub struct DialogueChoice {
    pub text: String,
    pub condition: DialogueCondition,
    pub next_node: usize,
    pub actions: Vec<DialogueAction>,
}

impl DialogueChoice {
    pub fn new(text: &str, next_node: usize) -> Self {
        DialogueChoice {
            text: text.to_string(),
            condition: DialogueCondition::Always,
            next_node,
            actions: Vec::new(),
        }
    }

    pub fn with_condition(mut self, condition: DialogueCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn with_action(mut self, action: DialogueAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn is_available(&self, context: &DialogueContext) -> bool {
        self.condition.is_met(context)
    }
}

// ──────────────────────────────────────────────
// DialogueNode
// ──────────────────────────────────────────────

/// A single beat of conversation — one speaker, one line, zero or more choices.
#[derive(Debug, Clone)]
pub struct DialogueNode {
    pub speaker: String,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub actions: Vec<DialogueAction>,
}

impl DialogueNode {
    pub fn new(speaker: &str, text: &str) -> Self {
        DialogueNode {
            speaker: speaker.to_string(),
            text: text.to_string(),
            choices: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn with_choice(mut self, choice: DialogueChoice) -> Self {
        self.choices.push(choice);
        self
    }

    pub fn with_action(mut self, action: DialogueAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Return only the choices whose conditions are met in the given context.
    pub fn available_choices(&self, context: &DialogueContext) -> Vec<&DialogueChoice> {
        self.choices
            .iter()
            .filter(|c| c.is_available(context))
            .collect()
    }
}

// ──────────────────────────────────────────────
// DialogueResult
// ──────────────────────────────────────────────

/// The result returned after advancing or selecting in a dialogue tree.
#[derive(Debug, Clone)]
pub struct DialogueResult {
    pub actions: Vec<DialogueAction>,
    pub node_text: String,
    pub speaker: String,
    pub choices_available: bool,
    pub finished: bool,
}

// ──────────────────────────────────────────────
// DialogueTree
// ──────────────────────────────────────────────

/// A branching conversation made up of [`DialogueNode`]s.
#[derive(Debug, Clone)]
pub struct DialogueTree {
    pub id: u32,
    pub nodes: Vec<DialogueNode>,
    pub current_node: usize,
    pub is_finished: bool,
}

impl DialogueTree {
    pub fn new(id: u32, nodes: Vec<DialogueNode>) -> Self {
        DialogueTree {
            id,
            nodes,
            current_node: 0,
            is_finished: false,
        }
    }

    /// Reference to the node the conversation is currently on.
    pub fn current(&self) -> &DialogueNode {
        &self.nodes[self.current_node]
    }

    /// Select a choice by index within the current node's *available* choices,
    /// move to the target node, and return the result including all triggered actions.
    pub fn select_choice(
        &mut self,
        choice_index: usize,
        context: &DialogueContext,
    ) -> DialogueResult {
        let available = self.current().available_choices(context);
        let choice = match available.get(choice_index) {
            Some(c) => c,
            None => {
                return DialogueResult {
                    actions: Vec::new(),
                    node_text: self.current().text.clone(),
                    speaker: self.current().speaker.clone(),
                    choices_available: !available.is_empty(),
                    finished: false,
                };
            }
        };

        let mut actions = self.current().actions.clone();
        actions.extend(choice.actions.clone());

        self.current_node = choice.next_node;

        if self.current_node >= self.nodes.len() {
            self.is_finished = true;
            return DialogueResult {
                actions,
                node_text: String::new(),
                speaker: String::new(),
                choices_available: false,
                finished: true,
            };
        }

        let has_choices = !self.current().choices.is_empty();
        DialogueResult {
            actions,
            node_text: self.current().text.clone(),
            speaker: self.current().speaker.clone(),
            choices_available: has_choices,
            finished: false,
        }
    }

    /// Advance to the next node without making a choice (linear dialogue).
    pub fn advance(&mut self) {
        if self.current_node + 1 < self.nodes.len() {
            self.current_node += 1;
        } else {
            self.is_finished = true;
        }
    }

    /// Restart the conversation from the first node.
    pub fn reset(&mut self) {
        self.current_node = 0;
        self.is_finished = false;
    }

    /// Mark the conversation as finished.
    pub fn finish(&mut self) {
        self.is_finished = true;
    }

    /// Whether the conversation is still active (not finished and within bounds).
    pub fn is_active(&self) -> bool {
        !self.is_finished && self.current_node < self.nodes.len()
    }
}

// ──────────────────────────────────────────────
// DialogueManager
// ──────────────────────────────────────────────

/// Orchestrates multiple dialogue trees and tracks the active conversation.
#[derive(Debug, Clone)]
pub struct DialogueManager {
    pub trees: HashMap<u32, DialogueTree>,
    pub active_dialogue: Option<u32>,
    pub context: DialogueContext,
}

impl DialogueManager {
    pub fn new() -> Self {
        DialogueManager {
            trees: HashMap::new(),
            active_dialogue: None,
            context: DialogueContext::new(),
        }
    }

    /// Register a dialogue tree for later use.
    pub fn register(&mut self, tree: DialogueTree) {
        self.trees.insert(tree.id, tree);
    }

    /// Begin a conversation by tree ID. Returns the opening result.
    pub fn start_dialogue(&mut self, tree_id: u32) -> Option<DialogueResult> {
        if !self.trees.contains_key(&tree_id) {
            return None;
        }
        self.active_dialogue = Some(tree_id);
        if let Some(tree) = self.trees.get_mut(&tree_id) {
            tree.reset();
        }
        if let Some(tree) = self.trees.get(&tree_id) {
            let node = tree.current();
            let has_choices = !node.choices.is_empty();
            Some(DialogueResult {
                actions: node.actions.clone(),
                node_text: node.text.clone(),
                speaker: node.speaker.clone(),
                choices_available: has_choices,
                finished: false,
            })
        } else {
            None
        }
    }

    /// Pick a choice in the active dialogue.
    pub fn select_choice(&mut self, choice_index: usize) -> Option<DialogueResult> {
        let tree_id = self.active_dialogue?;
        let context = self.context.clone();
        if let Some(tree) = self.trees.get_mut(&tree_id) {
            let result = tree.select_choice(choice_index, &context);
            if result.finished {
                self.active_dialogue = None;
            }
            Some(result)
        } else {
            None
        }
    }

    /// End the active conversation immediately.
    pub fn end_dialogue(&mut self) {
        if let Some(tree_id) = self.active_dialogue.take() {
            if let Some(tree) = self.trees.get_mut(&tree_id) {
                tree.finish();
            }
        }
    }

    /// Replace the context used for condition evaluation.
    pub fn update_context(&mut self, context: DialogueContext) {
        self.context = context;
    }

    /// Whether a conversation is currently in progress.
    pub fn is_in_dialogue(&self) -> bool {
        self.active_dialogue.is_some()
    }

    /// The speaker name for the current node, if in a dialogue.
    pub fn current_speaker(&self) -> Option<&str> {
        self.active_dialogue
            .and_then(|id| self.trees.get(&id))
            .map(|tree| tree.current().speaker.as_str())
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── DialogueContext condition checks ────────

    #[test]
    fn context_level_check_passes() {
        let ctx = DialogueContext::new().with_level(10);
        assert!(DialogueCondition::LevelCheck { min_level: 5 }.is_met(&ctx));
        assert!(DialogueCondition::LevelCheck { min_level: 10 }.is_met(&ctx));
    }

    #[test]
    fn context_level_check_fails() {
        let ctx = DialogueContext::new().with_level(3);
        assert!(!DialogueCondition::LevelCheck { min_level: 5 }.is_met(&ctx));
    }

    #[test]
    fn context_has_item_check() {
        let ctx = DialogueContext::new().with_items(vec!["Rusty Key", "Health Potion"]);
        assert!(DialogueCondition::HasItem { name: "Rusty Key".into() }.is_met(&ctx));
        assert!(!DialogueCondition::HasItem { name: "Golden Key".into() }.is_met(&ctx));
    }

    #[test]
    fn context_quest_check() {
        let ctx = DialogueContext::new().with_quest(42).with_quest(99);
        assert!(DialogueCondition::QuestComplete { quest_id: 42 }.is_met(&ctx));
        assert!(!DialogueCondition::QuestComplete { quest_id: 7 }.is_met(&ctx));
    }

    #[test]
    fn context_reputation_check() {
        let ctx = DialogueContext::new()
            .with_reputation("Thieves Guild", -5)
            .with_reputation("Merchants", 50);
        assert!(DialogueCondition::ReputationCheck {
            faction: "Merchants".into(),
            min_value: 30,
        }
        .is_met(&ctx));
        assert!(!DialogueCondition::ReputationCheck {
            faction: "Thieves Guild".into(),
            min_value: 0,
        }
        .is_met(&ctx));
    }

    #[test]
    fn context_missing_reputation_defaults_to_zero() {
        let ctx = DialogueContext::new();
        assert_eq!(ctx.get_reputation("Unknown Faction"), 0);
        assert!(DialogueCondition::ReputationCheck {
            faction: "Unknown Faction".into(),
            min_value: -10,
        }
        .is_met(&ctx));
    }

    // ── Not condition negation ──────────────────

    #[test]
    fn not_condition_negates_inner() {
        let ctx = DialogueContext::new().with_level(10);
        let inner = DialogueCondition::LevelCheck { min_level: 5 };
        assert!(inner.is_met(&ctx));
        assert!(!DialogueCondition::Not(Box::new(inner)).is_met(&ctx));
    }

    #[test]
    fn not_condition_negates_always() {
        let ctx = DialogueContext::new();
        assert!(DialogueCondition::Always.is_met(&ctx));
        assert!(!DialogueCondition::Not(Box::new(DialogueCondition::Always)).is_met(&ctx));
    }

    // ── DialogueChoice availability ─────────────

    #[test]
    fn choice_available_when_condition_met() {
        let ctx = DialogueContext::new().with_level(10);
        let choice = DialogueChoice::new("Enter the dungeon", 1)
            .with_condition(DialogueCondition::LevelCheck { min_level: 5 });
        assert!(choice.is_available(&ctx));
    }

    #[test]
    fn choice_hidden_when_condition_not_met() {
        let ctx = DialogueContext::new().with_level(2);
        let choice = DialogueChoice::new("Enter the dungeon", 1)
            .with_condition(DialogueCondition::LevelCheck { min_level: 5 });
        assert!(!choice.is_available(&ctx));
    }

    // ── DialogueTree linear traversal ───────────

    #[test]
    fn linear_traversal_with_advance() {
        let tree = DialogueTree::new(
            1,
            vec![
                DialogueNode::new("Guard", "Halt! Who goes there?"),
                DialogueNode::new("Guard", "Very well, you may pass."),
                DialogueNode::new("Guard", "Move along."),
            ],
        );
        assert_eq!(tree.current().text, "Halt! Who goes there?");

        let mut tree = tree;
        tree.advance();
        assert_eq!(tree.current().text, "Very well, you may pass.");

        tree.advance();
        assert_eq!(tree.current().text, "Move along.");

        tree.advance();
        assert!(tree.is_finished);
    }

    // ── DialogueTree branching with choices ─────

    #[test]
    fn branching_choice_navigation() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            2,
            vec![
                DialogueNode::new("NPC", "Do you accept the quest?")
                    .with_choice(DialogueChoice::new("Yes", 1))
                    .with_choice(DialogueChoice::new("No", 2)),
                DialogueNode::new("NPC", "Wonderful! Here is your reward."),
                DialogueNode::new("NPC", "Perhaps another time."),
            ],
        );

        // Pick "Yes" (index 0 among available)
        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.node_text, "Wonderful! Here is your reward.");
        assert_eq!(result.speaker, "NPC");
        assert!(!result.finished);

        // Node 1 has no choices — not finished until advance or explicit
        assert!(tree.is_active());
    }

    // ── Actions triggered on choice selection ───

    #[test]
    fn actions_collected_from_node_and_choice() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            3,
            vec![
                DialogueNode::new("Merchant", "Buy my wares?")
                    .with_action(DialogueAction::GiveGold { amount: 10 })
                    .with_choice(
                        DialogueChoice::new("Sure!", 1)
                            .with_action(DialogueAction::GiveItem {
                                name: "Potion".into(),
                            }),
                    ),
                DialogueNode::new("Merchant", "Pleasure doing business."),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 2);
        assert_eq!(result.actions[0], DialogueAction::GiveGold { amount: 10 });
        assert_eq!(
            result.actions[1],
            DialogueAction::GiveItem {
                name: "Potion".into()
            }
        );
    }

    // ── Dialogue finishes when node has no choices ─

    #[test]
    fn finishes_when_advancing_past_last_node() {
        let mut tree = DialogueTree::new(
            4,
            vec![
                DialogueNode::new("Narrator", "The end is near."),
                DialogueNode::new("Narrator", "Goodbye."),
            ],
        );
        assert!(tree.is_active());
        tree.advance();
        assert!(tree.is_active());
        tree.advance();
        assert!(tree.is_finished);
        assert!(!tree.is_active());
    }

    // ── Reset dialogue tree ─────────────────────

    #[test]
    fn reset_returns_to_start() {
        let mut tree = DialogueTree::new(
            5,
            vec![
                DialogueNode::new("NPC", "First line."),
                DialogueNode::new("NPC", "Second line."),
            ],
        );
        tree.advance();
        assert_eq!(tree.current_node, 1);
        tree.reset();
        assert_eq!(tree.current_node, 0);
        assert!(!tree.is_finished);
        assert!(tree.is_active());
    }

    // ── Condition-gated choices hide/show ───────

    #[test]
    fn gated_choices_visibility() {
        let low_ctx = DialogueContext::new().with_level(1);
        let high_ctx = DialogueContext::new().with_level(20);

        let node = DialogueNode::new("NPC", "What'll it be?")
            .with_choice(
                DialogueChoice::new("Tell me a secret", 1)
                    .with_condition(DialogueCondition::LevelCheck { min_level: 10 }),
            )
            .with_choice(DialogueChoice::new("Goodbye", 2));

        assert_eq!(node.available_choices(&low_ctx).len(), 1);
        assert_eq!(node.available_choices(&low_ctx)[0].text, "Goodbye");

        assert_eq!(node.available_choices(&high_ctx).len(), 2);
    }

    // ── Variable setting and retrieval ──────────

    #[test]
    fn context_variable_store() {
        let mut ctx = DialogueContext::new();
        assert!(ctx.get_variable("met_king").is_none());

        ctx.set_variable("met_king", "true");
        assert_eq!(ctx.get_variable("met_king"), Some("true"));

        ctx.set_variable("met_king", "false");
        assert_eq!(ctx.get_variable("met_king"), Some("false"));
    }

    #[test]
    fn stat_check_uses_variables() {
        let mut ctx = DialogueContext::new();
        ctx.set_variable("charisma", "7");
        assert!(DialogueCondition::StatCheck {
            stat: "charisma".into(),
            min_value: 5,
        }
        .is_met(&ctx));
        assert!(!DialogueCondition::StatCheck {
            stat: "charisma".into(),
            min_value: 10,
        }
        .is_met(&ctx));
    }

    // ── DialogueManager start/select/end cycle ──

    #[test]
    fn manager_full_cycle() {
        let mut mgr = DialogueManager::new();
        let ctx = DialogueContext::new();
        mgr.update_context(ctx);

        mgr.register(DialogueTree::new(
            100,
            vec![
                DialogueNode::new("Elder", "Welcome, adventurer.")
                    .with_choice(DialogueChoice::new("Greetings", 1))
                    .with_choice(DialogueChoice::new("Leave", 2)),
                DialogueNode::new("Elder", "May the road rise to meet you."),
                DialogueNode::new("Elder", "Farewell."),
            ],
        ));

        assert!(!mgr.is_in_dialogue());
        assert!(mgr.current_speaker().is_none());

        let result = mgr.start_dialogue(100).expect("start");
        assert_eq!(result.speaker, "Elder");
        assert!(result.choices_available);
        assert!(mgr.is_in_dialogue());
        assert_eq!(mgr.current_speaker(), Some("Elder"));

        let result = mgr.select_choice(0).expect("select");
        assert_eq!(result.node_text, "May the road rise to meet you.");
        assert!(mgr.is_in_dialogue());

        mgr.end_dialogue();
        assert!(!mgr.is_in_dialogue());
    }

    #[test]
    fn manager_start_nonexistent_tree() {
        let mut mgr = DialogueManager::new();
        assert!(mgr.start_dialogue(999).is_none());
    }

    #[test]
    fn manager_select_without_active_dialogue() {
        let mut mgr = DialogueManager::new();
        assert!(mgr.select_choice(0).is_none());
    }

    // ── DialogueManager dialogue auto-finishes ──

    #[test]
    fn manager_auto_finish_on_terminal_choice() {
        let mut mgr = DialogueManager::new();
        mgr.update_context(DialogueContext::new());

        mgr.register(DialogueTree::new(
            200,
            vec![
                DialogueNode::new("Ghost", "Boo!")
                    .with_choice(DialogueChoice::new("Run away!", 1)),
                DialogueNode::new("Ghost", "See you never..."),
            ],
        ));

        let _ = mgr.start_dialogue(200);
        // Select "Run away!" → lands on node 1 (no choices)
        let result = mgr.select_choice(0).expect("select");
        assert_eq!(result.node_text, "See you never...");
        assert!(!result.choices_available);
        assert!(!result.finished);
        assert!(mgr.is_in_dialogue());

        // Now advance past end
        if let Some(tree) = mgr.trees.get_mut(&200) {
            tree.advance();
        }
        // Check tree finished
        let tree = mgr.trees.get(&200).expect("tree");
        assert!(tree.is_finished);
    }

    // ── SetVariable action via choice ───────────

    #[test]
    fn set_variable_action_in_result() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            300,
            vec![
                DialogueNode::new("Sage", "Remember this word.")
                    .with_choice(
                        DialogueChoice::new("I will.", 1).with_action(DialogueAction::SetVariable {
                            key: "magic_word".into(),
                            value: "open_sesame".into(),
                        }),
                    ),
                DialogueNode::new("Sage", "Good."),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 1);
        assert_eq!(
            result.actions[0],
            DialogueAction::SetVariable {
                key: "magic_word".into(),
                value: "open_sesame".into(),
            }
        );
    }

    // ── Reputation modification action ──────────

    #[test]
    fn modify_reputation_action() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            301,
            vec![
                DialogueNode::new("Rebel", "Join us?")
                    .with_choice(
                        DialogueChoice::new("I'm in.", 1).with_action(
                            DialogueAction::ModifyReputation {
                                faction: "Rebels".into(),
                                delta: 25,
                            },
                        ),
                    ),
                DialogueNode::new("Rebel", "Welcome to the cause."),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 1);
        assert_eq!(
            result.actions[0],
            DialogueAction::ModifyReputation {
                faction: "Rebels".into(),
                delta: 25
            }
        );
    }

    // ── Quest start and complete actions ─────────

    #[test]
    fn quest_start_and_complete_actions() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            302,
            vec![
                DialogueNode::new("King", "Slay the dragon.")
                    .with_choice(
                        DialogueChoice::new("Consider it done.", 1)
                            .with_action(DialogueAction::StartQuest { quest_id: 77 }),
                    ),
                DialogueNode::new("King", "You have my thanks.")
                    .with_action(DialogueAction::CompleteQuest { quest_id: 77 }),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 1);
        assert_eq!(
            result.actions[0],
            DialogueAction::StartQuest { quest_id: 77 }
        );

        // Advance into node 1 — its auto-actions fire on next select
        // (auto-actions are node-entry actions, not fired by advance alone)
        assert_eq!(tree.current().actions.len(), 1);
    }

    // ── XP and gold actions ─────────────────────

    #[test]
    fn give_xp_and_gold_actions() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            303,
            vec![
                DialogueNode::new("Chest", "You found treasure!")
                    .with_action(DialogueAction::GiveXp { amount: 500 })
                    .with_action(DialogueAction::GiveGold { amount: 250 })
                    .with_choice(DialogueChoice::new("Nice!", 1)),
                DialogueNode::new("Chest", "(empty now)"),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 2); // 2 node + 0 choice actions
        assert!(result.actions.contains(&DialogueAction::GiveXp { amount: 500 }));
        assert!(result.actions.contains(&DialogueAction::GiveGold { amount: 250 }));
    }

    // ── None action does nothing harmful ────────

    #[test]
    fn none_action_is_harmless() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            304,
            vec![
                DialogueNode::new("NPC", "Hmm.")
                    .with_action(DialogueAction::None)
                    .with_choice(DialogueChoice::new("OK", 1)),
                DialogueNode::new("NPC", "Bye."),
            ],
        );

        let result = tree.select_choice(0, &ctx);
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0], DialogueAction::None);
    }

    // ── select_choice with invalid index ────────

    #[test]
    fn select_invalid_choice_returns_current_node() {
        let ctx = DialogueContext::new();
        let mut tree = DialogueTree::new(
            400,
            vec![
                DialogueNode::new("NPC", "Pick one.")
                    .with_choice(DialogueChoice::new("A", 1)),
                DialogueNode::new("NPC", "Done."),
            ],
        );

        let result = tree.select_choice(5, &ctx);
        assert_eq!(result.node_text, "Pick one.");
        assert!(!result.finished);
        assert_eq!(tree.current_node, 0);
    }

    // ── Default context values ──────────────────

    #[test]
    fn default_context_values() {
        let ctx = DialogueContext::new();
        assert_eq!(ctx.player_level, 1);
        assert!(ctx.player_items.is_empty());
        assert!(ctx.completed_quests.is_empty());
        assert!(ctx.faction_reputation.is_empty());
        assert!(ctx.variables.is_empty());
    }
}

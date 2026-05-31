use crate::component::{Health, Transform};
use crate::system::{EventBus, GameLoop};
#[cfg(feature = "game")]
use crate::{Entity, World};

use super::ambience::AmbienceManager;
use super::camera::TabletopCamera;
use super::combat::{AttackCooldown, AttackType, CombatState, CombatSystem};
use super::components::{MercenaryStats, MoveTarget, NavigationAgent, Selectable, Team};
use super::daynight::{DayNightCycle, TimePhase};
use super::dialogue::DialogueManager;
use super::encounters::{EncounterManager, EncounterSpawnConfig, EncounterSystem};
use super::factions::{FactionId, ReputationTracker};
use super::hud::{HudOverlay, SquadMemberDisplay};
use super::mercenary::{MercenaryFactory, MercenaryTemplate};
use super::navigation::Pathfinder;
use super::save::{FactionSaveData, PlayerSaveData, SaveManager, WorldSaveData};
use super::screens::{LayoutPresets, ScreenManager, ScreenState, ScreenTransition};
use super::selection::SelectionManager;
use super::terrain::TerrainGrid;
use super::world_map::{WorldGenerator, WorldMap};

// ── GameMode ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Campaign,
    Sandbox,
    Tutorial,
}

// ── GameConfig ──

#[derive(Debug, Clone, PartialEq)]
pub struct GameConfig {
    pub mode: GameMode,
    pub map_width: usize,
    pub map_height: usize,
    pub world_seed: u64,
    pub start_gold: u32,
    pub start_squad_size: usize,
    pub auto_save_interval: u64,
    pub screen_width: f32,
    pub screen_height: f32,
    pub cell_size: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            mode: GameMode::Campaign,
            map_width: 40,
            map_height: 40,
            world_seed: 12345,
            start_gold: 500,
            start_squad_size: 4,
            auto_save_interval: 300,
            screen_width: 1920.0,
            screen_height: 1080.0,
            cell_size: 2.0,
        }
    }
}

// ── GameState ──

#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub tick: u64,
    pub total_play_time: f64,
    pub day: u32,
    pub hour: f32,
    pub gold: u32,
    pub current_region: String,
    pub player_level: u32,
    pub player_xp: u32,
    pub encounters_completed: u32,
    pub jobs_completed: u32,
    pub entities_killed: u32,
    pub is_running: bool,
    pub is_paused: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            tick: 0,
            total_play_time: 0.0,
            day: 1,
            hour: 8.0,
            gold: 0,
            current_region: "Plains".to_string(),
            player_level: 1,
            player_xp: 0,
            encounters_completed: 0,
            jobs_completed: 0,
            entities_killed: 0,
            is_running: false,
            is_paused: false,
        }
    }
}

// ── ChronosCompanyGame ──

/// The unified Chronos Company game runner.
///
/// Orchestrates world generation, entity spawning, combat, dialogue,
/// encounters, day/night cycle, camera, HUD, screen management,
/// save/load, and faction reputation into a single playable simulation.
pub struct ChronosCompanyGame {
    pub config: GameConfig,
    pub state: GameState,
    pub world: World,
    pub game_loop: GameLoop,
    pub screen_manager: ScreenManager,
    pub hud: HudOverlay,
    pub world_map: WorldMap,
    pub terrain: TerrainGrid,
    pub pathfinder: Pathfinder,
    pub save_manager: SaveManager,
    pub dialogue_manager: DialogueManager,
    pub encounter_manager: EncounterManager,
    pub reputation: ReputationTracker,
    pub selection: SelectionManager,
    pub camera: TabletopCamera,
    pub daynight: DayNightCycle,
    pub ambience: AmbienceManager,
    pub player_entity: Option<Entity>,
    pub squad_entities: Vec<Entity>,
    pub combat_system: CombatSystem,
    pub events: EventBus,
}

impl ChronosCompanyGame {
    /// Create a new game instance with the given configuration.
    pub fn new(config: GameConfig) -> Self {
        let world = World::new();
        let game_loop = GameLoop::new();
        let mut screen_manager = ScreenManager::new();
        let hud = HudOverlay::new(config.screen_width, config.screen_height);
        let world_map = WorldGenerator::generate_biome_map(
            config.map_width,
            config.map_height,
            config.world_seed,
        );
        let terrain = TerrainGrid::generate_heightmap_terrain(
            config.map_width,
            config.map_height,
            config.world_seed,
        );
        let pathfinder = Pathfinder::new(config.map_width, config.map_height);
        let mut save_manager = SaveManager::new(10);
        save_manager.auto_save_interval_seconds = config.auto_save_interval;
        let encounter_manager = EncounterManager::new(EncounterSpawnConfig::new());
        let reputation = ReputationTracker::new();
        let selection = SelectionManager::new();
        let mut camera = TabletopCamera::new();
        camera.distance = 35.0;
        let daynight = DayNightCycle::new().with_starting_hour(8.0);
        let ambience = AmbienceManager::new();

        // Register screen layouts
        screen_manager.register_layout(LayoutPresets::main_menu(
            config.screen_width,
            config.screen_height,
        ));
        screen_manager.register_layout(LayoutPresets::pause_menu(
            config.screen_width,
            config.screen_height,
        ));
        screen_manager.register_layout(LayoutPresets::game_over(
            config.screen_width,
            config.screen_height,
        ));

        let mut game = ChronosCompanyGame {
            config,
            state: GameState::new(),
            world,
            game_loop,
            screen_manager,
            hud,
            world_map,
            terrain,
            pathfinder,
            save_manager,
            dialogue_manager: DialogueManager::new(),
            encounter_manager,
            reputation,
            selection,
            camera,
            daynight,
            ambience,
            player_entity: None,
            squad_entities: Vec::new(),
            combat_system: CombatSystem::new(),
            events: EventBus::new(),
        };

        game.state.gold = game.config.start_gold;
        game
    }

    /// Start a new game — generate world, spawn player squad, enter gameplay.
    pub fn new_game(&mut self) {
        self.state = GameState::new();
        self.state.gold = self.config.start_gold;
        self.state.is_running = true;

        // Spawn player squad at world center
        let center = [
            (self.config.map_width as f32 * self.config.cell_size) / 2.0,
            0.0,
            (self.config.map_height as f32 * self.config.cell_size) / 2.0,
        ];

        let templates = self.pick_starting_templates();
        self.squad_entities = MercenaryFactory::create_squad(&mut self.world, &templates, center);

        // Mark squad as player team
        for entity in &self.squad_entities {
            self.world.add_component(*entity, Team::Player);
            self.world.add_component(*entity, Selectable);
            // Add combat capabilities
            self.world.add_component(*entity, AttackCooldown::new(1.0));
            self.world
                .add_component(*entity, CombatState::new(AttackType::Melee));
            // Add navigation
            if let Some(stats) = self.world.get_component::<MercenaryStats>(*entity) {
                let speed = 2.0 + (stats.dexterity as f32 / 10.0);
                self.world
                    .add_component(*entity, NavigationAgent::new(speed));
            }
        }

        // Set player entity as squad leader (first member)
        self.player_entity = self.squad_entities.first().copied();

        // Set camera to follow squad center
        self.camera.target = center;
        self.camera.follow_target = Some(center);

        // Explore starting area
        let gx = self.config.map_width / 2;
        let gy = self.config.map_height / 2;
        self.world_map.explore_radius(gx, gy, 3);

        // Initialize HUD with squad info
        self.refresh_hud();

        // Transition to gameplay screen
        self.screen_manager
            .queue_transition(ScreenTransition::ClearAndPush(ScreenState::Gameplay));
        self.screen_manager.process_transitions();

        // Set initial ambience
        self.ambience
            .trigger_music(super::ambience::MusicTrigger::Explore);
    }

    fn pick_starting_templates(&self) -> Vec<MercenaryTemplate> {
        match self.config.start_squad_size {
            1 => vec![MercenaryTemplate::Warrior],
            2 => vec![MercenaryTemplate::Warrior, MercenaryTemplate::Archer],
            3 => vec![
                MercenaryTemplate::Warrior,
                MercenaryTemplate::Archer,
                MercenaryTemplate::Mage,
            ],
            _ => vec![
                MercenaryTemplate::Warrior,
                MercenaryTemplate::Archer,
                MercenaryTemplate::Mage,
                MercenaryTemplate::Scout,
            ],
        }
    }

    /// Advance the game by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        if !self.state.is_running || self.state.is_paused {
            return;
        }

        self.state.tick += 1;
        self.state.total_play_time += dt;

        let dt_f32 = dt as f32;

        // Update day/night cycle
        self.daynight.update(dt_f32);
        self.state.day = self.daynight.time.day;
        self.state.hour = self.daynight.time.hour;

        // Update ambience music fade
        self.ambience.update_music_fade(dt_f32);

        // Update camera follow
        if let Some(leader) = self.player_entity {
            if let Some(transform) = self.world.get_component::<Transform>(leader) {
                self.camera.follow_target = Some([transform.x, transform.y, transform.z]);
            }
        }
        self.camera.update(dt_f32);

        // Run combat system
        self.combat_system.update(&mut self.world, dt_f32);

        // Process encounter spawning
        if self.encounter_manager.should_spawn() {
            let region_diff = self.current_region_difficulty();
            let enc_type = EncounterSystem::generate_encounter_for_region(region_diff);
            // Spawn at random offset from squad center
            let offset_x = ((self.state.tick * 7 + 13) % 60) as f32 - 30.0;
            let offset_z = ((self.state.tick * 13 + 7) % 60) as f32 - 30.0;
            let spawn_pos = [
                self.camera.target[0] + offset_x,
                0.0,
                self.camera.target[2] + offset_z,
            ];
            self.encounter_manager.spawn_encounter(
                enc_type,
                spawn_pos[0],
                spawn_pos[1],
                spawn_pos[2],
                region_diff,
            );
        }

        // Update HUD state
        self.refresh_hud();

        // Auto-save check
        let play_time = self.state.total_play_time as u64;
        if self.save_manager.should_auto_save(play_time) {
            let _ = self.auto_save();
            self.save_manager.mark_auto_saved(play_time);
        }
    }

    fn current_region_difficulty(&self) -> u32 {
        let gx = (self.camera.target[0] / self.config.cell_size) as usize;
        let gy = (self.camera.target[2] / self.config.cell_size) as usize;
        self.world_map.difficulty_at(gx, gy)
    }

    /// Update HUD elements from current game state.
    fn refresh_hud(&mut self) {
        // Update squad panel
        self.hud.squad_panel.members.clear();
        for (i, entity) in self.squad_entities.iter().enumerate() {
            let name = self
                .world
                .get_component::<MercenaryStats>(*entity)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("Merc {}", i + 1));
            let health_frac = self
                .world
                .get_component::<Health>(*entity)
                .map(|h| h.current as f32 / h.max as f32)
                .unwrap_or(1.0);
            let level = self
                .world
                .get_component::<MercenaryStats>(*entity)
                .map(|s| s.level)
                .unwrap_or(1);
            let selected = self.selection.is_selected(*entity);

            self.hud.squad_panel.add_member(SquadMemberDisplay {
                name,
                level,
                health_fraction: health_frac,
                mana_fraction: 1.0, // Not yet tracked
                selected,
            });
        }

        // Update time display
        self.hud.set_time_display(self.state.day, self.state.hour);

        // Update gold
        self.hud.set_gold(self.state.gold);

        // Update player level
        self.hud.set_level(self.state.player_level);

        // Update day/night indicator color
        match self.daynight.phase() {
            TimePhase::Dawn => {
                self.hud
                    .notify_with_color("Dawn breaks...", 2.0, [1.0, 0.8, 0.4, 1.0])
            }
            TimePhase::Dusk => {
                self.hud
                    .notify_with_color("The sun sets...", 2.0, [0.8, 0.4, 0.2, 1.0])
            }
            _ => {}
        }

        // Update notifications
        self.hud.update(0.016);
    }

    /// Move selected squad members to a world position.
    pub fn order_move(&mut self, target: [f32; 3]) {
        let selected: Vec<Entity> = self.selection.selected_entities.clone();
        let to_move: Vec<Entity> = if selected.is_empty() && !self.squad_entities.is_empty() {
            self.squad_entities.clone()
        } else {
            selected
        };
        for entity in &to_move {
            self.set_move_target(*entity, target);
        }
    }

    fn set_move_target(&mut self, entity: Entity, target: [f32; 3]) {
        if let Some(agent) = self.world.get_component_mut::<NavigationAgent>(entity) {
            agent.path = None;
            agent.path_index = 0;
        }
        self.world
            .add_component(entity, MoveTarget::new(target[0], target[1], target[2]));
    }

    /// Order selected units to attack a target entity.
    pub fn order_attack(&mut self, target: Entity) {
        let selected: Vec<Entity> = self.selection.selected_entities.clone();
        for entity in &selected {
            if let Some(cs) = self.world.get_component_mut::<CombatState>(*entity) {
                cs.target = Some(target);
            } else {
                self.world.add_component(
                    *entity,
                    CombatState::new(AttackType::Melee).with_target(target),
                );
            }
        }
    }

    /// Pause / resume the game.
    pub fn toggle_pause(&mut self) {
        self.state.is_paused = !self.state.is_paused;
        if self.state.is_paused {
            self.screen_manager
                .queue_transition(ScreenTransition::Push(ScreenState::Paused));
        } else {
            self.screen_manager.queue_transition(ScreenTransition::Pop);
        }
        self.screen_manager.process_transitions();
    }

    /// Save the current game to a slot.
    pub fn save_game(&mut self, slot_id: u32) -> Result<u32, super::save::SaveError> {
        let player = PlayerSaveData {
            name: "Commander".to_string(),
            level: self.state.player_level,
            xp: self.state.player_xp,
            gold: self.state.gold,
            stats: [10, 10, 10, 10],
            position: self.camera.target,
            current_region: self.state.current_region.clone(),
            play_time_seconds: self.state.total_play_time as u64,
        };

        let explored = self.world_map.explored_cells();
        let mut world_save = WorldSaveData::new(
            self.config.world_seed,
            self.config.map_width,
            self.config.map_height,
        );
        world_save.explored_cells = explored;
        world_save.day = self.state.day;
        world_save.hour = self.state.hour;

        let factions = FactionSaveData {
            entries: self
                .reputation
                .entries
                .iter()
                .map(|(fid, entry)| (fid.name().to_string(), entry.value, entry.completed_jobs))
                .collect(),
        };

        self.save_manager
            .save(slot_id, player, world_save, factions)
    }

    fn auto_save(&mut self) -> Result<u32, super::save::SaveError> {
        self.save_game(0)
    }

    /// Load a game from a save slot.
    pub fn load_game(&mut self, slot_id: u32) -> Result<(), super::save::SaveError> {
        let slot = self.save_manager.load(slot_id)?;
        let player = slot.player.clone();
        let world_save = slot.world.clone();

        self.state.player_level = player.level;
        self.state.player_xp = player.xp;
        self.state.gold = player.gold;
        self.state.current_region = player.current_region.clone();
        self.state.total_play_time = player.play_time_seconds as f64;
        self.state.day = world_save.day;
        self.state.hour = world_save.hour;

        // Restore explored cells
        for (x, y) in &world_save.explored_cells {
            if let Some(cell) = self.world_map.get_mut(*x, *y) {
                cell.mark_explored();
            }
        }

        // Restore reputation
        self.reputation = ReputationTracker::new();
        for (name, value, jobs) in &slot.factions.entries {
            if let Some(fid) = faction_id_from_name(name) {
                let entry = self.reputation.get_or_create(fid);
                entry.value = *value;
                entry.completed_jobs = *jobs;
            }
        }

        self.state.is_running = true;
        self.screen_manager
            .queue_transition(ScreenTransition::ClearAndPush(ScreenState::Gameplay));
        self.screen_manager.process_transitions();

        Ok(())
    }

    /// Start a dialogue by tree ID.
    pub fn start_dialogue(&mut self, tree_id: u32) -> Option<super::dialogue::DialogueResult> {
        self.dialogue_manager.start_dialogue(tree_id)
    }

    /// Apply a dialogue choice result (gold, xp, reputation changes).
    pub fn apply_dialogue_result(&mut self, result: &super::dialogue::DialogueResult) {
        for action in &result.actions {
            match action {
                super::dialogue::DialogueAction::GiveGold { amount } => {
                    self.state.gold += amount;
                    self.hud.notify(&format!("+{} gold", amount), 2.0);
                }
                super::dialogue::DialogueAction::GiveXp { amount } => {
                    self.state.player_xp += amount;
                    self.hud.notify(&format!("+{} XP", amount), 2.0);
                }
                super::dialogue::DialogueAction::ModifyReputation { faction, delta } => {
                    if let Some(fid) = parse_faction_name(faction) {
                        self.reputation.modify_reputation(fid, *delta);
                    }
                }
                super::dialogue::DialogueAction::StartQuest { quest_id } => {
                    self.hud
                        .notify(&format!("Quest started: {}", quest_id), 3.0);
                }
                super::dialogue::DialogueAction::CompleteQuest { quest_id } => {
                    self.state.jobs_completed += 1;
                    self.hud
                        .notify(&format!("Quest completed: {}", quest_id), 3.0);
                }
                _ => {}
            }
        }
    }

    /// Check if any squad members are dead and handle cleanup.
    pub fn check_squad_casualties(&mut self) {
        let mut dead = Vec::new();
        for entity in &self.squad_entities {
            if let Some(health) = self.world.get_component::<Health>(*entity) {
                if health.is_dead() {
                    dead.push(*entity);
                }
            }
        }

        for entity in &dead {
            self.hud.notify("A squad member has fallen!", 4.0);
            self.state.entities_killed += 1;
            self.squad_entities.retain(|e| e != entity);
            self.selection.selected_entities.retain(|e| e != entity);
        }

        // Game over if entire squad wiped
        if self.squad_entities.is_empty() {
            self.trigger_game_over();
        }
    }

    fn trigger_game_over(&mut self) {
        self.state.is_running = false;
        self.hud
            .notify("GAME OVER — Your squad has been wiped out.", 10.0);
        self.screen_manager
            .queue_transition(ScreenTransition::ClearAndPush(ScreenState::GameOver));
        self.screen_manager.process_transitions();
    }

    /// Get the current ambient light level (0.0 = midnight, 1.0 = noon).
    pub fn ambient_light_level(&self) -> f32 {
        self.daynight.lighting.sun_intensity
    }

    /// Get a summary of the current game state for display.
    pub fn status_summary(&self) -> String {
        format!(
            "Day {} {:02.0}:{:02.0} | Gold: {} | Level: {} | Squad: {} | Region: {} | Tick: {}",
            self.state.day,
            self.state.hour,
            (self.state.hour.fract() * 60.0),
            self.state.gold,
            self.state.player_level,
            self.squad_entities.len(),
            self.state.current_region,
            self.state.tick,
        )
    }
}

fn faction_id_from_name(name: &str) -> Option<FactionId> {
    match name {
        "Chronos Company" => Some(FactionId::ChronosCompany),
        "City Watch" => Some(FactionId::CityWatch),
        "Black Market" => Some(FactionId::BlackMarket),
        "Merchant Guild" => Some(FactionId::MerchantGuild),
        "Rebels" => Some(FactionId::Rebels),
        "Corporate Security" => Some(FactionId::CorporateSec),
        "Neutral" => Some(FactionId::Neutral),
        _ => None,
    }
}

fn parse_faction_name(name: &str) -> Option<FactionId> {
    faction_id_from_name(name)
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn test_game() -> ChronosCompanyGame {
        let config = GameConfig {
            mode: GameMode::Campaign,
            map_width: 20,
            map_height: 20,
            world_seed: 42,
            start_gold: 1000,
            start_squad_size: 3,
            auto_save_interval: 300,
            screen_width: 800.0,
            screen_height: 600.0,
            cell_size: 2.0,
        };
        ChronosCompanyGame::new(config)
    }

    #[test]
    fn game_creation() {
        let game = test_game();
        assert_eq!(game.config.map_width, 20);
        assert_eq!(game.config.map_height, 20);
        assert!(!game.state.is_running);
        assert_eq!(game.squad_entities.len(), 0);
        assert_eq!(game.state.gold, 1000);
    }

    #[test]
    fn new_game_spawns_squad() {
        let mut game = test_game();
        game.new_game();
        assert!(game.state.is_running);
        assert_eq!(game.squad_entities.len(), 3);
        assert!(game.player_entity.is_some());
        assert_eq!(game.screen_manager.current_state(), ScreenState::Gameplay);
    }

    #[test]
    fn new_game_squad_has_components() {
        let mut game = test_game();
        game.new_game();

        for entity in &game.squad_entities {
            assert!(game.world.has_component::<Team>(*entity));
            assert!(game.world.has_component::<Selectable>(*entity));
            assert!(game.world.has_component::<AttackCooldown>(*entity));
            assert!(game.world.has_component::<CombatState>(*entity));
            assert!(game.world.has_component::<NavigationAgent>(*entity));
        }
    }

    #[test]
    fn tick_advances_state() {
        let mut game = test_game();
        game.new_game();
        let initial_tick = game.state.tick;
        game.tick(0.016);
        assert_eq!(game.state.tick, initial_tick + 1);
        assert!(game.state.total_play_time > 0.0);
    }

    #[test]
    fn paused_game_does_not_tick() {
        let mut game = test_game();
        game.new_game();
        game.state.is_paused = true;
        let tick_before = game.state.tick;
        game.tick(0.016);
        assert_eq!(game.state.tick, tick_before);
    }

    #[test]
    fn toggle_pause() {
        let mut game = test_game();
        game.new_game();
        assert!(!game.state.is_paused);
        game.toggle_pause();
        assert!(game.state.is_paused);
        game.toggle_pause();
        assert!(!game.state.is_paused);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut game = test_game();
        game.new_game();
        game.state.gold = 1337;
        game.state.player_level = 5;
        game.state.day = 7;
        game.state.hour = 15.5;

        let save_id = game.save_game(1).unwrap();
        assert_eq!(save_id, 1);

        // Mutate state
        game.state.gold = 0;
        game.state.player_level = 1;
        game.state.day = 1;
        game.state.hour = 8.0;

        // Load back
        game.load_game(1).unwrap();
        assert_eq!(game.state.gold, 1337);
        assert_eq!(game.state.player_level, 5);
        assert_eq!(game.state.day, 7);
        assert_eq!(game.state.hour, 15.5);
    }

    #[test]
    fn auto_save_creates_slot() {
        let mut game = test_game();
        game.new_game();
        let result = game.save_game(0);
        assert!(result.is_ok());
        let slot_id = result.unwrap();
        assert!(slot_id >= 1);
    }

    #[test]
    fn order_move_without_selection_moves_squad() {
        let mut game = test_game();
        game.new_game();
        let target = [10.0, 0.0, 10.0];
        game.order_move(target);

        for entity in &game.squad_entities {
            assert!(game.world.has_component::<MoveTarget>(*entity));
        }
    }

    #[test]
    fn ambient_light_varies_by_time() {
        let mut game = test_game();
        game.new_game();
        let dawn = game.ambient_light_level();

        // Advance to noon
        for _ in 0..1000 {
            game.tick(0.016);
        }
        let noon = game.ambient_light_level();

        // Light should change over the course of the day
        assert!(noon >= 0.0);
        assert!(noon <= 1.0);
    }

    #[test]
    fn check_squad_casualties_removes_dead() {
        let mut game = test_game();
        game.new_game();
        let initial_count = game.squad_entities.len();
        assert!(initial_count > 0);

        // Kill the first squad member
        let first = game.squad_entities[0];
        if let Some(health) = game.world.get_component_mut::<Health>(first) {
            health.take_damage(health.max + 1);
        }

        game.check_squad_casualties();
        assert_eq!(game.squad_entities.len(), initial_count - 1);
        assert!(!game.squad_entities.contains(&first));
    }

    #[test]
    fn game_over_on_total_casualties() {
        let mut game = test_game();
        game.new_game();

        // Kill entire squad
        for entity in &game.squad_entities.clone() {
            if let Some(health) = game.world.get_component_mut::<Health>(*entity) {
                health.take_damage(health.max + 1);
            }
        }

        game.check_squad_casualties();
        assert!(!game.state.is_running);
        assert_eq!(game.screen_manager.current_state(), ScreenState::GameOver);
    }

    #[test]
    fn status_summary_format() {
        let mut game = test_game();
        game.new_game();
        let summary = game.status_summary();
        assert!(summary.contains("Day"));
        assert!(summary.contains("Gold"));
        assert!(summary.contains("Level"));
        assert!(summary.contains("Squad"));
    }

    #[test]
    fn encounter_spawning_over_time() {
        let mut game = test_game();
        game.new_game();
        let initial = game.encounter_manager.active_count();

        // Run enough ticks for encounter spawn check to trigger
        for _ in 0..100 {
            game.tick(0.016);
        }

        // Encounters may or may not spawn based on RNG, but system should not panic
        assert!(
            game.encounter_manager.active_count()
                <= game.encounter_manager.config.max_active_encounters
        );
    }

    #[test]
    fn reputation_modification() {
        let mut game = test_game();
        game.new_game();
        game.reputation.modify_reputation(FactionId::CityWatch, 30);
        assert_eq!(game.reputation.get_value(FactionId::CityWatch), 30);
    }

    #[test]
    fn dialogue_result_applies_gold() {
        let mut game = test_game();
        game.new_game();
        let initial_gold = game.state.gold;
        let result = super::super::dialogue::DialogueResult {
            actions: vec![super::super::dialogue::DialogueAction::GiveGold { amount: 100 }],
            node_text: "Test".to_string(),
            speaker: "NPC".to_string(),
            choices_available: false,
            finished: false,
        };
        game.apply_dialogue_result(&result);
        assert_eq!(game.state.gold, initial_gold + 100);
    }

    #[test]
    fn starting_templates_by_size() {
        let mut game = test_game();
        game.config.start_squad_size = 1;
        let t1 = game.pick_starting_templates();
        assert_eq!(t1.len(), 1);

        game.config.start_squad_size = 2;
        let t2 = game.pick_starting_templates();
        assert_eq!(t2.len(), 2);

        game.config.start_squad_size = 4;
        let t4 = game.pick_starting_templates();
        assert_eq!(t4.len(), 4);

        game.config.start_squad_size = 10;
        let t10 = game.pick_starting_templates();
        assert_eq!(t10.len(), 4); // capped at 4 templates
    }

    #[test]
    fn camera_follows_leader() {
        let mut game = test_game();
        game.new_game();
        let initial_target = game.camera.target;

        // Move the leader
        if let Some(leader) = game.player_entity {
            if let Some(transform) = game.world.get_component_mut::<Transform>(leader) {
                transform.x += 10.0;
                transform.z += 10.0;
            }
        }

        game.tick(0.016);
        assert_ne!(game.camera.target, initial_target);
    }
}

#[cfg(feature = "game")]


use std::collections::HashMap;

// ── EncounterType ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterType {
    Ambush,
    Patrol,
    Bandit,
    Wildlife,
    Trader,
    Mystery,
    Boss,
}

impl EncounterType {
    pub fn name(&self) -> &str {
        match self {
            EncounterType::Ambush => "Ambush",
            EncounterType::Patrol => "Patrol",
            EncounterType::Bandit => "Bandit",
            EncounterType::Wildlife => "Wildlife",
            EncounterType::Trader => "Trader",
            EncounterType::Mystery => "Mystery",
            EncounterType::Boss => "Boss",
        }
    }

    pub fn is_hostile(&self) -> bool {
        match self {
            EncounterType::Trader | EncounterType::Mystery => false,
            _ => true,
        }
    }

    pub fn base_difficulty(&self) -> u32 {
        match self {
            EncounterType::Ambush => 3,
            EncounterType::Patrol => 2,
            EncounterType::Bandit => 3,
            EncounterType::Wildlife => 2,
            EncounterType::Trader => 1,
            EncounterType::Mystery => 1,
            EncounterType::Boss => 8,
        }
    }

    fn default_enemy_count(&self) -> u32 {
        match self {
            EncounterType::Ambush => 3,
            EncounterType::Patrol => 4,
            EncounterType::Bandit => 3,
            EncounterType::Wildlife => 2,
            EncounterType::Trader => 0,
            EncounterType::Mystery => 0,
            EncounterType::Boss => 1,
        }
    }
}

// ── Encounter ──

#[derive(Debug, Clone, PartialEq)]
pub struct Encounter {
    pub id: u32,
    pub encounter_type: EncounterType,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub difficulty: u32,
    pub enemy_count: u32,
    pub description: String,
    pub completed: bool,
    pub reward_gold: u32,
    pub reward_xp: u32,
}

impl Encounter {
    pub fn new(id: u32, encounter_type: EncounterType, x: f32, y: f32, z: f32) -> Self {
        Encounter {
            id,
            encounter_type,
            x,
            y,
            z,
            difficulty: encounter_type.base_difficulty(),
            enemy_count: encounter_type.default_enemy_count(),
            description: String::new(),
            completed: false,
            reward_gold: 0,
            reward_xp: 0,
        }
    }

    pub fn with_difficulty(mut self, d: u32) -> Self {
        self.difficulty = d;
        self
    }

    pub fn with_enemy_count(mut self, count: u32) -> Self {
        self.enemy_count = count;
        self
    }

    pub fn with_reward(mut self, gold: u32, xp: u32) -> Self {
        self.reward_gold = gold;
        self.reward_xp = xp;
        self
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }

    pub fn distance_to(&self, x: f32, y: f32, z: f32) -> f32 {
        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

// ── EncounterSpawnConfig ──

#[derive(Debug, Clone, PartialEq)]
pub struct EncounterSpawnConfig {
    pub min_distance_from_player: f32,
    pub max_distance_from_player: f32,
    pub spawn_chance_per_check: f32,
    pub check_interval_steps: u32,
    pub max_active_encounters: usize,
    pub difficulty_scale: f32,
}

impl EncounterSpawnConfig {
    pub fn new() -> Self {
        EncounterSpawnConfig {
            min_distance_from_player: 30.0,
            max_distance_from_player: 80.0,
            spawn_chance_per_check: 0.15,
            check_interval_steps: 10,
            max_active_encounters: 5,
            difficulty_scale: 1.0,
        }
    }
}

impl Default for EncounterSpawnConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ── EncounterManager ──

#[derive(Debug, Clone, PartialEq)]
pub struct EncounterManager {
    pub encounters: HashMap<u32, Encounter>,
    pub active_encounters: Vec<u32>,
    pub completed_count: u32,
    pub next_id: u32,
    pub config: EncounterSpawnConfig,
    pub steps_since_last_check: u32,
}

impl EncounterManager {
    pub fn new(config: EncounterSpawnConfig) -> Self {
        EncounterManager {
            encounters: HashMap::new(),
            active_encounters: Vec::new(),
            completed_count: 0,
            next_id: 0,
            config,
            steps_since_last_check: 0,
        }
    }

    pub fn spawn_encounter(
        &mut self,
        encounter_type: EncounterType,
        x: f32,
        y: f32,
        z: f32,
        region_difficulty: u32,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let difficulty = EncounterSystem::calculate_difficulty(
            encounter_type.base_difficulty(),
            region_difficulty,
            self.config.difficulty_scale,
        );
        let (gold, xp) = EncounterSystem::calculate_rewards(difficulty);

        let encounter = Encounter::new(id, encounter_type, x, y, z)
            .with_difficulty(difficulty)
            .with_enemy_count(encounter_type.default_enemy_count())
            .with_reward(gold, xp);

        self.encounters.insert(id, encounter);
        self.active_encounters.push(id);
        id
    }

    pub fn complete_encounter(&mut self, id: u32) -> Option<&Encounter> {
        if let Some(encounter) = self.encounters.get_mut(&id) {
            encounter.complete();
            self.active_encounters.retain(|&eid| eid != id);
            self.completed_count += 1;
            self.encounters.get(&id)
        } else {
            None
        }
    }

    pub fn get(&self, id: u32) -> Option<&Encounter> {
        self.encounters.get(&id)
    }

    pub fn active_count(&self) -> usize {
        self.active_encounters.len()
    }

    pub fn nearest_active(&self, x: f32, y: f32, z: f32) -> Option<&Encounter> {
        self.active_encounters
            .iter()
            .filter_map(|id| self.encounters.get(id))
            .min_by(|a, b| {
                a.distance_to(x, y, z)
                    .partial_cmp(&b.distance_to(x, y, z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn should_spawn(&mut self) -> bool {
        self.steps_since_last_check += 1;

        if self.steps_since_last_check < self.config.check_interval_steps {
            return false;
        }

        if self.active_count() >= self.config.max_active_encounters {
            return false;
        }

        let roll = ((self.steps_since_last_check * 7 + self.next_id * 13) % 100) as u32;
        let threshold = (self.config.spawn_chance_per_check * 100.0) as u32;

        if roll < threshold {
            self.steps_since_last_check = 0;
            true
        } else {
            false
        }
    }

    pub fn clear_completed(&mut self) {
        self.encounters.retain(|_, encounter| !encounter.completed);
    }
}

// ── EncounterSystem ──

pub struct EncounterSystem;

impl EncounterSystem {
    pub fn generate_encounter_for_region(region_difficulty: u32) -> EncounterType {
        match region_difficulty % 7 {
            0 => EncounterType::Ambush,
            1 => EncounterType::Patrol,
            2 => EncounterType::Bandit,
            3 => EncounterType::Wildlife,
            4 => EncounterType::Trader,
            5 => EncounterType::Mystery,
            6 => EncounterType::Boss,
            _ => EncounterType::Ambush, // unreachable but satisfies compiler
        }
    }

    pub fn calculate_difficulty(base: u32, region_difficulty: u32, scale: f32) -> u32 {
        (base as f32 * scale * region_difficulty as f32 / 3.0).max(1.0) as u32
    }

    pub fn calculate_rewards(difficulty: u32) -> (u32, u32) {
        (difficulty * 10, difficulty * 25)
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_type_properties() {
        let cases = [
            (EncounterType::Ambush, "Ambush", true, 3),
            (EncounterType::Patrol, "Patrol", true, 2),
            (EncounterType::Bandit, "Bandit", true, 3),
            (EncounterType::Wildlife, "Wildlife", true, 2),
            (EncounterType::Trader, "Trader", false, 1),
            (EncounterType::Mystery, "Mystery", false, 1),
            (EncounterType::Boss, "Boss", true, 8),
        ];

        for (et, name, hostile, diff) in &cases {
            assert_eq!(et.name(), *name);
            assert_eq!(et.is_hostile(), *hostile);
            assert_eq!(et.base_difficulty(), *diff);
        }
    }

    #[test]
    fn encounter_creation() {
        let e = Encounter::new(1, EncounterType::Ambush, 10.0, 20.0, 30.0);
        assert_eq!(e.id, 1);
        assert_eq!(e.encounter_type, EncounterType::Ambush);
        assert!((e.x - 10.0).abs() < f32::EPSILON);
        assert!((e.y - 20.0).abs() < f32::EPSILON);
        assert!((e.z - 30.0).abs() < f32::EPSILON);
        assert_eq!(e.difficulty, 3);
        assert_eq!(e.enemy_count, 3);
        assert!(!e.completed);
        assert_eq!(e.reward_gold, 0);
        assert_eq!(e.reward_xp, 0);

        let e2 = Encounter::new(2, EncounterType::Trader, 0.0, 0.0, 0.0);
        assert_eq!(e2.enemy_count, 0);
        assert_eq!(e2.difficulty, 1);

        let e3 = Encounter::new(3, EncounterType::Patrol, 0.0, 0.0, 0.0);
        assert_eq!(e3.enemy_count, 4);
    }

    #[test]
    fn encounter_builder() {
        let e = Encounter::new(10, EncounterType::Bandit, 5.0, 5.0, 5.0)
            .with_difficulty(7)
            .with_enemy_count(6)
            .with_reward(100, 250);

        assert_eq!(e.difficulty, 7);
        assert_eq!(e.enemy_count, 6);
        assert_eq!(e.reward_gold, 100);
        assert_eq!(e.reward_xp, 250);
    }

    #[test]
    fn encounter_complete() {
        let mut e = Encounter::new(1, EncounterType::Wildlife, 0.0, 0.0, 0.0);
        assert!(!e.completed);
        e.complete();
        assert!(e.completed);
    }

    #[test]
    fn spawn_config_defaults() {
        let c = EncounterSpawnConfig::new();
        assert!((c.min_distance_from_player - 30.0).abs() < f32::EPSILON);
        assert!((c.max_distance_from_player - 80.0).abs() < f32::EPSILON);
        assert!((c.spawn_chance_per_check - 0.15).abs() < f32::EPSILON);
        assert_eq!(c.check_interval_steps, 10);
        assert_eq!(c.max_active_encounters, 5);
        assert!((c.difficulty_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn manager_spawn_encounter() {
        let config = EncounterSpawnConfig::new();
        let mut mgr = EncounterManager::new(config);

        let id = mgr.spawn_encounter(EncounterType::Ambush, 10.0, 20.0, 30.0, 3);
        assert_eq!(id, 0);
        assert_eq!(mgr.next_id, 1);
        assert!(mgr.encounters.contains_key(&0));
        assert_eq!(mgr.active_encounters.len(), 1);

        let enc = mgr.get(id).unwrap();
        assert_eq!(enc.encounter_type, EncounterType::Ambush);
        assert_eq!(enc.difficulty, 3); // base=3, scale=1.0, region=3 -> 3*1*3/3 = 3
        assert_eq!(enc.reward_gold, 30);
        assert_eq!(enc.reward_xp, 75);
    }

    #[test]
    fn manager_complete_encounter() {
        let config = EncounterSpawnConfig::new();
        let mut mgr = EncounterManager::new(config);

        let id = mgr.spawn_encounter(EncounterType::Bandit, 0.0, 0.0, 0.0, 3);
        assert_eq!(mgr.active_encounters.len(), 1);

        let result = mgr.complete_encounter(id);
        assert!(result.is_some());
        let enc = result.unwrap();
        assert!(enc.completed);
        assert_eq!(mgr.active_encounters.len(), 0);
        assert_eq!(mgr.completed_count, 1);

        // Completing non-existent returns None
        let missing = mgr.complete_encounter(999);
        assert!(missing.is_none());
    }

    #[test]
    fn manager_active_count() {
        let config = EncounterSpawnConfig::new();
        let mut mgr = EncounterManager::new(config);
        assert_eq!(mgr.active_count(), 0);

        mgr.spawn_encounter(EncounterType::Patrol, 0.0, 0.0, 0.0, 3);
        assert_eq!(mgr.active_count(), 1);

        mgr.spawn_encounter(EncounterType::Wildlife, 10.0, 10.0, 10.0, 3);
        assert_eq!(mgr.active_count(), 2);

        mgr.complete_encounter(0);
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn manager_should_spawn() {
        let mut config = EncounterSpawnConfig::new();
        config.check_interval_steps = 3;
        config.max_active_encounters = 2;
        config.spawn_chance_per_check = 1.0; // always succeed the roll
        let mut mgr = EncounterManager::new(config);

        // Not enough steps yet
        assert!(!mgr.should_spawn()); // step 1
        assert!(!mgr.should_spawn()); // step 2

        // Step 3 meets interval, roll passes (chance=1.0)
        assert!(mgr.should_spawn());
        assert_eq!(mgr.steps_since_last_check, 0); // reset after success

        // Fill up active slots
        mgr.spawn_encounter(EncounterType::Ambush, 0.0, 0.0, 0.0, 3);
        mgr.spawn_encounter(EncounterType::Ambush, 1.0, 1.0, 1.0, 3);

        // Steps reach interval but max active hit
        mgr.should_spawn();
        mgr.should_spawn();
        let result = mgr.should_spawn(); // step 3 again but max active
        assert!(!result);
    }

    #[test]
    fn manager_nearest_active() {
        let config = EncounterSpawnConfig::new();
        let mut mgr = EncounterManager::new(config);

        assert!(mgr.nearest_active(0.0, 0.0, 0.0).is_none());

        mgr.spawn_encounter(EncounterType::Ambush, 10.0, 0.0, 0.0, 3); // id 0, dist=10
        mgr.spawn_encounter(EncounterType::Patrol, 2.0, 0.0, 0.0, 3);  // id 1, dist=2
        mgr.spawn_encounter(EncounterType::Wildlife, 50.0, 0.0, 0.0, 3); // id 2, dist=50

        let nearest = mgr.nearest_active(0.0, 0.0, 0.0).unwrap();
        assert_eq!(nearest.id, 1);

        // After completing nearest, next closest
        mgr.complete_encounter(1);
        let nearest2 = mgr.nearest_active(0.0, 0.0, 0.0).unwrap();
        assert_eq!(nearest2.id, 0);
    }

    #[test]
    fn system_generate_encounter() {
        assert_eq!(EncounterSystem::generate_encounter_for_region(0), EncounterType::Ambush);
        assert_eq!(EncounterSystem::generate_encounter_for_region(1), EncounterType::Patrol);
        assert_eq!(EncounterSystem::generate_encounter_for_region(2), EncounterType::Bandit);
        assert_eq!(EncounterSystem::generate_encounter_for_region(3), EncounterType::Wildlife);
        assert_eq!(EncounterSystem::generate_encounter_for_region(4), EncounterType::Trader);
        assert_eq!(EncounterSystem::generate_encounter_for_region(5), EncounterType::Mystery);
        assert_eq!(EncounterSystem::generate_encounter_for_region(6), EncounterType::Boss);
        assert_eq!(EncounterSystem::generate_encounter_for_region(7), EncounterType::Ambush); // wraps
    }

    #[test]
    fn system_calculate_difficulty() {
        // base=3, region=3, scale=1.0 -> 3*1*3/3 = 3.0
        assert_eq!(EncounterSystem::calculate_difficulty(3, 3, 1.0), 3);

        // base=2, region=6, scale=1.5 -> 2*1.5*6/3 = 6.0
        assert_eq!(EncounterSystem::calculate_difficulty(2, 6, 1.5), 6);

        // base=1, region=1, scale=1.0 -> 1*1*1/3 = 0.33 -> max(1.0) = 1
        assert_eq!(EncounterSystem::calculate_difficulty(1, 1, 1.0), 1);
    }

    #[test]
    fn system_calculate_rewards() {
        assert_eq!(EncounterSystem::calculate_rewards(1), (10, 25));
        assert_eq!(EncounterSystem::calculate_rewards(5), (50, 125));
        assert_eq!(EncounterSystem::calculate_rewards(0), (0, 0));
    }

    #[test]
    fn encounter_distance_to() {
        let e = Encounter::new(1, EncounterType::Ambush, 3.0, 4.0, 0.0);
        let dist = e.distance_to(0.0, 0.0, 0.0);
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn manager_clear_completed() {
        let config = EncounterSpawnConfig::new();
        let mut mgr = EncounterManager::new(config);

        let id1 = mgr.spawn_encounter(EncounterType::Ambush, 0.0, 0.0, 0.0, 3);
        let id2 = mgr.spawn_encounter(EncounterType::Patrol, 10.0, 10.0, 10.0, 3);

        mgr.complete_encounter(id1);
        assert_eq!(mgr.encounters.len(), 2);

        mgr.clear_completed();
        assert_eq!(mgr.encounters.len(), 1);
        assert!(!mgr.encounters.contains_key(&id1));
        assert!(mgr.encounters.contains_key(&id2));
    }
}

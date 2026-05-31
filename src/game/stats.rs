use super::components::MercenaryStats;
#[cfg(feature = "game")]
use crate::{Entity, World};

// ──────────────────────────────────────────────
// StatType enum
// ──────────────────────────────────────────────

/// The four primary stats used for mercenary RPG progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatType {
    Strength,
    Dexterity,
    Intelligence,
    Vitality,
}

// ──────────────────────────────────────────────
// StatBonuses
// ──────────────────────────────────────────────

/// A bundle of additive stat modifiers.
#[derive(Debug, Clone, Copy)]
pub struct StatBonuses {
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub vitality: i32,
}

impl StatBonuses {
    pub fn new(strength: i32, dexterity: i32, intelligence: i32, vitality: i32) -> Self {
        StatBonuses {
            strength,
            dexterity,
            intelligence,
            vitality,
        }
    }

    pub fn zero() -> Self {
        StatBonuses {
            strength: 0,
            dexterity: 0,
            intelligence: 0,
            vitality: 0,
        }
    }

    pub fn add(&self, other: &StatBonuses) -> StatBonuses {
        StatBonuses {
            strength: self.strength + other.strength,
            dexterity: self.dexterity + other.dexterity,
            intelligence: self.intelligence + other.intelligence,
            vitality: self.vitality + other.vitality,
        }
    }

    pub fn get(&self, stat: StatType) -> i32 {
        match stat {
            StatType::Strength => self.strength,
            StatType::Dexterity => self.dexterity,
            StatType::Intelligence => self.intelligence,
            StatType::Vitality => self.vitality,
        }
    }
}

// ──────────────────────────────────────────────
// LevelUpTable
// ──────────────────────────────────────────────

/// Precomputed XP thresholds for each level (1..=50).
///
/// Level N requires `100 * N^1.5` total XP. The table stores the cumulative
/// XP needed to *reach* each level so that look-ups are O(1).
pub struct LevelUpTable {
    xp_table: Vec<u32>,
}

impl Default for LevelUpTable {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelUpTable {
    const MAX_LEVEL: u32 = 50;
    const BASE_XP: u32 = 100;

    /// Build the XP table for levels 1 through 50.
    pub fn new() -> Self {
        let mut xp_table = Vec::with_capacity(Self::MAX_LEVEL as usize + 1);
        // Index 0 is unused (level 0 doesn't exist). We keep it at 0.
        xp_table.push(0);
        for level in 1..=Self::MAX_LEVEL {
            let xp_needed = (Self::BASE_XP as f64).mul_add((level as f64).powf(1.5), 0.0) as u32;
            xp_table.push(xp_needed);
        }
        LevelUpTable { xp_table }
    }

    /// Cumulative XP required to reach `level`.
    ///
    /// Clamps to the table bounds — requesting level 0 returns 0 and
    /// requesting anything above the cap returns the cap's threshold.
    pub fn xp_for_level(&self, level: u32) -> u32 {
        if level == 0 {
            return 0;
        }
        let idx = level.min(Self::MAX_LEVEL) as usize;
        self.xp_table[idx]
    }

    /// The highest level achievable with `xp` total experience.
    pub fn level_for_xp(&self, xp: u32) -> u32 {
        for level in 1..=Self::MAX_LEVEL {
            if xp < self.xp_table[level as usize] {
                return level - 1;
            }
        }
        Self::MAX_LEVEL
    }

    /// The hard level cap.
    pub fn max_level(&self) -> u32 {
        Self::MAX_LEVEL
    }
}

// ──────────────────────────────────────────────
// StatGrowth
// ──────────────────────────────────────────────

/// Defines per-level stat growth rates for a class archetype.
#[derive(Debug, Clone, Copy)]
pub struct StatGrowth {
    pub strength_growth: f32,
    pub dexterity_growth: f32,
    pub intelligence_growth: f32,
    pub vitality_growth: f32,
}

impl StatGrowth {
    pub fn new(str: f32, dex: f32, int: f32, vit: f32) -> Self {
        StatGrowth {
            strength_growth: str,
            dexterity_growth: dex,
            intelligence_growth: int,
            vitality_growth: vit,
        }
    }

    /// Warrior: high strength, moderate vitality.
    pub fn warrior() -> Self {
        StatGrowth::new(2.0, 1.0, 0.5, 1.5)
    }

    /// Archer: high dexterity.
    pub fn archer() -> Self {
        StatGrowth::new(1.0, 2.0, 0.5, 1.0)
    }

    /// Mage: high intelligence.
    pub fn mage() -> Self {
        StatGrowth::new(0.5, 1.0, 2.5, 0.5)
    }

    /// Scout: balanced dexterity and strength.
    pub fn scout() -> Self {
        StatGrowth::new(1.5, 1.5, 1.0, 1.0)
    }

    /// Per-level growth for a specific stat.
    pub fn growth_for(&self, stat: StatType) -> f32 {
        match stat {
            StatType::Strength => self.strength_growth,
            StatType::Dexterity => self.dexterity_growth,
            StatType::Intelligence => self.intelligence_growth,
            StatType::Vitality => self.vitality_growth,
        }
    }

    /// Calculate final stats at `target_level` given a `base` stat block.
    ///
    /// Returns `(strength, dexterity, intelligence, vitality)`.
    pub fn stats_at_level(&self, base: &MercenaryStats, target_level: u32) -> (u32, u32, u32, u32) {
        let levels_gained = target_level.saturating_sub(base.level) as f32;
        let str = base.strength + (self.strength_growth * levels_gained).floor() as u32;
        let dex = base.dexterity + (self.dexterity_growth * levels_gained).floor() as u32;
        let int = base.intelligence + (self.intelligence_growth * levels_gained).floor() as u32;
        let vit = base.vitality + (self.vitality_growth * levels_gained).floor() as u32;
        (str, dex, int, vit)
    }
}

// ──────────────────────────────────────────────
// StatAllocator
// ──────────────────────────────────────────────

/// Manages a pool of spendable stat points across the four primary stats.
#[derive(Debug, Clone)]
pub struct StatAllocator {
    points_available: u32,
    allocations: [u32; 4], // STR, DEX, INT, VIT
}

impl StatAllocator {
    pub fn new(points: u32) -> Self {
        StatAllocator {
            points_available: points,
            allocations: [0; 4],
        }
    }

    /// Index into the allocations array for each stat type.
    fn index_for(stat: StatType) -> usize {
        match stat {
            StatType::Strength => 0,
            StatType::Dexterity => 1,
            StatType::Intelligence => 2,
            StatType::Vitality => 3,
        }
    }

    /// Spend points into a stat. Returns `false` if insufficient points.
    pub fn allocate(&mut self, stat: StatType, points: u32) -> bool {
        if points > self.points_available {
            return false;
        }
        let idx = Self::index_for(stat);
        self.allocations[idx] += points;
        self.points_available -= points;
        true
    }

    /// Refund previously allocated points.
    pub fn deallocate(&mut self, stat: StatType, points: u32) {
        let idx = Self::index_for(stat);
        let current = self.allocations[idx];
        let refunded = points.min(current);
        self.allocations[idx] -= refunded;
        self.points_available += refunded;
    }

    /// Clear all allocations and restore the full point pool.
    pub fn reset(&mut self) {
        let total = self.total_allocated();
        self.allocations = [0; 4];
        self.points_available += total;
    }

    /// Sum of all currently allocated points.
    pub fn total_allocated(&self) -> u32 {
        self.allocations.iter().copied().sum()
    }

    /// Points allocated into a specific stat.
    pub fn get_allocation(&self, stat: StatType) -> u32 {
        self.allocations[Self::index_for(stat)]
    }
}

// ──────────────────────────────────────────────
// LevelUpSystem
// ──────────────────────────────────────────────

/// Core system for XP processing, level-up detection, and stat growth.
pub struct LevelUpSystem {
    table: LevelUpTable,
}

impl Default for LevelUpSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelUpSystem {
    /// Default warrior growth profile used by `check_level_up`.
    const DEFAULT_GROWTH: StatGrowth = StatGrowth {
        strength_growth: 2.0,
        dexterity_growth: 1.0,
        intelligence_growth: 0.5,
        vitality_growth: 1.5,
    };

    pub fn new() -> Self {
        LevelUpSystem {
            table: LevelUpTable::new(),
        }
    }

    /// Check if `stats` has enough XP to level up and apply growth.
    ///
    /// Returns the number of levels gained. Mutates `stats.level` and
    /// increases STR / DEX / INT / VIT using the default warrior growth.
    pub fn check_level_up(&self, stats: &mut MercenaryStats) -> u32 {
        let new_level = self.table.level_for_xp(stats.xp);
        if new_level <= stats.level {
            return 0;
        }
        let levels_gained = new_level - stats.level;
        Self::apply_stat_growth(stats, levels_gained);
        levels_gained
    }

    /// Add XP to an entity's [`MercenaryStats`] and apply any level-ups.
    ///
    /// Returns the total number of levels gained (0 if no level-up).
    pub fn grant_xp(world: &mut World, entity: Entity, amount: u32) -> u32 {
        let table = LevelUpTable::new();
        let old_level = match world.get_component::<MercenaryStats>(entity) {
            Some(s) => s.level,
            None => return 0,
        };

        if let Some(stats) = world.get_component_mut::<MercenaryStats>(entity) {
            stats.xp = stats.xp.saturating_add(amount);
        }

        let new_level =
            table
                .max_level()
                .min(match world.get_component::<MercenaryStats>(entity) {
                    Some(s) => table.level_for_xp(s.xp),
                    None => old_level,
                });

        if new_level <= old_level {
            return 0;
        }

        let levels_gained = new_level - old_level;
        if let Some(stats) = world.get_component_mut::<MercenaryStats>(entity) {
            Self::apply_stat_growth(stats, levels_gained);
        }

        levels_gained
    }

    /// Increase stats by the default (warrior) growth per level, floored.
    pub fn apply_stat_growth(stats: &mut MercenaryStats, levels_gained: u32) {
        let lg = levels_gained as f32;
        stats.strength += (Self::DEFAULT_GROWTH.strength_growth * lg).floor() as u32;
        stats.dexterity += (Self::DEFAULT_GROWTH.dexterity_growth * lg).floor() as u32;
        stats.intelligence += (Self::DEFAULT_GROWTH.intelligence_growth * lg).floor() as u32;
        stats.vitality += (Self::DEFAULT_GROWTH.vitality_growth * lg).floor() as u32;
        stats.level += levels_gained;
    }

    /// HP formula: `50 + vitality * 10 + level * 5`.
    pub fn hp_for_level(level: u32, vitality: u32) -> u32 {
        50 + vitality * 10 + level * 5
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new()
    }

    fn make_stats(name: &str) -> MercenaryStats {
        MercenaryStats::new(name)
    }

    // ── XP table generation ─────────────────────

    #[test]
    fn xp_table_level_1_threshold() {
        let table = LevelUpTable::new();
        // Level 1: 100 * 1^1.5 = 100
        assert_eq!(table.xp_for_level(1), 100);
    }

    #[test]
    fn xp_table_level_2_threshold() {
        let table = LevelUpTable::new();
        // Level 2: 100 * 2^1.5 ≈ 282 (rounded)
        let xp2 = table.xp_for_level(2);
        assert!(xp2 > 200, "level 2 threshold should be > 200, got {xp2}");
        assert!(xp2 < 300, "level 2 threshold should be < 300, got {xp2}");
    }

    // ── level_for_xp accuracy ───────────────────

    #[test]
    fn level_for_xp_at_zero() {
        let table = LevelUpTable::new();
        assert_eq!(table.level_for_xp(0), 0);
    }

    #[test]
    fn level_for_xp_below_level_1_threshold() {
        let table = LevelUpTable::new();
        assert_eq!(table.level_for_xp(50), 0);
        assert_eq!(table.level_for_xp(99), 0);
    }

    #[test]
    fn level_for_xp_at_level_1_threshold() {
        let table = LevelUpTable::new();
        let xp1 = table.xp_for_level(1);
        assert_eq!(table.level_for_xp(xp1), 1);
    }

    #[test]
    fn level_for_xp_between_levels() {
        let table = LevelUpTable::new();
        let xp1 = table.xp_for_level(1);
        let xp2 = table.xp_for_level(2);
        // XP between level 1 and level 2 thresholds → level 1
        let mid = xp1 + (xp2 - xp1) / 2;
        assert_eq!(table.level_for_xp(mid), 1);
    }

    // ── Max level cap ───────────────────────────

    #[test]
    fn max_level_is_50() {
        let table = LevelUpTable::new();
        assert_eq!(table.max_level(), 50);
    }

    #[test]
    fn level_for_xp_capped_at_max() {
        let table = LevelUpTable::new();
        // Way more XP than needed → still capped at 50
        assert_eq!(table.level_for_xp(999_999_999), 50);
    }

    // ── StatGrowth presets produce different results

    #[test]
    fn growth_presets_differ() {
        let w = StatGrowth::warrior();
        let a = StatGrowth::archer();
        let m = StatGrowth::mage();
        let s = StatGrowth::scout();

        assert_ne!(w.strength_growth, a.strength_growth);
        assert_ne!(a.dexterity_growth, m.dexterity_growth);
        assert_ne!(m.intelligence_growth, s.intelligence_growth);
    }

    // ── stats_at_level calculation ──────────────

    #[test]
    fn stats_at_level_warrior_growth() {
        let base = make_stats("Test");
        let growth = StatGrowth::warrior();
        // From level 1 to 5 → 4 levels of growth.
        // STR: 10 + floor(2.0 * 4) = 18
        // DEX: 10 + floor(1.0 * 4) = 14
        // INT: 10 + floor(0.5 * 4) = 12
        // VIT: 10 + floor(1.5 * 4) = 16
        let (str, dex, int, vit) = growth.stats_at_level(&base, 5);
        assert_eq!(str, 18);
        assert_eq!(dex, 14);
        assert_eq!(int, 12);
        assert_eq!(vit, 16);
    }

    #[test]
    fn stats_at_level_mage_growth() {
        let base = make_stats("Mage");
        let growth = StatGrowth::mage();
        // From level 1 to 3 → 2 levels of growth.
        // STR: 10 + floor(0.5 * 2) = 11
        // DEX: 10 + floor(1.0 * 2) = 12
        // INT: 10 + floor(2.5 * 2) = 15
        // VIT: 10 + floor(0.5 * 2) = 11
        let (str, dex, int, vit) = growth.stats_at_level(&base, 3);
        assert_eq!(str, 11);
        assert_eq!(dex, 12);
        assert_eq!(int, 15);
        assert_eq!(vit, 11);
    }

    // ── StatAllocator ───────────────────────────

    #[test]
    fn allocator_allocate_and_deallocate() {
        let mut alloc = StatAllocator::new(10);
        assert!(alloc.allocate(StatType::Strength, 5));
        assert_eq!(alloc.get_allocation(StatType::Strength), 5);
        assert_eq!(alloc.points_available, 5);

        // Can't overspend
        assert!(!alloc.allocate(StatType::Dexterity, 6));
        assert_eq!(alloc.get_allocation(StatType::Dexterity), 0);

        // Deallocate some
        alloc.deallocate(StatType::Strength, 3);
        assert_eq!(alloc.get_allocation(StatType::Strength), 2);
        assert_eq!(alloc.points_available, 8);
    }

    #[test]
    fn allocator_reset_clears_all() {
        let mut alloc = StatAllocator::new(20);
        alloc.allocate(StatType::Strength, 5);
        alloc.allocate(StatType::Intelligence, 7);
        alloc.reset();
        assert_eq!(alloc.total_allocated(), 0);
        assert_eq!(alloc.points_available, 20);
    }

    // ── Level-up triggers when XP crosses threshold

    #[test]
    fn level_up_triggers_on_xp_threshold() {
        let table = LevelUpTable::new();
        let system = LevelUpSystem::new();
        let mut stats = make_stats("Hero");
        // Set XP exactly to level 2 threshold
        stats.xp = table.xp_for_level(2);
        let gained = system.check_level_up(&mut stats);
        assert_eq!(gained, 1);
        assert_eq!(stats.level, 2);
    }

    #[test]
    fn no_level_up_when_below_threshold() {
        let table = LevelUpTable::new();
        let system = LevelUpSystem::new();
        let mut stats = make_stats("Hero");
        stats.xp = table.xp_for_level(1) - 1;
        let gained = system.check_level_up(&mut stats);
        assert_eq!(gained, 0);
        assert_eq!(stats.level, 1);
    }

    // ── Multiple level-ups at once ──────────────

    #[test]
    fn multiple_level_ups_at_once() {
        let table = LevelUpTable::new();
        let system = LevelUpSystem::new();
        let mut stats = make_stats("Hero");
        // Grant enough XP for level 5
        stats.xp = table.xp_for_level(5);
        let gained = system.check_level_up(&mut stats);
        assert_eq!(gained, 4);
        assert_eq!(stats.level, 5);
        // Warrior growth: 4 levels → str +8, dex +4, int +2, vit +6
        assert_eq!(stats.strength, 18); // 10 + 8
        assert_eq!(stats.dexterity, 14); // 10 + 4
        assert_eq!(stats.intelligence, 12); // 10 + 2
        assert_eq!(stats.vitality, 16); // 10 + 6
    }

    // ── HP formula ──────────────────────────────

    #[test]
    fn hp_formula_values() {
        // Level 1, vitality 10: 50 + 10*10 + 1*5 = 155
        assert_eq!(LevelUpSystem::hp_for_level(1, 10), 155);
        // Level 10, vitality 20: 50 + 20*10 + 10*5 = 300
        assert_eq!(LevelUpSystem::hp_for_level(10, 20), 300);
        // Level 0, vitality 0: 50 + 0 + 0 = 50
        assert_eq!(LevelUpSystem::hp_for_level(0, 0), 50);
    }

    // ── grant_xp on entity in world ─────────────

    #[test]
    fn grant_xp_levels_up_entity() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, make_stats("Grinder"));

        let table = LevelUpTable::new();
        // Grant enough XP for level 3
        let xp_needed = table.xp_for_level(3);
        let gained = LevelUpSystem::grant_xp(&mut world, entity, xp_needed);

        assert_eq!(gained, 2);
        let stats = world
            .get_component::<MercenaryStats>(entity)
            .expect("stats");
        assert_eq!(stats.level, 3);
        assert_eq!(stats.xp, xp_needed);
    }

    #[test]
    fn grant_xp_no_level_up() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, make_stats("Peon"));

        let gained = LevelUpSystem::grant_xp(&mut world, entity, 10);
        assert_eq!(gained, 0);
        let stats = world
            .get_component::<MercenaryStats>(entity)
            .expect("stats");
        assert_eq!(stats.level, 1);
        assert_eq!(stats.xp, 10);
    }

    #[test]
    fn grant_xp_entity_without_stats() {
        let mut world = test_world();
        let entity = world.create_entity();
        let gained = LevelUpSystem::grant_xp(&mut world, entity, 500);
        assert_eq!(gained, 0);
    }

    // ── StatBonuses ─────────────────────────────

    #[test]
    fn stat_bonuses_add_and_get() {
        let a = StatBonuses::new(2, 3, 0, -1);
        let b = StatBonuses::new(1, -1, 4, 2);
        let sum = a.add(&b);
        assert_eq!(sum.get(StatType::Strength), 3);
        assert_eq!(sum.get(StatType::Dexterity), 2);
        assert_eq!(sum.get(StatType::Intelligence), 4);
        assert_eq!(sum.get(StatType::Vitality), 1);
    }

    #[test]
    fn stat_bonuses_zero() {
        let z = StatBonuses::zero();
        assert_eq!(z.strength, 0);
        assert_eq!(z.dexterity, 0);
        assert_eq!(z.intelligence, 0);
        assert_eq!(z.vitality, 0);
    }
}

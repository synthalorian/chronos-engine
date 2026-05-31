use super::components::MercenaryStats;
use super::loot::{InventoryItem, ItemRarity, ItemType};
#[cfg(feature = "game")]
use crate::{Entity, World};

// ── EquipSlot ────────────────────────────────────────────────────────

/// The seven equipment slots a mercenary can fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Weapon,
    Helmet,
    Chest,
    Legs,
    Boots,
    Ring,
    Amulet,
}

impl EquipSlot {
    /// Maps each slot variant to a stable array index (0–6).
    pub fn as_index(&self) -> usize {
        match self {
            EquipSlot::Weapon => 0,
            EquipSlot::Helmet => 1,
            EquipSlot::Chest => 2,
            EquipSlot::Legs => 3,
            EquipSlot::Boots => 4,
            EquipSlot::Ring => 5,
            EquipSlot::Amulet => 6,
        }
    }

    /// All seven variants in index order.
    pub fn all() -> [EquipSlot; 7] {
        [
            EquipSlot::Weapon,
            EquipSlot::Helmet,
            EquipSlot::Chest,
            EquipSlot::Legs,
            EquipSlot::Boots,
            EquipSlot::Ring,
            EquipSlot::Amulet,
        ]
    }
}

// ── StatBonus ────────────────────────────────────────────────────────

/// Stat modifiers granted by a piece of equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatBonus {
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
    pub vitality: i32,
}

impl Default for StatBonus {
    fn default() -> Self {
        Self::new()
    }
}

impl StatBonus {
    pub fn new() -> Self {
        StatBonus {
            strength: 0,
            dexterity: 0,
            intelligence: 0,
            vitality: 0,
        }
    }

    /// Convenience alias — identical to [`new`](Self::new).
    pub fn zero() -> Self {
        Self::new()
    }

    pub fn with_strength(mut self, v: i32) -> Self {
        self.strength = v;
        self
    }

    pub fn with_dexterity(mut self, v: i32) -> Self {
        self.dexterity = v;
        self
    }

    pub fn with_intelligence(mut self, v: i32) -> Self {
        self.intelligence = v;
        self
    }

    pub fn with_vitality(mut self, v: i32) -> Self {
        self.vitality = v;
        self
    }

    /// Sum of all four bonuses.
    pub fn total(&self) -> i32 {
        self.strength + self.dexterity + self.intelligence + self.vitality
    }
}

impl std::ops::Add for StatBonus {
    type Output = StatBonus;

    fn add(self, rhs: StatBonus) -> StatBonus {
        StatBonus {
            strength: self.strength + rhs.strength,
            dexterity: self.dexterity + rhs.dexterity,
            intelligence: self.intelligence + rhs.intelligence,
            vitality: self.vitality + rhs.vitality,
        }
    }
}

// ── EquipmentData ────────────────────────────────────────────────────

/// A piece of equippable gear bound to a specific slot.
#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentData {
    pub item: InventoryItem,
    pub slot: EquipSlot,
    pub bonuses: StatBonus,
    pub required_level: u32,
}

impl EquipmentData {
    /// Create gear from a base item and a target slot.
    pub fn new(item: InventoryItem, slot: EquipSlot) -> Self {
        EquipmentData {
            item,
            slot,
            bonuses: StatBonus::new(),
            required_level: 1,
        }
    }

    pub fn with_bonuses(mut self, bonuses: StatBonus) -> Self {
        self.bonuses = bonuses;
        self
    }

    pub fn with_required_level(mut self, level: u32) -> Self {
        self.required_level = level;
        self
    }

    /// Whether the given stats satisfy the level requirement.
    pub fn can_equip(&self, stats: &MercenaryStats) -> bool {
        stats.level >= self.required_level
    }

    // ── Preset factories ──

    pub fn iron_sword() -> Self {
        EquipmentData::new(
            InventoryItem::new()
                .with_name("Iron Sword")
                .with_item_type(ItemType::Weapon)
                .with_rarity(ItemRarity::Common)
                .with_gold_value(10),
            EquipSlot::Weapon,
        )
        .with_bonuses(StatBonus::new().with_strength(3))
        .with_required_level(1)
    }

    pub fn steel_sword() -> Self {
        EquipmentData::new(
            InventoryItem::new()
                .with_name("Steel Sword")
                .with_item_type(ItemType::Weapon)
                .with_rarity(ItemRarity::Uncommon)
                .with_gold_value(35),
            EquipSlot::Weapon,
        )
        .with_bonuses(StatBonus::new().with_strength(6).with_dexterity(1))
        .with_required_level(5)
    }

    pub fn leather_armor() -> Self {
        EquipmentData::new(
            InventoryItem::new()
                .with_name("Leather Armor")
                .with_item_type(ItemType::Armor)
                .with_rarity(ItemRarity::Common)
                .with_gold_value(15),
            EquipSlot::Chest,
        )
        .with_bonuses(StatBonus::new().with_vitality(2))
        .with_required_level(1)
    }

    pub fn chain_mail() -> Self {
        EquipmentData::new(
            InventoryItem::new()
                .with_name("Chain Mail")
                .with_item_type(ItemType::Armor)
                .with_rarity(ItemRarity::Uncommon)
                .with_gold_value(50),
            EquipSlot::Chest,
        )
        .with_bonuses(StatBonus::new().with_vitality(4).with_strength(1))
        .with_required_level(3)
    }

    pub fn apprentice_ring() -> Self {
        EquipmentData::new(
            InventoryItem::new()
                .with_name("Apprentice Ring")
                .with_item_type(ItemType::Armor)
                .with_rarity(ItemRarity::Uncommon)
                .with_gold_value(25),
            EquipSlot::Ring,
        )
        .with_bonuses(StatBonus::new().with_intelligence(3))
        .with_required_level(1)
    }
}

// ── EquipmentManager ─────────────────────────────────────────────────

/// Component that tracks which gear is equipped in each slot.
#[derive(Debug, Clone)]
pub struct EquipmentManager {
    pub equipped: [Option<EquipmentData>; 7],
}

impl Default for EquipmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EquipmentManager {
    pub fn new() -> Self {
        EquipmentManager {
            equipped: [None, None, None, None, None, None, None],
        }
    }

    /// Equip gear into its designated slot, returning any previously equipped item.
    pub fn equip(&mut self, data: EquipmentData) -> Option<EquipmentData> {
        let idx = data.slot.as_index();
        self.equipped[idx].replace(data)
    }

    /// Remove gear from a slot, returning it.
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<EquipmentData> {
        self.equipped[slot.as_index()].take()
    }

    /// Inspect the item in a slot.
    pub fn get(&self, slot: EquipSlot) -> Option<&EquipmentData> {
        self.equipped[slot.as_index()].as_ref()
    }

    /// Mutably inspect the item in a slot.
    pub fn get_mut(&mut self, slot: EquipSlot) -> Option<&mut EquipmentData> {
        self.equipped[slot.as_index()].as_mut()
    }

    /// Sum of all stat bonuses across every equipped piece.
    pub fn total_bonuses(&self) -> StatBonus {
        let mut total = StatBonus::zero();
        for gear in self.equipped.iter().flatten() {
            total = total + gear.bonuses;
        }
        total
    }

    /// Whether a slot has no item.
    pub fn is_slot_empty(&self, slot: EquipSlot) -> bool {
        self.equipped[slot.as_index()].is_none()
    }

    /// How many slots currently hold gear.
    pub fn equipped_count(&self) -> usize {
        self.equipped.iter().filter(|s| s.is_some()).count()
    }
}

// ── EquipResult ──────────────────────────────────────────────────────

/// Outcome of an equip / unequip operation through the system layer.
#[derive(Debug, Clone)]
pub enum EquipResult {
    Success { replaced: Option<EquipmentData> },
    LevelTooLow { required: u32, current: u32 },
    SlotOccupied,
    InvalidSlot,
}

// ── EffectiveStats ───────────────────────────────────────────────────

/// Base stats plus equipment bonuses, clamped to a minimum of 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveStats {
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub vitality: u32,
}

impl EffectiveStats {
    fn clamp_positive(v: u32, bonus: i32) -> u32 {
        1.max((v as i32 + bonus).max(1) as u32)
    }
}

// ── EquipmentSystem ──────────────────────────────────────────────────

/// High-level system that mediates between the ECS world and equipment state.
pub struct EquipmentSystem;

impl Default for EquipmentSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl EquipmentSystem {
    pub fn new() -> Self {
        EquipmentSystem
    }

    /// Attempt to equip a piece of gear onto an entity.
    ///
    /// - Adds an `EquipmentManager` component if the entity lacks one.
    /// - Enforces level requirements via `MercenaryStats`.
    /// - Returns the previously equipped item (if any) inside [`EquipResult::Success`].
    pub fn equip_item(world: &mut World, entity: Entity, equipment: EquipmentData) -> EquipResult {
        // Ensure the entity has an EquipmentManager
        if !world.has_component::<EquipmentManager>(entity) {
            world.add_component(entity, EquipmentManager::new());
        }

        // Level gate
        if let Some(stats) = world.get_component::<MercenaryStats>(entity) {
            if !equipment.can_equip(stats) {
                return EquipResult::LevelTooLow {
                    required: equipment.required_level,
                    current: stats.level,
                };
            }
        }

        let replaced = world
            .get_component_mut::<EquipmentManager>(entity)
            .map(|mgr| mgr.equip(equipment))
            .unwrap_or(None);

        EquipResult::Success { replaced }
    }

    /// Remove whatever is in `slot` and return it.
    pub fn unequip_slot(world: &mut World, entity: Entity, slot: EquipSlot) -> EquipResult {
        if !world.has_component::<EquipmentManager>(entity) {
            return EquipResult::InvalidSlot;
        }

        let removed = world
            .get_component_mut::<EquipmentManager>(entity)
            .and_then(|mgr| mgr.unequip(slot));

        EquipResult::Success { replaced: removed }
    }

    /// Compute effective stats: base + all equipment bonuses, clamped to >= 1.
    pub fn calculate_effective_stats(
        stats: &MercenaryStats,
        equipment: &EquipmentManager,
    ) -> EffectiveStats {
        let bonuses = equipment.total_bonuses();
        EffectiveStats {
            strength: EffectiveStats::clamp_positive(stats.strength, bonuses.strength),
            dexterity: EffectiveStats::clamp_positive(stats.dexterity, bonuses.dexterity),
            intelligence: EffectiveStats::clamp_positive(stats.intelligence, bonuses.intelligence),
            vitality: EffectiveStats::clamp_positive(stats.vitality, bonuses.vitality),
        }
    }

    /// For each slot, equip the candidate with the highest bonus total.
    ///
    /// Skips candidates that fail the entity's level requirement.
    pub fn auto_equip_best(world: &mut World, entity: Entity, candidates: &[EquipmentData]) {
        if candidates.is_empty() {
            return;
        }

        // Ensure equipment manager exists
        if !world.has_component::<EquipmentManager>(entity) {
            world.add_component(entity, EquipmentManager::new());
        }

        // Gather level once to avoid repeated borrows
        let level = world
            .get_component::<MercenaryStats>(entity)
            .map(|s| s.level)
            .unwrap_or(1);

        for slot in EquipSlot::all() {
            // Find the best candidate for this slot that meets level req
            let best = candidates
                .iter()
                .filter(|c| c.slot == slot && level >= c.required_level)
                .max_by_key(|c| c.bonuses.total())
                .cloned();

            if let Some(gear) = best {
                let _ = world
                    .get_component_mut::<EquipmentManager>(entity)
                    .map(|mgr| mgr.equip(gear));
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new()
    }

    fn lvl1_stats() -> MercenaryStats {
        MercenaryStats::new("Test").with_stats(10, 10, 10, 10)
    }

    fn lvl5_stats() -> MercenaryStats {
        let mut s = MercenaryStats::new("Veteran").with_stats(15, 12, 8, 14);
        s.level = 5;
        s
    }

    // ── 1. EquipSlot index mapping ──

    #[test]
    fn equip_slot_indices() {
        assert_eq!(EquipSlot::Weapon.as_index(), 0);
        assert_eq!(EquipSlot::Helmet.as_index(), 1);
        assert_eq!(EquipSlot::Chest.as_index(), 2);
        assert_eq!(EquipSlot::Legs.as_index(), 3);
        assert_eq!(EquipSlot::Boots.as_index(), 4);
        assert_eq!(EquipSlot::Ring.as_index(), 5);
        assert_eq!(EquipSlot::Amulet.as_index(), 6);
    }

    #[test]
    fn equip_slot_all_returns_seven() {
        let all = EquipSlot::all();
        assert_eq!(all.len(), 7);
        for (i, slot) in all.iter().enumerate() {
            assert_eq!(slot.as_index(), i);
        }
    }

    // ── 2. StatBonus addition and total ──

    #[test]
    fn stat_bonus_zero() {
        let b = StatBonus::zero();
        assert_eq!(b.total(), 0);
    }

    #[test]
    fn stat_bonus_builder_and_total() {
        let b = StatBonus::new()
            .with_strength(4)
            .with_dexterity(2)
            .with_intelligence(1)
            .with_vitality(3);
        assert_eq!(b.strength, 4);
        assert_eq!(b.dexterity, 2);
        assert_eq!(b.intelligence, 1);
        assert_eq!(b.vitality, 3);
        assert_eq!(b.total(), 10);
    }

    #[test]
    fn stat_bonus_addition() {
        let a = StatBonus::new().with_strength(3).with_vitality(2);
        let b = StatBonus::new().with_dexterity(4).with_intelligence(1);
        let sum = a + b;
        assert_eq!(sum.strength, 3);
        assert_eq!(sum.dexterity, 4);
        assert_eq!(sum.intelligence, 1);
        assert_eq!(sum.vitality, 2);
        assert_eq!(sum.total(), 10);
    }

    // ── 3. EquipmentData creation and level check ──

    #[test]
    fn equipment_data_creation_and_can_equip() {
        let sword = EquipmentData::iron_sword();
        assert_eq!(sword.item.name, "Iron Sword");
        assert_eq!(sword.slot, EquipSlot::Weapon);
        assert_eq!(sword.bonuses.strength, 3);
        assert_eq!(sword.required_level, 1);
        assert!(sword.can_equip(&lvl1_stats()));

        let steel = EquipmentData::steel_sword();
        assert_eq!(steel.required_level, 5);
        assert!(!steel.can_equip(&lvl1_stats()));
        assert!(steel.can_equip(&lvl5_stats()));
    }

    // ── 4. EquipmentManager equip/unequip cycle ──

    #[test]
    fn manager_equip_and_unequip() {
        let mut mgr = EquipmentManager::new();
        assert!(mgr.is_slot_empty(EquipSlot::Weapon));
        assert_eq!(mgr.equipped_count(), 0);

        let sword = EquipmentData::iron_sword();
        let old = mgr.equip(sword.clone());
        assert!(old.is_none());
        assert!(!mgr.is_slot_empty(EquipSlot::Weapon));
        assert_eq!(mgr.equipped_count(), 1);

        let retrieved = mgr.get(EquipSlot::Weapon).expect("should be equipped");
        assert_eq!(retrieved.item.name, "Iron Sword");

        let removed = mgr.unequip(EquipSlot::Weapon).expect("should unequip");
        assert_eq!(removed.item.name, "Iron Sword");
        assert!(mgr.is_slot_empty(EquipSlot::Weapon));
        assert_eq!(mgr.equipped_count(), 0);
    }

    // ── 5. Equipping replaces existing item ──

    #[test]
    fn equipping_replaces_existing_item() {
        let mut mgr = EquipmentManager::new();

        let iron = EquipmentData::iron_sword();
        let _ = mgr.equip(iron);

        let steel = EquipmentData::steel_sword();
        let replaced = mgr.equip(steel).expect("should return iron sword");
        assert_eq!(replaced.item.name, "Iron Sword");

        let current = mgr.get(EquipSlot::Weapon).expect("should have steel");
        assert_eq!(current.item.name, "Steel Sword");
        assert_eq!(mgr.equipped_count(), 1);
    }

    // ── 6. Total bonuses from multiple slots ──

    #[test]
    fn total_bonuses_multiple_slots() {
        let mut mgr = EquipmentManager::new();
        assert_eq!(mgr.total_bonuses().total(), 0);

        mgr.equip(EquipmentData::iron_sword()); // +3 STR
        mgr.equip(EquipmentData::leather_armor()); // +2 VIT
        mgr.equip(EquipmentData::apprentice_ring()); // +3 INT

        let bonuses = mgr.total_bonuses();
        assert_eq!(bonuses.strength, 3);
        assert_eq!(bonuses.vitality, 2);
        assert_eq!(bonuses.intelligence, 3);
        assert_eq!(bonuses.dexterity, 0);
        assert_eq!(bonuses.total(), 8);
    }

    // ── 7. Level requirement enforcement via system ──

    #[test]
    fn level_requirement_enforcement() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl1_stats());

        let steel = EquipmentData::steel_sword(); // req level 5
        let result = EquipmentSystem::equip_item(&mut world, entity, steel);
        match result {
            EquipResult::LevelTooLow { required, current } => {
                assert_eq!(required, 5);
                assert_eq!(current, 1);
            }
            other => panic!("expected LevelTooLow, got {:?}", other),
        }

        // Manager should still be empty
        let mgr = world
            .get_component::<EquipmentManager>(entity)
            .expect("mgr");
        assert!(mgr.is_slot_empty(EquipSlot::Weapon));
    }

    #[test]
    fn level_requirement_met_equips() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl5_stats());

        let steel = EquipmentData::steel_sword();
        let result = EquipmentSystem::equip_item(&mut world, entity, steel);
        assert!(matches!(result, EquipResult::Success { replaced: None }));

        let mgr = world
            .get_component::<EquipmentManager>(entity)
            .expect("mgr");
        assert!(!mgr.is_slot_empty(EquipSlot::Weapon));
    }

    // ── 8. Effective stats calculation ──

    #[test]
    fn effective_stats_base_plus_equipment() {
        let stats = lvl1_stats(); // 10/10/10/10
        let mut mgr = EquipmentManager::new();
        mgr.equip(EquipmentData::iron_sword()); // +3 STR
        mgr.equip(EquipmentData::leather_armor()); // +2 VIT
        mgr.equip(EquipmentData::apprentice_ring()); // +3 INT

        let eff = EquipmentSystem::calculate_effective_stats(&stats, &mgr);
        assert_eq!(eff.strength, 13); // 10 + 3
        assert_eq!(eff.dexterity, 10); // 10 + 0
        assert_eq!(eff.intelligence, 13); // 10 + 3
        assert_eq!(eff.vitality, 12); // 10 + 2
    }

    #[test]
    fn effective_stats_clamped_to_minimum_one() {
        let stats = MercenaryStats::new("Weak").with_stats(2, 2, 2, 2);
        let mut mgr = EquipmentManager::new();

        // Apply large negative bonuses (simulating cursed gear)
        let cursed = EquipmentData::new(
            InventoryItem::new().with_name("Cursed Blade"),
            EquipSlot::Weapon,
        )
        .with_bonuses(StatBonus::new().with_strength(-10).with_dexterity(-5));
        mgr.equip(cursed);

        let eff = EquipmentSystem::calculate_effective_stats(&stats, &mgr);
        assert_eq!(eff.strength, 1); // 2 - 10 → clamped to 1
        assert_eq!(eff.dexterity, 1); // 2 - 5 → clamped to 1
        assert_eq!(eff.intelligence, 2); // unchanged
        assert_eq!(eff.vitality, 2); // unchanged
    }

    // ── 9. Auto-equip picks best per slot ──

    #[test]
    fn auto_equip_best_per_slot() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl1_stats());

        let iron = EquipmentData::iron_sword(); // weapon, +3 STR, lvl 1
        let steel = EquipmentData::steel_sword(); // weapon, +6 STR +1 DEX, lvl 5 (too high)
        let leather = EquipmentData::leather_armor(); // chest, +2 VIT, lvl 1
        let chain = EquipmentData::chain_mail(); // chest, +4 VIT +1 STR, lvl 3 (too high)

        let candidates = vec![iron, steel, leather, chain];
        EquipmentSystem::auto_equip_best(&mut world, entity, &candidates);

        let mgr = world
            .get_component::<EquipmentManager>(entity)
            .expect("mgr");
        assert_eq!(mgr.equipped_count(), 2); // weapon + chest

        // Iron sword wins (only weapon that passes level check)
        let weapon = mgr.get(EquipSlot::Weapon).expect("weapon");
        assert_eq!(weapon.item.name, "Iron Sword");

        // Leather armor wins (only chest that passes level check)
        let chest = mgr.get(EquipSlot::Chest).expect("chest");
        assert_eq!(chest.item.name, "Leather Armor");
    }

    #[test]
    fn auto_equip_best_picks_highest_bonus() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl5_stats());

        let weak_ring = EquipmentData::new(
            InventoryItem::new().with_name("Copper Ring"),
            EquipSlot::Ring,
        )
        .with_bonuses(StatBonus::new().with_intelligence(1))
        .with_required_level(1);

        let strong_ring = EquipmentData::new(
            InventoryItem::new().with_name("Platinum Ring"),
            EquipSlot::Ring,
        )
        .with_bonuses(StatBonus::new().with_intelligence(5).with_vitality(2))
        .with_required_level(3);

        let candidates = vec![weak_ring, strong_ring];
        EquipmentSystem::auto_equip_best(&mut world, entity, &candidates);

        let mgr = world
            .get_component::<EquipmentManager>(entity)
            .expect("mgr");
        let ring = mgr.get(EquipSlot::Ring).expect("ring");
        assert_eq!(ring.item.name, "Platinum Ring");
    }

    // ── 10. Empty slot queries ──

    #[test]
    fn empty_slot_queries_on_fresh_manager() {
        let mgr = EquipmentManager::new();
        for slot in EquipSlot::all() {
            assert!(mgr.is_slot_empty(slot));
            assert!(mgr.get(slot).is_none());
        }
        assert_eq!(mgr.equipped_count(), 0);
    }

    #[test]
    fn get_mut_allows_modification() {
        let mut mgr = EquipmentManager::new();
        mgr.equip(EquipmentData::iron_sword());

        if let Some(gear) = mgr.get_mut(EquipSlot::Weapon) {
            gear.bonuses = gear.bonuses.with_strength(99);
        }

        assert_eq!(mgr.get(EquipSlot::Weapon).unwrap().bonuses.strength, 99);
    }

    // ── Equip through system — adds manager automatically ──

    #[test]
    fn system_adds_equipment_manager_if_missing() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl1_stats());

        assert!(!world.has_component::<EquipmentManager>(entity));

        let result = EquipmentSystem::equip_item(&mut world, entity, EquipmentData::iron_sword());
        assert!(matches!(result, EquipResult::Success { .. }));
        assert!(world.has_component::<EquipmentManager>(entity));
    }

    // ── Unequip through system ──

    #[test]
    fn system_unequip_returns_item() {
        let mut world = test_world();
        let entity = world.create_entity();
        world.add_component(entity, lvl1_stats());

        EquipmentSystem::equip_item(&mut world, entity, EquipmentData::iron_sword());

        let result = EquipmentSystem::unequip_slot(&mut world, entity, EquipSlot::Weapon);
        match result {
            EquipResult::Success { replaced } => {
                let item = replaced.expect("should have iron sword");
                assert_eq!(item.item.name, "Iron Sword");
            }
            _ => panic!("expected Success, got {:?}", result),
        }

        let mgr = world
            .get_component::<EquipmentManager>(entity)
            .expect("mgr");
        assert!(mgr.is_slot_empty(EquipSlot::Weapon));
    }

    // ── Preset factory coverage ──

    #[test]
    fn preset_factories() {
        let iron = EquipmentData::iron_sword();
        assert_eq!(iron.slot, EquipSlot::Weapon);
        assert_eq!(iron.bonuses.strength, 3);
        assert_eq!(iron.required_level, 1);

        let steel = EquipmentData::steel_sword();
        assert_eq!(steel.bonuses.strength, 6);
        assert_eq!(steel.bonuses.dexterity, 1);
        assert_eq!(steel.required_level, 5);

        let leather = EquipmentData::leather_armor();
        assert_eq!(leather.slot, EquipSlot::Chest);
        assert_eq!(leather.bonuses.vitality, 2);

        let chain = EquipmentData::chain_mail();
        assert_eq!(chain.bonuses.vitality, 4);
        assert_eq!(chain.bonuses.strength, 1);
        assert_eq!(chain.required_level, 3);

        let ring = EquipmentData::apprentice_ring();
        assert_eq!(ring.slot, EquipSlot::Ring);
        assert_eq!(ring.bonuses.intelligence, 3);
    }
}

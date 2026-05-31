use super::components::*;
use crate::component::{Health, Transform};
#[cfg(feature = "game")]
use crate::{Entity, World};

// ---------------------------------------------------------------------------
// Ability type classification
// ---------------------------------------------------------------------------

/// The elemental or mechanical category of an ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityType {
    MeleeStrike,
    RangedShot,
    Fireball,
    Heal,
    ShieldBash,
    PoisonArrow,
}

// ---------------------------------------------------------------------------
// Targeting
// ---------------------------------------------------------------------------

/// What an ability targets when activated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AbilityTarget {
    /// No target — instant / self-cast.
    None,
    /// Single enemy entity.
    SingleEnemy(Entity),
    /// Point-targeted AoE (position + radius).
    Aoe([f32; 3], f32),
    /// Ally entity (for heals / buffs).
    Ally(Entity),
}

// ---------------------------------------------------------------------------
// Ability definition
// ---------------------------------------------------------------------------

/// An individual ability with cooldown, range, damage, and mana cost.
#[derive(Debug, Clone)]
pub struct Ability {
    pub name: String,
    pub ability_type: AbilityType,
    pub cooldown: f32,
    pub range: f32,
    pub base_damage: u32,
    pub mana_cost: u32,
    pub remaining_cooldown: f32,
}

impl Default for Ability {
    fn default() -> Self {
        Self::new()
    }
}

impl Ability {
    pub fn new() -> Self {
        Ability {
            name: String::new(),
            ability_type: AbilityType::MeleeStrike,
            cooldown: 1.0,
            range: 1.5,
            base_damage: 10,
            mana_cost: 0,
            remaining_cooldown: 0.0,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_type(mut self, ability_type: AbilityType) -> Self {
        self.ability_type = ability_type;
        self
    }

    pub fn with_cooldown(mut self, cooldown: f32) -> Self {
        self.cooldown = cooldown;
        self
    }

    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    pub fn with_damage(mut self, base_damage: u32) -> Self {
        self.base_damage = base_damage;
        self
    }

    pub fn with_mana_cost(mut self, mana_cost: u32) -> Self {
        self.mana_cost = mana_cost;
        self
    }

    /// Returns `true` when the cooldown has elapsed.
    pub fn is_ready(&self) -> bool {
        self.remaining_cooldown <= 0.0
    }

    /// Advance the cooldown timer by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if self.remaining_cooldown > 0.0 {
            self.remaining_cooldown = (self.remaining_cooldown - dt).max(0.0);
        }
    }

    /// Attempt to activate — resets cooldown on success.
    pub fn use_ability(&mut self) -> bool {
        if self.is_ready() {
            self.remaining_cooldown = self.cooldown;
            true
        } else {
            false
        }
    }

    // ── Preset abilities ────────────────────────────────────────────────

    pub fn warrior_slash() -> Self {
        Ability::new()
            .with_name("Warrior Slash")
            .with_type(AbilityType::MeleeStrike)
            .with_cooldown(1.0)
            .with_range(1.5)
            .with_damage(15)
            .with_mana_cost(0)
    }

    pub fn archer_shot() -> Self {
        Ability::new()
            .with_name("Archer Shot")
            .with_type(AbilityType::RangedShot)
            .with_cooldown(1.5)
            .with_range(12.0)
            .with_damage(10)
            .with_mana_cost(0)
    }

    pub fn mage_fireball() -> Self {
        Ability::new()
            .with_name("Fireball")
            .with_type(AbilityType::Fireball)
            .with_cooldown(2.0)
            .with_range(10.0)
            .with_damage(20)
            .with_mana_cost(15)
    }

    pub fn priest_heal() -> Self {
        Ability::new()
            .with_name("Heal")
            .with_type(AbilityType::Heal)
            .with_cooldown(3.0)
            .with_range(8.0)
            .with_damage(15)
            .with_mana_cost(10)
    }
}

// ---------------------------------------------------------------------------
// Ability slots (4 per unit)
// ---------------------------------------------------------------------------

/// Holds up to four abilities for a single unit.
#[derive(Debug, Clone)]
pub struct AbilitySlot {
    pub abilities: [Option<Ability>; 4],
}

impl Default for AbilitySlot {
    fn default() -> Self {
        Self::new()
    }
}

impl AbilitySlot {
    pub fn new() -> Self {
        AbilitySlot {
            abilities: [None, None, None, None],
        }
    }

    pub fn set(&mut self, slot_index: usize, ability: Ability) {
        if slot_index < 4 {
            self.abilities[slot_index] = Some(ability);
        }
    }

    pub fn get(&self, slot_index: usize) -> Option<&Ability> {
        if slot_index < 4 {
            self.abilities[slot_index].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, slot_index: usize) -> Option<&mut Ability> {
        if slot_index < 4 {
            self.abilities[slot_index].as_mut()
        } else {
            None
        }
    }

    /// Tick cooldowns on every ability in the slot.
    pub fn tick_all(&mut self, dt: f32) {
        for ref mut ab in self.abilities.iter_mut().flatten() {
            ab.tick(dt);
        }
    }

    /// Returns the index of the first ability that is off cooldown.
    pub fn first_ready(&self) -> Option<usize> {
        for (i, slot) in self.abilities.iter().enumerate() {
            if let Some(ref ab) = slot {
                if ab.is_ready() {
                    return Some(i);
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Mana pool component
// ---------------------------------------------------------------------------

/// Mana pool attached to casters that spend mana on abilities.
#[derive(Debug, Clone, Copy)]
pub struct ManaPool {
    pub current: u32,
    pub max: u32,
}

impl ManaPool {
    pub fn new(max: u32) -> Self {
        ManaPool { current: max, max }
    }

    /// Subtract mana. Returns `false` if not enough.
    pub fn spend(&mut self, amount: u32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    /// Restore mana up to max.
    pub fn regen(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }
}

// ---------------------------------------------------------------------------
// Ability result
// ---------------------------------------------------------------------------

/// Outcome of an ability activation attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AbilityResult {
    Success { damage: u32, target: Entity },
    OnCooldown,
    OutOfRange,
    InvalidTarget,
    NoAbility,
    NotEnoughMana,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Euclidean distance between two transforms (ignores rotation / scale).
fn distance_3d(a: Transform, b: Transform) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

// ---------------------------------------------------------------------------
// Ability system
// ---------------------------------------------------------------------------

/// Processes ability cooldowns and activation.
pub struct AbilitySystem;

impl Default for AbilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AbilitySystem {
    pub fn new() -> Self {
        AbilitySystem
    }

    /// Tick all `AbilitySlot` cooldowns in the world.
    pub fn update(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.get_entities_with::<AbilitySlot>();
        for entity in entities {
            if let Some(slots) = world.get_component_mut::<AbilitySlot>(entity) {
                slots.tick_all(dt);
            }
        }
    }

    /// Compute total damage / heal amount scaled by mercenary stats.
    pub fn calculate_ability_damage(ability: &Ability, stats: &MercenaryStats) -> u32 {
        match ability.ability_type {
            AbilityType::MeleeStrike => (stats.strength as f32 * 2.0) as u32 + ability.base_damage,
            AbilityType::RangedShot => (stats.dexterity as f32 * 2.0) as u32 + ability.base_damage,
            AbilityType::Fireball => (stats.intelligence as f32 * 3.0) as u32 + ability.base_damage,
            AbilityType::Heal => (stats.intelligence as f32 * 2.0) as u32 + ability.base_damage,
            AbilityType::ShieldBash => (stats.strength as f32 * 1.5) as u32 + ability.base_damage,
            AbilityType::PoisonArrow => (stats.dexterity as f32 * 1.5) as u32 + ability.base_damage,
        }
    }

    /// Attempt to use an ability from `caster`'s `slot` on `target`.
    pub fn use_ability_on_target(
        world: &mut World,
        caster: Entity,
        slot: usize,
        target: AbilityTarget,
    ) -> AbilityResult {
        // ── Read caster data (borrows dropped immediately) ──────────
        let ability_data = {
            let slots = match world.get_component::<AbilitySlot>(caster) {
                Some(s) => s,
                None => return AbilityResult::NoAbility,
            };
            match slots.get(slot) {
                Some(a) => a.clone(),
                None => return AbilityResult::NoAbility,
            }
        };

        if !ability_data.is_ready() {
            return AbilityResult::OnCooldown;
        }

        let has_mana = if ability_data.mana_cost > 0 {
            match world.get_component::<ManaPool>(caster) {
                Some(m) => m.current >= ability_data.mana_cost,
                None => false,
            }
        } else {
            true
        };

        if !has_mana {
            return AbilityResult::NotEnoughMana;
        }

        let caster_team = match world.get_component::<Team>(caster) {
            Some(&t) => t,
            None => return AbilityResult::InvalidTarget,
        };

        let caster_pos = match world.get_component::<Transform>(caster) {
            Some(&t) => t,
            None => return AbilityResult::InvalidTarget,
        };

        let stats = match world.get_component::<MercenaryStats>(caster) {
            Some(s) => s.clone(),
            None => return AbilityResult::InvalidTarget,
        };

        // ── Validate target and apply effects ───────────────────────
        match target {
            AbilityTarget::None => {
                let amount = Self::calculate_ability_damage(&ability_data, &stats);
                Self::apply_caster_effects(world, caster, slot, ability_data.mana_cost);
                AbilityResult::Success {
                    damage: amount,
                    target: caster,
                }
            }

            AbilityTarget::SingleEnemy(target_entity) => {
                let target_team = match world.get_component::<Team>(target_entity) {
                    Some(&t) => t,
                    None => return AbilityResult::InvalidTarget,
                };
                if target_team == caster_team {
                    return AbilityResult::InvalidTarget;
                }
                let target_dead = match world.get_component::<Health>(target_entity) {
                    Some(h) => h.is_dead(),
                    None => return AbilityResult::InvalidTarget,
                };
                if target_dead {
                    return AbilityResult::InvalidTarget;
                }
                let target_pos = match world.get_component::<Transform>(target_entity) {
                    Some(&t) => t,
                    None => return AbilityResult::InvalidTarget,
                };
                if distance_3d(caster_pos, target_pos) > ability_data.range {
                    return AbilityResult::OutOfRange;
                }

                let damage = Self::calculate_ability_damage(&ability_data, &stats);
                if let Some(health) = world.get_component_mut::<Health>(target_entity) {
                    health.take_damage(damage);
                }
                Self::apply_caster_effects(world, caster, slot, ability_data.mana_cost);
                AbilityResult::Success {
                    damage,
                    target: target_entity,
                }
            }

            AbilityTarget::Ally(target_entity) => {
                let target_team = match world.get_component::<Team>(target_entity) {
                    Some(&t) => t,
                    None => return AbilityResult::InvalidTarget,
                };
                if target_team != caster_team {
                    return AbilityResult::InvalidTarget;
                }
                let target_dead = match world.get_component::<Health>(target_entity) {
                    Some(h) => h.is_dead(),
                    None => return AbilityResult::InvalidTarget,
                };
                if target_dead {
                    return AbilityResult::InvalidTarget;
                }
                let target_pos = match world.get_component::<Transform>(target_entity) {
                    Some(&t) => t,
                    None => return AbilityResult::InvalidTarget,
                };
                if distance_3d(caster_pos, target_pos) > ability_data.range {
                    return AbilityResult::OutOfRange;
                }

                let heal_amount = Self::calculate_ability_damage(&ability_data, &stats);
                if let Some(health) = world.get_component_mut::<Health>(target_entity) {
                    health.current = (health.current + heal_amount).min(health.max);
                }
                Self::apply_caster_effects(world, caster, slot, ability_data.mana_cost);
                AbilityResult::Success {
                    damage: heal_amount,
                    target: target_entity,
                }
            }

            AbilityTarget::Aoe(_position, _radius) => {
                // AoE resolves at the position; a full implementation would
                // spatial-query entities inside the radius.  For now the
                // ability is consumed and returns the calculated damage.
                let amount = Self::calculate_ability_damage(&ability_data, &stats);
                Self::apply_caster_effects(world, caster, slot, ability_data.mana_cost);
                AbilityResult::Success {
                    damage: amount,
                    target: caster,
                }
            }
        }
    }

    /// Reset cooldown and spend mana on the caster.
    fn apply_caster_effects(world: &mut World, caster: Entity, slot: usize, mana_cost: u32) {
        if let Some(slots) = world.get_component_mut::<AbilitySlot>(caster) {
            if let Some(ab) = slots.get_mut(slot) {
                ab.use_ability();
            }
        }
        if mana_cost > 0 {
            if let Some(mana) = world.get_component_mut::<ManaPool>(caster) {
                mana.spend(mana_cost);
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_world() -> World {
        World::new()
    }

    /// Helper: create a warrior-type entity at the given position.
    fn spawn_warrior(world: &mut World, team: Team, pos: [f32; 3]) -> Entity {
        let e = world.create_entity();
        world.add_component(e, Transform::new(pos[0], pos[1], pos[2]));
        world.add_component(e, Health::new(100));
        world.add_component(
            e,
            MercenaryStats::new("TestWarrior").with_stats(15, 8, 5, 12),
        );
        world.add_component(e, team);
        world.add_component(e, ManaPool::new(50));
        let mut slots = AbilitySlot::new();
        slots.set(0, Ability::warrior_slash());
        world.add_component(e, slots);
        e
    }

    /// Helper: create a mage-type entity at the given position.
    fn spawn_mage(world: &mut World, team: Team, pos: [f32; 3]) -> Entity {
        let e = world.create_entity();
        world.add_component(e, Transform::new(pos[0], pos[1], pos[2]));
        world.add_component(e, Health::new(80));
        world.add_component(e, MercenaryStats::new("TestMage").with_stats(5, 8, 18, 6));
        world.add_component(e, team);
        world.add_component(e, ManaPool::new(100));
        let mut slots = AbilitySlot::new();
        slots.set(0, Ability::mage_fireball());
        slots.set(1, Ability::priest_heal());
        world.add_component(e, slots);
        e
    }

    // ── 1. Ability creation and builder pattern ─────────────────────

    #[test]
    fn ability_builder() {
        let ab = Ability::new()
            .with_name("Test Strike")
            .with_type(AbilityType::MeleeStrike)
            .with_cooldown(2.0)
            .with_range(3.0)
            .with_damage(25)
            .with_mana_cost(5);

        assert_eq!(ab.name, "Test Strike");
        assert_eq!(ab.ability_type, AbilityType::MeleeStrike);
        assert!((ab.cooldown - 2.0).abs() < f32::EPSILON);
        assert!((ab.range - 3.0).abs() < f32::EPSILON);
        assert_eq!(ab.base_damage, 25);
        assert_eq!(ab.mana_cost, 5);
        assert!(ab.is_ready());
    }

    // ── 2. Cooldown tick and readiness ──────────────────────────────

    #[test]
    fn cooldown_tick_readiness() {
        let mut ab = Ability::warrior_slash();
        assert!(ab.is_ready());
        ab.use_ability();
        assert!(!ab.is_ready());
        assert!((ab.remaining_cooldown - 1.0).abs() < f32::EPSILON);

        ab.tick(0.4);
        assert!(!ab.is_ready());
        assert!((ab.remaining_cooldown - 0.6).abs() < 1e-4);

        ab.tick(0.6);
        assert!(ab.is_ready());
        assert!(ab.remaining_cooldown.abs() < f32::EPSILON);
    }

    // ── 3. Ability use resets cooldown ──────────────────────────────

    #[test]
    fn use_ability_resets_cooldown() {
        let mut ab = Ability::archer_shot();
        assert!(ab.use_ability());
        assert!(!ab.use_ability()); // on CD now
        assert!((ab.remaining_cooldown - 1.5).abs() < f32::EPSILON);
    }

    // ── 4. AbilitySlot set / get / first_ready ──────────────────────

    #[test]
    fn ability_slot_operations() {
        let mut slot = AbilitySlot::new();
        assert!(slot.get(0).is_none());
        assert_eq!(slot.first_ready(), None);

        slot.set(0, Ability::warrior_slash());
        slot.set(2, Ability::archer_shot());
        assert!(slot.get(0).is_some());
        assert!(slot.get(1).is_none());
        assert!(slot.get(2).is_some());
        assert_eq!(slot.first_ready(), Some(0));

        // Put slot 0 on cooldown → first_ready returns slot 2
        slot.get_mut(0).unwrap().use_ability();
        assert_eq!(slot.first_ready(), Some(2));

        // Out-of-bounds returns None
        assert!(slot.get(4).is_none());
        assert!(slot.get_mut(99).is_none());
    }

    // ── 5. Damage calculation per ability type ──────────────────────

    #[test]
    fn damage_calculation_per_type() {
        let stats = MercenaryStats::new("Dummy").with_stats(10, 10, 10, 10);

        let melee = Ability::new()
            .with_type(AbilityType::MeleeStrike)
            .with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&melee, &stats),
            10 * 2 + 5
        );

        let ranged = Ability::new()
            .with_type(AbilityType::RangedShot)
            .with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&ranged, &stats),
            10 * 2 + 5
        );

        let fireball = Ability::new()
            .with_type(AbilityType::Fireball)
            .with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&fireball, &stats),
            10 * 3 + 5
        );

        let heal = Ability::new().with_type(AbilityType::Heal).with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&heal, &stats),
            10 * 2 + 5
        );

        let bash = Ability::new()
            .with_type(AbilityType::ShieldBash)
            .with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&bash, &stats),
            (10.0_f32 * 1.5) as u32 + 5
        );

        let poison = Ability::new()
            .with_type(AbilityType::PoisonArrow)
            .with_damage(5);
        assert_eq!(
            AbilitySystem::calculate_ability_damage(&poison, &stats),
            (10.0_f32 * 1.5) as u32 + 5
        );
    }

    // ── 6. Use ability on valid target (applies damage) ─────────────

    #[test]
    fn valid_target_takes_damage() {
        let mut world = fresh_world();
        let caster = spawn_warrior(&mut world, Team::Player, [0.0, 0.0, 0.0]);
        let enemy = spawn_warrior(&mut world, Team::Enemy, [1.0, 0.0, 0.0]);

        let result = AbilitySystem::use_ability_on_target(
            &mut world,
            caster,
            0,
            AbilityTarget::SingleEnemy(enemy),
        );

        match result {
            AbilityResult::Success { damage, target } => {
                assert_eq!(target, enemy);
                // strength 15 → 15*2 + 15 = 45
                assert_eq!(damage, 45);
            }
            other => panic!("expected Success, got {:?}", other),
        }

        let health = world.get_component::<Health>(enemy).unwrap();
        assert_eq!(health.current, 100 - 45);
    }

    // ── 7. Use ability on invalid target (same team) ────────────────

    #[test]
    fn same_team_is_invalid_target() {
        let mut world = fresh_world();
        let caster = spawn_warrior(&mut world, Team::Player, [0.0, 0.0, 0.0]);
        let ally = spawn_warrior(&mut world, Team::Player, [1.0, 0.0, 0.0]);

        let result = AbilitySystem::use_ability_on_target(
            &mut world,
            caster,
            0,
            AbilityTarget::SingleEnemy(ally),
        );
        assert_eq!(result, AbilityResult::InvalidTarget);
    }

    // ── 8. Out of range rejection ───────────────────────────────────

    #[test]
    fn out_of_range_rejected() {
        let mut world = fresh_world();
        let caster = spawn_warrior(&mut world, Team::Player, [0.0, 0.0, 0.0]);
        let enemy = spawn_warrior(&mut world, Team::Enemy, [50.0, 0.0, 0.0]);

        // warrior_slash range is 1.5, target is 50 units away
        let result = AbilitySystem::use_ability_on_target(
            &mut world,
            caster,
            0,
            AbilityTarget::SingleEnemy(enemy),
        );
        assert_eq!(result, AbilityResult::OutOfRange);
    }

    // ── 9. Mana cost enforcement ────────────────────────────────────

    #[test]
    fn mana_cost_enforcement() {
        let mut world = fresh_world();
        let caster = spawn_mage(&mut world, Team::Player, [0.0, 0.0, 0.0]);
        let enemy = spawn_warrior(&mut world, Team::Enemy, [1.0, 0.0, 0.0]);

        // Drain mana to 0
        {
            let mana = world.get_component_mut::<ManaPool>(caster).unwrap();
            mana.current = 3; // fireball costs 15
        }

        let result = AbilitySystem::use_ability_on_target(
            &mut world,
            caster,
            0,
            AbilityTarget::SingleEnemy(enemy),
        );
        assert_eq!(result, AbilityResult::NotEnoughMana);
    }

    // ── 10. Heal ability on ally ────────────────────────────────────

    #[test]
    fn heal_ability_on_ally() {
        let mut world = fresh_world();
        let healer = spawn_mage(&mut world, Team::Player, [0.0, 0.0, 0.0]);
        let ally = spawn_warrior(&mut world, Team::Player, [1.0, 0.0, 0.0]);

        // Damage the ally first
        {
            let health = world.get_component_mut::<Health>(ally).unwrap();
            health.take_damage(60);
        }
        assert_eq!(world.get_component::<Health>(ally).unwrap().current, 40);

        // Priest heal is in slot 1, mage intelligence = 18 → 18*2 + 15 = 51
        let result =
            AbilitySystem::use_ability_on_target(&mut world, healer, 1, AbilityTarget::Ally(ally));

        match result {
            AbilityResult::Success { damage, target } => {
                assert_eq!(target, ally);
                assert_eq!(damage, 51);
            }
            other => panic!("expected Success, got {:?}", other),
        }

        // 40 + 51 = 91 (capped at max 100)
        let health = world.get_component::<Health>(ally).unwrap();
        assert_eq!(health.current, 91);

        // Mana was spent (priest heal costs 10)
        let mana = world.get_component::<ManaPool>(healer).unwrap();
        assert_eq!(mana.current, 90);
    }

    // ── 11. System update ticks cooldowns ───────────────────────────

    #[test]
    fn system_update_ticks_cooldowns() {
        let mut world = fresh_world();
        let entity = spawn_warrior(&mut world, Team::Player, [0.0, 0.0, 0.0]);

        // Use the ability to start cooldown
        {
            let slots = world.get_component_mut::<AbilitySlot>(entity).unwrap();
            slots.get_mut(0).unwrap().use_ability();
        }

        let cd_before = world
            .get_component::<AbilitySlot>(entity)
            .unwrap()
            .get(0)
            .unwrap()
            .remaining_cooldown;
        assert!(cd_before > 0.0);

        let mut sys = AbilitySystem::new();
        sys.update(&mut world, 0.5);

        let cd_after = world
            .get_component::<AbilitySlot>(entity)
            .unwrap()
            .get(0)
            .unwrap()
            .remaining_cooldown;
        assert!(cd_after < cd_before);
        assert!((cd_after - 0.5).abs() < 1e-4);
    }

    // ── 12. NoAbility when caster lacks AbilitySlot ─────────────────

    #[test]
    fn no_ability_when_no_slot() {
        let mut world = fresh_world();
        let entity = world.create_entity();
        world.add_component(entity, Transform::new(0.0, 0.0, 0.0));
        world.add_component(entity, Team::Player);

        let result =
            AbilitySystem::use_ability_on_target(&mut world, entity, 0, AbilityTarget::None);
        assert_eq!(result, AbilityResult::NoAbility);
    }
}

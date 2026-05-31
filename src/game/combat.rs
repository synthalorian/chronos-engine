use super::components::*;
use crate::component::{Dead, Health, Transform};
#[cfg(feature = "game")]
use crate::{Entity, World};

/// The type of attack being performed.
///
/// `Melee` has a fixed range of 1.5 units. `Ranged` and `Magic` carry
/// their own maximum engagement distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttackType {
    Melee,
    Ranged(f32),
    Magic(f32),
}

/// Tracks the remaining cooldown before an entity can attack again.
#[derive(Debug, Clone, Copy)]
pub struct AttackCooldown {
    pub remaining: f32,
    pub base_duration: f32,
}

impl AttackCooldown {
    /// Create a new cooldown with the given base duration, ready to fire.
    pub fn new(duration: f32) -> Self {
        AttackCooldown {
            remaining: 0.0,
            base_duration: duration,
        }
    }

    /// Returns `true` when the entity is allowed to attack.
    pub fn is_ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Advance the cooldown by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    /// Reset the cooldown to its base duration.
    pub fn reset(&mut self) {
        self.remaining = self.base_duration;
    }

    /// Builder: start with a specific remaining time.
    pub fn with_remaining(mut self, remaining: f32) -> Self {
        self.remaining = remaining;
        self
    }
}

/// Tracks an entity's active combat engagement.
#[derive(Debug, Clone, Copy)]
pub struct CombatState {
    pub attacker: Option<Entity>,
    pub target: Option<Entity>,
    pub attack_type: AttackType,
    pub last_attack_time: f32,
}

impl CombatState {
    /// Create combat state with the given attack type and no engagement.
    pub fn new(attack_type: AttackType) -> Self {
        CombatState {
            attacker: None,
            target: None,
            attack_type,
            last_attack_time: 0.0,
        }
    }

    /// Builder: set the attacker entity.
    pub fn with_attacker(mut self, attacker: Entity) -> Self {
        self.attacker = Some(attacker);
        self
    }

    /// Builder: set the target entity.
    pub fn with_target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
    }

    /// Builder: set the last attack timestamp.
    pub fn with_last_attack_time(mut self, time: f32) -> Self {
        self.last_attack_time = time;
        self
    }
}

/// A recorded damage event, suitable for logging or replay.
#[derive(Debug, Clone)]
pub struct DamageEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub raw_damage: u32,
    pub damage_type: AttackType,
}

impl DamageEvent {
    pub fn new(attacker: Entity, target: Entity, raw_damage: u32, damage_type: AttackType) -> Self {
        DamageEvent {
            attacker,
            target,
            raw_damage,
            damage_type,
        }
    }
}

/// Core combat system that resolves attacks each tick.
///
/// On every call to [`CombatSystem::update`] the system:
/// 1. Ticks all [`AttackCooldown`] components.
/// 2. For every entity that has both [`CombatState`] and [`Health`] (and
///    is alive), validates the current target, checks range, and applies
///    damage when the cooldown is ready.
/// 3. Marks killed targets with the [`Dead`] component and clears the
///    attacker's combat state.
pub struct CombatSystem;

impl Default for CombatSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatSystem {
    /// Create a new combat system.
    pub fn new() -> Self {
        CombatSystem
    }

    /// Advance the combat simulation by `dt` seconds.
    pub fn update(&self, world: &mut World, dt: f32) {
        // Phase 1: Tick all cooldowns.
        let cooldown_entities = world.get_entities_with::<AttackCooldown>();
        for entity in cooldown_entities {
            if let Some(cooldown) = world.get_component_mut::<AttackCooldown>(entity) {
                cooldown.tick(dt);
            }
        }

        // Phase 2: Resolve combat for each entity with a CombatState.
        let combat_entities = world.get_entities_with::<CombatState>();

        for entity in combat_entities {
            // Attacker must be alive.
            if !world.has_component::<Health>(entity) {
                continue;
            }
            if world
                .get_component::<Health>(entity)
                .is_none_or(|h| h.is_dead())
            {
                continue;
            }

            // Copy out combat state so we can mutate the world freely.
            let combat = match world.get_component::<CombatState>(entity) {
                Some(c) => *c,
                None => continue,
            };

            let target = match combat.target {
                Some(t) => t,
                None => continue,
            };

            // Target must exist, have Health, and be alive.
            if !world.entity_exists(target) {
                continue;
            }
            if !world.has_component::<Health>(target) {
                continue;
            }
            if world
                .get_component::<Health>(target)
                .is_none_or(|h| h.is_dead())
            {
                continue;
            }

            // Friendly-fire check.
            if !Self::is_enemy(world, entity, target) {
                continue;
            }

            // Cooldown must be ready (or absent).
            let cooldown_ready = world
                .get_component::<AttackCooldown>(entity)
                .is_none_or(|c| c.is_ready());
            if !cooldown_ready {
                continue;
            }

            // Range check.
            let max_range = match combat.attack_type {
                AttackType::Melee => 1.5,
                AttackType::Ranged(r) => r,
                AttackType::Magic(r) => r,
            };

            match Self::distance_between(world, entity, target) {
                Some(d) if d <= max_range => {}
                _ => continue,
            }

            // Damage calculation.
            let damage = match world.get_component::<MercenaryStats>(entity) {
                Some(stats) => Self::calculate_damage(stats, &combat.attack_type),
                None => continue,
            };

            // Apply damage to target.
            if let Some(health) = world.get_component_mut::<Health>(target) {
                health.take_damage(damage);
            }

            // Reset attacker cooldown.
            if let Some(cooldown) = world.get_component_mut::<AttackCooldown>(entity) {
                cooldown.reset();
            }

            // Death check — mark dead and clear attacker's engagement.
            let target_dead = world
                .get_component::<Health>(target)
                .is_some_and(|h| h.is_dead());

            if target_dead {
                world.add_component(target, Dead);
                if let Some(cs) = world.get_component_mut::<CombatState>(entity) {
                    cs.target = None;
                }
            }
        }
    }

    /// Compute damage from attacker stats and attack type.
    ///
    /// - **Melee:** `strength * 2 + level`
    /// - **Ranged:** `dexterity * 2 + level`
    /// - **Magic:** `intelligence * 3 + level`
    pub fn calculate_damage(stats: &MercenaryStats, attack_type: &AttackType) -> u32 {
        match attack_type {
            AttackType::Melee => stats.strength * 2 + stats.level,
            AttackType::Ranged(_) => stats.dexterity * 2 + stats.level,
            AttackType::Magic(_) => stats.intelligence * 3 + stats.level,
        }
    }

    /// Euclidean distance between two entities based on their [`Transform`].
    ///
    /// Returns `None` if either entity lacks a Transform.
    pub fn distance_between(world: &World, a: Entity, b: Entity) -> Option<f32> {
        let ta = world.get_component::<Transform>(a)?;
        let tb = world.get_component::<Transform>(b)?;
        let dx = ta.x - tb.x;
        let dy = ta.y - tb.y;
        let dz = ta.z - tb.z;
        Some((dx * dx + dy * dy + dz * dz).sqrt())
    }

    /// Returns `true` when `a` and `b` are on different, non-Neutral teams.
    pub fn is_enemy(world: &World, a: Entity, b: Entity) -> bool {
        let team_a = match world.get_component::<Team>(a) {
            Some(t) => *t,
            None => return false,
        };
        let team_b = match world.get_component::<Team>(b) {
            Some(t) => *t,
            None => return false,
        };
        if team_a == Team::Neutral || team_b == Team::Neutral {
            return false;
        }
        team_a != team_b
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

    fn make_combatant(
        world: &mut World,
        team: Team,
        pos: [f32; 3],
        stats: Option<MercenaryStats>,
        health: u32,
        attack_type: AttackType,
    ) -> Entity {
        let e = world.create_entity();
        world.add_component(e, Transform::new(pos[0], pos[1], pos[2]));
        world.add_component(e, Health::new(health));
        world.add_component(e, team);
        if let Some(s) = stats {
            world.add_component(e, s);
        }
        let _ = attack_type; // caller adds CombatState separately if needed
        e
    }

    // ── AttackCooldown ──────────────────────────

    #[test]
    fn cooldown_tick_and_reset() {
        let mut cd = AttackCooldown::new(2.0).with_remaining(2.0);
        assert!(!cd.is_ready());

        cd.tick(1.0);
        assert!(!cd.is_ready());
        assert!((cd.remaining - 1.0).abs() < f32::EPSILON);

        cd.tick(1.0);
        assert!(cd.is_ready());

        cd.reset();
        assert!(!cd.is_ready());
        assert!((cd.remaining - 2.0).abs() < f32::EPSILON);
    }

    // ── Damage calculation ──────────────────────

    #[test]
    fn melee_damage_uses_strength() {
        let stats = MercenaryStats::new("Brute").with_stats(20, 5, 3, 10);
        let dmg = CombatSystem::calculate_damage(&stats, &AttackType::Melee);
        // strength(20) * 2 + level(1) = 41
        assert_eq!(dmg, 41);
    }

    #[test]
    fn ranged_damage_uses_dexterity() {
        let stats = MercenaryStats::new("Sniper").with_stats(5, 18, 3, 8);
        let dmg = CombatSystem::calculate_damage(&stats, &AttackType::Ranged(10.0));
        // dexterity(18) * 2 + level(1) = 37
        assert_eq!(dmg, 37);
    }

    #[test]
    fn magic_damage_uses_intelligence() {
        let stats = MercenaryStats::new("Wizard").with_stats(3, 5, 25, 6);
        let dmg = CombatSystem::calculate_damage(&stats, &AttackType::Magic(12.0));
        // intelligence(25) * 3 + level(1) = 76
        assert_eq!(dmg, 76);
    }

    // ── Distance calculation ────────────────────

    #[test]
    fn distance_between_entities() {
        let mut world = test_world();
        let a = world.create_entity();
        world.add_component(a, Transform::new(0.0, 0.0, 0.0));
        let b = world.create_entity();
        world.add_component(b, Transform::new(3.0, 4.0, 0.0));

        let dist = CombatSystem::distance_between(&world, a, b).unwrap();
        assert!((dist - 5.0).abs() < 1e-4);
    }

    #[test]
    fn distance_missing_transform_returns_none() {
        let mut world = test_world();
        let a = world.create_entity();
        world.add_component(a, Transform::new(0.0, 0.0, 0.0));
        let b = world.create_entity(); // no Transform

        assert!(CombatSystem::distance_between(&world, a, b).is_none());
    }

    // ── Enemy check ─────────────────────────────

    #[test]
    fn enemy_check_same_and_different_teams() {
        let mut world = test_world();

        let player = world.create_entity();
        world.add_component(player, Team::Player);

        let enemy = world.create_entity();
        world.add_component(enemy, Team::Enemy);

        let neutral = world.create_entity();
        world.add_component(neutral, Team::Neutral);

        let ally = world.create_entity();
        world.add_component(ally, Team::Player);

        // Different teams → true
        assert!(CombatSystem::is_enemy(&world, player, enemy));
        assert!(CombatSystem::is_enemy(&world, enemy, player));

        // Same team → false
        assert!(!CombatSystem::is_enemy(&world, player, ally));

        // Neutral involved → false
        assert!(!CombatSystem::is_enemy(&world, player, neutral));
        assert!(!CombatSystem::is_enemy(&world, neutral, enemy));
    }

    // ── Combat: applies damage to target ────────

    #[test]
    fn combat_applies_damage() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [1.0, 0.0, 0.0],
            None,
            100,
            AttackType::Melee,
        );

        // Wire up the target
        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        let target_health = world.get_component::<Health>(target).unwrap();
        // Melee damage: strength(10) * 2 + level(1) = 21
        assert_eq!(target_health.current, 79);
    }

    // ── Combat: kills target and adds Dead ──────

    #[test]
    fn combat_kills_target() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(50, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        // Target has only 5 HP — will be killed in one hit.
        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [0.5, 0.0, 0.0],
            None,
            5,
            AttackType::Melee,
        );

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        // Target should be dead.
        assert!(world.has_component::<Dead>(target));
        assert!(world.get_component::<Health>(target).unwrap().is_dead());

        // Attacker's combat state should be cleared.
        let cs = world.get_component::<CombatState>(attacker).unwrap();
        assert!(cs.target.is_none());
    }

    // ── Invalid target: dead target is skipped ──

    #[test]
    fn invalid_target_dead_is_skipped() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [0.5, 0.0, 0.0],
            None,
            10,
            AttackType::Melee,
        );
        // Kill the target first.
        world
            .get_component_mut::<Health>(target)
            .unwrap()
            .take_damage(10);
        assert!(world.get_component::<Health>(target).unwrap().is_dead());

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        // No Dead component should be re-added (it wasn't there before).
        // The cooldown should remain ready since no attack was made.
        let cd = world.get_component::<AttackCooldown>(attacker).unwrap();
        assert!(cd.is_ready());
    }

    // ── Invalid target: missing components ──────

    #[test]
    fn invalid_target_missing_health_is_skipped() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        // Target has no Health component.
        let target = world.create_entity();
        world.add_component(target, Team::Enemy);
        world.add_component(target, Transform::new(0.5, 0.0, 0.0));

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        // Cooldown should remain ready since no attack occurred.
        let cd = world.get_component::<AttackCooldown>(attacker).unwrap();
        assert!(cd.is_ready());
    }

    // ── Cooldown prevents double attack ─────────

    #[test]
    fn cooldown_prevents_double_attack() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0).with_remaining(0.5));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [0.5, 0.0, 0.0],
            None,
            100,
            AttackType::Melee,
        );

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        // First tick — cooldown not ready, should not attack.
        system.update(&mut world, 0.016);

        let target_health = world.get_component::<Health>(target).unwrap();
        assert_eq!(target_health.current, 100); // untouched
    }

    // ── Out of range prevents attack ────────────

    #[test]
    fn out_of_range_prevents_attack() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        // Place target 10 units away — well outside melee range (1.5).
        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [10.0, 0.0, 0.0],
            None,
            100,
            AttackType::Melee,
        );

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        let target_health = world.get_component::<Health>(target).unwrap();
        assert_eq!(target_health.current, 100); // untouched
    }

    // ── Ranged attack within range hits ─────────

    #[test]
    fn ranged_attack_in_range_hits() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Archer").with_stats(5, 15, 3, 8)),
            100,
            AttackType::Ranged(10.0),
        );
        world.add_component(attacker, AttackCooldown::new(0.5));
        world.add_component(attacker, CombatState::new(AttackType::Ranged(10.0)));

        let target = make_combatant(
            &mut world,
            Team::Enemy,
            [5.0, 0.0, 0.0],
            None,
            50,
            AttackType::Ranged(10.0),
        );

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(target);

        system.update(&mut world, 0.016);

        let target_health = world.get_component::<Health>(target).unwrap();
        // Ranged damage: dexterity(15) * 2 + level(1) = 31
        assert_eq!(target_health.current, 19);
    }

    // ── Friendly fire is blocked ────────────────

    #[test]
    fn friendly_fire_is_blocked() {
        let mut world = test_world();
        let system = CombatSystem::new();

        let attacker = make_combatant(
            &mut world,
            Team::Player,
            [0.0, 0.0, 0.0],
            Some(MercenaryStats::new("Hero").with_stats(10, 5, 3, 10)),
            100,
            AttackType::Melee,
        );
        world.add_component(attacker, AttackCooldown::new(1.0));
        world.add_component(attacker, CombatState::new(AttackType::Melee));

        // Same team — should not be attacked.
        let ally = make_combatant(
            &mut world,
            Team::Player,
            [0.5, 0.0, 0.0],
            None,
            100,
            AttackType::Melee,
        );

        world
            .get_component_mut::<CombatState>(attacker)
            .unwrap()
            .target = Some(ally);

        system.update(&mut world, 0.016);

        let ally_health = world.get_component::<Health>(ally).unwrap();
        assert_eq!(ally_health.current, 100);
    }
}

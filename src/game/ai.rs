use super::components::*;
use crate::component::{Health, Transform};
#[cfg(feature = "game")]
use crate::{Entity, World};

// ──────────────────────────────────────────────
// AI State enum
// ──────────────────────────────────────────────

/// Possible states for an enemy AI-controlled entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Patrol,
    Chase,
    Attack,
    ReturnToPost,
    Dead,
}

impl AiState {
    pub fn name(&self) -> &'static str {
        match self {
            AiState::Idle => "Idle",
            AiState::Patrol => "Patrol",
            AiState::Chase => "Chase",
            AiState::Attack => "Attack",
            AiState::ReturnToPost => "ReturnToPost",
            AiState::Dead => "Dead",
        }
    }
}

// ──────────────────────────────────────────────
// PatrolRoute
// ──────────────────────────────────────────────

/// A cyclic patrol route defined by waypoints.
#[derive(Debug, Clone)]
pub struct PatrolRoute {
    pub waypoints: Vec<[f32; 3]>,
    pub current_index: usize,
}

impl PatrolRoute {
    pub fn new(waypoints: Vec<[f32; 3]>) -> Self {
        PatrolRoute {
            waypoints,
            current_index: 0,
        }
    }

    /// Returns the next waypoint and advances the index.
    /// Cycles back to the start when the route is complete.
    pub fn advance(&mut self) -> Option<[f32; 3]> {
        if self.waypoints.is_empty() {
            return None;
        }
        let wp = self.waypoints[self.current_index];
        self.current_index = (self.current_index + 1) % self.waypoints.len();
        Some(wp)
    }

    /// Reset to the beginning of the route.
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Returns true if the route has no waypoints.
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Create an empty patrol route (stationary enemy).
    pub fn empty() -> Self {
        PatrolRoute {
            waypoints: Vec::new(),
            current_index: 0,
        }
    }
}

// ──────────────────────────────────────────────
// EnemyController component
// ──────────────────────────────────────────────

/// Component attached to AI-controlled enemy entities.
#[derive(Debug, Clone)]
pub struct EnemyController {
    pub state: AiState,
    pub patrol: PatrolRoute,
    pub home_position: [f32; 3],
    pub aggro_target: Option<Entity>,
    pub chase_speed: f32,
    pub idle_timer: f32,
    pub attack_cooldown: f32,
}

impl EnemyController {
    pub fn new(home: [f32; 3]) -> Self {
        EnemyController {
            state: AiState::Idle,
            patrol: PatrolRoute::empty(),
            home_position: home,
            aggro_target: None,
            chase_speed: 4.0,
            idle_timer: 0.0,
            attack_cooldown: 0.0,
        }
    }

    pub fn with_patrol(mut self, waypoints: Vec<[f32; 3]>) -> Self {
        self.patrol = PatrolRoute::new(waypoints);
        self
    }

    pub fn with_chase_speed(mut self, speed: f32) -> Self {
        self.chase_speed = speed;
        self
    }

    pub fn with_aggro_target(mut self, target: Entity) -> Self {
        self.aggro_target = Some(target);
        self
    }
}

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

const IDLE_DURATION: f32 = 1.0;
const ATTACK_RANGE: f32 = 1.5;
const ARRIVAL_THRESHOLD: f32 = 0.5;

// ──────────────────────────────────────────────
// EnemyFactory
// ──────────────────────────────────────────────

/// Factory for spawning pre-configured enemy entities.
pub struct EnemyFactory;

impl EnemyFactory {
    /// Creates a fully equipped enemy entity in the world.
    pub fn create_enemy(
        world: &mut World,
        position: [f32; 3],
        patrol_waypoints: Vec<[f32; 3]>,
        team: Team,
    ) -> Entity {
        let entity = world.create_entity();

        world.add_component(
            entity,
            Transform::new(position[0], position[1], position[2]),
        );
        world.add_component(entity, Health::new(100));
        world.add_component(entity, team);
        world.add_component(entity, AggroRadius::new(8.0));
        world.add_component(
            entity,
            EnemyController::new(position).with_patrol(patrol_waypoints),
        );
        world.add_component(entity, HealthBar::new(1.0, 0.1));
        world.add_component(entity, NavigationAgent::new(3.0));

        entity
    }

    /// Convenience: creates a patrol enemy on the Enemy team.
    pub fn create_patrol_enemy(
        world: &mut World,
        position: [f32; 3],
        waypoints: Vec<[f32; 3]>,
    ) -> Entity {
        Self::create_enemy(world, position, waypoints, Team::Enemy)
    }
}

// ──────────────────────────────────────────────
// AiSystem
// ──────────────────────────────────────────────

/// System that updates all enemy AI controllers each frame.
pub struct AiSystem;

impl AiSystem {
    /// Advance every enemy AI by `dt` seconds.
    pub fn update(world: &mut World, dt: f32) {
        // Collect all AI entities up-front so we can mutate freely.
        let ai_entities: Vec<Entity> = world.get_entities_with::<EnemyController>();

        for entity in ai_entities {
            // ── Dead check ───────────────────────────
            if let Some(health) = world.get_component::<Health>(entity) {
                if health.is_dead() {
                    if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                        ctrl.state = AiState::Dead;
                    }
                    continue;
                }
            }

            // ── Aggro detection (for Idle and Patrol) ─
            let should_detect = match world.get_component::<EnemyController>(entity) {
                Some(ctrl) => matches!(ctrl.state, AiState::Idle | AiState::Patrol),
                None => false,
            };

            if should_detect {
                let aggro_radius = world
                    .get_component::<AggroRadius>(entity)
                    .map(|a| a.radius)
                    .unwrap_or(0.0);

                let enemy_pos = world
                    .get_component::<Transform>(entity)
                    .map(|t| [t.x, t.y, t.z]);

                if let Some(ep) = enemy_pos {
                    let closest_target = Self::find_closest_player(world, ep, aggro_radius);
                    if let Some(target) = closest_target {
                        if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                            ctrl.aggro_target = Some(target);
                            ctrl.state = AiState::Chase;
                        }
                    }
                }
            }

            // ── State machine ────────────────────────
            let current_state = world
                .get_component::<EnemyController>(entity)
                .map(|c| c.state);

            let state = match current_state {
                Some(s) => s,
                None => continue,
            };

            match state {
                AiState::Idle => Self::update_idle(world, entity, dt),
                AiState::Patrol => Self::update_patrol(world, entity),
                AiState::Chase => Self::update_chase(world, entity),
                AiState::Attack => Self::update_attack(world, entity),
                AiState::ReturnToPost => Self::update_return(world, entity),
                AiState::Dead => { /* no-op */ }
            }
        }
    }

    // ── Idle ──────────────────────────────────

    fn update_idle(world: &mut World, entity: Entity, dt: f32) {
        let should_transition = {
            let ctrl = world.get_component_mut::<EnemyController>(entity);
            match ctrl {
                Some(c) => {
                    c.idle_timer += dt;
                    c.idle_timer >= IDLE_DURATION
                }
                None => false,
            }
        };

        if should_transition {
            if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                ctrl.idle_timer = 0.0;
                if !ctrl.patrol.is_empty() {
                    ctrl.state = AiState::Patrol;
                }
            }
        }
    }

    // ── Patrol ────────────────────────────────

    fn update_patrol(world: &mut World, entity: Entity) {
        // Check if we already have a move target (still moving).
        let has_target = world.has_component::<MoveTarget>(entity);

        if !has_target {
            if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                if let Some(wp) = ctrl.patrol.advance() {
                    world.add_component(entity, MoveTarget::new(wp[0], wp[1], wp[2]));
                }
            }
        } else {
            // Check arrival at waypoint.
            let arrived = Self::has_arrived_at_target(world, entity);
            if arrived {
                world.remove_component::<MoveTarget>(entity);
            }
        }
    }

    // ── Chase ─────────────────────────────────

    fn update_chase(world: &mut World, entity: Entity) {
        let target_entity = world
            .get_component::<EnemyController>(entity)
            .and_then(|c| c.aggro_target);

        let target_entity = match target_entity {
            Some(t) => t,
            None => {
                Self::begin_return(world, entity);
                return;
            }
        };

        // Target dead?
        let target_dead = world
            .get_component::<Health>(target_entity)
            .map(|h| h.is_dead())
            .unwrap_or(true);

        if target_dead {
            Self::begin_return(world, entity);
            return;
        }

        // Target out of 2x aggro range?
        let aggro_radius = world
            .get_component::<AggroRadius>(entity)
            .map(|a| a.radius)
            .unwrap_or(0.0);

        let home = world
            .get_component::<EnemyController>(entity)
            .map(|c| c.home_position)
            .unwrap_or([0.0, 0.0, 0.0]);

        let target_pos = world
            .get_component::<Transform>(target_entity)
            .map(|t| [t.x, t.y, t.z]);

        let my_pos = world
            .get_component::<Transform>(entity)
            .map(|t| [t.x, t.y, t.z]);

        if let (Some(tp), Some(mp)) = (target_pos, my_pos) {
            let dist_to_target = euclidean_dist(mp, tp);

            // Close enough to attack?
            if dist_to_target <= ATTACK_RANGE {
                if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                    ctrl.state = AiState::Attack;
                    ctrl.attack_cooldown = 0.0;
                }
                world.remove_component::<MoveTarget>(entity);
                return;
            }

            // Out of leash range (2x aggro from home)?
            let dist_from_home = euclidean_dist(tp, home);
            if dist_from_home > aggro_radius * 2.0 {
                Self::begin_return(world, entity);
                return;
            }

            // Update move target toward player.
            world.remove_component::<MoveTarget>(entity);
            world.add_component(entity, MoveTarget::new(tp[0], tp[1], tp[2]));
        }
    }

    // ── Attack ────────────────────────────────

    fn update_attack(world: &mut World, entity: Entity) {
        let target_entity = world
            .get_component::<EnemyController>(entity)
            .and_then(|c| c.aggro_target);

        let target_entity = match target_entity {
            Some(t) => t,
            None => {
                Self::begin_return(world, entity);
                return;
            }
        };

        // Target dead?
        let target_dead = world
            .get_component::<Health>(target_entity)
            .map(|h| h.is_dead())
            .unwrap_or(true);

        if target_dead {
            Self::begin_return(world, entity);
            return;
        }

        let target_pos = world
            .get_component::<Transform>(target_entity)
            .map(|t| [t.x, t.y, t.z]);

        let my_pos = world
            .get_component::<Transform>(entity)
            .map(|t| [t.x, t.y, t.z]);

        if let (Some(tp), Some(mp)) = (target_pos, my_pos) {
            let dist = euclidean_dist(mp, tp);
            if dist > ATTACK_RANGE {
                if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                    ctrl.state = AiState::Chase;
                }
            } else {
                // Placeholder: combat system handles actual damage.
                // Tick the cooldown timer so tests can observe progress.
                if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                    ctrl.attack_cooldown += 0.0; // no-op placeholder
                }
            }
        }
    }

    // ── ReturnToPost ──────────────────────────

    fn update_return(world: &mut World, entity: Entity) {
        let home = world
            .get_component::<EnemyController>(entity)
            .map(|c| c.home_position)
            .unwrap_or([0.0, 0.0, 0.0]);

        // Ensure we have a move target for home.
        let has_target = world.has_component::<MoveTarget>(entity);
        if !has_target {
            world.add_component(entity, MoveTarget::new(home[0], home[1], home[2]));
        }

        // Check arrival.
        if Self::has_arrived_at_target(world, entity) {
            world.remove_component::<MoveTarget>(entity);
            // Snap to home position.
            if let Some(t) = world.get_component_mut::<Transform>(entity) {
                t.x = home[0];
                t.y = home[1];
                t.z = home[2];
            }
            if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
                ctrl.state = AiState::Idle;
                ctrl.idle_timer = 0.0;
                ctrl.aggro_target = None;
                ctrl.patrol.reset();
            }
        }
    }

    // ── Helpers ───────────────────────────────

    fn begin_return(world: &mut World, entity: Entity) {
        world.remove_component::<MoveTarget>(entity);
        if let Some(ctrl) = world.get_component_mut::<EnemyController>(entity) {
            ctrl.state = AiState::ReturnToPost;
            ctrl.aggro_target = None;
        }
    }

    fn has_arrived_at_target(world: &mut World, entity: Entity) -> bool {
        let target_pos = world
            .get_component::<MoveTarget>(entity)
            .map(|mt| [mt.x, mt.y, mt.z]);

        let my_pos = world
            .get_component::<Transform>(entity)
            .map(|t| [t.x, t.y, t.z]);

        match (target_pos, my_pos) {
            (Some(tp), Some(mp)) => euclidean_dist(mp, tp) < ARRIVAL_THRESHOLD,
            _ => false,
        }
    }

    fn find_closest_player(world: &World, from: [f32; 3], radius: f32) -> Option<Entity> {
        let players = world.get_entities_with::<Team>();
        let mut closest: Option<(Entity, f32)> = None;

        for player_entity in players {
            // Only consider Player team members.
            let is_player = world
                .get_component::<Team>(player_entity)
                .map(|t| *t == Team::Player)
                .unwrap_or(false);

            if !is_player {
                continue;
            }

            // Skip dead players.
            let dead = world
                .get_component::<Health>(player_entity)
                .map(|h| h.is_dead())
                .unwrap_or(false);

            if dead {
                continue;
            }

            let pos = world
                .get_component::<Transform>(player_entity)
                .map(|t| [t.x, t.y, t.z]);

            if let Some(pp) = pos {
                let dist = euclidean_dist(from, pp);
                if dist <= radius {
                    match closest {
                        Some((_, cd)) if dist < cd => {
                            closest = Some((player_entity, dist));
                        }
                        None => {
                            closest = Some((player_entity, dist));
                        }
                        _ => {}
                    }
                }
            }
        }

        closest.map(|(e, _)| e)
    }
}

// ──────────────────────────────────────────────
// Utility
// ──────────────────────────────────────────────

fn euclidean_dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
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

    // ── 1. PatrolRoute traversal and reset ────

    #[test]
    fn patrol_route_traversal_and_reset() {
        let wps = vec![[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]];
        let mut route = PatrolRoute::new(wps);

        assert!(!route.is_empty());
        assert_eq!(route.advance(), Some([1.0, 0.0, 0.0]));
        assert_eq!(route.advance(), Some([2.0, 0.0, 0.0]));
        assert_eq!(route.advance(), Some([3.0, 0.0, 0.0]));

        // Cyclic: back to start.
        assert_eq!(route.advance(), Some([1.0, 0.0, 0.0]));

        route.reset();
        assert_eq!(route.current_index, 0);
        assert_eq!(route.advance(), Some([1.0, 0.0, 0.0]));
    }

    #[test]
    fn patrol_route_empty() {
        let mut route = PatrolRoute::empty();
        assert!(route.is_empty());
        assert!(route.advance().is_none());
    }

    // ── 2. EnemyFactory creates correct components ──

    #[test]
    fn factory_creates_entity_with_correct_components() {
        let mut world = test_world();
        let entity =
            EnemyFactory::create_patrol_enemy(&mut world, [5.0, 0.0, 5.0], vec![[10.0, 0.0, 10.0]]);

        assert!(world.has_component::<Transform>(entity));
        assert!(world.has_component::<Health>(entity));
        assert!(world.has_component::<Team>(entity));
        assert!(world.has_component::<AggroRadius>(entity));
        assert!(world.has_component::<EnemyController>(entity));
        assert!(world.has_component::<HealthBar>(entity));
        assert!(world.has_component::<NavigationAgent>(entity));

        let team = world.get_component::<Team>(entity).expect("team");
        assert_eq!(*team, Team::Enemy);

        let tf = world.get_component::<Transform>(entity).expect("transform");
        assert!((tf.x - 5.0).abs() < f32::EPSILON);
        assert!((tf.z - 5.0).abs() < f32::EPSILON);
    }

    // ── 3. Idle timer expiration → Patrol ─────

    #[test]
    fn idle_transitions_to_patrol_after_timer() {
        let mut world = test_world();
        let entity =
            EnemyFactory::create_patrol_enemy(&mut world, [0.0, 0.0, 0.0], vec![[1.0, 0.0, 0.0]]);

        // Starts Idle.
        let state = world
            .get_component::<EnemyController>(entity)
            .expect("ctrl")
            .state;
        assert_eq!(state, AiState::Idle);

        // Tick short of threshold — still Idle.
        AiSystem::update(&mut world, 0.5);
        let state = world
            .get_component::<EnemyController>(entity)
            .expect("ctrl")
            .state;
        assert_eq!(state, AiState::Idle);

        // Tick past threshold — should transition.
        AiSystem::update(&mut world, 0.6);
        let state = world
            .get_component::<EnemyController>(entity)
            .expect("ctrl")
            .state;
        assert_eq!(state, AiState::Patrol);
    }

    // ── 4. Aggro detection (Patrol → Chase) ───

    #[test]
    fn aggro_radius_detects_player() {
        let mut world = test_world();

        // Enemy at origin with aggro radius 5.
        let enemy = EnemyFactory::create_enemy(
            &mut world,
            [0.0, 0.0, 0.0],
            vec![[1.0, 0.0, 0.0]],
            Team::Enemy,
        );

        // Set aggro radius to 5.
        if let Some(ar) = world.get_component_mut::<AggroRadius>(enemy) {
            ar.radius = 5.0;
        }

        // Force to Patrol so aggro detection runs.
        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Patrol;
        }

        // Player within range.
        let player = world.create_entity();
        world.add_component(player, Transform::new(3.0, 0.0, 0.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Chase);
        assert!(ctrl.aggro_target.is_some());
        assert_eq!(ctrl.aggro_target.expect("target"), player);
    }

    #[test]
    fn aggro_ignores_distant_player() {
        let mut world = test_world();

        let enemy = EnemyFactory::create_enemy(
            &mut world,
            [0.0, 0.0, 0.0],
            vec![[1.0, 0.0, 0.0]],
            Team::Enemy,
        );

        if let Some(ar) = world.get_component_mut::<AggroRadius>(enemy) {
            ar.radius = 5.0;
        }

        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Patrol;
        }

        // Player far away.
        let player = world.create_entity();
        world.add_component(player, Transform::new(50.0, 0.0, 0.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Patrol);
        assert!(ctrl.aggro_target.is_none());
    }

    // ── 5. Chase → Attack ─────────────────────

    #[test]
    fn chase_transitions_to_attack_when_in_range() {
        let mut world = test_world();

        let enemy = EnemyFactory::create_enemy(&mut world, [0.0, 0.0, 0.0], vec![], Team::Enemy);

        let player = world.create_entity();
        world.add_component(player, Transform::new(1.0, 0.0, 0.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        // Manually set to Chase with target.
        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Chase;
            ctrl.aggro_target = Some(player);
        }

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Attack);
    }

    // ── 6. Chase → ReturnToPost (target out of leash) ──

    #[test]
    fn chase_returns_to_post_when_target_out_of_leash() {
        let mut world = test_world();

        // Enemy home at origin, aggro radius 5, so leash is 10.
        let enemy = EnemyFactory::create_enemy(&mut world, [0.0, 0.0, 0.0], vec![], Team::Enemy);

        if let Some(ar) = world.get_component_mut::<AggroRadius>(enemy) {
            ar.radius = 5.0;
        }

        // Player at 12 — beyond 2x aggro from home.
        let player = world.create_entity();
        world.add_component(player, Transform::new(12.0, 0.0, 0.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        // Enemy at origin, chasing.
        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Chase;
            ctrl.aggro_target = Some(player);
        }

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::ReturnToPost);
    }

    // ── 7. Dead state transition ──────────────

    #[test]
    fn dead_state_when_health_depleted() {
        let mut world = test_world();

        let enemy =
            EnemyFactory::create_patrol_enemy(&mut world, [0.0, 0.0, 0.0], vec![[1.0, 0.0, 0.0]]);

        // Kill the enemy.
        if let Some(hp) = world.get_component_mut::<Health>(enemy) {
            hp.take_damage(100);
        }

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Dead);
    }

    // ── 8. ReturnToPost → Idle ────────────────

    #[test]
    fn return_to_post_transitions_to_idle_on_arrival() {
        let mut world = test_world();

        let enemy =
            EnemyFactory::create_patrol_enemy(&mut world, [0.0, 0.0, 0.0], vec![[1.0, 0.0, 0.0]]);

        // Place enemy at home already and set to ReturnToPost.
        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::ReturnToPost;
        }

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Idle);
        assert!(ctrl.aggro_target.is_none());
    }

    // ── 9. Attack → Chase when target moves away ──

    #[test]
    fn attack_transitions_to_chase_when_target_moves() {
        let mut world = test_world();

        let enemy = EnemyFactory::create_enemy(&mut world, [0.0, 0.0, 0.0], vec![], Team::Enemy);

        // Player far away now.
        let player = world.create_entity();
        world.add_component(player, Transform::new(10.0, 0.0, 0.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Attack;
            ctrl.aggro_target = Some(player);
        }

        AiSystem::update(&mut world, 0.1);

        let ctrl = world.get_component::<EnemyController>(enemy).expect("ctrl");
        assert_eq!(ctrl.state, AiState::Chase);
    }

    // ── 10. Chase behavior sets MoveTarget toward player ──

    #[test]
    fn chase_sets_move_target_toward_player() {
        let mut world = test_world();

        let enemy = EnemyFactory::create_enemy(&mut world, [0.0, 0.0, 0.0], vec![], Team::Enemy);

        if let Some(ar) = world.get_component_mut::<AggroRadius>(enemy) {
            ar.radius = 20.0;
        }

        let player = world.create_entity();
        world.add_component(player, Transform::new(5.0, 0.0, 3.0));
        world.add_component(player, Team::Player);
        world.add_component(player, Health::new(100));

        if let Some(ctrl) = world.get_component_mut::<EnemyController>(enemy) {
            ctrl.state = AiState::Chase;
            ctrl.aggro_target = Some(player);
        }

        AiSystem::update(&mut world, 0.1);

        let mt = world
            .get_component::<MoveTarget>(enemy)
            .expect("move target");
        assert!((mt.x - 5.0).abs() < f32::EPSILON);
        assert!((mt.y - 0.0).abs() < f32::EPSILON);
        assert!((mt.z - 3.0).abs() < f32::EPSILON);
    }
}

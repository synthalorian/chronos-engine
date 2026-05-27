use crate::component::{CircleRadius, Damage, Dead, Gravity, Grounded, Health, Position, RigidBody, Sprite, Velocity};
use crate::entity::Entity;
use crate::spatial::{AABB, Quadtree, QuadtreeObject};
use crate::world::World;
use std::collections::VecDeque;

// ──────────────────────────────────────────────
// Event System
// ──────────────────────────────────────────────

/// An event that can be emitted and consumed by systems.
#[derive(Debug, Clone)]
pub enum Event {
    /// Two entities collided: (entity_a, entity_b).
    Collision(u32, u32),
    /// An entity took damage: (entity, amount).
    DamageTaken(u32, u32),
    /// An entity was destroyed: (entity).
    EntityDestroyed(u32),
    /// An entity died (health reached zero): (entity).
    EntityDied(u32),
    /// Custom event payload (system-specific).
    Custom(String, String),
}

/// A simple event bus for intra-frame communication.
///
/// Systems emit events during `update` and events are consumed
/// between system runs or at the end of the frame.
#[derive(Debug)]
pub struct EventBus {
    events: VecDeque<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            events: VecDeque::new(),
        }
    }

    /// Emit an event.
    pub fn emit(&mut self, event: Event) {
        self.events.push_back(event);
    }

    /// Drain all pending events.
    pub fn drain(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    /// Check if there are pending events.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Count pending events.
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

// ──────────────────────────────────────────────
// System Trait
// ──────────────────────────────────────────────

/// A system that operates on entities and components.
///
/// Systems are the "behavior" layer of the ECS. Each system
/// encapsulates a specific update logic (physics, rendering,
/// AI, etc.) and operates on components it needs.
pub trait System {
    /// Update the system for the current tick.
    fn update(&mut self, world: &mut World, events: &mut EventBus, dt: f64);

    /// A human-readable name for debugging/logging.
    fn name(&self) -> &str {
        "unnamed-system"
    }
}

// ──────────────────────────────────────────────
// Movement System
// ──────────────────────────────────────────────

/// Updates positions based on velocity every tick.
///
/// For every entity with both `Position` and `Velocity`,
/// this moves `position += velocity * dt`.
pub struct MovementSystem {
    pub name: String,
}

impl MovementSystem {
    pub fn new() -> Self {
        MovementSystem {
            name: "movement".to_string(),
        }
    }
}

impl System for MovementSystem {
    fn update(&mut self, world: &mut World, _events: &mut EventBus, dt: f64) {
        // Collect entity IDs first to avoid borrow conflicts
        let positions: Vec<(u32, f32, f32)> = world
            .query::<Position>()
            .map(|(e, p)| (e.index(), p.x, p.y))
            .collect();

        let velocities: Vec<(u32, f32, f32)> = world
            .query::<Velocity>()
            .map(|(e, v)| (e.index(), v.x, v.y))
            .collect();

        // Build velocity lookup
        let vel_map: std::collections::HashMap<u32, (f32, f32)> =
            velocities.into_iter().map(|(i, vx, vy)| (i, (vx, vy))).collect();

        for (idx, _px, _py) in positions {
            if let Some(&(vx, vy)) = vel_map.get(&idx) {
                if let Some(pos) = world.get_component_mut::<Position>(world.entity_from_index(idx)) {
                    pos.x += vx * dt as f32;
                    pos.y += vy * dt as f32;
                }
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Health System
// ──────────────────────────────────────────────

/// Processes damage and marks entities as dead.
///
/// Entities with the `Damage` tag have their damage applied
/// to their `Health`, and if health reaches zero, they get
/// a `Dead` component emitted.
pub struct HealthSystem {
    pub name: String,
}

impl HealthSystem {
    pub fn new() -> Self {
        HealthSystem {
            name: "health".to_string(),
        }
    }
}

impl System for HealthSystem {
    fn update(&mut self, world: &mut World, events: &mut EventBus, _dt: f64) {
        // Gather all damage events
        let damage_entities: Vec<(u32, u32)> = {
            let mut results = Vec::new();
            let q = world.query::<Damage>();
            for (entity, damage) in q {
                results.push((entity.index(), damage.0));
            }
            results
        };

        // Apply damage and remove the Damage component
        for (idx, amount) in &damage_entities {
            let entity = world.entity_from_index(*idx);
            if !world.entity_exists(entity) {
                continue;
            }

            if let Some(health) = world.get_component_mut::<Health>(entity) {
                health.take_damage(*amount);
                events.emit(Event::DamageTaken(*idx, *amount));

                if health.is_dead() {
                    world.add_component(entity, Dead);
                    events.emit(Event::EntityDied(*idx));
                }
            }

            // Remove the damage component after processing
            world.remove_component::<Damage>(entity);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Collision System (Quadtree)
// ──────────────────────────────────────────────

pub struct CollisionSystem {
    pub name: String,
    pub collision_radius: f32,
    pub world_bounds: AABB,
    active_pairs: std::collections::HashSet<(u32, u32)>,
}

impl CollisionSystem {
    pub fn new(radius: f32) -> Self {
        CollisionSystem {
            name: "collision".to_string(),
            collision_radius: radius,
            world_bounds: AABB::new(0.0, 0.0, 500.0, 500.0),
            active_pairs: std::collections::HashSet::new(),
        }
    }

    pub fn with_bounds(radius: f32, bounds: AABB) -> Self {
        CollisionSystem {
            name: "collision".to_string(),
            collision_radius: radius,
            world_bounds: bounds,
            active_pairs: std::collections::HashSet::new(),
        }
    }
}

impl System for CollisionSystem {
    fn update(&mut self, world: &mut World, events: &mut EventBus, _dt: f64) {
        let mut objects: Vec<QuadtreeObject> = Vec::new();
        for (entity, pos) in world.query::<Position>() {
            let radius = if world.has_component::<CircleRadius>(entity) {
                world.get_component::<CircleRadius>(entity).unwrap().0
            } else {
                self.collision_radius
            };
            objects.push(QuadtreeObject {
                entity: entity.index(),
                x: pos.x,
                y: pos.y,
                radius,
            });
        }

        if objects.is_empty() {
            return;
        }

        let aabb = self.world_bounds;
        let mut qt = Quadtree::new(aabb, 4, 6);
        qt.extend(objects.clone());

        let mut seen = std::collections::HashSet::new();
        let mut pairs = Vec::new();
        for obj in &objects {
            let search_radius = obj.radius * 2.0;
            let nearby = qt.query_circle(obj.x, obj.y, search_radius);
            for other in nearby {
                if other.entity > obj.entity {
                    let dx = obj.x - other.x;
                    let dy = obj.y - other.y;
                    let combined_radius = obj.radius + other.radius;
                    if dx * dx + dy * dy <= combined_radius * combined_radius {
                        if seen.insert((obj.entity, other.entity)) {
                            pairs.push((obj.entity, other.entity));
                        }
                    }
                }
            }
        }

        let mut new_pairs = std::collections::HashSet::new();
        for (a, b) in pairs {
            new_pairs.insert((a, b));
            if !self.active_pairs.contains(&(a, b)) {
                events.emit(Event::Collision(a, b));
            }
        }
        self.active_pairs = new_pairs;
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Death Cleanup System
// ──────────────────────────────────────────────

/// Removes entities with the Dead component.
pub struct DeathCleanupSystem {
    pub name: String,
}

impl DeathCleanupSystem {
    pub fn new() -> Self {
        DeathCleanupSystem {
            name: "death-cleanup".to_string(),
        }
    }
}

impl System for DeathCleanupSystem {
    fn update(&mut self, world: &mut World, events: &mut EventBus, _dt: f64) {
        let dead: Vec<Entity> = world.get_entities_with::<crate::component::Dead>();
        let count = dead.len();
        for entity in dead {
            world.destroy_entity_with_event(entity, events);
        }
        let _ = count;
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Debug Render System (terminal-based)
// ──────────────────────────────────────────────

/// Renders a simple terminal-based view of the world.
///
/// For entities with `Position` and `Sprite`, prints their
/// symbol at their position. Useful for debugging and demos.
pub struct DebugRenderSystem {
    pub name: String,
    pub width: usize,
    pub height: usize,
}

impl DebugRenderSystem {
    pub fn new(width: usize, height: usize) -> Self {
        DebugRenderSystem {
            name: "debug-render".to_string(),
            width,
            height,
        }
    }
}

impl System for DebugRenderSystem {
    fn update(&mut self, world: &mut World, _events: &mut EventBus, _dt: f64) {
        let mut grid = vec![vec![' '; self.width]; self.height];

        let sprites: Vec<(usize, usize, char)> = world
            .query::<Sprite>()
            .filter_map(|(e, s)| {
                world.get_component::<Position>(e).map(|p| {
                    let x = (p.x as usize).min(self.width.saturating_sub(1));
                    let y = (p.y as usize).min(self.height.saturating_sub(1));
                    (x, y, s.symbol)
                })
            })
            .collect();

        for (x, y, symbol) in &sprites {
            grid[*y][*x] = *symbol;
        }

        println!("\x1b[1;36m+{}+", "-".repeat(self.width));
        for row in &grid {
            let line: String = row.iter().collect();
            println!("\x1b[1;36m|\x1b[0m{}\x1b[1;36m|", line);
        }
        println!("\x1b[1;36m+{}+", "-".repeat(self.width));
        println!("\x1b[0m");
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Gravity System
// ──────────────────────────────────────────────

/// Applies gravity to entities with `RigidBody` (non-static) each tick.
///
/// For each entity with `Velocity` and `RigidBody` (where mass > 0),
/// adds the gravity acceleration from the `Gravity` component to the velocity.
pub struct GravitySystem {
    pub name: String,
}

impl GravitySystem {
    pub fn new() -> Self {
        GravitySystem {
            name: "gravity".to_string(),
        }
    }
}

impl System for GravitySystem {
    fn update(&mut self, world: &mut World, _events: &mut EventBus, dt: f64) {
        let gravity = world
            .query::<Gravity>()
            .map(|(_, g)| (g.x, g.y))
            .next();

        let (gx, gy) = if let Some(g) = gravity { g } else { return };
        let dt = dt as f32;

        let bodies: Vec<(u32, f32, f32, f32, f32)> = world
            .query::<RigidBody>()
            .filter(|(_, rb)| !rb.is_static())
            .filter_map(|(e, rb)| {
                world.get_component::<Velocity>(e).map(|v| (e.index(), rb.mass, v.x, v.y, rb.damping))
            })
            .collect();

        for (idx, mass, vx, vy, damping) in bodies {
            if mass == 0.0 {
                continue;
            }
            let entity = world.entity_from_index(idx);
            if let Some(vel) = world.get_component_mut::<Velocity>(entity) {
                vel.x = (vx + gx * dt) * (1.0 - damping * dt);
                vel.y = (vy + gy * dt) * (1.0 - damping * dt);
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Platformer System
// ──────────────────────────────────────────────

/// Platformer physics: ground check, jump impulse, and wall-slide.
///
/// Processes entities with `RigidBody` and `Grounded` components.
/// A ground check re-applies the `Grounded` tag if the entity
/// is adjacent to a static object below it (within a threshold).
/// If the entity has a `Damage` component, it's treated as a jump impulse.
pub struct PlatformerSystem {
    pub name: String,
    pub ground_threshold: f32,
    pub jump_force: f32,
}

impl PlatformerSystem {
    pub fn new() -> Self {
        PlatformerSystem {
            name: "platformer".to_string(),
            ground_threshold: 2.0,
            jump_force: 5.0,
        }
    }
}

impl System for PlatformerSystem {
    fn update(&mut self, world: &mut World, _events: &mut EventBus, _dt: f64) {
        let positions: Vec<(u32, f32, f32)> = world
            .query::<Position>()
            .map(|(e, p)| (e.index(), p.x, p.y))
            .collect();

        let grounded_entities: Vec<u32> = world
            .query::<Grounded>()
            .map(|(e, _)| e.index())
            .collect();

        for idx in grounded_entities {
            let still_grounded = positions.iter().any(|&(other_idx, ox, oy)| {
                if other_idx == idx {
                    return false;
                }
                let rb = world.get_component::<RigidBody>(world.entity_from_index(other_idx));
                if let Some(rb) = rb {
                    if rb.is_static() {
                        let entity_pos = positions.iter().find(|&&(i, _, _)| i == idx);
                        let dx = ox - entity_pos.map(|&(_i, px, _py)| px).unwrap_or(0.0);
                        let dy = oy - entity_pos.map(|&(_i, _px, py)| py).unwrap_or(0.0);
                        dx.abs() < self.ground_threshold && dy > 0.0 && dy < self.ground_threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            if !still_grounded {
                world.remove_component::<Grounded>(world.entity_from_index(idx));
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// Raycast System
// ──────────────────────────────────────────────

/// Raycasting system for point-in-world queries.
pub struct RaycastSystem {
    pub name: String,
    pub world_bounds: AABB,
    pub capacity: usize,
    pub max_depth: usize,
}

impl RaycastSystem {
    pub fn new(world_bounds: AABB, capacity: usize, max_depth: usize) -> Self {
        RaycastSystem {
            name: "raycast".to_string(),
            world_bounds,
            capacity,
            max_depth,
        }
    }

    pub fn new_default(width: f32, height: f32) -> Self {
        RaycastSystem::new(AABB::new(0.0, 0.0, width, height), 4, 6)
    }

    pub fn find_at_point(&self, world: &World, x: f32, y: f32) -> Vec<(u32, f32)> {
        let mut objects: Vec<QuadtreeObject> = Vec::new();
        for (entity, pos) in world.query::<Position>() {
            let radius = if world.has_component::<CircleRadius>(entity) {
                world.get_component::<CircleRadius>(entity).unwrap().0
            } else {
                5.0
            };
            objects.push(QuadtreeObject {
                entity: entity.index(),
                x: pos.x,
                y: pos.y,
                radius,
            });
        }

        let mut qt = Quadtree::new(self.world_bounds, self.capacity, self.max_depth);
        qt.extend(objects);

        let hits = qt.query_point(x, y);
        hits.iter().map(|obj| (obj.entity, obj.radius)).collect()
    }

    pub fn cast(&self, world: &World, ray: crate::spatial::Ray) -> Vec<(u32, crate::spatial::RaycastHit)> {
        let mut objects: Vec<QuadtreeObject> = Vec::new();
        for (entity, pos) in world.query::<Position>() {
            let radius = if world.has_component::<CircleRadius>(entity) {
                world.get_component::<CircleRadius>(entity).unwrap().0
            } else {
                5.0
            };
            objects.push(QuadtreeObject {
                entity: entity.index(),
                x: pos.x,
                y: pos.y,
                radius,
            });
        }

        let mut qt = Quadtree::new(self.world_bounds, self.capacity, self.max_depth);
        qt.extend(objects);

        qt.raycast_with_entities(ray)
    }
}

impl System for RaycastSystem {
    fn update(&mut self, _world: &mut World, _events: &mut EventBus, _dt: f64) {}

    fn name(&self) -> &str {
        &self.name
    }
}

// ──────────────────────────────────────────────
// System Scheduler / Pipeline
// ──────────────────────────────────────────────

/// Defines the execution phase for a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPhase {
    PreUpdate,
    Update,
    PostUpdate,
    Render,
    Cleanup,
}

/// A scheduled system with its phase.
pub struct ScheduledSystem {
    pub system: Box<dyn System>,
    pub phase: SystemPhase,
}

/// The game loop — drives the ECS tick cycle with delta time.
///
/// The loop structure:
/// 1. PreUpdate phase (input, event processing)
/// 2. Update phase (movement, AI, physics, collision)
/// 3. PostUpdate phase (health, death, cleanup)
/// 4. Render phase (display, logging)
pub struct GameLoop {
    systems: Vec<ScheduledSystem>,
    pub event_bus: EventBus,
    pub paused: bool,
    pub tick_count: u64,
}

impl GameLoop {
    pub fn new() -> Self {
        GameLoop {
            systems: Vec::new(),
            event_bus: EventBus::new(),
            paused: false,
            tick_count: 0,
        }
    }

    /// Add a system to the pipeline.
    pub fn add_system(&mut self, system: impl System + 'static, phase: SystemPhase) {
        self.systems.push(ScheduledSystem {
            system: Box::new(system),
            phase,
        });
    }

    /// Run a single tick with the given delta time.
    pub fn tick(&mut self, world: &mut World, dt: f64) {
        if self.paused {
            return;
        }

        self.tick_count += 1;
        let phases = [
            SystemPhase::PreUpdate,
            SystemPhase::Update,
            SystemPhase::PostUpdate,
            SystemPhase::Render,
            SystemPhase::Cleanup,
        ];

        for phase in &phases {
            for scheduled in &mut self.systems {
                if scheduled.phase == *phase {
                    scheduled.system.update(world, &mut self.event_bus, dt);
                }
            }
        }
    }

    /// Run N ticks with fixed timestep.
    pub fn run_fixed(&mut self, world: &mut World, ticks: u64, dt: f64) {
        for _ in 0..ticks {
            self.tick(world, dt);
        }
    }

    /// Get system names grouped by phase for debugging.
    pub fn system_report(&self) -> Vec<(SystemPhase, Vec<&str>)> {
        let mut report: Vec<(SystemPhase, Vec<&str>)> = Vec::new();
        let phases = [
            SystemPhase::PreUpdate,
            SystemPhase::Update,
            SystemPhase::PostUpdate,
            SystemPhase::Render,
            SystemPhase::Cleanup,
        ];

        for phase in &phases {
            let names: Vec<&str> = self
                .systems
                .iter()
                .filter(|s| &s.phase == phase)
                .map(|s| s.system.name())
                .collect();
            if !names.is_empty() {
                report.push((*phase, names));
            }
        }

        report
    }
}

// ──────────────────────────────────────────────
// Stage/Tick-based simulation (for deterministic RTS)
// ──────────────────────────────────────────────

/// A tick-based scheduler for deterministic games (RTS, strategy).
///
/// Runs systems in strict order every tick with a fixed dt.
pub struct TickScheduler {
    systems: Vec<Box<dyn System>>,
    pub tick_count: u64,
    pub dt: f64,
}

impl TickScheduler {
    pub fn new(dt: f64) -> Self {
        TickScheduler {
            systems: Vec::new(),
            tick_count: 0,
            dt,
        }
    }

    pub fn add(&mut self, system: impl System + 'static) {
        self.systems.push(Box::new(system));
    }

    pub fn tick(&mut self, world: &mut World, events: &mut EventBus) {
        self.tick_count += 1;
        for system in &mut self.systems {
            system.update(world, events, self.dt);
        }
    }

    pub fn run(&mut self, world: &mut World, events: &mut EventBus, ticks: u64) {
        for _ in 0..ticks {
            self.tick(world, events);
        }
    }
}

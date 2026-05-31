#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Undo/Redo command system for the Chronos Engine editor.
//!
//! Implements the Command pattern with a dual-stack architecture.
//! Each editor operation (move, create, destroy, modify) is captured
//! as a concrete `EditorCommand` that can reverse and replay its
//! effect on the `World`. Commands are type-erased via
//! `Box<dyn EditorCommand>` for heterogeneous storage.
//!
//! # Architecture
//!
//! ```text
//! [UndoStack]
//!   ├── undo_stack: Vec<Box<dyn EditorCommand>>   ← pop to undo
//!   └── redo_stack: Vec<Box<dyn EditorCommand>>   ← pop to redo
//! ```
//!
//! Pushing a new command clears the redo stack (standard editor
//! convention). Oldest entries are trimmed when `max_history` is
//! exceeded to bound memory usage.

use std::any::TypeId;
use std::cell::Cell;

use crate::component::Position;
use crate::entity::Entity;
use crate::world::World;

// ── UndoAction ───────────────────────────────────────────────────

/// Describes the category of an undo/redo operation.
///
/// Lightweight descriptor for UI display and logging. The actual
/// undo data lives in the concrete `EditorCommand` implementations.
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// An entity was created.
    CreateEntity { entity: Entity },
    /// An entity was destroyed.
    DestroyEntity { entity: Entity },
    /// An entity was repositioned.
    MoveEntity {
        entity: Entity,
        from: Position,
        to: Position,
    },
    /// A component was modified.
    ModifyComponent { entity: Entity, type_name: String },
}

// ── EditorCommand Trait ─────────────────────────────────────────

/// Trait for editor operations that can be undone and redone.
///
/// Each command captures enough state to reverse and replay its
/// effect. Commands are stored as trait objects in the `UndoStack`,
/// enabling heterogeneous collections of different command types.
pub trait EditorCommand {
    /// Human-readable description of this command.
    fn describe(&self) -> String;
    /// Reverse the effect of this command on the world.
    fn undo(&self, world: &mut World);
    /// Re-apply the effect of this command on the world.
    fn redo(&self, world: &mut World);
    /// Clone this command into a boxed trait object.
    fn box_clone(&self) -> Box<dyn EditorCommand>;
}

impl Clone for Box<dyn EditorCommand> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

// ── Component Snapshot (type-erased restore) ────────────────────

/// Trait for type-erased component snapshots.
///
/// Each snapshot wraps a cloned component value and knows how to
/// restore it. This avoids the need to clone `Box<dyn Any>`, which
/// is not possible in Rust, by capturing the concrete type at
/// construction time.
trait ComponentSnapshot: Send + Sync {
    /// Restore the component onto `entity` in `world`.
    fn restore(&self, world: &mut World, entity: Entity);
    /// Clone this snapshot into a new boxed trait object.
    fn clone_boxed(&self) -> Box<dyn ComponentSnapshot>;
    /// The `std::any::type_name` of the wrapped component.
    fn type_name(&self) -> &'static str;
}

/// Generic snapshot wrapping any `Clone + Send + Sync + 'static` value.
struct TypedSnapshot<T> {
    value: T,
}

impl<T: Clone + Send + Sync + 'static> ComponentSnapshot for TypedSnapshot<T> {
    fn restore(&self, world: &mut World, entity: Entity) {
        world.add_component(entity, self.value.clone());
    }

    fn clone_boxed(&self) -> Box<dyn ComponentSnapshot> {
        Box::new(TypedSnapshot {
            value: self.value.clone(),
        })
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

// ── MoveEntityCommand ───────────────────────────────────────────

/// Command to change an entity's `Position` component.
///
/// Stores both old and new positions. If the entity had no `Position`
/// before the move, `old_position` is `None` and undo will remove it.
pub struct MoveEntityCommand {
    entity: Entity,
    old_position: Option<Position>,
    new_position: Option<Position>,
}

impl MoveEntityCommand {
    /// Create a new move command.
    ///
    /// `old_position`: the position before the move (`None` if absent).
    /// `new_position`: the position after the move (`None` to remove).
    pub fn new(
        entity: Entity,
        old_position: Option<Position>,
        new_position: Option<Position>,
    ) -> Self {
        Self {
            entity,
            old_position,
            new_position,
        }
    }
}

impl EditorCommand for MoveEntityCommand {
    fn describe(&self) -> String {
        let old = self
            .old_position
            .map(|p| format!("({:.1}, {:.1})", p.x, p.y))
            .unwrap_or_else(|| "none".into());
        let new = self
            .new_position
            .map(|p| format!("({:.1}, {:.1})", p.x, p.y))
            .unwrap_or_else(|| "none".into());
        format!(
            "Move entity {} from {} to {}",
            self.entity.index(),
            old,
            new
        )
    }

    fn undo(&self, world: &mut World) {
        if !world.entity_exists(self.entity) {
            return;
        }
        match self.old_position {
            Some(pos) => {
                if world.has_component::<Position>(self.entity) {
                    *world
                        .get_component_mut::<Position>(self.entity)
                        .expect("entity has Position component after has_component check") = pos;
                } else {
                    world.add_component(self.entity, pos);
                }
            }
            None => {
                let _ = world.remove_component::<Position>(self.entity);
            }
        }
    }

    fn redo(&self, world: &mut World) {
        if !world.entity_exists(self.entity) {
            return;
        }
        match self.new_position {
            Some(pos) => {
                if world.has_component::<Position>(self.entity) {
                    *world
                        .get_component_mut::<Position>(self.entity)
                        .expect("entity has Position component after has_component check") = pos;
                } else {
                    world.add_component(self.entity, pos);
                }
            }
            None => {
                let _ = world.remove_component::<Position>(self.entity);
            }
        }
    }

    fn box_clone(&self) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity: self.entity,
            old_position: self.old_position,
            new_position: self.new_position,
        })
    }
}

// ── CreateEntityCommand ─────────────────────────────────────────

/// Command to create a new entity.
///
/// Uses interior mutability (`Cell`) to track the entity handle,
/// which may change after redo due to slot recycling.
pub struct CreateEntityCommand {
    entity: Cell<Entity>,
}

impl CreateEntityCommand {
    /// Wrap an already-created entity into a command.
    ///
    /// Call this *after* `world.create_entity()` so the command
    /// knows which entity to destroy on undo.
    pub fn new(entity: Entity) -> Self {
        Self {
            entity: Cell::new(entity),
        }
    }

    /// The current entity handle (may differ from the original after redo).
    pub fn entity(&self) -> Entity {
        self.entity.get()
    }
}

impl EditorCommand for CreateEntityCommand {
    fn describe(&self) -> String {
        let e = self.entity.get();
        format!(
            "Create entity (index {}, gen {})",
            e.index(),
            e.generation()
        )
    }

    fn undo(&self, world: &mut World) {
        let entity = self.entity.get();
        if world.entity_exists(entity) {
            world.destroy_entity(entity);
        }
    }

    fn redo(&self, world: &mut World) {
        let new_entity = world.create_entity();
        self.entity.set(new_entity);
    }

    fn box_clone(&self) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity: Cell::new(self.entity.get()),
        })
    }
}

// ── DestroyEntityCommand ────────────────────────────────────────

/// Command to destroy an entity and all its components.
///
/// Snapshots component data via type-erased `ComponentSnapshot`
/// objects. On undo, a new entity is created and all components
/// are restored. Uses interior mutability to track the potentially
/// recycled entity handle.
pub struct DestroyEntityCommand {
    entity: Cell<Entity>,
    snapshots: Vec<Box<dyn ComponentSnapshot>>,
}

impl DestroyEntityCommand {
    /// Begin building a destroy command for `entity` (no snapshots yet).
    ///
    /// Call `with_snapshot` for each component the entity owns, then
    /// push the command onto the undo stack.
    pub fn new(entity: Entity) -> Self {
        Self {
            entity: Cell::new(entity),
            snapshots: Vec::new(),
        }
    }

    /// Attach a cloned component snapshot to be restored on undo.
    pub fn with_snapshot<T: Clone + Send + Sync + 'static>(mut self, value: T) -> Self {
        self.snapshots.push(Box::new(TypedSnapshot { value }));
        self
    }

    /// Convenience: capture a component from the world if present.
    pub fn with_world_snapshot<T: Clone + Send + Sync + 'static>(mut self, world: &World) -> Self {
        if let Some(comp) = world.get_component::<T>(self.entity.get()) {
            self.snapshots.push(Box::new(TypedSnapshot {
                value: comp.clone(),
            }));
        }
        self
    }

    /// The current entity handle.
    pub fn entity(&self) -> Entity {
        self.entity.get()
    }

    /// Number of component snapshots stored.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl EditorCommand for DestroyEntityCommand {
    fn describe(&self) -> String {
        let e = self.entity.get();
        format!(
            "Destroy entity (index {}, gen {}) with {} component(s)",
            e.index(),
            e.generation(),
            self.snapshots.len()
        )
    }

    fn undo(&self, world: &mut World) {
        // Create a new entity (may differ from original due to recycling).
        let new_entity = world.create_entity();
        self.entity.set(new_entity);
        // Restore all component snapshots onto the new entity.
        for snapshot in &self.snapshots {
            snapshot.restore(world, new_entity);
        }
    }

    fn redo(&self, world: &mut World) {
        let entity = self.entity.get();
        if world.entity_exists(entity) {
            world.destroy_entity(entity);
        }
    }

    fn box_clone(&self) -> Box<dyn EditorCommand> {
        Box::new(DestroyEntityCommand {
            entity: Cell::new(self.entity.get()),
            snapshots: self.snapshots.iter().map(|s| s.clone_boxed()).collect(),
        })
    }
}

// ── ModifyComponentCommand ──────────────────────────────────────

/// Generic command for modifying any `Clone + Send + Sync` component.
///
/// Stores old and new values for precise undo/redo without needing
/// type-specific command implementations for every component.
pub struct ModifyComponentCommand<T> {
    entity: Entity,
    old_value: T,
    new_value: T,
    component_type_id: TypeId,
    type_name: &'static str,
}

impl<T: Clone + Send + Sync + 'static> ModifyComponentCommand<T> {
    /// Create a new modify command.
    pub fn new(entity: Entity, old_value: T, new_value: T) -> Self {
        Self {
            entity,
            old_value,
            new_value,
            component_type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }

    /// The `TypeId` of the modified component.
    pub fn component_type_id(&self) -> TypeId {
        self.component_type_id
    }
}

impl<T: Clone + Send + Sync + 'static> EditorCommand for ModifyComponentCommand<T> {
    fn describe(&self) -> String {
        format!(
            "Modify {} on entity {}",
            self.type_name,
            self.entity.index()
        )
    }

    fn undo(&self, world: &mut World) {
        if !world.entity_exists(self.entity) {
            return;
        }
        if let Some(comp) = world.get_component_mut::<T>(self.entity) {
            *comp = self.old_value.clone();
        }
    }

    fn redo(&self, world: &mut World) {
        if !world.entity_exists(self.entity) {
            return;
        }
        if let Some(comp) = world.get_component_mut::<T>(self.entity) {
            *comp = self.new_value.clone();
        }
    }

    fn box_clone(&self) -> Box<dyn EditorCommand> {
        Box::new(Self {
            entity: self.entity,
            old_value: self.old_value.clone(),
            new_value: self.new_value.clone(),
            component_type_id: self.component_type_id,
            type_name: self.type_name,
        })
    }
}

// ── UndoStack ───────────────────────────────────────────────────

/// Dual-stack undo/redo history for editor commands.
///
/// Pushing a new command clears the redo stack (standard editor
/// convention). Oldest entries are trimmed when `max_history` is
/// exceeded to bound memory usage.
///
/// # Example
///
/// ```ignore
/// let mut stack = UndoStack::new();
/// stack.push(Box::new(MoveEntityCommand::new(e, old, new)));
/// stack.undo(&mut world);  // reverses the move
/// stack.redo(&mut world);  // re-applies it
/// ```
pub struct UndoStack {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
    max_history: usize,
}

impl UndoStack {
    /// Create a new stack with a default history limit of 100.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    /// Set a custom history limit. Oldest commands are trimmed on push.
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max.max(1);
        self
    }

    /// Record a new command. Clears the redo stack.
    pub fn push(&mut self, cmd: Box<dyn EditorCommand>) {
        self.redo_stack.clear();
        self.undo_stack.push(cmd);
        self.trim();
    }

    /// Undo the most recent command.
    ///
    /// Returns the command description if there was one to undo.
    pub fn undo(&mut self, world: &mut World) -> Option<String> {
        let cmd = self.undo_stack.pop()?;
        let desc = cmd.describe();
        cmd.undo(world);
        self.redo_stack.push(cmd);
        Some(desc)
    }

    /// Redo the most recently undone command.
    ///
    /// Returns the command description if there was one to redo.
    pub fn redo(&mut self, world: &mut World) -> Option<String> {
        let cmd = self.redo_stack.pop()?;
        let desc = cmd.describe();
        cmd.redo(world);
        self.undo_stack.push(cmd);
        Some(desc)
    }

    /// Whether there are commands available to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether there are commands available to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Number of commands on the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of commands on the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Description of the top-most undo command, if any.
    pub fn description(&self) -> Option<String> {
        self.undo_stack.last().map(|c| c.describe())
    }

    /// Trim oldest entries if the undo stack exceeds `max_history`.
    fn trim(&mut self) {
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{Health, Velocity};

    // Helper: create a world with one entity that has a Position.
    fn world_with_positioned_entity() -> (World, Entity, Position) {
        let mut world = World::new();
        let entity = world.create_entity();
        let pos = Position::new(10.0, 20.0);
        world.add_component(entity, pos);
        (world, entity, pos)
    }

    // ── 1. new() defaults ──

    #[test]
    fn test_new_defaults() {
        let stack = UndoStack::new();
        assert!(!stack.can_undo(), "new stack should not be undoable");
        assert!(!stack.can_redo(), "new stack should not be redoable");
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert!(stack.description().is_none());
    }

    // ── 2. push + undo + redo cycle ──

    #[test]
    fn test_push_undo_redo_cycle() {
        let (mut world, entity, original_pos) = world_with_positioned_entity();
        let mut stack = UndoStack::new();
        let new_pos = Position::new(50.0, 60.0);
        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(original_pos),
            Some(new_pos),
        )));

        // Undo reverts to old position.
        let desc = stack.undo(&mut world).expect("should undo");
        assert!(desc.contains("Move"));
        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 10.0).abs() < 0.01 && (pos.y - 20.0).abs() < 0.01);

        // Redo moves forward again.
        let desc = stack.redo(&mut world).expect("should redo");
        assert!(desc.contains("Move"));
        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 50.0).abs() < 0.01 && (pos.y - 60.0).abs() < 0.01);
    }

    // ── 3. push clears redo stack ──

    #[test]
    fn test_push_clears_redo() {
        let (mut world, entity, original_pos) = world_with_positioned_entity();
        let mut stack = UndoStack::new();

        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(original_pos),
            Some(Position::new(1.0, 2.0)),
        )));
        stack.undo(&mut world).unwrap();
        assert_eq!(stack.redo_count(), 1, "should have one redo");

        // Push a new command — redo stack must be cleared.
        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(original_pos),
            Some(Position::new(3.0, 4.0)),
        )));
        assert_eq!(stack.redo_count(), 0, "push must clear redo stack");
        assert_eq!(stack.undo_count(), 1);
    }

    // ── 4. can_undo / can_redo ──

    #[test]
    fn test_can_undo_can_redo() {
        let (mut world, entity, pos) = world_with_positioned_entity();
        let mut stack = UndoStack::new();

        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(pos),
            Some(Position::new(5.0, 5.0)),
        )));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo(&mut world).unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    // ── 5. max_history trimming ──

    #[test]
    fn test_max_history_trimming() {
        let mut world = World::new();
        let mut stack = UndoStack::new().with_max_history(3);

        for _ in 0..5 {
            let e = world.create_entity();
            stack.push(Box::new(CreateEntityCommand::new(e)));
        }

        assert_eq!(stack.undo_count(), 3, "should trim to max_history");
    }

    // ── 6. MoveEntityCommand undo/redo ──

    #[test]
    fn test_move_entity_command() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(0.0, 0.0));

        let cmd = MoveEntityCommand::new(
            entity,
            Some(Position::new(0.0, 0.0)),
            Some(Position::new(42.0, 84.0)),
        );

        // The move already happened in the world.
        *world.get_component_mut::<Position>(entity).unwrap() = Position::new(42.0, 84.0);

        cmd.undo(&mut world);
        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 0.0).abs() < 0.01 && (pos.y - 0.0).abs() < 0.01);

        cmd.redo(&mut world);
        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 42.0).abs() < 0.01 && (pos.y - 84.0).abs() < 0.01);
    }

    // ── 7. ModifyComponentCommand undo/redo ──

    #[test]
    fn test_modify_component_undo_redo() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Health::new(100));

        let cmd = ModifyComponentCommand::new(
            entity,
            Health::new(100),
            Health {
                current: 30,
                max: 100,
            },
        );

        // Simulate: modification already applied.
        world.get_component_mut::<Health>(entity).unwrap().current = 30;

        cmd.undo(&mut world);
        let hp = world.get_component::<Health>(entity).unwrap();
        assert_eq!(hp.current, 100, "undo should restore old value");

        cmd.redo(&mut world);
        let hp = world.get_component::<Health>(entity).unwrap();
        assert_eq!(hp.current, 30, "redo should set new value");
    }

    // ── 8. clear() ──

    #[test]
    fn test_clear() {
        let (mut world, entity, pos) = world_with_positioned_entity();
        let mut stack = UndoStack::new();

        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(pos),
            Some(Position::new(1.0, 1.0)),
        )));
        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(Position::new(1.0, 1.0)),
            Some(Position::new(2.0, 2.0)),
        )));
        stack.undo(&mut world).unwrap();

        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 1);

        stack.clear();
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    // ── 9. DestroyEntityCommand undo/redo ──

    #[test]
    fn test_destroy_entity_command() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(7.0, 8.0));
        world.add_component(entity, Velocity::new(1.0, -1.0));

        let cmd = DestroyEntityCommand::new(entity)
            .with_snapshot(Position::new(7.0, 8.0))
            .with_snapshot(Velocity::new(1.0, -1.0));

        // Destroy the entity in the world.
        world.destroy_entity(entity);
        assert!(!world.entity_exists(entity));

        // Undo: recreate entity with components.
        cmd.undo(&mut world);
        let new_entity = cmd.entity();
        assert!(world.entity_exists(new_entity));
        assert_eq!(cmd.snapshot_count(), 2);

        let pos = world.get_component::<Position>(new_entity).unwrap();
        assert!((pos.x - 7.0).abs() < 0.01 && (pos.y - 8.0).abs() < 0.01);
        let vel = world.get_component::<Velocity>(new_entity).unwrap();
        assert!((vel.x - 1.0).abs() < 0.01 && (vel.y - (-1.0_f32)).abs() < 0.01);

        // Redo: destroy again.
        cmd.redo(&mut world);
        assert!(!world.entity_exists(new_entity));
    }

    // ── 10. CreateEntityCommand undo/redo ──

    #[test]
    fn test_create_entity_command() {
        let mut world = World::new();
        let entity = world.create_entity();
        let cmd = CreateEntityCommand::new(entity);

        assert_eq!(world.entity_count(), 1);

        // Undo: destroy.
        cmd.undo(&mut world);
        assert_eq!(world.entity_count(), 0);

        // Redo: create new entity (handle may differ).
        cmd.redo(&mut world);
        assert_eq!(world.entity_count(), 1);
        let new_entity = cmd.entity();
        assert!(world.entity_exists(new_entity));
    }

    // ── 11. description() returns top of undo stack ──

    #[test]
    fn test_description() {
        let mut world = World::new();
        let entity = world.create_entity();
        let mut stack = UndoStack::new();

        assert!(stack.description().is_none());

        stack.push(Box::new(CreateEntityCommand::new(entity)));
        assert!(stack.description().unwrap().contains("Create entity"));

        stack.push(Box::new(MoveEntityCommand::new(
            entity,
            Some(Position::new(0.0, 0.0)),
            Some(Position::new(1.0, 1.0)),
        )));
        assert!(stack.description().unwrap().contains("Move"));
    }

    // ── 12. ModifyComponentCommand type_id matches ──

    #[test]
    fn test_modify_component_type_id() {
        let entity = Entity::new(0, 0);
        let cmd = ModifyComponentCommand::new(
            entity,
            Health::new(100),
            Health {
                current: 50,
                max: 100,
            },
        );
        assert_eq!(cmd.component_type_id(), TypeId::of::<Health>());
    }

    // ── 13. Commands gracefully no-op on dead entities ──

    #[test]
    fn test_noop_on_dead_entity() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(5.0, 5.0));

        let move_cmd = MoveEntityCommand::new(
            entity,
            Some(Position::new(5.0, 5.0)),
            Some(Position::new(99.0, 99.0)),
        );
        let modify_cmd =
            ModifyComponentCommand::new(entity, Position::new(5.0, 5.0), Position::new(99.0, 99.0));

        // Destroy entity — commands should gracefully no-op.
        world.destroy_entity(entity);
        move_cmd.undo(&mut world);
        move_cmd.redo(&mut world);
        modify_cmd.undo(&mut world);
        modify_cmd.redo(&mut world);
        // No panic = pass.
    }

    // ── 14. Box<dyn EditorCommand> clone works ──

    #[test]
    fn test_box_clone() {
        let cmd: Box<dyn EditorCommand> = Box::new(MoveEntityCommand::new(
            Entity::new(0, 0),
            Some(Position::new(1.0, 2.0)),
            Some(Position::new(3.0, 4.0)),
        ));
        let cloned = cmd.clone();
        assert_eq!(cmd.describe(), cloned.describe());
    }

    // ── 15. DestroyEntityCommand with_world_snapshot ──

    #[test]
    fn test_destroy_with_world_snapshot() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(11.0, 22.0));
        world.add_component(entity, Velocity::new(3.0, 4.0));

        let cmd = DestroyEntityCommand::new(entity)
            .with_world_snapshot::<Position>(&world)
            .with_world_snapshot::<Velocity>(&world);

        assert_eq!(cmd.snapshot_count(), 2);

        world.destroy_entity(entity);
        cmd.undo(&mut world);

        let new_entity = cmd.entity();
        let pos = world.get_component::<Position>(new_entity).unwrap();
        assert!((pos.x - 11.0).abs() < 0.01);
        let vel = world.get_component::<Velocity>(new_entity).unwrap();
        assert!((vel.x - 3.0).abs() < 0.01);
    }
}

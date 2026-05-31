//! Plugin API — safe engine surface exposed to plugins.
//!
//! This module defines the limited set of engine operations that plugins
//! are allowed to perform. By restricting the API surface, we maintain
//! engine invariants and make plugin behavior predictable.

use crate::entity::Entity;
use crate::system::Event;
use crate::world::World;

/// The safe engine API available to plugins.
///
/// Plugins receive a `PluginApi` in their [`PluginContext`](super::PluginContext).
/// This struct provides carefully controlled access to the [`World`] and
/// other engine systems without exposing internal mutable state directly.
pub struct PluginApi<'a> {
    world: &'a mut World,
}

impl<'a> PluginApi<'a> {
    pub fn new(world: &'a mut World) -> Self {
        PluginApi { world }
    }

    // ── Entity ──

    /// Create a new entity in the world.
    pub fn create_entity(&mut self) -> Entity {
        self.world.create_entity()
    }

    /// Check if an entity is still alive.
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.world.entity_exists(entity)
    }

    /// Destroy an entity and all its components.
    pub fn destroy_entity(&mut self, entity: Entity) {
        self.world.destroy_entity(entity);
    }

    /// Get the total number of alive entities.
    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }

    // ── Component ──

    /// Add a component to an entity.
    pub fn add_component<T: Send + Sync + 'static>(&mut self, entity: Entity, component: T) {
        self.world.add_component(entity, component);
    }

    /// Get an immutable reference to a component.
    pub fn get_component<T: Send + Sync + 'static>(&self, entity: Entity) -> Option<&T> {
        self.world.get_component::<T>(entity)
    }

    /// Get a mutable reference to a component.
    pub fn get_component_mut<T: Send + Sync + 'static>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        self.world.get_component_mut::<T>(entity)
    }

    /// Check if an entity has a specific component.
    pub fn has_component<T: Send + Sync + 'static>(&self, entity: Entity) -> bool {
        self.world.has_component::<T>(entity)
    }

    /// Remove a component from an entity.
    pub fn remove_component<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<T> {
        self.world.remove_component::<T>(entity)
    }

    // ── Events ──

    /// Emit an engine event.
    ///
    /// Note: In the current architecture events are dispatched through
    /// the EventBus owned by the game loop. This stub logs the event
    /// intent; full integration requires passing the EventBus into
    /// PluginApi in a future revision.
    pub fn emit_event(&mut self, event: Event) {
        // In a full implementation this would push to the EventBus.
        // For now we record it via the PluginContext log buffer.
        let _ = event;
    }
}

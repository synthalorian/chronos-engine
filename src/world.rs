#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::entity::Entity;
use crate::storage::StorageRegistry;
use crate::system::EventBus;
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};

/// Represents a unique combination of component types.
///
/// An archetype groups all entities that have the same set of component
/// types. When an entity gains or loses a component, it moves between
/// archetypes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArchetypeKey(pub Vec<TypeId>);

impl ArchetypeKey {
    pub fn new(component_ids: Vec<TypeId>) -> Self {
        let mut ids = component_ids;
        ids.sort();
        ArchetypeKey(ids)
    }
}

/// A group of entities that share the same component signature.
#[derive(Debug)]
pub struct Archetype {
    /// The component type IDs that define this archetype (sorted).
    pub key: ArchetypeKey,
    /// Entities in this archetype.
    pub entities: Vec<Entity>,
}

impl Archetype {
    pub fn new(key: ArchetypeKey) -> Self {
        Archetype {
            key,
            entities: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        if !self.entities.contains(&entity) {
            self.entities.push(entity);
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.entities.retain(|e| *e != entity);
    }

    /// Get all entities in this archetype (immutable references).
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }
}

/// Registry that maps component signatures to archetypes.
#[derive(Debug)]
pub struct ArchetypeRegistry {
    archetypes: HashMap<ArchetypeKey, Archetype>,
}

impl Default for ArchetypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchetypeRegistry {
    pub fn new() -> Self {
        ArchetypeRegistry {
            archetypes: HashMap::new(),
        }
    }

    /// Get or create an archetype for the given component signature.
    pub fn get_or_create(&mut self, key: ArchetypeKey) -> &mut Archetype {
        self.archetypes
            .entry(key.clone())
            .or_insert_with(|| Archetype::new(key))
    }

    /// Get an archetype by its key.
    pub fn get(&self, key: &ArchetypeKey) -> Option<&Archetype> {
        self.archetypes.get(key)
    }

    /// Iterate over all archetypes.
    pub fn iter(&self) -> impl Iterator<Item = (&ArchetypeKey, &Archetype)> {
        self.archetypes.iter()
    }

    /// Iterate over all archetypes mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ArchetypeKey, &mut Archetype)> {
        self.archetypes.iter_mut()
    }
}

/// The World — the central registry of entities and components.
///
/// The World is the heart of the ECS. It manages:
/// 1. Entity lifecycle (creation, destruction with slot reuse)
/// 2. Component storage (where data lives)
/// 3. Entity-component mapping (which entity has which components)
/// 4. Archetype tracking for efficient queries
/// 5. Multi-component queries via archetype intersection
#[derive(Default)]
pub struct World {
    /// Registry of component storages by type.
    pub storage: StorageRegistry,

    /// Entity slots. None means the slot is free.
    entities: Vec<Option<Entity>>,

    /// Free entity indices ready for reuse.
    pub free_slots: VecDeque<u32>,

    /// Generations for each slot (incremented when a slot is freed).
    pub generations: Vec<u32>,

    /// Archetype tracking.
    pub archetypes: ArchetypeRegistry,

    /// Maps entity index → archetype key.
    entity_archetype_keys: HashMap<u32, ArchetypeKey>,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        World {
            storage: StorageRegistry::new(),
            entities: Vec::new(),
            free_slots: VecDeque::new(),
            generations: Vec::new(),
            archetypes: ArchetypeRegistry::new(),
            entity_archetype_keys: HashMap::new(),
        }
    }

    /// Create a new entity.
    ///
    /// Reuses freed slots when available, with incremented generation
    /// to prevent use-after-free via stale handles.
    pub fn create_entity(&mut self) -> Entity {
        if let Some(index) = self.free_slots.pop_front() {
            let generation = self.generations[index as usize];
            let entity = Entity::new(index, generation);

            self.entities[index as usize] = Some(entity);

            // Create the initial archetype (empty component set)
            let key = ArchetypeKey::new(Vec::new());
            let archetype = self.archetypes.get_or_create(key.clone());
            archetype.add_entity(entity);
            self.entity_archetype_keys.insert(index, key);

            entity
        } else {
            let index = self.entities.len() as u32;
            let generation = 0;
            let entity = Entity::new(index, generation);

            self.entities.push(Some(entity));
            self.generations.push(0);

            // Create the initial archetype (empty component set)
            let key = ArchetypeKey::new(Vec::new());
            let archetype = self.archetypes.get_or_create(key.clone());
            archetype.add_entity(entity);
            self.entity_archetype_keys.insert(index, key);

            entity
        }
    }

    /// Destroy an entity and all its components.
    pub fn destroy_entity(&mut self, entity: Entity) {
        if !self.entity_exists(entity) {
            return;
        }

        let idx = entity.index() as usize;

        // Remove from its archetype
        if let Some(key) = self.entity_archetype_keys.remove(&entity.index()) {
            if let Some(archetype) = self.archetypes.archetypes.get_mut(&key) {
                archetype.remove_entity(entity);
            }
        }

        // Remove all components for this entity
        for (_type_id, storage) in self.storage.storages.iter_mut() {
            storage.remove_entity(entity);
        }

        // Mark slot as free and increment generation
        self.entities[idx] = None;
        self.generations[idx] += 1;
        self.free_slots.push_back(entity.index());
    }

    /// Destroy an entity and emit an EntityDestroyed event.
    pub fn destroy_entity_with_event(&mut self, entity: Entity, events: &mut EventBus) {
        let idx = entity.index();
        self.destroy_entity(entity);
        events.emit(crate::system::Event::EntityDestroyed(idx));
    }

    /// Check if an entity exists and is alive.
    pub fn entity_exists(&self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        if idx >= self.entities.len() {
            return false;
        }
        self.entities[idx] == Some(entity)
    }

    /// Reconstruct an Entity from its index (for internal use).
    /// Returns an invalid/dead entity if the slot is free.
    pub fn entity_from_index(&self, index: u32) -> Entity {
        let idx = index as usize;
        if idx < self.entities.len() {
            self.entities[idx].unwrap_or(Entity::new(index, u32::MAX))
        } else {
            Entity::new(index, u32::MAX)
        }
    }

    /// Get the number of alive entities.
    pub fn entity_count(&self) -> usize {
        self.entities.iter().filter(|e| e.is_some()).count()
    }

    /// Get total capacity (used + free slots).
    pub fn entity_capacity(&self) -> usize {
        self.entities.len()
    }

    /// Get all alive entities in the world.
    pub fn all_entities(&self) -> Vec<Entity> {
        self.entities.iter().filter_map(|e| *e).collect()
    }

    // ──────────────────────────────────────────────
    // Component attachment methods
    // ──────────────────────────────────────────────

    /// Add a component to an entity.
    ///
    /// This automatically creates the component storage if it doesn't exist
    /// yet, and updates the entity's archetype membership.
    pub fn add_component<T: Send + Sync + 'static>(&mut self, entity: Entity, component: T) {
        // Get or create the storage for this component type
        if !self.storage.has::<T>() {
            let new_storage = crate::storage::ComponentStorage::new::<T>();
            self.storage.insert::<T>(new_storage);
        }

        // Insert the component into storage
        self.storage.get_mut::<T>().expect("storage should exist after entry.insert")
            .insert(entity, component);

        // Update archetype membership
        self.recalc_archetype(entity);
    }

    /// Remove a component from an entity.
    ///
    /// Returns the removed component value if it was present.
    pub fn remove_component<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<T> {
        let removed = self
            .storage
            .get_mut::<T>()
            .and_then(|s| s.remove::<T>(entity));

        if removed.is_some() {
            self.recalc_archetype(entity);
        }

        removed
    }

    /// Get an immutable reference to a component on an entity.
    pub fn get_component<T: Send + Sync + 'static>(&self, entity: Entity) -> Option<&T> {
        self.storage.get::<T>().and_then(|s| s.get(entity))
    }

    /// Get a mutable reference to a component on an entity.
    pub fn get_component_mut<T: Send + Sync + 'static>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        self.storage.get_mut::<T>().and_then(|s| s.get_mut(entity))
    }

    /// Check if an entity has a specific component.
    pub fn has_component<T: Send + Sync + 'static>(&self, entity: Entity) -> bool {
        self.storage
            .get::<T>()
            .is_some_and(|s| s.has_entity(entity))
    }

    /// Get all entities that have a specific component type.
    pub fn get_entities_with<T: Send + Sync + 'static>(&self) -> Vec<Entity> {
        self.storage
            .get::<T>()
            .map(|s| {
                s.get_entities_indices::<T>()
                    .into_iter()
                    .filter_map(|idx| self.entities.get(idx as usize).and_then(|e| *e))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ──────────────────────────────────────────────
    // Query system
    // ──────────────────────────────────────────────

    /// Iterate over all entities that have a specific component (immutable).
    ///
    /// Returns an iterator of `(Entity, &T)` pairs. The iterator only yields
    /// entities that are still alive (generation matches).
    pub fn query<T: Send + Sync + 'static>(&self) -> ComponentQuery<'_, T> {
        ComponentQuery::new(&self.storage, &self.entities)
    }

    /// Iterate over all entities that have a specific component (mutable).
    ///
    /// Returns an iterator of `(Entity, &mut T)` pairs.
    pub fn query_mut<T: Send + Sync + 'static>(&mut self) -> ComponentQueryMut<'_, T> {
        ComponentQueryMut::new(&mut self.storage, &self.entities)
    }

    /// Iterate over entities that have BOTH component T AND all the specified types.
    ///
    /// This performs an archetype-based intersection: only entities in archetypes
    /// that contain ALL the given TypeIds are yielded. Falls back to component
    /// filtering if archetype information is insufficient.
    pub fn query_with_all<T: Send + Sync + 'static>(
        &self,
        extra_types: &[TypeId],
    ) -> Vec<(Entity, &T)> {
        // Build a set of all required types
        let main_type = TypeId::of::<T>();
        let mut required: Vec<TypeId> = extra_types.to_vec();
        required.push(main_type);
        required.sort();

        let required_key = ArchetypeKey(required);

        // Try archetype-first: find archetypes that have all components
        // An archetype matches if its key contains all required types
        let mut results = Vec::new();

        for (key, archetype) in &self.archetypes.archetypes {
            if Self::archetype_contains_all(key, &required_key) {
                for &entity in &archetype.entities {
                    if self.entity_exists(entity) {
                        if let Some(comp) = self.storage.get::<T>().and_then(|s| s.get(entity)) {
                            results.push((entity, comp));
                        }
                    }
                }
            }
        }

        results
    }

    /// Check if an archetype key contains all types from another key.
    fn archetype_contains_all(key: &ArchetypeKey, subset: &ArchetypeKey) -> bool {
        let mut i = 0;
        let mut j = 0;
        while i < key.0.len() && j < subset.0.len() {
            if key.0[i] == subset.0[j] {
                i += 1;
                j += 1;
            } else if key.0[i] < subset.0[j] {
                i += 1;
            } else {
                return false;
            }
        }
        j == subset.0.len()
    }

    /// Remove all entities that are marked as dead (have the Dead component).
    /// Returns the count of removed entities.
    pub fn remove_dead(&mut self) -> usize {
        let dead: Vec<Entity> = self.get_entities_with::<crate::component::Dead>();
        let count = dead.len();
        for entity in dead {
            self.destroy_entity(entity);
        }
        count
    }

    // ──────────────────────────────────────────────
    // Internal helpers
    // ──────────────────────────────────────────────

    /// Recompute the archetype for an entity after a component was added/removed.
    fn recalc_archetype(&mut self, entity: Entity) {
        let new_key = self.compute_archetype_key(entity);
        let old_key = self.entity_archetype_keys.get(&entity.index());

        // If the key changed, move the entity between archetypes
        if old_key != Some(&new_key) {
            // Remove from old archetype
            if let Some(old_key) = old_key.cloned() {
                if let Some(old_arch) = self.archetypes.archetypes.get_mut(&old_key) {
                    old_arch.remove_entity(entity);
                }
            }

            // Add to new archetype
            let new_arch = self.archetypes.get_or_create(new_key.clone());
            new_arch.add_entity(entity);
            self.entity_archetype_keys.insert(entity.index(), new_key);
        }
    }

    /// Compute the archetype key for an entity (sorted list of component TypeIds).
    fn compute_archetype_key(&self, entity: Entity) -> ArchetypeKey {
        let mut key: Vec<TypeId> = Vec::new();
        for (type_id, storage) in &self.storage.storages {
            if storage.has_entity(entity) {
                key.push(*type_id);
            }
        }
        key.sort();
        ArchetypeKey(key)
    }
}

/// Iterator over entities with a specific component (immutable).
///
/// Yields `(Entity, &T)` pairs for every alive entity that has component `T`.
pub struct ComponentQuery<'a, T: 'static> {
    inner: Option<Box<dyn Iterator<Item = (Entity, &'a T)> + 'a>>,
}

impl<'a, T: Send + Sync + 'static> ComponentQuery<'a, T> {
    fn new(storage: &'a StorageRegistry, entities: &'a [Option<Entity>]) -> Self {
        let inner = storage.get::<T>().map(|s| {
            let iter: Box<dyn Iterator<Item = (Entity, &'a T)> + 'a> =
                Box::new(s.data.iter().filter_map(move |(idx, boxed)| {
                    let entity = entities.get(*idx as usize).and_then(|e| *e);
                    boxed
                        .downcast_ref::<T>()
                        .and_then(|comp| entity.map(|e| (e, comp)))
                }));
            iter
        });
        ComponentQuery { inner }
    }
}

impl<'a, T: 'static> Iterator for ComponentQuery<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut()?.next()
    }
}

/// Iterator over entities with a specific component (mutable).
///
/// Yields `(Entity, &mut T)` pairs for every alive entity that has component `T`.
pub struct ComponentQueryMut<'a, T: 'static> {
    inner: Option<Box<dyn Iterator<Item = (Entity, &'a mut T)> + 'a>>,
}

impl<'a, T: Send + Sync + 'static> ComponentQueryMut<'a, T> {
    fn new(storage: &'a mut StorageRegistry, entities: &'a [Option<Entity>]) -> Self {
        let inner = storage.get_mut::<T>().map(|s| {
            let iter: Box<dyn Iterator<Item = (Entity, &'a mut T)> + 'a> =
                Box::new(s.data.iter_mut().filter_map(move |(idx, boxed)| {
                    let entity = entities.get(*idx as usize).and_then(|e| *e);
                    boxed
                        .downcast_mut::<T>()
                        .and_then(|comp| entity.map(|e| (e, comp)))
                }));
            iter
        });
        ComponentQueryMut { inner }
    }
}

impl<'a, T: 'static> Iterator for ComponentQueryMut<'a, T> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut()?.next()
    }
}

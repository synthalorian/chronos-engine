use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::entity::Entity;

/// A typed storage container for a single component type.
///
/// Each component type (Position, Health, etc.) has its own storage instance
/// that holds all instances of that component across all entities.
///
/// Internally we store a HashMap from Entity → boxed data. This is the
/// simplest "per-entity" storage. A real production engine would use
/// cache-friendly sparse-set or vector-of-structs, but this gets us
/// working for Phase 2.
pub struct ComponentStorage {
    type_id: TypeId,
    /// entity index → boxed component data
    pub(crate) data: HashMap<u32, Box<dyn Any + Send + Sync>>,
}

impl ComponentStorage {
    /// Create a new empty typed storage.
    pub fn new<T: Send + Sync + 'static>() -> Self {
        ComponentStorage {
            type_id: TypeId::of::<T>(),
            data: HashMap::new(),
        }
    }

    /// Insert a component for a specific entity.
    pub fn insert<T: Send + Sync + 'static>(&mut self, entity: Entity, component: T) {
        self.data.insert(entity.index(), Box::new(component));
    }

    /// Get an immutable reference to a component for a specific entity.
    pub fn get<T: Send + Sync + 'static>(&self, entity: Entity) -> Option<&T> {
        self.data
            .get(&entity.index())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to a component for a specific entity.
    pub fn get_mut<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.data
            .get_mut(&entity.index())
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Remove a component for a specific entity. Returns the removed value
    /// if it was present.
    pub fn remove<T: Send + Sync + 'static>(&mut self, entity: Entity) -> Option<T> {
        self.data
            .remove(&entity.index())
            .and_then(|boxed| boxed.downcast::<T>().ok().map(|boxed| *boxed))
    }

    /// Check if a specific entity has this component.
    pub fn has_entity(&self, entity: Entity) -> bool {
        self.data.contains_key(&entity.index())
    }

    /// Remove a component by entity index without type information.
    /// Used during entity destruction when we don't know the component type.
    pub(crate) fn remove_entity(&mut self, entity: Entity) {
        self.data.remove(&entity.index());
    }

    /// Get the type ID of the component stored in this storage.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if this storage holds the given type.
    pub fn is<T: Send + Sync + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    /// Iterate over all entities that have this component (immutable).
    pub fn iter_entities(&self) -> impl Iterator<Item = u32> + '_ {
        self.data.keys().copied()
    }

    /// Iterate over (entity_index, &T) entries.
    pub fn iter_entries<T: Send + Sync + 'static>(
        &self,
    ) -> impl Iterator<Item = (u32, &T)> + '_ {
        self.data.iter().filter_map(|(idx, boxed)| {
            boxed.downcast_ref::<T>().map(|c| (*idx, c))
        })
    }

    /// Get all entity indices that have this component.
    pub fn get_entities_indices<T: Send + Sync + 'static>(&self) -> Vec<u32> {
        self.iter_entries::<T>()
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// The global storage registry.
///
/// Maps TypeId to ComponentStorage. This is where components
/// are actually stored in memory — one storage instance per
/// component type.
pub struct StorageRegistry {
    pub(crate) storages: HashMap<TypeId, ComponentStorage>,
}

impl StorageRegistry {
    pub fn new() -> Self {
        StorageRegistry {
            storages: HashMap::new(),
        }
    }

    /// Insert a new component storage for type T.
    /// Panics if a storage for this type already exists.
    pub fn insert<T: Send + Sync + 'static>(&mut self, storage: ComponentStorage) {
        assert!(
            !self.has::<T>(),
            "Component storage for type {:?} already exists",
            TypeId::of::<T>()
        );
        self.storages.insert(TypeId::of::<T>(), storage);
    }

    /// Get a mutable reference to the storage for type T.
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut ComponentStorage> {
        self.storages.get_mut(&TypeId::of::<T>())
    }

    /// Get an immutable reference to the storage for type T.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&ComponentStorage> {
        self.storages.get(&TypeId::of::<T>())
    }

    /// Remove a component storage for type T.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<ComponentStorage> {
        self.storages.remove(&TypeId::of::<T>())
    }

    /// Check if a component type exists.
    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.storages.contains_key(&TypeId::of::<T>())
    }

    /// Iterate over all storages.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeId, &ComponentStorage)> {
        self.storages.iter()
    }

    /// Iterate over all storages mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&TypeId, &mut ComponentStorage)> {
        self.storages.iter_mut()
    }
}

#[cfg(feature = "game")]
use crate::{Entity, World};
use crate::component::{Health, Transform, Dead};
use super::components::*;

/// Rarity tier for loot items, each with a distinct UI color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl ItemRarity {
    /// RGBA color for rendering the rarity label in the UI.
    pub fn color(&self) -> [f32; 4] {
        match self {
            ItemRarity::Common => [0.8, 0.8, 0.8, 1.0],
            ItemRarity::Uncommon => [0.0, 1.0, 0.0, 1.0],
            ItemRarity::Rare => [0.0, 0.5, 1.0, 1.0],
            ItemRarity::Epic => [0.6, 0.0, 1.0, 1.0],
            ItemRarity::Legendary => [1.0, 0.5, 0.0, 1.0],
        }
    }
}

/// Category of an inventory item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Weapon,
    Armor,
    Consumable,
    Material,
    QuestItem,
    Gold,
}

/// A single item that can live in an inventory or on the ground.
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryItem {
    pub name: String,
    pub item_type: ItemType,
    pub rarity: ItemRarity,
    pub stack_size: u32,
    pub max_stack: u32,
    pub gold_value: u32,
}

impl InventoryItem {
    /// Create an item with default values.
    pub fn new() -> Self {
        InventoryItem {
            name: String::new(),
            item_type: ItemType::Material,
            rarity: ItemRarity::Common,
            stack_size: 1,
            max_stack: 1,
            gold_value: 0,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_item_type(mut self, item_type: ItemType) -> Self {
        self.item_type = item_type;
        self
    }

    pub fn with_rarity(mut self, rarity: ItemRarity) -> Self {
        self.rarity = rarity;
        self
    }

    pub fn with_stack_size(mut self, size: u32) -> Self {
        self.stack_size = size;
        self
    }

    pub fn with_max_stack(mut self, max: u32) -> Self {
        self.max_stack = max;
        self
    }

    pub fn with_gold_value(mut self, value: u32) -> Self {
        self.gold_value = value;
        self
    }

    /// Two items can stack if they share name, type, and rarity.
    pub fn can_stack_with(&self, other: &InventoryItem) -> bool {
        self.name == other.name
            && self.item_type == other.item_type
            && self.rarity == other.rarity
    }

    /// Add `count` items to this stack. Returns the amount that overflowed.
    pub fn add_to_stack(&mut self, count: u32) -> u32 {
        let space = self.max_stack.saturating_sub(self.stack_size);
        let to_add = count.min(space);
        self.stack_size += to_add;
        count.saturating_sub(to_add)
    }

    // ── Preset item factories ──

    pub fn iron_sword() -> Self {
        InventoryItem::new()
            .with_name("Iron Sword")
            .with_item_type(ItemType::Weapon)
            .with_rarity(ItemRarity::Common)
            .with_gold_value(10)
    }

    pub fn health_potion() -> Self {
        InventoryItem::new()
            .with_name("Health Potion")
            .with_item_type(ItemType::Consumable)
            .with_rarity(ItemRarity::Common)
            .with_max_stack(10)
            .with_stack_size(1)
            .with_gold_value(5)
    }

    pub fn gold_coins(amount: u32) -> Self {
        InventoryItem::new()
            .with_name("Gold Coins")
            .with_item_type(ItemType::Gold)
            .with_rarity(ItemRarity::Common)
            .with_stack_size(amount)
            .with_max_stack(9999)
            .with_gold_value(amount)
    }
}

/// How close a player must be to pick up a ground item.
#[derive(Debug, Clone, Copy)]
pub struct PickupRadius {
    pub radius: f32,
}

impl PickupRadius {
    pub fn new(radius: f32) -> Self {
        PickupRadius { radius }
    }
}

/// Component attached to a dropped loot entity on the ground.
#[derive(Debug, Clone)]
pub struct LootPickup {
    pub item: Option<InventoryItem>,
    pub gold_amount: u32,
    pub spawn_time: f32,
    pub despawn_time: f32,
}

impl LootPickup {
    /// Create an empty pickup with sensible defaults (spawns now, despawns in 300s).
    pub fn new() -> Self {
        LootPickup {
            item: None,
            gold_amount: 0,
            spawn_time: 0.0,
            despawn_time: 300.0,
        }
    }

    pub fn with_item(mut self, item: InventoryItem) -> Self {
        self.item = Some(item);
        self
    }

    pub fn with_gold(mut self, amount: u32) -> Self {
        self.gold_amount = amount;
        self
    }

    /// Whether this pickup has exceeded its lifetime.
    pub fn is_expired(&self, current_time: f32) -> bool {
        current_time >= self.despawn_time
    }
}

/// An entity's inventory — holds items and gold.
#[derive(Debug, Clone)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub gold: u32,
    pub max_slots: usize,
}

impl Inventory {
    pub fn new(max_slots: usize) -> Self {
        Inventory {
            items: Vec::new(),
            gold: 0,
            max_slots,
        }
    }

    /// Try to add an item. Stacks with existing items first, then uses a new slot.
    /// Returns `Some(item)` if the inventory is full and the item couldn't be added.
    pub fn add_item(&mut self, mut item: InventoryItem) -> Option<InventoryItem> {
        // Try stacking with compatible existing items
        for existing in &mut self.items {
            if existing.can_stack_with(&item) && existing.stack_size < existing.max_stack {
                let overflow = existing.add_to_stack(item.stack_size);
                if overflow == 0 {
                    return None;
                }
                item.stack_size = overflow;
            }
        }

        // Remaining stack goes into a new slot
        if !self.is_full() {
            self.items.push(item);
            return None;
        }

        Some(item)
    }

    pub fn add_gold(&mut self, amount: u32) {
        self.gold += amount;
    }

    pub fn remove_item(&mut self, index: usize) -> Option<InventoryItem> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn find_item(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|i| i.name == name)
    }

    pub fn is_full(&self) -> bool {
        self.items.len() >= self.max_slots
    }

    /// Sum of gold in purse plus the sell value of every item.
    pub fn total_gold_value(&self) -> u32 {
        let item_value: u32 = self
            .items
            .iter()
            .map(|i| i.gold_value.saturating_mul(i.stack_size))
            .sum();
        self.gold.saturating_add(item_value)
    }
}

/// Outcome of a pickup attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupResult {
    Success,
    InventoryFull,
    NoLoot,
    OutOfRange,
}

/// Spawns loot drop entities into the world.
pub struct LootSpawner;

impl LootSpawner {
    /// Spawn ground entities for gold and items described by a [`LootDrop`].
    pub fn spawn_loot(world: &mut World, position: [f32; 3], loot: LootDrop) -> Vec<Entity> {
        let mut entities = Vec::new();
        let mut idx: u32 = 0;

        if loot.gold > 0 {
            let offset_x = Self::scatter(idx, 0.37);
            let offset_z = Self::scatter(idx, 0.53);
            let entity = world.create_entity();
            world.add_component(
                entity,
                Transform::new(position[0] + offset_x, position[1], position[2] + offset_z),
            );
            world.add_component(
                entity,
                LootPickup::new().with_gold(loot.gold),
            );
            world.add_component(entity, PickupRadius::new(2.0));
            entities.push(entity);
            idx += 1;
        }

        for item_name in &loot.items {
            let offset_x = Self::scatter(idx, 0.37);
            let offset_z = Self::scatter(idx, 0.53);
            let item = InventoryItem::new()
                .with_name(item_name.clone())
                .with_item_type(ItemType::Material)
                .with_rarity(ItemRarity::Common);
            let entity = world.create_entity();
            world.add_component(
                entity,
                Transform::new(position[0] + offset_x, position[1], position[2] + offset_z),
            );
            world.add_component(entity, LootPickup::new().with_item(item));
            world.add_component(entity, PickupRadius::new(2.0));
            entities.push(entity);
            idx += 1;
        }

        entities
    }

    /// Convenience: spawn a single gold pile.
    pub fn spawn_gold_pile(world: &mut World, position: [f32; 3], amount: u32) -> Entity {
        let entity = world.create_entity();
        world.add_component(entity, Transform::new(position[0], position[1], position[2]));
        world.add_component(entity, LootPickup::new().with_gold(amount));
        world.add_component(entity, PickupRadius::new(2.0));
        entity
    }

    /// Deterministic scatter within ±0.5 using sin.
    fn scatter(idx: u32, seed: f32) -> f32 {
        (idx as f32 * seed * 17.31).sin() * 0.5
    }
}

/// System that processes loot despawning and auto-pickup each frame.
pub struct LootSystem;

impl LootSystem {
    pub fn new() -> Self {
        LootSystem
    }

    /// Tick the loot system. Despawns expired pickups and auto-picks up gold
    /// for living entities with an [`Inventory`] that are within range.
    pub fn update(&mut self, world: &mut World, _dt: f32, current_time: f32) {
        // ── Phase 1: Despawn expired loot ──
        let expired: Vec<Entity> = {
            let loot_entities = world.get_entities_with::<LootPickup>();
            loot_entities
                .iter()
                .filter(|e| {
                    world
                        .get_component::<LootPickup>(**e)
                        .map(|p| p.is_expired(current_time))
                        .unwrap_or(false)
                })
                .copied()
                .collect()
        };
        for entity in expired {
            world.destroy_entity(entity);
        }

        // ── Phase 2: Auto-pickup gold ──
        let to_pickup: Vec<(Entity, Entity)> = {
            let pickers = world.get_entities_with::<Inventory>();
            let loot_entities = world.get_entities_with::<LootPickup>();

            let mut picks = Vec::new();
            for picker in &pickers {
                // Skip dead pickers
                if world.has_component::<Dead>(*picker) {
                    continue;
                }
                if let Some(health) = world.get_component::<Health>(*picker) {
                    if health.is_dead() {
                        continue;
                    }
                }
                let picker_pos = match world.get_component::<Transform>(*picker) {
                    Some(t) => [t.x, t.y, t.z],
                    None => continue,
                };

                for loot_entity in &loot_entities {
                    let in_range = {
                        let radius = match world.get_component::<PickupRadius>(*loot_entity) {
                            Some(r) => r.radius,
                            None => continue,
                        };
                        let loot_pos = match world.get_component::<Transform>(*loot_entity) {
                            Some(t) => [t.x, t.y, t.z],
                            None => continue,
                        };
                        let is_gold = world
                            .get_component::<LootPickup>(*loot_entity)
                            .map(|l| l.gold_amount > 0)
                            .unwrap_or(false);
                        if !is_gold {
                            continue;
                        }
                        let dx = picker_pos[0] - loot_pos[0];
                        let dy = picker_pos[1] - loot_pos[1];
                        let dz = picker_pos[2] - loot_pos[2];
                        (dx * dx + dy * dy + dz * dz).sqrt() <= radius
                    };
                    if in_range {
                        picks.push((*picker, *loot_entity));
                    }
                }
            }
            picks
        };

        for (picker, loot_entity) in to_pickup {
            Self::pickup_item(world, picker, loot_entity);
        }
    }

    /// Attempt to pick up a loot entity into a picker's inventory.
    pub fn pickup_item(world: &mut World, picker: Entity, pickup: Entity) -> PickupResult {
        if !world.has_component::<Inventory>(picker) {
            return PickupResult::NoLoot;
        }
        if !world.has_component::<LootPickup>(pickup) {
            return PickupResult::NoLoot;
        }

        // Range check
        let in_range = {
            let picker_pos = world.get_component::<Transform>(picker);
            let pickup_pos = world.get_component::<Transform>(pickup);
            let radius = world.get_component::<PickupRadius>(pickup);
            match (picker_pos, pickup_pos, radius) {
                (Some(p), Some(l), Some(r)) => {
                    let dx = p.x - l.x;
                    let dy = p.y - l.y;
                    let dz = p.z - l.z;
                    (dx * dx + dy * dy + dz * dz).sqrt() <= r.radius
                }
                _ => true, // no transform/radius means no range restriction
            }
        };
        if !in_range {
            return PickupResult::OutOfRange;
        }

        // Clone pickup data to release the borrow before mutating
        let pickup_data = match world.get_component::<LootPickup>(pickup).cloned() {
            Some(data) => data,
            None => return PickupResult::NoLoot,
        };

        // Gold pickup
        if pickup_data.gold_amount > 0 {
            if let Some(inventory) = world.get_component_mut::<Inventory>(picker) {
                inventory.add_gold(pickup_data.gold_amount);
            }
            world.destroy_entity(pickup);
            return PickupResult::Success;
        }

        // Item pickup
        if let Some(item) = pickup_data.item {
            let added = match world.get_component_mut::<Inventory>(picker) {
                Some(inv) => inv.add_item(item).is_none(),
                None => false,
            };
            if !added {
                return PickupResult::InventoryFull;
            }
            world.destroy_entity(pickup);
            return PickupResult::Success;
        }

        PickupResult::NoLoot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new()
    }

    #[test]
    fn inventory_item_creation_and_stacking() {
        let sword = InventoryItem::iron_sword();
        assert_eq!(sword.name, "Iron Sword");
        assert_eq!(sword.item_type, ItemType::Weapon);
        assert_eq!(sword.rarity, ItemRarity::Common);
        assert_eq!(sword.gold_value, 10);
        assert_eq!(sword.stack_size, 1);

        let potion = InventoryItem::health_potion();
        assert_eq!(potion.name, "Health Potion");
        assert_eq!(potion.item_type, ItemType::Consumable);
        assert_eq!(potion.max_stack, 10);
    }

    #[test]
    fn can_stack_with_same_and_different() {
        let potion_a = InventoryItem::health_potion();
        let potion_b = InventoryItem::health_potion();
        assert!(potion_a.can_stack_with(&potion_b));

        let sword = InventoryItem::iron_sword();
        assert!(!potion_a.can_stack_with(&sword));

        let uncommon_potion = InventoryItem::health_potion().with_rarity(ItemRarity::Uncommon);
        assert!(!potion_a.can_stack_with(&uncommon_potion));
    }

    #[test]
    fn add_to_stack_with_overflow() {
        let mut potion = InventoryItem::health_potion().with_stack_size(8);
        let overflow = potion.add_to_stack(5);
        assert_eq!(potion.stack_size, 10);
        assert_eq!(overflow, 3);

        let mut gold = InventoryItem::gold_coins(5000);
        let overflow = gold.add_to_stack(6000);
        assert_eq!(gold.stack_size, 9999);
        assert_eq!(overflow, 1001);
    }

    #[test]
    fn inventory_add_remove_items() {
        let mut inv = Inventory::new(5);
        let sword = InventoryItem::iron_sword();
        assert!(inv.add_item(sword).is_none());
        assert_eq!(inv.items.len(), 1);

        let idx = inv.find_item("Iron Sword").expect("should find sword");
        assert_eq!(idx, 0);

        let removed = inv.remove_item(0).expect("should remove");
        assert_eq!(removed.name, "Iron Sword");
        assert_eq!(inv.items.len(), 0);
        assert!(inv.find_item("Iron Sword").is_none());
    }

    #[test]
    fn inventory_full_check() {
        let mut inv = Inventory::new(2);
        assert!(!inv.is_full());

        inv.add_item(InventoryItem::iron_sword());
        assert!(!inv.is_full());

        inv.add_item(InventoryItem::iron_sword().with_name("Shield"));
        assert!(inv.is_full());

        let overflow = inv.add_item(InventoryItem::health_potion());
        assert!(overflow.is_some());
    }

    #[test]
    fn gold_add_and_total_value() {
        let mut inv = Inventory::new(10);
        inv.add_gold(50);
        assert_eq!(inv.gold, 50);

        inv.add_item(InventoryItem::iron_sword());
        inv.add_item(InventoryItem::health_potion().with_stack_size(3));

        // 50 gold + 10 (sword) + 3*5 (potions) = 75
        assert_eq!(inv.total_gold_value(), 75);
    }

    #[test]
    fn loot_spawner_creates_entities() {
        let mut world = test_world();
        let drop = LootDrop::new(25).with_items(vec!["Rusty Dagger".to_string()]);
        let entities = LootSpawner::spawn_loot(&mut world, [5.0, 0.0, 5.0], drop);

        assert_eq!(entities.len(), 2); // 1 gold + 1 item

        for entity in &entities {
            assert!(world.has_component::<Transform>(*entity));
            assert!(world.has_component::<LootPickup>(*entity));
            assert!(world.has_component::<PickupRadius>(*entity));
        }

        // Verify gold entity
        let gold_entity = entities[0];
        let gold_pickup = world.get_component::<LootPickup>(gold_entity).expect("pickup");
        assert_eq!(gold_pickup.gold_amount, 25);
        assert!(gold_pickup.item.is_none());

        // Verify item entity
        let item_entity = entities[1];
        let item_pickup = world.get_component::<LootPickup>(item_entity).expect("pickup");
        assert_eq!(item_pickup.gold_amount, 0); // item entity has no gold
        assert!(item_pickup.item.is_some());
        assert_eq!(item_pickup.item.as_ref().unwrap().name, "Rusty Dagger");
    }

    #[test]
    fn loot_pickup_despawn_timer() {
        let pickup = LootPickup::new().with_gold(10);
        assert!(!pickup.is_expired(299.9));
        assert!(pickup.is_expired(300.0));
        assert!(pickup.is_expired(500.0));

        let fast_despawn = LootPickup::new().with_gold(5);
        // Default despawn is 300.0 — spawn_time is 0.0
        assert!(!fast_despawn.is_expired(50.0));
    }

    #[test]
    fn pickup_gold_into_inventory() {
        let mut world = test_world();

        let picker = world.create_entity();
        world.add_component(picker, Transform::new(0.0, 0.0, 0.0));
        world.add_component(picker, Inventory::new(10));

        let gold_pile = LootSpawner::spawn_gold_pile(&mut world, [0.5, 0.0, 0.5], 100);

        let result = LootSystem::pickup_item(&mut world, picker, gold_pile);
        assert_eq!(result, PickupResult::Success);

        let inv = world.get_component::<Inventory>(picker).expect("inventory");
        assert_eq!(inv.gold, 100);
        assert!(!world.entity_exists(gold_pile));
    }

    #[test]
    fn pickup_item_into_inventory() {
        let mut world = test_world();

        let picker = world.create_entity();
        world.add_component(picker, Transform::new(1.0, 0.0, 1.0));
        world.add_component(picker, Inventory::new(10));

        let loot = world.create_entity();
        world.add_component(loot, Transform::new(1.0, 0.0, 1.0));
        let sword = InventoryItem::iron_sword();
        world.add_component(loot, LootPickup::new().with_item(sword));
        world.add_component(loot, PickupRadius::new(2.0));

        let result = LootSystem::pickup_item(&mut world, picker, loot);
        assert_eq!(result, PickupResult::Success);

        let inv = world.get_component::<Inventory>(picker).expect("inventory");
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].name, "Iron Sword");
        assert!(!world.entity_exists(loot));
    }

    #[test]
    fn inventory_full_returns_overflow() {
        let mut world = test_world();

        let picker = world.create_entity();
        world.add_component(picker, Transform::new(0.0, 0.0, 0.0));
        world.add_component(picker, Inventory::new(1)); // only 1 slot

        // Fill the slot
        let loot_a = world.create_entity();
        world.add_component(loot_a, Transform::new(0.0, 0.0, 0.0));
        world.add_component(
            loot_a,
            LootPickup::new().with_item(InventoryItem::iron_sword()),
        );
        world.add_component(loot_a, PickupRadius::new(2.0));
        let result_a = LootSystem::pickup_item(&mut world, picker, loot_a);
        assert_eq!(result_a, PickupResult::Success);

        // Try to pick up another — should fail
        let loot_b = world.create_entity();
        world.add_component(loot_b, Transform::new(0.0, 0.0, 0.0));
        world.add_component(
            loot_b,
            LootPickup::new().with_item(InventoryItem::health_potion()),
        );
        world.add_component(loot_b, PickupRadius::new(2.0));
        let result_b = LootSystem::pickup_item(&mut world, picker, loot_b);
        assert_eq!(result_b, PickupResult::InventoryFull);

        // Loot entity should still exist
        assert!(world.entity_exists(loot_b));
    }

    #[test]
    fn gold_coins_factory() {
        let coins = InventoryItem::gold_coins(250);
        assert_eq!(coins.name, "Gold Coins");
        assert_eq!(coins.item_type, ItemType::Gold);
        assert_eq!(coins.stack_size, 250);
        assert_eq!(coins.max_stack, 9999);
        assert_eq!(coins.gold_value, 250);
    }
}

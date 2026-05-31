#[cfg(feature = "game")]
use super::loot::{InventoryItem, ItemRarity, ItemType};

// ── SortMethod ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMethod {
    ByName,
    ByRarity,
    ByItemType,
    ByValue,
}

// ── ItemFilter ──

#[derive(Debug, Clone, PartialEq)]
pub enum ItemFilter {
    All,
    ByRarity(ItemRarity),
    ByType(ItemType),
    ByText(String),
}

impl ItemFilter {
    pub fn matches(&self, item: &InventoryItem) -> bool {
        match self {
            ItemFilter::All => true,
            ItemFilter::ByRarity(rarity) => item.rarity == *rarity,
            ItemFilter::ByType(item_type) => item.item_type == *item_type,
            ItemFilter::ByText(query) => item.name.to_lowercase().contains(&query.to_lowercase()),
        }
    }
}

// ── AddResult ──

#[derive(Debug, Clone, PartialEq)]
pub enum AddResult {
    Added,
    Stacked { new_size: u32 },
    StackedPartial { remaining: u32 },
    Full,
}

// ── InventoryManager ──

#[derive(Debug, Clone)]
pub struct InventoryManager {
    pub items: Vec<InventoryItem>,
    pub capacity: usize,
    pub gold: u32,
}

impl InventoryManager {
    pub fn new(capacity: usize) -> Self {
        InventoryManager {
            items: Vec::new(),
            capacity,
            gold: 0,
        }
    }

    pub fn with_gold(mut self, gold: u32) -> Self {
        self.gold = gold;
        self
    }

    pub fn add_item(&mut self, item: InventoryItem) -> AddResult {
        for existing in &mut self.items {
            if existing.can_stack_with(&item) && existing.stack_size < existing.max_stack {
                let remaining = existing.add_to_stack(item.stack_size);
                if remaining == 0 {
                    return AddResult::Stacked {
                        new_size: existing.stack_size,
                    };
                }
                return AddResult::StackedPartial { remaining };
            }
        }

        if self.items.len() >= self.capacity {
            return AddResult::Full;
        }

        self.items.push(item);
        AddResult::Added
    }

    pub fn remove_item(&mut self, index: usize) -> Option<InventoryItem> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn remove_stack(&mut self, index: usize, count: u32) -> Option<InventoryItem> {
        let item = self.items.get_mut(index)?;
        if item.stack_size <= count {
            return Some(self.items.remove(index));
        }
        item.stack_size -= count;
        let mut removed = item.clone();
        removed.stack_size = count;
        Some(removed)
    }

    pub fn total_items(&self) -> u32 {
        self.items.iter().map(|i| i.stack_size).sum()
    }

    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity
    }

    pub fn sort(&mut self, method: SortMethod) {
        self.items
            .sort_by(|a, b| ItemComparator::compare(a, b, method));
    }

    pub fn filter(&self, filter: &ItemFilter) -> Vec<(usize, &InventoryItem)> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| filter.matches(item))
            .collect()
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|i| i.name == name)
    }

    pub fn gold_total(&self) -> u32 {
        self.gold
    }

    pub fn add_gold(&mut self, amount: u32) {
        self.gold += amount;
    }

    pub fn spend_gold(&mut self, amount: u32) -> bool {
        if self.gold >= amount {
            self.gold -= amount;
            true
        } else {
            false
        }
    }
}

// ── ItemComparator ──

pub struct ItemComparator;

impl ItemComparator {
    pub fn compare(a: &InventoryItem, b: &InventoryItem, method: SortMethod) -> std::cmp::Ordering {
        match method {
            SortMethod::ByName => a.name.cmp(&b.name),
            SortMethod::ByRarity => Self::rarity_tier(a.rarity).cmp(&Self::rarity_tier(b.rarity)),
            SortMethod::ByItemType => format!("{:?}", a.item_type)
                .cmp(&format!("{:?}", b.item_type))
                .then_with(|| a.name.cmp(&b.name)),
            SortMethod::ByValue => b
                .gold_value
                .cmp(&a.gold_value)
                .then_with(|| a.name.cmp(&b.name)),
        }
    }

    fn rarity_tier(rarity: ItemRarity) -> u8 {
        match rarity {
            ItemRarity::Common => 0,
            ItemRarity::Uncommon => 1,
            ItemRarity::Rare => 2,
            ItemRarity::Epic => 3,
            ItemRarity::Legendary => 4,
        }
    }
}

// ── DragState ──

#[derive(Debug, Clone, PartialEq)]
pub enum DragState {
    Idle,
    Dragging {
        source_index: usize,
    },
    Hovering {
        source_index: usize,
        target_index: usize,
    },
}

impl DragState {
    pub fn is_dragging(&self) -> bool {
        !matches!(self, DragState::Idle)
    }

    pub fn source(&self) -> Option<usize> {
        match self {
            DragState::Idle => None,
            DragState::Dragging { source_index } | DragState::Hovering { source_index, .. } => {
                Some(*source_index)
            }
        }
    }
}

// ── InventoryUI ──

#[derive(Debug, Clone)]
pub struct InventoryUI {
    pub visible: bool,
    pub selected_index: Option<usize>,
    pub drag_state: DragState,
    pub sort_method: SortMethod,
    pub filter: ItemFilter,
    pub scroll_offset: usize,
}

impl Default for InventoryUI {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryUI {
    pub fn new() -> Self {
        InventoryUI {
            visible: false,
            selected_index: None,
            drag_state: DragState::Idle,
            sort_method: SortMethod::ByName,
            filter: ItemFilter::All,
            scroll_offset: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn select(&mut self, index: usize) {
        self.selected_index = Some(index);
    }

    pub fn deselect(&mut self) {
        self.selected_index = None;
    }

    pub fn begin_drag(&mut self, index: usize) {
        self.drag_state = DragState::Dragging {
            source_index: index,
        };
    }

    pub fn update_hover(&mut self, target: usize) {
        if let DragState::Dragging { source_index } = self.drag_state {
            self.drag_state = DragState::Hovering {
                source_index,
                target_index: target,
            };
        }
    }

    pub fn end_drag(&mut self) -> Option<(usize, usize)> {
        if let DragState::Hovering {
            source_index,
            target_index,
        } = self.drag_state
        {
            self.drag_state = DragState::Idle;
            Some((source_index, target_index))
        } else {
            self.drag_state = DragState::Idle;
            None
        }
    }

    pub fn cancel_drag(&mut self) {
        self.drag_state = DragState::Idle;
    }

    pub fn set_sort(&mut self, method: SortMethod) {
        self.sort_method = method;
    }

    pub fn set_filter(&mut self, filter: ItemFilter) {
        self.filter = filter;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self, max: usize) {
        if self.scroll_offset < max {
            self.scroll_offset += 1;
        }
    }
}

// ── InventorySystem ──

pub struct InventorySystem;

impl Default for InventorySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl InventorySystem {
    pub fn new() -> Self {
        InventorySystem
    }

    pub fn swap_items(inventory: &mut InventoryManager, a: usize, b: usize) {
        if a != b && a < inventory.items.len() && b < inventory.items.len() {
            inventory.items.swap(a, b);
        }
    }

    pub fn move_item(from: &mut InventoryManager, to: &mut InventoryManager, index: usize) -> bool {
        let item = match from.remove_item(index) {
            Some(i) => i,
            None => return false,
        };
        let can_stack = to
            .items
            .iter()
            .any(|s| s.can_stack_with(&item) && s.stack_size < s.max_stack);
        if !can_stack && to.items.len() >= to.capacity {
            from.items.push(item);
            return false;
        }
        to.add_item(item);
        true
    }

    pub fn use_consumable(inventory: &mut InventoryManager, index: usize) -> Option<InventoryItem> {
        let item = inventory.items.get(index)?;
        if item.item_type == ItemType::Consumable {
            inventory.remove_item(index)
        } else {
            None
        }
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(
        name: &str,
        item_type: ItemType,
        rarity: ItemRarity,
        stack: u32,
        max_stack: u32,
        value: u32,
    ) -> InventoryItem {
        InventoryItem::new()
            .with_name(name)
            .with_item_type(item_type)
            .with_rarity(rarity)
            .with_stack_size(stack)
            .with_max_stack(max_stack)
            .with_gold_value(value)
    }

    #[test]
    fn add_new_item() {
        let mut inv = InventoryManager::new(10);
        let sword = make_item("Iron Sword", ItemType::Weapon, ItemRarity::Common, 1, 1, 10);
        let result = inv.add_item(sword);
        assert_eq!(result, AddResult::Added);
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].name, "Iron Sword");
    }

    #[test]
    fn add_stack_existing() {
        let mut inv = InventoryManager::new(10);
        let potion = make_item(
            "Health Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            3,
            10,
            5,
        );
        inv.add_item(potion);
        let more = make_item(
            "Health Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            4,
            10,
            5,
        );
        let result = inv.add_item(more);
        assert_eq!(result, AddResult::Stacked { new_size: 7 });
        assert_eq!(inv.items.len(), 1);
    }

    #[test]
    fn add_stack_partial() {
        let mut inv = InventoryManager::new(10);
        let potion = make_item(
            "Health Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            8,
            10,
            5,
        );
        inv.add_item(potion);
        let overflow = make_item(
            "Health Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            5,
            10,
            5,
        );
        let result = inv.add_item(overflow);
        assert_eq!(result, AddResult::StackedPartial { remaining: 3 });
        assert_eq!(inv.items[0].stack_size, 10);
    }

    #[test]
    fn add_when_full() {
        let mut inv = InventoryManager::new(1);
        let sword = make_item("Iron Sword", ItemType::Weapon, ItemRarity::Common, 1, 1, 10);
        inv.add_item(sword);
        let shield = make_item(
            "Wooden Shield",
            ItemType::Armor,
            ItemRarity::Common,
            1,
            1,
            5,
        );
        let result = inv.add_item(shield);
        assert_eq!(result, AddResult::Full);
    }

    #[test]
    fn remove_item_correct() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "A",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            0,
        ));
        inv.add_item(make_item("B", ItemType::Armor, ItemRarity::Common, 1, 1, 0));
        let removed = inv.remove_item(0).unwrap();
        assert_eq!(removed.name, "A");
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].name, "B");
    }

    #[test]
    fn remove_stack_partial() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Arrows",
            ItemType::Material,
            ItemRarity::Common,
            20,
            50,
            1,
        ));
        let removed = inv.remove_stack(0, 5).unwrap();
        assert_eq!(removed.stack_size, 5);
        assert_eq!(inv.items[0].stack_size, 15);
    }

    #[test]
    fn sort_by_name() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Zephyr Bow",
            ItemType::Weapon,
            ItemRarity::Rare,
            1,
            1,
            50,
        ));
        inv.add_item(make_item(
            "Apple",
            ItemType::Consumable,
            ItemRarity::Common,
            1,
            1,
            1,
        ));
        inv.add_item(make_item(
            "Mithril Ore",
            ItemType::Material,
            ItemRarity::Epic,
            1,
            1,
            100,
        ));
        inv.sort(SortMethod::ByName);
        let names: Vec<&str> = inv.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "Mithril Ore", "Zephyr Bow"]);
    }

    #[test]
    fn sort_by_rarity() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Epic Gem",
            ItemType::Material,
            ItemRarity::Epic,
            1,
            1,
            200,
        ));
        inv.add_item(make_item(
            "Common Rock",
            ItemType::Material,
            ItemRarity::Common,
            1,
            1,
            1,
        ));
        inv.add_item(make_item(
            "Legendary Blade",
            ItemType::Weapon,
            ItemRarity::Legendary,
            1,
            1,
            999,
        ));
        inv.add_item(make_item(
            "Rare Ring",
            ItemType::Armor,
            ItemRarity::Rare,
            1,
            1,
            75,
        ));
        inv.add_item(make_item(
            "Uncommon Helm",
            ItemType::Armor,
            ItemRarity::Uncommon,
            1,
            1,
            25,
        ));
        inv.sort(SortMethod::ByRarity);
        let rarities: Vec<ItemRarity> = inv.items.iter().map(|i| i.rarity).collect();
        assert_eq!(
            rarities,
            vec![
                ItemRarity::Common,
                ItemRarity::Uncommon,
                ItemRarity::Rare,
                ItemRarity::Epic,
                ItemRarity::Legendary,
            ]
        );
    }

    #[test]
    fn filter_by_type() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Sword",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            10,
        ));
        inv.add_item(make_item(
            "Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            1,
            1,
            5,
        ));
        inv.add_item(make_item(
            "Bow",
            ItemType::Weapon,
            ItemRarity::Uncommon,
            1,
            1,
            20,
        ));
        let results = inv.filter(&ItemFilter::ByType(ItemType::Weapon));
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1.name, "Sword");
        assert_eq!(results[1].1.name, "Bow");
    }

    #[test]
    fn filter_by_text() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Iron Sword",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            10,
        ));
        inv.add_item(make_item(
            "Iron Shield",
            ItemType::Armor,
            ItemRarity::Common,
            1,
            1,
            8,
        ));
        inv.add_item(make_item(
            "Gold Ring",
            ItemType::Armor,
            ItemRarity::Rare,
            1,
            1,
            50,
        ));
        let results = inv.filter(&ItemFilter::ByText("iron".to_string()));
        assert_eq!(results.len(), 2);
        let upper = inv.filter(&ItemFilter::ByText("IRON".to_string()));
        assert_eq!(upper.len(), 2);
    }

    #[test]
    fn gold_operations() {
        let mut inv = InventoryManager::new(10).with_gold(100);
        assert_eq!(inv.gold_total(), 100);
        inv.add_gold(50);
        assert_eq!(inv.gold_total(), 150);
        assert!(inv.spend_gold(80));
        assert_eq!(inv.gold_total(), 70);
        assert!(!inv.spend_gold(100));
        assert_eq!(inv.gold_total(), 70);
    }

    #[test]
    fn drag_state_transitions() {
        let mut ui = InventoryUI::new();
        assert_eq!(ui.drag_state, DragState::Idle);
        assert!(!ui.drag_state.is_dragging());

        ui.begin_drag(2);
        assert_eq!(ui.drag_state, DragState::Dragging { source_index: 2 });
        assert!(ui.drag_state.is_dragging());
        assert_eq!(ui.drag_state.source(), Some(2));

        ui.update_hover(5);
        assert_eq!(
            ui.drag_state,
            DragState::Hovering {
                source_index: 2,
                target_index: 5
            }
        );

        let result = ui.end_drag();
        assert_eq!(result, Some((2, 5)));
        assert_eq!(ui.drag_state, DragState::Idle);
    }

    #[test]
    fn swap_items() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "A",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            0,
        ));
        inv.add_item(make_item("B", ItemType::Armor, ItemRarity::Common, 1, 1, 0));
        InventorySystem::swap_items(&mut inv, 0, 1);
        assert_eq!(inv.items[0].name, "B");
        assert_eq!(inv.items[1].name, "A");
    }

    #[test]
    fn use_consumable() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Sword",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            10,
        ));
        inv.add_item(make_item(
            "Potion",
            ItemType::Consumable,
            ItemRarity::Common,
            1,
            1,
            5,
        ));
        assert!(InventorySystem::use_consumable(&mut inv, 0).is_none());
        let used = InventorySystem::use_consumable(&mut inv, 1);
        assert!(used.is_some());
        assert_eq!(used.unwrap().name, "Potion");
        assert_eq!(inv.items.len(), 1);
    }

    #[test]
    fn find_by_name() {
        let mut inv = InventoryManager::new(10);
        inv.add_item(make_item(
            "Alpha",
            ItemType::Weapon,
            ItemRarity::Common,
            1,
            1,
            0,
        ));
        inv.add_item(make_item(
            "Beta",
            ItemType::Armor,
            ItemRarity::Common,
            1,
            1,
            0,
        ));
        inv.add_item(make_item(
            "Gamma",
            ItemType::Material,
            ItemRarity::Common,
            1,
            1,
            0,
        ));
        assert_eq!(inv.find_by_name("Beta"), Some(1));
        assert_eq!(inv.find_by_name("Omega"), None);
    }
}

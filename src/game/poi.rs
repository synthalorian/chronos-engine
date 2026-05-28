use std::collections::HashMap;

// ── PoiType ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoiType {
    Town,
    Dungeon,
    Camp,
    Resource,
    Shrine,
    Landmark,
    QuestLocation,
}

impl PoiType {
    pub fn name(&self) -> &str {
        match self {
            PoiType::Town => "Town",
            PoiType::Dungeon => "Dungeon",
            PoiType::Camp => "Camp",
            PoiType::Resource => "Resource",
            PoiType::Shrine => "Shrine",
            PoiType::Landmark => "Landmark",
            PoiType::QuestLocation => "Quest Location",
        }
    }

    pub fn is_enterable(&self) -> bool {
        matches!(self, PoiType::Town | PoiType::Dungeon | PoiType::Camp | PoiType::Shrine)
    }

    pub fn icon(&self) -> &str {
        match self {
            PoiType::Town => "\u{1F3D8}",
            PoiType::Dungeon => "\u{2694}",
            PoiType::Camp => "\u{26FA}",
            PoiType::Resource => "\u{1F48E}",
            PoiType::Shrine => "\u{26E9}",
            PoiType::Landmark => "\u{1F4CD}",
            PoiType::QuestLocation => "\u{2757}",
        }
    }
}

// ── PointOfInterest ──

#[derive(Debug, Clone, PartialEq)]
pub struct PointOfInterest {
    pub id: u32,
    pub name: String,
    pub poi_type: PoiType,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub discovered: bool,
    pub level_requirement: u32,
    pub description: String,
    pub linked_quest: Option<u32>,
}

impl PointOfInterest {
    pub fn new(id: u32, name: &str, poi_type: PoiType, x: f32, y: f32, z: f32) -> Self {
        Self {
            id,
            name: name.to_string(),
            poi_type,
            x,
            y,
            z,
            discovered: false,
            level_requirement: 0,
            description: String::new(),
            linked_quest: None,
        }
    }

    pub fn with_level_requirement(mut self, level: u32) -> Self {
        self.level_requirement = level;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_linked_quest(mut self, quest_id: u32) -> Self {
        self.linked_quest = Some(quest_id);
        self
    }

    pub fn discover(&mut self) {
        self.discovered = true;
    }

    pub fn is_discovered(&self) -> bool {
        self.discovered
    }

    pub fn distance_to(&self, x: f32, y: f32, z: f32) -> f32 {
        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn can_enter(&self, player_level: u32) -> bool {
        self.discovered
            && player_level >= self.level_requirement
            && self.poi_type.is_enterable()
    }
}

// ── PoiRegistry ──

#[derive(Debug, Clone, PartialEq)]
pub struct PoiRegistry {
    pub pois: HashMap<u32, PointOfInterest>,
    pub next_id: u32,
}

impl PoiRegistry {
    pub fn new() -> Self {
        Self {
            pois: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn register(&mut self, mut poi: PointOfInterest) -> u32 {
        if poi.id == 0 {
            poi.id = self.next_id;
            self.next_id += 1;
        }
        let id = poi.id;
        self.pois.insert(id, poi);
        id
    }

    pub fn get(&self, id: u32) -> Option<&PointOfInterest> {
        self.pois.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut PointOfInterest> {
        self.pois.get_mut(&id)
    }

    pub fn remove(&mut self, id: u32) -> Option<PointOfInterest> {
        self.pois.remove(&id)
    }

    pub fn all(&self) -> Vec<&PointOfInterest> {
        self.pois.values().collect()
    }

    pub fn discovered(&self) -> Vec<&PointOfInterest> {
        self.pois.values().filter(|p| p.discovered).collect()
    }

    pub fn by_type(&self, poi_type: PoiType) -> Vec<&PointOfInterest> {
        self.pois.values().filter(|p| p.poi_type == poi_type).collect()
    }

    pub fn nearest(&self, x: f32, y: f32, z: f32) -> Option<&PointOfInterest> {
        self.pois
            .values()
            .min_by(|a, b| {
                a.distance_to(x, y, z)
                    .partial_cmp(&b.distance_to(x, y, z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn nearest_of_type(&self, poi_type: PoiType, x: f32, y: f32, z: f32) -> Option<&PointOfInterest> {
        self.pois
            .values()
            .filter(|p| p.poi_type == poi_type)
            .min_by(|a, b| {
                a.distance_to(x, y, z)
                    .partial_cmp(&b.distance_to(x, y, z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn in_radius(&self, x: f32, y: f32, z: f32, radius: f32) -> Vec<&PointOfInterest> {
        self.pois
            .values()
            .filter(|p| p.distance_to(x, y, z) <= radius)
            .collect()
    }
}

impl Default for PoiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── DiscoverySystem ──

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoverySystem {
    pub discovery_radius: f32,
}

impl DiscoverySystem {
    pub fn new(radius: f32) -> Self {
        Self {
            discovery_radius: radius,
        }
    }

    pub fn check_discoveries(
        &self,
        registry: &mut PoiRegistry,
        player_x: f32,
        player_y: f32,
        player_z: f32,
    ) -> Vec<u32> {
        let radius = self.discovery_radius;
        let mut discovered_ids = Vec::new();
        for poi in registry.pois.values_mut() {
            if !poi.discovered && poi.distance_to(player_x, player_y, player_z) <= radius {
                poi.discover();
                discovered_ids.push(poi.id);
            }
        }
        discovered_ids
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poi_type_properties() {
        let cases: Vec<(PoiType, &str, bool, &str)> = vec![
            (PoiType::Town, "Town", true, "\u{1F3D8}"),
            (PoiType::Dungeon, "Dungeon", true, "\u{2694}"),
            (PoiType::Camp, "Camp", true, "\u{26FA}"),
            (PoiType::Resource, "Resource", false, "\u{1F48E}"),
            (PoiType::Shrine, "Shrine", true, "\u{26E9}"),
            (PoiType::Landmark, "Landmark", false, "\u{1F4CD}"),
            (PoiType::QuestLocation, "Quest Location", false, "\u{2757}"),
        ];
        for (pt, name, enterable, icon) in cases {
            assert_eq!(pt.name(), name);
            assert_eq!(pt.is_enterable(), enterable);
            assert_eq!(pt.icon(), icon);
        }
    }

    #[test]
    fn poi_creation_and_builder() {
        let poi = PointOfInterest::new(1, "Ironhold", PoiType::Town, 10.0, 20.0, 5.0)
            .with_level_requirement(5)
            .with_description("A fortified mining town.")
            .with_linked_quest(42);
        assert_eq!(poi.id, 1);
        assert_eq!(poi.name, "Ironhold");
        assert_eq!(poi.poi_type, PoiType::Town);
        assert!((poi.x - 10.0).abs() < f32::EPSILON);
        assert!((poi.y - 20.0).abs() < f32::EPSILON);
        assert!((poi.z - 5.0).abs() < f32::EPSILON);
        assert!(!poi.discovered);
        assert_eq!(poi.level_requirement, 5);
        assert_eq!(poi.description, "A fortified mining town.");
        assert_eq!(poi.linked_quest, Some(42));
    }

    #[test]
    fn poi_discovery() {
        let mut poi = PointOfInterest::new(1, "Shrine", PoiType::Shrine, 0.0, 0.0, 0.0);
        assert!(!poi.is_discovered());
        poi.discover();
        assert!(poi.is_discovered());
    }

    #[test]
    fn poi_distance() {
        let poi = PointOfInterest::new(1, "A", PoiType::Landmark, 1.0, 2.0, 3.0);
        // distance from origin: sqrt(1+4+9) = sqrt(14)
        let d = poi.distance_to(0.0, 0.0, 0.0);
        assert!((d - 14.0_f32.sqrt()).abs() < 0.001);
        // distance to self is zero
        assert!((poi.distance_to(1.0, 2.0, 3.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn poi_can_enter() {
        let mut poi = PointOfInterest::new(1, "Crypt", PoiType::Dungeon, 0.0, 0.0, 0.0)
            .with_level_requirement(10);
        // not discovered yet
        assert!(!poi.can_enter(15));
        poi.discover();
        // level too low
        assert!(!poi.can_enter(5));
        // good
        assert!(poi.can_enter(10));
        assert!(poi.can_enter(20));
        // non-enterable type
        let mut landmark =
            PointOfInterest::new(2, "Obelisk", PoiType::Landmark, 0.0, 0.0, 0.0);
        landmark.discover();
        assert!(!landmark.can_enter(1));
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = PoiRegistry::new();
        let poi = PointOfInterest::new(10, "Village", PoiType::Town, 0.0, 0.0, 0.0);
        let id = reg.register(poi);
        assert_eq!(id, 10);
        assert!(reg.get(10).is_some());
        assert_eq!(reg.get(10).unwrap().name, "Village");
        // mutate
        reg.get_mut(10).unwrap().discover();
        assert!(reg.get(10).unwrap().discovered);
        // remove
        let removed = reg.remove(10);
        assert!(removed.is_some());
        assert!(reg.get(10).is_none());
    }

    #[test]
    fn registry_auto_assign_id() {
        let mut reg = PoiRegistry::new();
        let poi = PointOfInterest::new(0, "Auto", PoiType::Camp, 0.0, 0.0, 0.0);
        let id = reg.register(poi);
        assert_eq!(id, 1);
        assert_eq!(reg.get(1).unwrap().id, 1);
        let poi2 = PointOfInterest::new(0, "Auto2", PoiType::Camp, 1.0, 1.0, 1.0);
        let id2 = reg.register(poi2);
        assert_eq!(id2, 2);
    }

    #[test]
    fn registry_discovered_filter() {
        let mut reg = PoiRegistry::new();
        let mut p1 = PointOfInterest::new(1, "A", PoiType::Town, 0.0, 0.0, 0.0);
        let p2 = PointOfInterest::new(2, "B", PoiType::Town, 1.0, 1.0, 1.0);
        p1.discover();
        reg.register(p1);
        reg.register(p2);
        let disc = reg.discovered();
        assert_eq!(disc.len(), 1);
        assert_eq!(disc[0].name, "A");
    }

    #[test]
    fn registry_by_type() {
        let mut reg = PoiRegistry::new();
        reg.register(PointOfInterest::new(1, "Town1", PoiType::Town, 0.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(2, "Crypt", PoiType::Dungeon, 0.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(3, "Town2", PoiType::Town, 5.0, 5.0, 5.0));
        let towns = reg.by_type(PoiType::Town);
        assert_eq!(towns.len(), 2);
        let dungeons = reg.by_type(PoiType::Dungeon);
        assert_eq!(dungeons.len(), 1);
        let shrines = reg.by_type(PoiType::Shrine);
        assert!(shrines.is_empty());
    }

    #[test]
    fn registry_nearest() {
        let mut reg = PoiRegistry::new();
        reg.register(PointOfInterest::new(1, "Far", PoiType::Town, 100.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(2, "Close", PoiType::Town, 1.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(3, "Mid", PoiType::Town, 10.0, 0.0, 0.0));
        let nearest = reg.nearest(0.0, 0.0, 0.0).unwrap();
        assert_eq!(nearest.name, "Close");
        let nearest_town = reg.nearest_of_type(PoiType::Town, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(nearest_town.name, "Close");
        assert!(reg.nearest_of_type(PoiType::Shrine, 0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn registry_in_radius() {
        let mut reg = PoiRegistry::new();
        reg.register(PointOfInterest::new(1, "A", PoiType::Town, 0.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(2, "B", PoiType::Town, 5.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(3, "C", PoiType::Town, 50.0, 0.0, 0.0));
        let within = reg.in_radius(0.0, 0.0, 0.0, 10.0);
        assert_eq!(within.len(), 2);
        let within_large = reg.in_radius(0.0, 0.0, 0.0, 100.0);
        assert_eq!(within_large.len(), 3);
    }

    #[test]
    fn discovery_system() {
        let mut reg = PoiRegistry::new();
        reg.register(PointOfInterest::new(1, "A", PoiType::Town, 2.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(2, "B", PoiType::Town, 20.0, 0.0, 0.0));
        reg.register(PointOfInterest::new(3, "C", PoiType::Town, 1.0, 1.0, 0.0));
        let ds = DiscoverySystem::new(5.0);
        let found = ds.check_discoveries(&mut reg, 0.0, 0.0, 0.0);
        assert_eq!(found.len(), 2);
        assert!(found.contains(&1));
        assert!(found.contains(&3));
        assert!(!found.contains(&2));
        // B is still undiscovered
        assert!(!reg.get(2).unwrap().is_discovered());
        // A and C are now discovered
        assert!(reg.get(1).unwrap().is_discovered());
        assert!(reg.get(3).unwrap().is_discovered());
        // Running again discovers nothing new
        let found2 = ds.check_discoveries(&mut reg, 0.0, 0.0, 0.0);
        assert!(found2.is_empty());
    }
}

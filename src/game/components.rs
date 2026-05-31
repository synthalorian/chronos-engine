/// Marks an entity as selectable by the player.
#[derive(Debug, Clone, Copy)]
pub struct Selectable;

/// Marks an entity as currently selected.
#[derive(Debug, Clone, Copy)]
pub struct Selected;

/// Visual selection ring rendered around a selected entity.
#[derive(Debug, Clone, Copy)]
pub struct SelectionRing {
    pub color: [f32; 4],
    pub radius: f32,
    pub visible: bool,
}

impl SelectionRing {
    pub fn new(radius: f32) -> Self {
        SelectionRing {
            color: [0.0, 1.0, 0.0, 1.0],
            radius,
            visible: false,
        }
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }
}

/// Which side an entity belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

/// Links an entity to a squad by ID.
#[derive(Debug, Clone, Copy)]
pub struct SquadMember {
    pub squad_id: u32,
}

impl SquadMember {
    pub fn new(squad_id: u32) -> Self {
        SquadMember { squad_id }
    }
}

/// 3D movement target for navigation.
#[derive(Debug, Clone, Copy)]
pub struct MoveTarget {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl MoveTarget {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        MoveTarget { x, y, z }
    }

    /// Euclidean distance from this target to an arbitrary 3D position.
    pub fn distance_to(&self, pos: [f32; 3]) -> f32 {
        let dx = self.x - pos[0];
        let dy = self.y - pos[1];
        let dz = self.z - pos[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Pathfinding state for a navigating entity.
#[derive(Debug, Clone)]
pub struct NavigationAgent {
    pub path: Option<Vec<[f32; 3]>>,
    pub path_index: usize,
    pub speed: f32,
}

impl NavigationAgent {
    pub fn new(speed: f32) -> Self {
        NavigationAgent {
            path: None,
            path_index: 0,
            speed,
        }
    }

    pub fn with_path(mut self, waypoints: Vec<[f32; 3]>) -> Self {
        self.path = Some(waypoints);
        self.path_index = 0;
        self
    }
}

/// RPG-style stats for a mercenary unit.
#[derive(Debug, Clone)]
pub struct MercenaryStats {
    pub name: String,
    pub level: u32,
    pub xp: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub vitality: u32,
}

impl MercenaryStats {
    pub fn new(name: impl Into<String>) -> Self {
        MercenaryStats {
            name: name.into(),
            level: 1,
            xp: 0,
            strength: 10,
            dexterity: 10,
            intelligence: 10,
            vitality: 10,
        }
    }

    pub fn with_stats(
        mut self,
        strength: u32,
        dexterity: u32,
        intelligence: u32,
        vitality: u32,
    ) -> Self {
        self.strength = strength;
        self.dexterity = dexterity;
        self.intelligence = intelligence;
        self.vitality = vitality;
        self
    }
}

/// Floating health bar rendered above an entity.
#[derive(Debug, Clone, Copy)]
pub struct HealthBar {
    pub width: f32,
    pub height: f32,
    pub offset_y: f32,
    pub show: bool,
}

impl HealthBar {
    pub fn new(width: f32, height: f32) -> Self {
        HealthBar {
            width,
            height,
            offset_y: 1.5,
            show: true,
        }
    }
}

/// Radius within which an entity detects and engages enemies.
#[derive(Debug, Clone, Copy)]
pub struct AggroRadius {
    pub radius: f32,
}

impl AggroRadius {
    pub fn new(radius: f32) -> Self {
        AggroRadius { radius }
    }
}

/// Loot dropped when the entity dies.
#[derive(Debug, Clone)]
pub struct LootDrop {
    pub gold: u32,
    pub items: Vec<String>,
}

impl LootDrop {
    pub fn new(gold: u32) -> Self {
        LootDrop {
            gold,
            items: Vec::new(),
        }
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_equality() {
        assert_eq!(Team::Player, Team::Player);
        assert_ne!(Team::Player, Team::Enemy);
        assert_ne!(Team::Neutral, Team::Enemy);
    }

    #[test]
    fn mercenary_stats_creation() {
        let stats = MercenaryStats::new("Grimjaw");
        assert_eq!(stats.name, "Grimjaw");
        assert_eq!(stats.level, 1);
        assert_eq!(stats.xp, 0);
        assert_eq!(stats.strength, 10);
        assert_eq!(stats.dexterity, 10);
        assert_eq!(stats.intelligence, 10);
        assert_eq!(stats.vitality, 10);
    }

    #[test]
    fn navigation_agent_default_path() {
        let agent = NavigationAgent::new(3.5);
        assert!(agent.path.is_none());
        assert_eq!(agent.path_index, 0);
        assert!((agent.speed - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn move_target_distance() {
        let target = MoveTarget::new(3.0, 0.0, 4.0);
        let dist = target.distance_to([0.0, 0.0, 0.0]);
        assert!((dist - 5.0).abs() < 1e-4);
    }
}

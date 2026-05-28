#[cfg(feature = "game")]

// ── SaveVersion ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveVersion {
    pub major: u32,
    pub minor: u32,
}

impl SaveVersion {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn current() -> Self {
        Self { major: 1, minor: 0 }
    }

    pub fn is_compatible(&self, other: &SaveVersion) -> bool {
        self.major == other.major
    }
}

// ── PlayerSaveData ──

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSaveData {
    pub name: String,
    pub level: u32,
    pub xp: u32,
    pub gold: u32,
    pub stats: [u32; 4],
    pub position: [f32; 3],
    pub current_region: String,
    pub play_time_seconds: u64,
}

impl PlayerSaveData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            level: 1,
            xp: 0,
            gold: 0,
            stats: [10, 10, 10, 10],
            position: [0.0, 0.0, 0.0],
            current_region: "Plains".to_string(),
            play_time_seconds: 0,
        }
    }
}

// ── WorldSaveData ──

#[derive(Debug, Clone, PartialEq)]
pub struct WorldSaveData {
    pub seed: u64,
    pub map_width: usize,
    pub map_height: usize,
    pub explored_cells: Vec<(usize, usize)>,
    pub day: u32,
    pub hour: f32,
    pub completed_jobs: Vec<u32>,
    pub discovered_pois: Vec<u32>,
    pub completed_encounters: Vec<u32>,
}

impl WorldSaveData {
    pub fn new(seed: u64, width: usize, height: usize) -> Self {
        Self {
            seed,
            map_width: width,
            map_height: height,
            explored_cells: Vec::new(),
            day: 1,
            hour: 8.0,
            completed_jobs: Vec::new(),
            discovered_pois: Vec::new(),
            completed_encounters: Vec::new(),
        }
    }
}

// ── FactionSaveData ──

#[derive(Debug, Clone, PartialEq)]
pub struct FactionSaveData {
    pub entries: Vec<(String, i32, u32)>,
}

impl FactionSaveData {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

// ── SaveSlot ──

#[derive(Debug, Clone, PartialEq)]
pub struct SaveSlot {
    pub id: u32,
    pub name: String,
    pub timestamp: u64,
    pub version: SaveVersion,
    pub player: PlayerSaveData,
    pub world: WorldSaveData,
    pub factions: FactionSaveData,
}

impl SaveSlot {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            timestamp: 0,
            version: SaveVersion::current(),
            player: PlayerSaveData::new(""),
            world: WorldSaveData::new(0, 0, 0),
            factions: FactionSaveData::new(),
        }
    }
}

// ── SaveError ──

#[derive(Debug, Clone, PartialEq)]
pub enum SaveError {
    SlotFull,
    SlotNotFound,
    InvalidSlot,
    VersionMismatch,
    SaveFailed(String),
}

// ── SaveManager ──

pub struct SaveManager {
    pub slots: Vec<SaveSlot>,
    pub max_slots: usize,
    pub auto_save_enabled: bool,
    pub auto_save_interval_seconds: u64,
    pub last_auto_save: u64,
}

impl SaveManager {
    pub fn new(max_slots: usize) -> Self {
        Self {
            slots: Vec::new(),
            max_slots,
            auto_save_enabled: true,
            auto_save_interval_seconds: 300,
            last_auto_save: 0,
        }
    }

    pub fn save(
        &mut self,
        slot_id: u32,
        player: PlayerSaveData,
        world: WorldSaveData,
        factions: FactionSaveData,
    ) -> Result<u32, SaveError> {
        if slot_id == 0 {
            // Find first empty slot id
            let used_ids: Vec<u32> = self.slots.iter().map(|s| s.id).collect();
            let next_id = (1..=self.max_slots as u32)
                .find(|id| !used_ids.contains(id))
                .ok_or(SaveError::SlotFull)?;

            let slot = SaveSlot {
                id: next_id,
                name: player.name.clone(),
                timestamp: 0,
                version: SaveVersion::current(),
                player,
                world,
                factions,
            };
            self.slots.push(slot);
            return Ok(next_id);
        }

        // Update existing slot
        if let Some(slot) = self.slots.iter_mut().find(|s| s.id == slot_id) {
            slot.player = player;
            slot.world = world;
            slot.factions = factions;
            slot.version = SaveVersion::current();
            return Ok(slot_id);
        }

        // New slot with specified id
        if self.slots.len() >= self.max_slots {
            return Err(SaveError::SlotFull);
        }

        let slot = SaveSlot {
            id: slot_id,
            name: player.name.clone(),
            timestamp: 0,
            version: SaveVersion::current(),
            player,
            world,
            factions,
        };
        self.slots.push(slot);
        Ok(slot_id)
    }

    pub fn load(&self, slot_id: u32) -> Result<&SaveSlot, SaveError> {
        self.slots
            .iter()
            .find(|s| s.id == slot_id)
            .ok_or(SaveError::SlotNotFound)
    }

    pub fn delete(&mut self, slot_id: u32) -> Result<SaveSlot, SaveError> {
        let idx = self
            .slots
            .iter()
            .position(|s| s.id == slot_id)
            .ok_or(SaveError::SlotNotFound)?;
        Ok(self.slots.remove(idx))
    }

    pub fn list_slots(&self) -> Vec<&SaveSlot> {
        self.slots.iter().collect()
    }

    pub fn has_slot(&self, slot_id: u32) -> bool {
        self.slots.iter().any(|s| s.id == slot_id)
    }

    pub fn should_auto_save(&self, current_time: u64) -> bool {
        self.auto_save_enabled
            && current_time.saturating_sub(self.last_auto_save) >= self.auto_save_interval_seconds
    }

    pub fn mark_auto_saved(&mut self, current_time: u64) {
        self.last_auto_save = current_time;
    }
}

// ── SaveSerializer ──

pub struct SaveSerializer;

impl SaveSerializer {
    pub fn serialize_slot(slot: &SaveSlot) -> String {
        let mut out = String::new();
        out.push_str(&format!("slot_id={}\n", slot.id));
        out.push_str(&format!("name={}\n", slot.name));
        out.push_str(&format!("timestamp={}\n", slot.timestamp));
        out.push_str(&format!(
            "version={}.{}\n",
            slot.version.major, slot.version.minor
        ));
        out.push_str("\n[player]\n");
        out.push_str(&Self::serialize_player(&slot.player));
        out.push_str("\n[world]\n");
        out.push_str(&Self::serialize_world(&slot.world));
        out.push_str("\n[factions]\n");
        for (name, rep, jobs) in &slot.factions.entries {
            out.push_str(&format!("faction={} reputation={} jobs={}\n", name, rep, jobs));
        }
        out
    }

    pub fn serialize_player(data: &PlayerSaveData) -> String {
        let mut out = String::new();
        out.push_str(&format!("name={}\n", data.name));
        out.push_str(&format!("level={}\n", data.level));
        out.push_str(&format!("xp={}\n", data.xp));
        out.push_str(&format!("gold={}\n", data.gold));
        out.push_str(&format!(
            "stats=STR:{} DEX:{} INT:{} VIT:{}\n",
            data.stats[0], data.stats[1], data.stats[2], data.stats[3]
        ));
        out.push_str(&format!(
            "position={:.2},{:.2},{:.2}\n",
            data.position[0], data.position[1], data.position[2]
        ));
        out.push_str(&format!("region={}\n", data.current_region));
        out.push_str(&format!("play_time_seconds={}\n", data.play_time_seconds));
        out
    }

    pub fn serialize_world(data: &WorldSaveData) -> String {
        let mut out = String::new();
        out.push_str(&format!("seed={}\n", data.seed));
        out.push_str(&format!("map_width={}\n", data.map_width));
        out.push_str(&format!("map_height={}\n", data.map_height));
        out.push_str(&format!(
            "explored={}\n",
            data.explored_cells
                .iter()
                .map(|(x, y)| format!("({},{})", x, y))
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str(&format!("day={}\n", data.day));
        out.push_str(&format!("hour={:.2}\n", data.hour));
        out.push_str(&format!(
            "completed_jobs={}\n",
            data.completed_jobs
                .iter()
                .map(|j| j.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str(&format!(
            "discovered_pois={}\n",
            data.discovered_pois
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str(&format!(
            "completed_encounters={}\n",
            data.completed_encounters
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        out
    }

    pub fn checksum(data: &str) -> u64 {
        let mut hash: u64 = 0;
        for ch in data.chars() {
            hash = hash.wrapping_mul(31).wrapping_add(ch as u64);
        }
        hash
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_version_compatibility() {
        let v1 = SaveVersion::new(1, 0);
        let v2 = SaveVersion::new(1, 5);
        let v3 = SaveVersion::new(2, 0);

        assert!(v1.is_compatible(&v2));
        assert!(v2.is_compatible(&v1));
        assert!(!v1.is_compatible(&v3));
        assert!(!v3.is_compatible(&v1));
        assert!(v1.is_compatible(&SaveVersion::current()));
    }

    #[test]
    fn player_save_data_defaults() {
        let p = PlayerSaveData::new("TestHero");
        assert_eq!(p.name, "TestHero");
        assert_eq!(p.level, 1);
        assert_eq!(p.xp, 0);
        assert_eq!(p.gold, 0);
        assert_eq!(p.stats, [10, 10, 10, 10]);
        assert_eq!(p.position, [0.0, 0.0, 0.0]);
        assert_eq!(p.current_region, "Plains");
        assert_eq!(p.play_time_seconds, 0);
    }

    #[test]
    fn world_save_data_creation() {
        let w = WorldSaveData::new(42, 100, 200);
        assert_eq!(w.seed, 42);
        assert_eq!(w.map_width, 100);
        assert_eq!(w.map_height, 200);
        assert!(w.explored_cells.is_empty());
        assert!(w.completed_jobs.is_empty());
        assert!(w.discovered_pois.is_empty());
        assert!(w.completed_encounters.is_empty());
    }

    #[test]
    fn save_slot_creation() {
        let slot = SaveSlot::new(3, "MySave");
        assert_eq!(slot.id, 3);
        assert_eq!(slot.name, "MySave");
        assert_eq!(slot.timestamp, 0);
        assert_eq!(slot.version, SaveVersion::current());
    }

    #[test]
    fn save_manager_save_and_load() {
        let mut mgr = SaveManager::new(10);
        let player = PlayerSaveData::new("Hero");
        let world = WorldSaveData::new(99, 50, 50);
        let factions = FactionSaveData::new();

        let id = mgr.save(0, player.clone(), world.clone(), factions.clone()).unwrap();
        assert!(id >= 1);

        let loaded = mgr.load(id).unwrap();
        assert_eq!(loaded.player.name, "Hero");
        assert_eq!(loaded.world.seed, 99);

        // Update existing
        let updated_player = PlayerSaveData { level: 5, ..PlayerSaveData::new("Hero") };
        mgr.save(id, updated_player, world, factions).unwrap();
        assert_eq!(mgr.load(id).unwrap().player.level, 5);
    }

    #[test]
    fn save_manager_slot_full() {
        let mut mgr = SaveManager::new(2);
        let player = PlayerSaveData::new("A");
        let world = WorldSaveData::new(1, 10, 10);
        let factions = FactionSaveData::new();

        mgr.save(1, player.clone(), world.clone(), factions.clone()).unwrap();
        mgr.save(2, player.clone(), world.clone(), factions.clone()).unwrap();
        let result = mgr.save(3, player, world, factions);
        assert_eq!(result, Err(SaveError::SlotFull));
    }

    #[test]
    fn save_manager_delete() {
        let mut mgr = SaveManager::new(10);
        let player = PlayerSaveData::new("X");
        let world = WorldSaveData::new(1, 10, 10);
        let factions = FactionSaveData::new();

        mgr.save(5, player.clone(), world.clone(), factions.clone()).unwrap();
        assert!(mgr.has_slot(5));

        let deleted = mgr.delete(5).unwrap();
        assert_eq!(deleted.id, 5);
        assert!(!mgr.has_slot(5));
        assert_eq!(mgr.load(5), Err(SaveError::SlotNotFound));
    }

    #[test]
    fn save_manager_list_slots() {
        let mut mgr = SaveManager::new(10);
        assert!(mgr.list_slots().is_empty());

        let player = PlayerSaveData::new("A");
        let world = WorldSaveData::new(1, 10, 10);
        let factions = FactionSaveData::new();

        mgr.save(1, player.clone(), world.clone(), factions.clone()).unwrap();
        mgr.save(2, player.clone(), world.clone(), factions.clone()).unwrap();

        let listed = mgr.list_slots();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn save_manager_auto_save() {
        let mut mgr = SaveManager::new(10);
        assert!(mgr.should_auto_save(300));
        assert!(mgr.should_auto_save(500));

        mgr.mark_auto_saved(300);
        assert!(!mgr.should_auto_save(400));
        assert!(mgr.should_auto_save(600));
    }

    #[test]
    fn save_serializer_format() {
        let slot = SaveSlot {
            id: 1,
            name: "Test".to_string(),
            timestamp: 1000,
            version: SaveVersion::new(1, 0),
            player: PlayerSaveData::new("Test"),
            world: WorldSaveData::new(42, 100, 100),
            factions: FactionSaveData {
                entries: vec![("Guild".to_string(), 50, 3)],
            },
        };

        let text = SaveSerializer::serialize_slot(&slot);
        assert!(text.contains("slot_id=1"));
        assert!(text.contains("name=Test"));
        assert!(text.contains("timestamp=1000"));
        assert!(text.contains("version=1.0"));
        assert!(text.contains("[player]"));
        assert!(text.contains("[world]"));
        assert!(text.contains("[factions]"));
        assert!(text.contains("faction=Guild reputation=50 jobs=3"));
        assert!(text.contains("seed=42"));

        let player_text = SaveSerializer::serialize_player(&slot.player);
        assert!(player_text.contains("level=1"));
        assert!(player_text.contains("region=Plains"));

        let world_text = SaveSerializer::serialize_world(&slot.world);
        assert!(world_text.contains("map_width=100"));
    }

    #[test]
    fn save_serializer_checksum() {
        let a = SaveSerializer::checksum("hello");
        let b = SaveSerializer::checksum("hello");
        let c = SaveSerializer::checksum("world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn save_error_variants() {
        let errors = vec![
            SaveError::SlotFull,
            SaveError::SlotNotFound,
            SaveError::InvalidSlot,
            SaveError::VersionMismatch,
            SaveError::SaveFailed("io error".to_string()),
        ];
        assert_eq!(errors.len(), 5);
        assert_eq!(SaveError::SlotFull, SaveError::SlotFull);
        assert_eq!(
            SaveError::SaveFailed("x".to_string()),
            SaveError::SaveFailed("x".to_string())
        );
    }
}

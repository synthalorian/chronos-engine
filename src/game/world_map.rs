#[cfg(feature = "game")]
// ── Region ──────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Plains,
    Forest,
    Mountain,
    Desert,
    Swamp,
    Coastal,
    Town,
    Dungeon,
}

impl Region {
    pub fn name(&self) -> &str {
        match self {
            Region::Plains => "Plains",
            Region::Forest => "Forest",
            Region::Mountain => "Mountain",
            Region::Desert => "Desert",
            Region::Swamp => "Swamp",
            Region::Coastal => "Coastal",
            Region::Town => "Town",
            Region::Dungeon => "Dungeon",
        }
    }

    pub fn difficulty(&self) -> u32 {
        match self {
            Region::Plains => 1,
            Region::Forest => 2,
            Region::Swamp => 3,
            Region::Coastal => 2,
            Region::Desert => 4,
            Region::Mountain => 5,
            Region::Town => 1,
            Region::Dungeon => 6,
        }
    }

    pub fn is_hostile(&self) -> bool {
        !matches!(self, Region::Town)
    }
}

// ── WorldCell ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WorldCell {
    pub region: Region,
    pub elevation: f32,
    pub walkable: bool,
    pub explored: bool,
    pub poi_id: Option<u32>,
}

impl WorldCell {
    pub fn new(region: Region) -> Self {
        WorldCell {
            region,
            elevation: 0.5,
            walkable: true,
            explored: false,
            poi_id: None,
        }
    }

    pub fn with_elevation(mut self, e: f32) -> Self {
        self.elevation = e;
        self
    }

    pub fn with_walkable(mut self, w: bool) -> Self {
        self.walkable = w;
        self
    }

    pub fn mark_explored(&mut self) {
        self.explored = true;
    }
}

// ── WorldMap ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WorldMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<WorldCell>>,
    pub name: String,
    pub seed: u64,
}

impl WorldMap {
    pub fn new(name: &str, width: usize, height: usize, seed: u64) -> Self {
        let cells = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let elev = Self::procedural_elevation(x, y, seed);
                        WorldCell::new(Region::Plains).with_elevation(elev)
                    })
                    .collect()
            })
            .collect();

        WorldMap {
            width,
            height,
            cells,
            name: name.to_string(),
            seed,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&WorldCell> {
        if self.in_bounds(x, y) {
            Some(&self.cells[y][x])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut WorldCell> {
        if self.in_bounds(x, y) {
            Some(&mut self.cells[y][x])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, cell: WorldCell) {
        if self.in_bounds(x, y) {
            self.cells[y][x] = cell;
        }
    }

    pub fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    pub fn region_at(&self, x: usize, y: usize) -> Option<Region> {
        self.get(x, y).map(|c| c.region)
    }

    pub fn difficulty_at(&self, x: usize, y: usize) -> u32 {
        self.region_at(x, y).map(|r| r.difficulty()).unwrap_or(0)
    }

    pub fn explored_cells(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[y][x].explored {
                    result.push((x, y));
                }
            }
        }
        result
    }

    pub fn explore_radius(&mut self, cx: usize, cy: usize, radius: usize) {
        let r = radius as i64;
        let cy_i = cy as i64;
        let cx_i = cx as i64;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let nx = cx_i + dx;
                    let ny = cy_i + dy;
                    if nx >= 0 && ny >= 0 {
                        if let Some(cell) = self.get_mut(nx as usize, ny as usize) {
                            cell.mark_explored();
                        }
                    }
                }
            }
        }
    }

    pub fn cells_in_region(&self, region: Region) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[y][x].region == region {
                    result.push((x, y));
                }
            }
        }
        result
    }

    pub fn walkable_neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let dirs: [(i64, i64); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
        let mut result = Vec::new();
        let xi = x as i64;
        let yi = y as i64;

        for (dx, dy) in &dirs {
            let nx = xi + dx;
            let ny = yi + dy;
            if nx >= 0 && ny >= 0 {
                let ux = nx as usize;
                let uy = ny as usize;
                if let Some(cell) = self.get(ux, uy) {
                    if cell.walkable {
                        result.push((ux, uy));
                    }
                }
            }
        }
        result
    }

    pub fn procedural_elevation(x: usize, y: usize, seed: u64) -> f32 {
        let hash = ((x as u64).wrapping_mul(73856093)
            ^ (y as u64).wrapping_mul(19349663)
            ^ seed.wrapping_mul(83492791))
            % 1000;
        hash as f32 / 1000.0
    }
}

// ── WorldGenerator ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WorldGenerator;

impl WorldGenerator {
    fn elevation_to_region(elevation: f32) -> Region {
        if elevation < 0.2 {
            Region::Swamp
        } else if elevation < 0.35 {
            Region::Coastal
        } else if elevation < 0.55 {
            Region::Plains
        } else if elevation < 0.7 {
            Region::Forest
        } else if elevation < 0.85 {
            Region::Mountain
        } else {
            Region::Desert
        }
    }

    pub fn generate_biome_map(width: usize, height: usize, seed: u64) -> WorldMap {
        let mut map = WorldMap::new("Generated World", width, height, seed);

        // Assign regions based on elevation thresholds
        for y in 0..height {
            for x in 0..width {
                let elevation = map.cells[y][x].elevation;
                let region = Self::elevation_to_region(elevation);
                map.cells[y][x].region = region;
            }
        }

        // Place Town at center
        let town_x = width / 2;
        let town_y = height / 2;
        Self::place_town(&mut map, town_x, town_y);

        // Place Dungeon at highest elevation point
        let mut max_elev = -1.0_f32;
        let mut dungeon_x = 0;
        let mut dungeon_y = 0;
        for y in 0..height {
            for x in 0..width {
                let e = map.cells[y][x].elevation;
                if e > max_elev {
                    max_elev = e;
                    dungeon_x = x;
                    dungeon_y = y;
                }
            }
        }
        Self::place_dungeon(&mut map, dungeon_x, dungeon_y);

        map
    }

    pub fn place_town(map: &mut WorldMap, x: usize, y: usize) {
        map.set(
            x,
            y,
            WorldCell {
                region: Region::Town,
                elevation: map.get(x, y).map(|c| c.elevation).unwrap_or(0.5),
                walkable: true,
                explored: true,
                poi_id: None,
            },
        );
    }

    pub fn place_dungeon(map: &mut WorldMap, x: usize, y: usize) {
        map.set(
            x,
            y,
            WorldCell {
                region: Region::Dungeon,
                elevation: map.get(x, y).map(|c| c.elevation).unwrap_or(0.5),
                walkable: true,
                explored: false,
                poi_id: None,
            },
        );
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_properties() {
        let cases: Vec<(Region, &str, u32, bool)> = vec![
            (Region::Plains, "Plains", 1, true),
            (Region::Forest, "Forest", 2, true),
            (Region::Swamp, "Swamp", 3, true),
            (Region::Coastal, "Coastal", 2, true),
            (Region::Desert, "Desert", 4, true),
            (Region::Mountain, "Mountain", 5, true),
            (Region::Town, "Town", 1, false),
            (Region::Dungeon, "Dungeon", 6, true),
        ];
        for (region, name, diff, hostile) in &cases {
            assert_eq!(region.name(), *name);
            assert_eq!(region.difficulty(), *diff);
            assert_eq!(region.is_hostile(), *hostile);
        }
    }

    #[test]
    fn world_cell_defaults() {
        let cell = WorldCell::new(Region::Forest);
        assert_eq!(cell.region, Region::Forest);
        assert!((cell.elevation - 0.5).abs() < f32::EPSILON);
        assert!(cell.walkable);
        assert!(!cell.explored);
        assert!(cell.poi_id.is_none());
    }

    #[test]
    fn world_cell_builder() {
        let cell = WorldCell::new(Region::Mountain)
            .with_elevation(0.9)
            .with_walkable(false);
        assert!((cell.elevation - 0.9).abs() < f32::EPSILON);
        assert!(!cell.walkable);
    }

    #[test]
    fn world_map_creation() {
        let map = WorldMap::new("Test", 5, 4, 42);
        assert_eq!(map.width, 5);
        assert_eq!(map.height, 4);
        assert_eq!(map.name, "Test");
        assert_eq!(map.seed, 42);
        assert_eq!(map.cells.len(), 4);
        assert_eq!(map.cells[0].len(), 5);
        for row in &map.cells {
            for cell in row {
                assert_eq!(cell.region, Region::Plains);
            }
        }
    }

    #[test]
    fn get_set_cells() {
        let mut map = WorldMap::new("T", 3, 3, 0);
        let cell = WorldCell::new(Region::Dungeon).with_elevation(0.99);
        map.set(1, 2, cell.clone());
        let fetched = map.get(1, 2).unwrap();
        assert_eq!(*fetched, cell);
        assert!(map.get(5, 5).is_none());
    }

    #[test]
    fn in_bounds() {
        let map = WorldMap::new("T", 10, 10, 0);
        assert!(map.in_bounds(0, 0));
        assert!(map.in_bounds(9, 9));
        assert!(!map.in_bounds(10, 0));
        assert!(!map.in_bounds(0, 10));
        assert!(!map.in_bounds(10, 10));
    }

    #[test]
    fn explore_radius() {
        let mut map = WorldMap::new("T", 10, 10, 0);
        map.explore_radius(5, 5, 1);
        let explored = map.explored_cells();
        // radius 1: center + 4 neighbors = 5 cells
        assert_eq!(explored.len(), 5);
        assert!(explored.contains(&(5, 5)));
        assert!(explored.contains(&(4, 5)));
        assert!(explored.contains(&(6, 5)));
        assert!(explored.contains(&(5, 4)));
        assert!(explored.contains(&(5, 6)));
    }

    #[test]
    fn explored_cells_list() {
        let mut map = WorldMap::new("T", 5, 5, 0);
        assert!(map.explored_cells().is_empty());
        map.cells[0][0].mark_explored();
        map.cells[2][3].mark_explored();
        let list = map.explored_cells();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&(0, 0)));
        assert!(list.contains(&(3, 2)));
    }

    #[test]
    fn cells_in_region_filter() {
        let mut map = WorldMap::new("T", 3, 3, 0);
        map.cells[0][1].region = Region::Desert;
        map.cells[2][2].region = Region::Desert;
        let desert = map.cells_in_region(Region::Desert);
        assert_eq!(desert.len(), 2);
        assert!(desert.contains(&(1, 0)));
        assert!(desert.contains(&(2, 2)));
        let plains = map.cells_in_region(Region::Plains);
        assert_eq!(plains.len(), 7);
    }

    #[test]
    fn walkable_neighbors() {
        let mut map = WorldMap::new("T", 5, 5, 0);
        // Block right neighbor (x=2, y=2) → cells[2][2]
        map.cells[2][2] = WorldCell::new(Region::Mountain).with_walkable(false);
        let neighbors = map.walkable_neighbors(1, 2);
        // (x=1,y=2): up=(1,1), down=(1,3), left=(0,2), right=(2,2) blocked
        assert_eq!(neighbors.len(), 3);
        assert!(neighbors.contains(&(1, 1)));
        assert!(neighbors.contains(&(1, 3)));
        assert!(neighbors.contains(&(0, 2)));
        assert!(!neighbors.contains(&(2, 2)));
    }

    #[test]
    fn walkable_neighbors_edge() {
        let map = WorldMap::new("T", 5, 5, 0);
        // Corner cell (0,0) — only down and right
        let neighbors = map.walkable_neighbors(0, 0);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&(1, 0)));
        assert!(neighbors.contains(&(0, 1)));
    }

    #[test]
    fn walkable_neighbors_blocked() {
        let mut map = WorldMap::new("T", 5, 5, 0);
        // Block right neighbor of (2,2): that's (3,2) → cells[2][3]
        map.cells[2][3] = WorldCell::new(Region::Mountain).with_walkable(false);
        let neighbors = map.walkable_neighbors(2, 2);
        assert_eq!(neighbors.len(), 3);
        assert!(!neighbors.contains(&(3, 2)));
    }

    #[test]
    fn biome_generation() {
        let map = WorldGenerator::generate_biome_map(20, 20, 12345);
        assert_eq!(map.width, 20);
        assert_eq!(map.height, 20);

        // Town at center
        let center = map.get(10, 10).unwrap();
        assert_eq!(center.region, Region::Town);
        assert!(center.explored);
        assert!(center.walkable);

        // At least one Dungeon exists
        let dungeons = map.cells_in_region(Region::Dungeon);
        assert!(!dungeons.is_empty());

        // At least 3 distinct region types present
        let mut regions = std::collections::HashSet::new();
        for row in &map.cells {
            for cell in row {
                regions.insert(cell.region);
            }
        }
        assert!(
            regions.len() >= 3,
            "Expected >=3 regions, got {}",
            regions.len()
        );
    }

    #[test]
    fn procedural_elevation_deterministic() {
        let a = WorldMap::procedural_elevation(5, 10, 42);
        let b = WorldMap::procedural_elevation(5, 10, 42);
        assert_eq!(a, b);

        // Different inputs should generally produce different outputs
        let c = WorldMap::procedural_elevation(6, 10, 42);
        assert_ne!(a, c);

        // Output is in [0.0, 1.0)
        for x in 0..20 {
            for y in 0..20 {
                let e = WorldMap::procedural_elevation(x, y, 999);
                assert!(e >= 0.0 && e < 1.0, "elevation {} out of range", e);
            }
        }
    }

    #[test]
    fn difficulty_at_out_of_bounds() {
        let map = WorldMap::new("T", 3, 3, 0);
        assert_eq!(map.difficulty_at(0, 0), 1); // Plains
        assert_eq!(map.difficulty_at(99, 99), 0); // out of bounds
    }

    #[test]
    fn get_mut_works() {
        let mut map = WorldMap::new("T", 3, 3, 0);
        if let Some(cell) = map.get_mut(1, 1) {
            cell.region = Region::Desert;
            cell.mark_explored();
        }
        assert_eq!(map.region_at(1, 1), Some(Region::Desert));
        assert!(map.get(1, 1).unwrap().explored);
        assert!(map.get_mut(5, 5).is_none());
    }

    #[test]
    fn place_town_and_dungeon() {
        let mut map = WorldMap::new("T", 5, 5, 0);
        WorldGenerator::place_town(&mut map, 2, 2);
        let town = map.get(2, 2).unwrap();
        assert_eq!(town.region, Region::Town);
        assert!(town.walkable);
        assert!(town.explored);

        WorldGenerator::place_dungeon(&mut map, 4, 4);
        let dungeon = map.get(4, 4).unwrap();
        assert_eq!(dungeon.region, Region::Dungeon);
        assert!(dungeon.walkable);
        assert!(!dungeon.explored);
    }

    #[test]
    fn explore_radius_large() {
        let mut map = WorldMap::new("T", 10, 10, 0);
        map.explore_radius(5, 5, 2);
        let explored = map.explored_cells();
        // radius 2: all cells within euclidean distance 2
        // Center + ring1 (4) + ring2 corners (4) + ring2 edges (4) = 13
        assert_eq!(explored.len(), 13);
    }
}

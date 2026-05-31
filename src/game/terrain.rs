#[cfg(feature = "game")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTile {
    Flat,
    Hill,
    Water,
    Wall,
    Path,
}

impl TerrainTile {
    pub fn is_walkable(&self) -> bool {
        match self {
            TerrainTile::Flat | TerrainTile::Path | TerrainTile::Hill => true,
            TerrainTile::Water | TerrainTile::Wall => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainGrid {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TerrainTile>,
    pub heights: Vec<f32>,
}

impl TerrainGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        TerrainGrid {
            width,
            height,
            tiles: vec![TerrainTile::Flat; size],
            heights: vec![0.0; size],
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile: TerrainTile) {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            self.tiles[idx] = tile;
        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> Option<TerrainTile> {
        if x < self.width && y < self.height {
            Some(self.tiles[self.index(x, y)])
        } else {
            None
        }
    }

    pub fn set_height(&mut self, x: usize, y: usize, h: f32) {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            self.heights[idx] = h;
        }
    }

    pub fn get_height(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.heights[self.index(x, y)]
        } else {
            0.0
        }
    }

    pub fn is_walkable(&self, x: usize, y: usize) -> bool {
        self.get_tile(x, y).is_some_and(|t| t.is_walkable())
    }

    pub fn world_to_grid(world_x: f32, world_z: f32, cell_size: f32) -> (usize, usize) {
        let gx = if world_x >= 0.0 {
            (world_x / cell_size) as usize
        } else {
            0
        };
        let gy = if world_z >= 0.0 {
            (world_z / cell_size) as usize
        } else {
            0
        };
        (gx, gy)
    }

    pub fn grid_to_world(gx: usize, gy: usize, cell_size: f32) -> (f32, f32) {
        let wx = (gx as f32 + 0.5) * cell_size;
        let wz = (gy as f32 + 0.5) * cell_size;
        (wx, wz)
    }

    pub fn generate_heightmap_terrain(width: usize, height: usize, seed: u64) -> Self {
        let mut grid = TerrainGrid::new(width, height);
        let s = seed as f32;

        for y in 0..height {
            for x in 0..width {
                let fx = x as f32;
                let fy = y as f32;

                let h1 = (fx * 0.3 + s).sin() * (fy * 0.3 + s * 0.7).cos() * 2.0;
                let h2 = (fx * 0.15 + s * 1.3).cos() * (fy * 0.15 + s * 0.4).sin() * 1.5;
                let h3 = (fx * 0.6 + fy * 0.6 + s * 0.9).sin() * 0.5;
                let raw = (h1 + h2 + h3).abs();

                grid.set_height(x, y, raw);

                let tile = if raw < 0.5 {
                    TerrainTile::Flat
                } else if raw < 1.5 {
                    TerrainTile::Hill
                } else if raw < 2.5 {
                    TerrainTile::Path
                } else {
                    TerrainTile::Wall
                };
                grid.set_tile(x, y, tile);
            }
        }

        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_walkability() {
        assert!(TerrainTile::Flat.is_walkable());
        assert!(TerrainTile::Path.is_walkable());
        assert!(TerrainTile::Hill.is_walkable());
        assert!(!TerrainTile::Water.is_walkable());
        assert!(!TerrainTile::Wall.is_walkable());
    }

    #[test]
    fn grid_set_get_tile() {
        let mut grid = TerrainGrid::new(5, 5);
        assert_eq!(grid.get_tile(2, 3), Some(TerrainTile::Flat));

        grid.set_tile(2, 3, TerrainTile::Wall);
        assert_eq!(grid.get_tile(2, 3), Some(TerrainTile::Wall));

        assert_eq!(grid.get_tile(10, 10), None);
        grid.set_tile(10, 10, TerrainTile::Water);
        assert_eq!(grid.get_tile(10, 10), None);
    }

    #[test]
    fn world_grid_coordinate_conversion() {
        let cell = 2.0;

        let (gx, gy) = TerrainGrid::world_to_grid(5.0, 9.0, cell);
        assert_eq!(gx, 2);
        assert_eq!(gy, 4);

        let (wx, wz) = TerrainGrid::grid_to_world(gx, gy, cell);
        assert!((wx - 5.0).abs() < f32::EPSILON);
        assert!((wz - 9.0).abs() < f32::EPSILON);

        let (gx0, gy0) = TerrainGrid::world_to_grid(-1.0, -1.0, cell);
        assert_eq!((gx0, gy0), (0, 0));
    }

    #[test]
    fn heightmap_generation_nonzero() {
        let grid = TerrainGrid::generate_heightmap_terrain(10, 10, 42);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);

        let has_nonzero = grid.heights.iter().any(|&h| h > 0.01);
        assert!(has_nonzero, "heightmap should produce non-zero heights");

        let mut variety = std::collections::HashSet::new();
        for &h in &grid.heights {
            variety.insert((h * 10.0) as u32);
        }
        assert!(variety.len() > 1, "heightmap should have height variety");
    }

    #[test]
    fn height_set_get() {
        let mut grid = TerrainGrid::new(3, 3);
        assert!((grid.get_height(1, 1) - 0.0).abs() < f32::EPSILON);

        grid.set_height(1, 1, 5.5);
        assert!((grid.get_height(1, 1) - 5.5).abs() < f32::EPSILON);

        assert!((grid.get_height(99, 99) - 0.0).abs() < f32::EPSILON);
    }
}

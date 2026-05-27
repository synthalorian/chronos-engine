//! Tile map system for chunked grid-based level rendering.

/// A single tile in the map.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    /// Atlas frame name for this tile's visual.
    pub frame: u32,
    /// Whether this tile blocks movement.
    pub solid: bool,
}

/// A chunk of tiles (fixed-size grid section).
pub const CHUNK_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct TileChunk {
    /// Grid position of this chunk (chunk_x, chunk_y).
    pub cx: i32,
    pub cy: i32,
    /// Tiles in row-major order (CHUNK_SIZE × CHUNK_SIZE).
    pub tiles: [[Tile; CHUNK_SIZE]; CHUNK_SIZE],
}

impl TileChunk {
    pub fn new(cx: i32, cy: i32) -> Self {
        TileChunk {
            cx,
            cy,
            tiles: [[Tile { frame: 0, solid: false }; CHUNK_SIZE]; CHUNK_SIZE],
        }
    }

    pub fn get_tile(&self, local_x: usize, local_y: usize) -> &Tile {
        &self.tiles[local_y][local_x]
    }

    pub fn set_tile(&mut self, local_x: usize, local_y: usize, tile: Tile) {
        self.tiles[local_y][local_x] = tile;
    }
}

/// A tile map composed of chunks.
pub struct TileMap {
    /// Tile size in world units.
    pub tile_size: f32,
    /// Active chunks keyed by (chunk_x, chunk_y).
    chunks: std::collections::HashMap<(i32, i32), TileChunk>,
}

impl TileMap {
    pub fn new(tile_size: f32) -> Self {
        TileMap {
            tile_size,
            chunks: std::collections::HashMap::new(),
        }
    }

    /// Get or create a chunk at the given chunk coordinates.
    pub fn get_or_create_chunk(&mut self, cx: i32, cy: i32) -> &mut TileChunk {
        self.chunks
            .entry((cx, cy))
            .or_insert_with(|| TileChunk::new(cx, cy))
    }

    /// Set a tile at world coordinates.
    pub fn set_tile(&mut self, world_x: i32, world_y: i32, tile: Tile) {
        let cx = world_x.div_euclid(CHUNK_SIZE as i32);
        let cy = world_y.div_euclid(CHUNK_SIZE as i32);
        let lx = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let ly = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;
        self.get_or_create_chunk(cx, cy).set_tile(lx, ly, tile);
    }

    /// Get a tile at world coordinates.
    pub fn get_tile(&self, world_x: i32, world_y: i32) -> Option<&Tile> {
        let cx = world_x.div_euclid(CHUNK_SIZE as i32);
        let cy = world_y.div_euclid(CHUNK_SIZE as i32);
        let lx = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let ly = world_y.rem_euclid(CHUNK_SIZE as i32) as usize;
        self.chunks
            .get(&(cx, cy))
            .map(|chunk| chunk.get_tile(lx, ly))
    }

    /// Get chunks visible within a camera viewport.
    pub fn visible_chunks(
        &self,
        cam_x: f32,
        cam_y: f32,
        view_w: f32,
        view_h: f32,
    ) -> Vec<&TileChunk> {
        let half_w = view_w / 2.0;
        let half_h = view_h / 2.0;
        let chunk_world = self.tile_size * CHUNK_SIZE as f32;

        let min_cx = ((cam_x - half_w) / chunk_world).floor() as i32;
        let max_cx = ((cam_x + half_w) / chunk_world).ceil() as i32;
        let min_cy = ((cam_y - half_h) / chunk_world).floor() as i32;
        let max_cy = ((cam_y + half_h) / chunk_world).ceil() as i32;

        let mut visible = Vec::new();
        for cy in min_cy..=max_cy {
            for cx in min_cx..=max_cx {
                if let Some(chunk) = self.chunks.get(&(cx, cy)) {
                    visible.push(chunk);
                }
            }
        }
        visible
    }

    /// Get the total number of chunks.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Iterator over all chunks.
    pub fn chunks(&self) -> impl Iterator<Item = &TileChunk> {
        self.chunks.values()
    }
}

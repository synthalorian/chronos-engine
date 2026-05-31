//! Chronos Engine — General Systems (Phase 8D)
//!
//! 2D camera improvements, tilemap enhancements, 2D pathfinding, and audio zones.
//!
//! This module provides:
//! - `Camera2D` — 2D camera with follow, shake, bounds, and coordinate transforms
//! - `TileLayer` / `TilemapEx` — layered tilemap with autotiling and collision
//! - `Pathfinder2D` — A* pathfinding decoupled from any specific grid via `PathfindingGrid` trait
//! - `AudioZone` / `AudioZoneManager` / `FootstepSystem` — spatial audio zone blending and footstep tracking

// ── Subsystem 1: Camera2D ──────────────────────────────────────────────────

/// Optional movement bounds for a 2D camera.
#[derive(Debug, Clone, Copy)]
pub struct CameraBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl CameraBounds {
    pub fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Self {
        CameraBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Clamp a world-space position within these bounds.
    pub fn clamp(&self, pos: [f32; 2]) -> [f32; 2] {
        [
            pos[0].clamp(self.min_x, self.max_x),
            pos[1].clamp(self.min_y, self.max_y),
        ]
    }

    /// Returns `true` if `pos` lies within the bounds (inclusive).
    pub fn contains(&self, pos: [f32; 2]) -> bool {
        pos[0] >= self.min_x && pos[0] <= self.max_x && pos[1] >= self.min_y && pos[1] <= self.max_y
    }
}

/// 2D camera with follow target, screen shake, bounds, and coordinate transforms.
#[derive(Debug, Clone)]
pub struct Camera2D {
    pub position: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
    pub viewport_size: [f32; 2],
    pub bounds: Option<CameraBounds>,
    pub shake_offset: [f32; 2],
    pub shake_intensity: f32,
    pub shake_decay: f32,
    pub follow_target: Option<[f32; 2]>,
    pub follow_speed: f32,
}

impl Camera2D {
    pub fn new(width: f32, height: f32) -> Self {
        Camera2D {
            position: [0.0, 0.0],
            zoom: 1.0,
            rotation: 0.0,
            viewport_size: [width, height],
            bounds: None,
            shake_offset: [0.0, 0.0],
            shake_intensity: 0.0,
            shake_decay: 5.0,
            follow_target: None,
            follow_speed: 3.0,
        }
    }

    // ── Builder methods ──

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }

    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    pub fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    pub fn with_bounds(mut self, bounds: CameraBounds) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn with_follow_speed(mut self, speed: f32) -> Self {
        self.follow_speed = speed;
        self
    }

    // ── Matrices ──

    /// 2D view transform as a 3×3 matrix (translate + rotate + scale).
    ///
    /// Column-major layout: `[[col0], [col1], [col2]]`.
    pub fn view_matrix(&self) -> [[f32; 3]; 3] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let z = self.zoom;
        let inv_z = 1.0 / z;
        let px = -(self.position[0] * cos - self.position[1] * sin) * inv_z;
        let py = -(self.position[0] * sin + self.position[1] * cos) * inv_z;

        [
            [cos * inv_z, sin * inv_z, 0.0],
            [-sin * inv_z, cos * inv_z, 0.0],
            [px, py, 1.0],
        ]
    }

    /// Orthographic projection matrix (3×3).
    pub fn projection_matrix(&self) -> [[f32; 3]; 3] {
        let hw = 2.0 / self.viewport_size[0];
        let hh = 2.0 / self.viewport_size[1];
        [[hw, 0.0, 0.0], [0.0, hh, 0.0], [0.0, 0.0, 1.0]]
    }

    /// Convert a screen-space pixel coordinate to world space.
    pub fn screen_to_world(&self, screen_pos: [f32; 2]) -> [f32; 2] {
        let half_w = self.viewport_size[0] / 2.0;
        let half_h = self.viewport_size[1] / 2.0;
        let nx = (screen_pos[0] - half_w) / half_w;
        let ny = -(screen_pos[1] - half_h) / half_h;

        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let z = self.zoom;
        let rx = (nx * half_w) / z;
        let ry = (ny * half_h) / z;

        [
            rx * cos - ry * sin + self.position[0],
            rx * sin + ry * cos + self.position[1],
        ]
    }

    /// Convert a world-space coordinate to screen-space pixels.
    pub fn world_to_screen(&self, world_pos: [f32; 2]) -> [f32; 2] {
        let dx = world_pos[0] - self.position[0];
        let dy = world_pos[1] - self.position[1];
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let rx = dx * cos + dy * sin;
        let ry = -dx * sin + dy * cos;

        let z = self.zoom;
        let half_w = self.viewport_size[0] / 2.0;
        let half_h = self.viewport_size[1] / 2.0;

        [half_w + rx * z, half_h - ry * z]
    }

    /// Apply shake decay and follow-target interpolation.
    pub fn update(&mut self, dt: f32) {
        // Follow target
        if let Some(target) = self.follow_target {
            let t = (self.follow_speed * dt).min(1.0);
            self.position[0] += (target[0] - self.position[0]) * t;
            self.position[1] += (target[1] - self.position[1]) * t;
        }

        // Clamp to bounds
        if let Some(ref b) = self.bounds {
            self.position = b.clamp(self.position);
        }

        // Shake decay
        if self.shake_intensity > 0.01 {
            let angle =
                (self.position[0] * 7.3 + self.position[1] * 13.7 + self.shake_intensity * 41.1)
                    .to_radians()
                    .sin()
                    .acos();
            self.shake_offset = [
                angle.cos() * self.shake_intensity,
                angle.sin() * self.shake_intensity,
            ];
            self.shake_intensity *= (1.0 - self.shake_decay * dt).max(0.0);
        } else {
            self.shake_intensity = 0.0;
            self.shake_offset = [0.0, 0.0];
        }
    }

    /// Trigger a screen shake of the given intensity.
    pub fn shake(&mut self, intensity: f32) {
        self.shake_intensity = self.shake_intensity.max(intensity);
    }

    /// Immediately move the camera to look at a world position.
    pub fn look_at(&mut self, pos: [f32; 2]) {
        self.position = pos;
        if let Some(ref b) = self.bounds {
            self.position = b.clamp(self.position);
        }
    }

    /// Return the (min, max) visible area in world coordinates.
    pub fn visible_bounds(&self) -> ([f32; 2], [f32; 2]) {
        let half_w = self.viewport_size[0] / 2.0 / self.zoom;
        let half_h = self.viewport_size[1] / 2.0 / self.zoom;
        (
            [self.position[0] - half_w, self.position[1] - half_h],
            [self.position[0] + half_w, self.position[1] + half_h],
        )
    }
}

// ── Subsystem 2: Tilemap Enhancements ──────────────────────────────────────

/// Collision type for a single tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileCollision {
    None,
    Solid,
    OneWay,
    Slope { angle: f32 },
    Damage { amount: f32 },
}

/// Per-tile data: sprite index, collision shape, pathfinding cost.
#[derive(Debug, Clone, Copy)]
pub struct TileData {
    pub sprite_index: u32,
    pub collision: TileCollision,
    pub cost: f32,
}

impl Default for TileData {
    fn default() -> Self {
        TileData {
            sprite_index: 0,
            collision: TileCollision::None,
            cost: 1.0,
        }
    }
}

/// A single layer of tiles in a layered tilemap.
#[derive(Debug, Clone)]
pub struct TileLayer {
    pub name: String,
    pub tiles: Vec<Vec<TileData>>,
    pub z_order: i32,
    pub opacity: f32,
    pub visible: bool,
    pub collision_enabled: bool,
}

impl TileLayer {
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        TileLayer {
            name: name.to_string(),
            tiles: vec![vec![TileData::default(); width]; height],
            z_order: 0,
            opacity: 1.0,
            visible: true,
            collision_enabled: true,
        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> Option<&TileData> {
        self.tiles.get(y).and_then(|row| row.get(x))
    }

    pub fn set_tile(&mut self, x: usize, y: usize, tile: TileData) {
        if let Some(row) = self.tiles.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                *cell = tile;
            }
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.tiles
            .resize_with(new_height, || vec![TileData::default(); new_width]);
        for row in &mut self.tiles {
            row.resize(new_width, TileData::default());
        }
    }
}

/// Autotiling rule: maps a center tile + neighbor connections to variant sprites.
#[derive(Debug, Clone)]
pub struct AutotileRule {
    pub center_tile: u32,
    pub result_tiles: [u32; 4], // [up, right, down, left] connected variants
}

impl AutotileRule {
    pub fn new(center_tile: u32, result_tiles: [u32; 4]) -> Self {
        AutotileRule {
            center_tile,
            result_tiles,
        }
    }

    /// Given neighbor connectivity [up, right, down, left], return the
    /// appropriate variant tile index, or `None` if no rule applies.
    pub fn matches(&self, neighbors: [bool; 4]) -> Option<u32> {
        let connected_count = neighbors.iter().filter(|&&b| b).count();
        if connected_count == 0 {
            return Some(self.center_tile);
        }
        for (i, &connected) in neighbors.iter().enumerate() {
            if connected {
                return Some(self.result_tiles[i]);
            }
        }
        Some(self.center_tile)
    }
}

/// Enhanced tilemap with layers, collision, and autotiling.
#[derive(Debug, Clone)]
pub struct TilemapEx {
    pub layers: Vec<TileLayer>,
    pub tile_size: f32,
    pub autotile_rules: Vec<AutotileRule>,
}

impl TilemapEx {
    pub fn new(tile_size: f32) -> Self {
        TilemapEx {
            layers: Vec::new(),
            tile_size,
            autotile_rules: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, layer: TileLayer) -> usize {
        let idx = self.layers.len();
        self.layers.push(layer);
        idx
    }

    /// Apply autotiling rules to a single tile on the given layer.
    pub fn apply_autotile(&mut self, layer_index: usize, x: usize, y: usize) {
        if layer_index >= self.layers.len() {
            return;
        }
        let layer = &self.layers[layer_index];
        let current = match layer.get_tile(x, y) {
            Some(t) => t.sprite_index,
            None => return,
        };

        let neighbors = [
            layer
                .get_tile(x, y.saturating_sub(1))
                .is_some_and(|t| t.sprite_index == current),
            layer
                .get_tile(x + 1, y)
                .is_some_and(|t| t.sprite_index == current),
            layer
                .get_tile(x, y + 1)
                .is_some_and(|t| t.sprite_index == current),
            layer
                .get_tile(x.saturating_sub(1), y)
                .is_some_and(|t| t.sprite_index == current),
        ];

        for rule in &self.autotile_rules {
            if rule.center_tile == current {
                if let Some(new_tile) = rule.matches(neighbors) {
                    if let Some(tile) = self.layers[layer_index]
                        .tiles
                        .get_mut(y)
                        .and_then(|r| r.get_mut(x))
                    {
                        tile.sprite_index = new_tile;
                    }
                    return;
                }
            }
        }
    }

    /// Get the collision type of a tile on a specific layer.
    pub fn get_collision(&self, x: usize, y: usize, layer: usize) -> TileCollision {
        self.layers
            .get(layer)
            .and_then(|l| l.get_tile(x, y))
            .map(|t| t.collision)
            .unwrap_or(TileCollision::None)
    }

    /// Check if a tile is passable (not solid) considering all collision-enabled layers.
    pub fn walkable(&self, x: usize, y: usize) -> bool {
        for layer in &self.layers {
            if !layer.collision_enabled {
                continue;
            }
            if let Some(tile) = layer.get_tile(x, y) {
                if tile.collision == TileCollision::Solid {
                    return false;
                }
            }
        }
        true
    }

    /// Width of the first layer (0 if no layers exist).
    pub fn width(&self) -> usize {
        self.layers
            .first()
            .map(|l| l.tiles.first().map_or(0, |r| r.len()))
            .unwrap_or(0)
    }

    /// Height of the first layer (0 if no layers exist).
    pub fn height(&self) -> usize {
        self.layers.first().map(|l| l.tiles.len()).unwrap_or(0)
    }
}

// ── Subsystem 3: 2D Pathfinding ────────────────────────────────────────────

/// Trait that decouples `Pathfinder2D` from any specific tile/grid representation.
pub trait PathfindingGrid {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn is_walkable(&self, x: usize, y: usize) -> bool;

    /// Movement cost to enter tile (x, y). Default is 1.0.
    fn cost(&self, _x: usize, _y: usize) -> f32 {
        1.0
    }
}

/// Internal A* node.
#[derive(Debug, Clone)]
struct PathNode {
    position: [usize; 2],
    g_cost: f32,
    h_cost: f32,
    parent: Option<[usize; 2]>,
}

impl PathNode {
    fn f_cost(&self) -> f32 {
        self.g_cost + self.h_cost
    }
}

/// Result of a weighted pathfinding query.
#[derive(Debug, Clone)]
pub struct PathResult {
    pub path: Vec<[usize; 2]>,
    pub total_cost: f32,
    pub nodes_explored: usize,
}

/// A* pathfinder operating on any `PathfindingGrid`.
pub struct Pathfinder2D;

impl Pathfinder2D {
    /// Find the shortest path (4-directional) from `start` to `end`.
    pub fn find_path(
        grid: &dyn PathfindingGrid,
        start: [usize; 2],
        end: [usize; 2],
    ) -> Option<Vec<[usize; 2]>> {
        Self::find_path_weighted(grid, start, end).map(|r| r.path)
    }

    /// Find a weighted path and return full statistics.
    pub fn find_path_weighted(
        grid: &dyn PathfindingGrid,
        start: [usize; 2],
        end: [usize; 2],
    ) -> Option<PathResult> {
        let (sx, sy) = (start[0], start[1]);
        let (ex, ey) = (end[0], end[1]);

        if sx >= grid.width() || sy >= grid.height() || ex >= grid.width() || ey >= grid.height() {
            return None;
        }

        if !grid.is_walkable(sx, sy) || !grid.is_walkable(ex, ey) {
            return None;
        }

        if sx == ex && sy == ey {
            return Some(PathResult {
                path: vec![start],
                total_cost: 0.0,
                nodes_explored: 1,
            });
        }

        let mut open: Vec<PathNode> = Vec::new();
        let mut closed: Vec<([usize; 2], Option<[usize; 2]>)> = Vec::new();
        let mut nodes_explored = 0usize;

        let h = manhattan(sx, sy, ex, ey);
        open.push(PathNode {
            position: [sx, sy],
            g_cost: 0.0,
            h_cost: h,
            parent: None,
        });

        let directions: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

        while !open.is_empty() {
            // Select node with lowest f-cost
            let mut best_idx = 0;
            for i in 1..open.len() {
                if open[i].f_cost() < open[best_idx].f_cost() {
                    best_idx = i;
                }
            }
            let current = open.remove(best_idx);
            nodes_explored += 1;

            // Goal reached
            if current.position[0] == ex && current.position[1] == ey {
                closed.push((current.position, current.parent));
                let path = reconstruct_path(&closed, [ex, ey]);
                let total_cost = current.g_cost;
                return Some(PathResult {
                    path,
                    total_cost,
                    nodes_explored,
                });
            }

            closed.push((current.position, current.parent));

            for (dx, dy) in &directions {
                let nx = current.position[0] as i32 + dx;
                let ny = current.position[1] as i32 + dy;

                if nx < 0 || ny < 0 {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                if nx >= grid.width() || ny >= grid.height() {
                    continue;
                }
                if !grid.is_walkable(nx, ny) {
                    continue;
                }
                if closed.iter().any(|(pos, _)| pos[0] == nx && pos[1] == ny) {
                    continue;
                }

                let move_cost = grid.cost(nx, ny);
                let tentative_g = current.g_cost + move_cost;

                // Check existing open node
                if let Some(existing) = open
                    .iter()
                    .find(|n| n.position[0] == nx && n.position[1] == ny)
                {
                    if tentative_g >= existing.g_cost {
                        continue;
                    }
                }
                open.retain(|n| n.position[0] != nx || n.position[1] != ny);

                open.push(PathNode {
                    position: [nx, ny],
                    g_cost: tentative_g,
                    h_cost: manhattan(nx, ny, ex, ey),
                    parent: Some(current.position),
                });
            }
        }

        None
    }
}

#[inline]
fn manhattan(x1: usize, y1: usize, x2: usize, y2: usize) -> f32 {
    (x1 as f32 - x2 as f32).abs() + (y1 as f32 - y2 as f32).abs()
}

fn reconstruct_path(
    closed: &[([usize; 2], Option<[usize; 2]>)],
    end: [usize; 2],
) -> Vec<[usize; 2]> {
    let mut path = Vec::new();
    let mut current = Some(end);
    while let Some(pos) = current {
        path.push(pos);
        if let Some((_, parent)) = closed
            .iter()
            .find(|(p, _)| p[0] == pos[0] && p[1] == pos[1])
        {
            current = *parent;
        } else {
            break;
        }
    }
    path.reverse();
    path
}

// ── Subsystem 4: Audio Zones ───────────────────────────────────────────────

/// A circular spatial audio zone.
#[derive(Debug, Clone)]
pub struct AudioZone {
    pub name: String,
    pub center: [f32; 2],
    pub radius: f32,
    pub ambient_track: String,
    pub volume: f32,
    pub priority: i32,
    pub blend_distance: f32,
}

impl AudioZone {
    pub fn new(name: &str, x: f32, y: f32, radius: f32) -> Self {
        AudioZone {
            name: name.to_string(),
            center: [x, y],
            radius,
            ambient_track: String::new(),
            volume: 1.0,
            priority: 0,
            blend_distance: radius * 0.3,
        }
    }

    pub fn with_track(mut self, track: &str) -> Self {
        self.ambient_track = track.to_string();
        self
    }

    pub fn with_volume(mut self, vol: f32) -> Self {
        self.volume = vol;
        self
    }

    pub fn with_priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }

    pub fn with_blend_distance(mut self, d: f32) -> Self {
        self.blend_distance = d;
        self
    }

    /// Returns `true` if `pos` is inside the zone (within `radius`).
    pub fn contains(&self, pos: [f32; 2]) -> bool {
        let dx = pos[0] - self.center[0];
        let dy = pos[1] - self.center[1];
        (dx * dx + dy * dy) <= self.radius * self.radius
    }

    /// Returns 0.0 at the edge (or outside), ramping to 1.0 at the center.
    pub fn distance_factor(&self, pos: [f32; 2]) -> f32 {
        let dx = pos[0] - self.center[0];
        let dy = pos[1] - self.center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist >= self.radius {
            0.0
        } else if dist <= self.radius - self.blend_distance {
            1.0
        } else {
            (self.radius - dist) / self.blend_distance
        }
    }
}

/// Update result returned by `AudioZoneManager::update`.
#[derive(Debug, Clone)]
pub struct AudioZoneUpdate {
    pub active_zone: Option<String>,
    pub volume: f32,
    pub should_crossfade: bool,
}

/// Manages multiple overlapping audio zones.
#[derive(Debug, Clone)]
pub struct AudioZoneManager {
    pub zones: Vec<AudioZone>,
    pub active_zone: Option<usize>,
}

impl Default for AudioZoneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioZoneManager {
    pub fn new() -> Self {
        AudioZoneManager {
            zones: Vec::new(),
            active_zone: None,
        }
    }

    pub fn add_zone(&mut self, zone: AudioZone) -> usize {
        let idx = self.zones.len();
        self.zones.push(zone);
        idx
    }

    pub fn remove_zone(&mut self, index: usize) {
        if index < self.zones.len() {
            self.zones.remove(index);
            // Adjust active_zone index if necessary
            match self.active_zone {
                Some(ai) if ai == index => self.active_zone = None,
                Some(ai) if ai > index => self.active_zone = Some(ai - 1),
                _ => {}
            }
        }
    }

    /// Evaluate which zone the listener is in. Returns transition info.
    pub fn update(&mut self, listener_pos: [f32; 2]) -> AudioZoneUpdate {
        // Find the highest-priority zone the listener is inside
        let mut best: Option<(usize, f32)> = None; // (index, distance_factor)

        for (i, zone) in self.zones.iter().enumerate() {
            if zone.contains(listener_pos) {
                let factor = zone.distance_factor(listener_pos);
                match best {
                    None => best = Some((i, factor)),
                    Some((bi, bf)) => {
                        // Higher priority wins; on tie, higher factor wins
                        if zone.priority > self.zones[bi].priority
                            || (zone.priority == self.zones[bi].priority && factor > bf)
                        {
                            best = Some((i, factor));
                        }
                    }
                }
            }
        }

        let previous = self.active_zone;
        self.active_zone = best.map(|(i, _)| i);

        let should_crossfade = previous != self.active_zone;

        let (name, vol) = match best {
            Some((i, factor)) => {
                let z = &self.zones[i];
                (Some(z.name.clone()), z.volume * factor)
            }
            None => (None, 0.0),
        };

        AudioZoneUpdate {
            active_zone: name,
            volume: vol,
            should_crossfade,
        }
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }
}

/// Simple distance-based footstep trigger.
#[derive(Debug, Clone)]
pub struct FootstepSystem {
    pub step_interval: f32,
    pub distance_accumulated: f32,
    pub last_position: [f32; 2],
}

impl FootstepSystem {
    pub fn new(interval: f32) -> Self {
        FootstepSystem {
            step_interval: interval.max(0.0),
            distance_accumulated: 0.0,
            last_position: [0.0, 0.0],
        }
    }

    /// Call each frame with the listener's current position.
    /// Returns `true` when a footstep should play.
    pub fn update(&mut self, position: [f32; 2]) -> bool {
        let dx = position[0] - self.last_position[0];
        let dy = position[1] - self.last_position[1];
        let dist = (dx * dx + dy * dy).sqrt();
        self.last_position = position;

        if self.step_interval <= 0.0 {
            return false;
        }

        self.distance_accumulated += dist;

        if self.distance_accumulated >= self.step_interval {
            self.distance_accumulated -= self.step_interval;
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.distance_accumulated = 0.0;
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Camera2D ──

    #[test]
    fn camera2d_creation_and_defaults() {
        let cam = Camera2D::new(800.0, 600.0);
        assert_eq!(cam.viewport_size, [800.0, 600.0]);
        assert_eq!(cam.position, [0.0, 0.0]);
        assert_eq!(cam.zoom, 1.0);
        assert_eq!(cam.rotation, 0.0);
        assert!(cam.bounds.is_none());
        assert_eq!(cam.shake_offset, [0.0, 0.0]);
        assert_eq!(cam.shake_intensity, 0.0);
        assert!(cam.follow_target.is_none());
    }

    #[test]
    fn camera2d_view_projection_matrices() {
        let cam = Camera2D::new(800.0, 600.0);
        let view = cam.view_matrix();
        // At identity position/rotation/zoom, the view should be essentially identity-like
        assert!((view[0][0] - 1.0).abs() < 1e-5);
        assert!((view[1][1] - 1.0).abs() < 1e-5);
        assert!((view[2][2] - 1.0).abs() < 1e-5);

        let proj = cam.projection_matrix();
        assert!((proj[0][0] - (2.0 / 800.0)).abs() < 1e-5);
        assert!((proj[1][1] - (2.0 / 600.0)).abs() < 1e-5);
    }

    #[test]
    fn camera2d_screen_world_roundtrip() {
        let cam = Camera2D::new(800.0, 600.0).with_position(100.0, 100.0);
        let screen = [400.0, 300.0]; // center of viewport
        let world = cam.screen_to_world(screen);
        // Center of screen should map to camera position
        assert!((world[0] - 100.0).abs() < 1e-3);
        assert!((world[1] - 100.0).abs() < 1e-3);

        // Roundtrip
        let back = cam.world_to_screen(world);
        assert!((back[0] - screen[0]).abs() < 1e-3);
        assert!((back[1] - screen[1]).abs() < 1e-3);
    }

    #[test]
    fn camera2d_shake_and_decay() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.shake(10.0);
        assert!(cam.shake_intensity >= 10.0);

        // Decay over a second
        cam.update(1.0);
        assert!(cam.shake_intensity < 10.0);

        // Eventually stops
        for _ in 0..20 {
            cam.update(1.0);
        }
        assert_eq!(cam.shake_intensity, 0.0);
        assert_eq!(cam.shake_offset, [0.0, 0.0]);
    }

    #[test]
    fn camera2d_follow_target() {
        let mut cam = Camera2D::new(800.0, 600.0).with_follow_speed(10.0);
        cam.follow_target = Some([100.0, 0.0]);
        cam.update(0.1);
        // Should have moved toward target
        assert!(cam.position[0] > 0.0);
        assert!(cam.position[0] <= 100.0);
    }

    // ── CameraBounds ──

    #[test]
    fn camera_bounds_clamp_and_contains() {
        let bounds = CameraBounds::new(-100.0, 100.0, -50.0, 50.0);

        let clamped = bounds.clamp([200.0, -200.0]);
        assert_eq!(clamped, [100.0, -50.0]);

        assert!(bounds.contains([0.0, 0.0]));
        assert!(bounds.contains([100.0, 50.0]));
        assert!(!bounds.contains([101.0, 0.0]));
        assert!(!bounds.contains([0.0, 51.0]));

        // Camera with bounds
        let mut cam = Camera2D::new(800.0, 600.0)
            .with_position(500.0, 0.0)
            .with_bounds(bounds);
        cam.look_at([999.0, 999.0]);
        assert_eq!(cam.position, [100.0, 50.0]);
    }

    // ── TileLayer / TileData / TileCollision ──

    #[test]
    fn tile_layer_creation_get_set_resize() {
        let mut layer = TileLayer::new("ground", 5, 3);
        assert_eq!(layer.name, "ground");
        assert_eq!(layer.tiles.len(), 3);
        assert_eq!(layer.tiles[0].len(), 5);

        // Default tile
        let t = layer.get_tile(0, 0).unwrap();
        assert_eq!(t.sprite_index, 0);
        assert_eq!(t.cost, 1.0);

        // Set tile
        let solid = TileData {
            sprite_index: 5,
            collision: TileCollision::Solid,
            cost: 2.0,
        };
        layer.set_tile(2, 1, solid);
        let t = layer.get_tile(2, 1).unwrap();
        assert_eq!(t.sprite_index, 5);
        assert_eq!(t.collision, TileCollision::Solid);
        assert!((t.cost - 2.0).abs() < 1e-5);

        // Out of bounds
        assert!(layer.get_tile(10, 10).is_none());

        // Resize
        layer.resize(8, 5);
        assert_eq!(layer.tiles.len(), 5);
        assert_eq!(layer.tiles[0].len(), 8);
        // Original data preserved
        let t = layer.get_tile(2, 1).unwrap();
        assert_eq!(t.sprite_index, 5);
    }

    #[test]
    fn tile_collision_variants() {
        let none = TileCollision::None;
        let solid = TileCollision::Solid;
        let oneway = TileCollision::OneWay;
        let slope = TileCollision::Slope { angle: 45.0 };
        let damage = TileCollision::Damage { amount: 10.0 };

        assert_eq!(none, TileCollision::None);
        assert_eq!(solid, TileCollision::Solid);
        assert_eq!(oneway, TileCollision::OneWay);
        assert_eq!(slope, TileCollision::Slope { angle: 45.0 });
        assert_eq!(damage, TileCollision::Damage { amount: 10.0 });
        assert!(slope != TileCollision::Slope { angle: 30.0 });
    }

    // ── AutotileRule ──

    #[test]
    fn autotile_rule_matching() {
        let rule = AutotileRule::new(1, [10, 11, 12, 13]);

        // No neighbors — return center
        assert_eq!(rule.matches([false, false, false, false]), Some(1));

        // Up connected
        assert_eq!(rule.matches([true, false, false, false]), Some(10));

        // Right connected
        assert_eq!(rule.matches([false, true, false, false]), Some(11));

        // Down connected
        assert_eq!(rule.matches([false, false, true, false]), Some(12));

        // Left connected
        assert_eq!(rule.matches([false, false, false, true]), Some(13));
    }

    // ── TilemapEx ──

    #[test]
    fn tilemap_ex_layer_management() {
        let mut tm = TilemapEx::new(32.0);
        assert_eq!(tm.tile_size, 32.0);
        assert_eq!(tm.width(), 0);
        assert_eq!(tm.height(), 0);

        let layer0 = TileLayer::new("background", 10, 8);
        let layer1 = TileLayer::new("foreground", 10, 8);

        let i0 = tm.add_layer(layer0);
        let i1 = tm.add_layer(layer1);
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(tm.width(), 10);
        assert_eq!(tm.height(), 8);

        // Set a solid tile on layer 1
        let solid = TileData {
            sprite_index: 3,
            collision: TileCollision::Solid,
            cost: 1.0,
        };
        tm.layers[1].set_tile(5, 4, solid);

        // Collision check — layer 0 is passable, layer 1 has solid at (5,4)
        assert!(!tm.walkable(5, 4));
        assert!(tm.walkable(0, 0));
        assert_eq!(tm.get_collision(5, 4, 1), TileCollision::Solid);
        assert_eq!(tm.get_collision(5, 4, 0), TileCollision::None);
    }

    // ── Pathfinder2D ──

    /// Simple test grid for pathfinding.
    struct TestGrid {
        width: usize,
        height: usize,
        walls: Vec<[usize; 2]>,
        costs: Vec<([usize; 2], f32)>,
    }

    impl TestGrid {
        fn new(w: usize, h: usize) -> Self {
            TestGrid {
                width: w,
                height: h,
                walls: Vec::new(),
                costs: Vec::new(),
            }
        }
        fn add_wall(&mut self, x: usize, y: usize) {
            self.walls.push([x, y]);
        }
        fn set_cost(&mut self, x: usize, y: usize, cost: f32) {
            self.costs.push(([x, y], cost));
        }
    }

    impl PathfindingGrid for TestGrid {
        fn width(&self) -> usize {
            self.width
        }
        fn height(&self) -> usize {
            self.height
        }
        fn is_walkable(&self, x: usize, y: usize) -> bool {
            !self.walls.iter().any(|&[wx, wy]| wx == x && wy == y)
        }
        fn cost(&self, x: usize, y: usize) -> f32 {
            self.costs
                .iter()
                .find(|&&(pos, _)| pos[0] == x && pos[1] == y)
                .map(|&(_, c)| c)
                .unwrap_or(1.0)
        }
    }

    #[test]
    fn pathfinder_basic_path_open_grid() {
        let grid = TestGrid::new(10, 10);
        let path = Pathfinder2D::find_path(&grid, [0, 0], [5, 5]);
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.first().unwrap(), &[0, 0]);
        assert_eq!(p.last().unwrap(), &[5, 5]);
        // Manhattan distance = 10, so 11 nodes
        assert_eq!(p.len(), 11);
    }

    #[test]
    fn pathfinder_path_with_obstacles() {
        let mut grid = TestGrid::new(10, 10);
        // Wall across the middle, with gap at (5, 5)
        for x in 0..10 {
            grid.add_wall(x, 5);
        }
        grid.walls.retain(|&[x, y]| !(x == 5 && y == 5));

        let path = Pathfinder2D::find_path(&grid, [0, 0], [0, 9]);
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.iter().any(|&pos| pos[0] == 5 && pos[1] == 5));
    }

    #[test]
    fn pathfinder_no_path_when_blocked() {
        let mut grid = TestGrid::new(5, 5);
        for x in 0..5 {
            grid.add_wall(x, 2);
        }
        let path = Pathfinder2D::find_path(&grid, [0, 0], [0, 4]);
        assert!(path.is_none());
    }

    #[test]
    fn pathfinder_weighted_path() {
        let mut grid = TestGrid::new(5, 5);
        // Make the center column expensive
        for y in 0..5 {
            grid.set_cost(2, y, 10.0);
        }
        let result = Pathfinder2D::find_path_weighted(&grid, [0, 0], [4, 0]);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.nodes_explored > 0);
        assert!(r.total_cost > 0.0);
        assert_eq!(r.path.first().unwrap(), &[0, 0]);
        assert_eq!(r.path.last().unwrap(), &[4, 0]);
    }

    #[test]
    fn pathfinder_out_of_bounds_returns_none() {
        let grid = TestGrid::new(5, 5);
        assert!(Pathfinder2D::find_path(&grid, [10, 10], [0, 0]).is_none());
        assert!(Pathfinder2D::find_path(&grid, [0, 0], [10, 10]).is_none());
    }

    #[test]
    fn pathfinder_start_equals_end() {
        let grid = TestGrid::new(5, 5);
        let result = Pathfinder2D::find_path_weighted(&grid, [2, 2], [2, 2]);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path.len(), 1);
        assert_eq!(r.total_cost, 0.0);
    }

    #[test]
    fn pathfinding_grid_trait_default_cost() {
        let grid = TestGrid::new(3, 3);
        assert!((grid.cost(0, 0) - 1.0).abs() < 1e-5);
        assert!((grid.cost(2, 2) - 1.0).abs() < 1e-5);
    }

    // ── AudioZone ──

    #[test]
    fn audio_zone_contains_and_distance_factor() {
        let zone = AudioZone::new("forest", 0.0, 0.0, 100.0).with_blend_distance(30.0);

        // Center
        assert!(zone.contains([0.0, 0.0]));
        assert!((zone.distance_factor([0.0, 0.0]) - 1.0).abs() < 1e-5);

        // Inside, within blend region
        let factor = zone.distance_factor([80.0, 0.0]);
        assert!(factor > 0.0 && factor < 1.0);

        // Edge
        assert!(zone.contains([99.0, 0.0]));
        assert!(!zone.contains([101.0, 0.0]));

        // Outside
        assert!(!zone.contains([200.0, 0.0]));
        assert!((zone.distance_factor([200.0, 0.0])).abs() < 1e-5);
    }

    #[test]
    fn audio_zone_manager_transitions() {
        let mut mgr = AudioZoneManager::new();
        assert_eq!(mgr.zone_count(), 0);

        mgr.add_zone(
            AudioZone::new("forest", 0.0, 0.0, 100.0)
                .with_track("forest_amb")
                .with_volume(0.8),
        );
        mgr.add_zone(
            AudioZone::new("cave", 200.0, 0.0, 80.0)
                .with_track("cave_amb")
                .with_volume(1.0)
                .with_priority(1),
        );
        assert_eq!(mgr.zone_count(), 2);

        // Listener in forest (first update, transitions from None → Some)
        let upd = mgr.update([0.0, 0.0]);
        assert_eq!(upd.active_zone, Some("forest".to_string()));
        assert!(upd.should_crossfade); // first zone entry is a transition

        // Stay in forest — no transition
        let upd = mgr.update([0.0, 0.0]);
        assert!(!upd.should_crossfade);

        // Move to cave — higher priority
        let upd = mgr.update([200.0, 0.0]);
        assert_eq!(upd.active_zone, Some("cave".to_string()));
        assert!(upd.should_crossfade);

        // Stay in cave
        let upd = mgr.update([200.0, 0.0]);
        assert!(!upd.should_crossfade);

        // Move to nowhere
        let upd = mgr.update([500.0, 500.0]);
        assert_eq!(upd.active_zone, None);
        assert!(upd.should_crossfade);
    }

    // ── FootstepSystem ──

    #[test]
    fn footstep_system_step_detection() {
        let mut fs = FootstepSystem::new(2.0);
        assert_eq!(fs.step_interval, 2.0);
        assert_eq!(fs.distance_accumulated, 0.0);

        // Move 1 unit — no step yet
        assert!(!fs.update([1.0, 0.0]));
        assert!((fs.distance_accumulated - 1.0).abs() < 1e-5);

        // Move another 1.5 units — total 2.5, exceeds 2.0
        assert!(fs.update([2.5, 0.0]));

        // Reset
        fs.reset();
        assert_eq!(fs.distance_accumulated, 0.0);
    }

    #[test]
    fn footstep_system_zero_interval() {
        let mut fs = FootstepSystem::new(0.0);
        assert!(!fs.update([100.0, 100.0]));
        assert!(!fs.update([200.0, 200.0]));
    }
}

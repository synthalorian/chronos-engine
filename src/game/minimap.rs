#[cfg(feature = "game")]

// ── Terrain Color Constants ──

pub fn terrain_color_default() -> [f32; 3] {
    [0.3, 0.6, 0.2]
}

pub fn terrain_plains() -> [f32; 3] {
    [0.3, 0.6, 0.2]
}

pub fn terrain_forest() -> [f32; 3] {
    [0.1, 0.4, 0.1]
}

pub fn terrain_mountain() -> [f32; 3] {
    [0.5, 0.45, 0.4]
}

pub fn terrain_desert() -> [f32; 3] {
    [0.8, 0.7, 0.4]
}

pub fn terrain_water() -> [f32; 3] {
    [0.2, 0.3, 0.7]
}

pub fn terrain_town() -> [f32; 3] {
    [0.7, 0.6, 0.3]
}

// ── MinimapCell ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapCell {
    pub explored: bool,
    pub terrain_color: [f32; 3],
    pub has_poi: bool,
    pub has_player: bool,
    pub has_enemy: bool,
}

impl MinimapCell {
    pub fn new() -> Self {
        Self {
            explored: false,
            terrain_color: [0.0, 0.0, 0.0],
            has_poi: false,
            has_player: false,
            has_enemy: false,
        }
    }

    pub fn unexplored() -> Self {
        Self {
            explored: false,
            terrain_color: [0.0, 0.0, 0.0],
            has_poi: false,
            has_player: false,
            has_enemy: false,
        }
    }

    pub fn fog_color() -> [f32; 3] {
        [0.1, 0.1, 0.15]
    }
}

// ── MinimapData ──

pub struct MinimapData {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<MinimapCell>>,
    pub player_x: usize,
    pub player_y: usize,
    pub scale: f32,
}

impl MinimapData {
    pub fn new(width: usize, height: usize, scale: f32) -> Self {
        let cells = vec![vec![MinimapCell::unexplored(); width]; height];
        let player_x = width / 2;
        let player_y = height / 2;
        Self {
            width,
            height,
            cells,
            player_x,
            player_y,
            scale,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&MinimapCell> {
        self.cells.get(y).and_then(|row| row.get(x))
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut MinimapCell> {
        self.cells.get_mut(y).and_then(|row| row.get_mut(x))
    }

    pub fn set_explored(&mut self, x: usize, y: usize, terrain_color: [f32; 3]) {
        if let Some(cell) = self.get_mut(x, y) {
            cell.explored = true;
            cell.terrain_color = terrain_color;
        }
    }

    pub fn update_player_position(&mut self, world_x: f32, world_y: f32) {
        let new_x = (world_x / self.scale).round() as usize;
        let new_y = (world_y / self.scale).round() as usize;
        self.player_x = if new_x < self.width {
            new_x
        } else {
            self.width.saturating_sub(1)
        };
        self.player_y = if new_y < self.height {
            new_y
        } else {
            self.height.saturating_sub(1)
        };
    }

    pub fn mark_poi(&mut self, x: usize, y: usize) {
        if let Some(cell) = self.get_mut(x, y) {
            cell.has_poi = true;
        }
    }

    pub fn mark_enemy(&mut self, x: usize, y: usize) {
        if let Some(cell) = self.get_mut(x, y) {
            cell.has_enemy = true;
        }
    }

    pub fn clear_enemies(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                cell.has_enemy = false;
            }
        }
    }

    pub fn reveal_radius(
        &mut self,
        cx: usize,
        cy: usize,
        radius: usize,
        terrain_colors: &Vec<Vec<[f32; 3]>>,
    ) {
        let r = radius as isize;
        let cy_signed = cy as isize;
        let cx_signed = cx as isize;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let nx = cx_signed + dx;
                    let ny = cy_signed + dy;
                    if nx >= 0 && ny >= 0 {
                        let ux = nx as usize;
                        let uy = ny as usize;
                        if ux < self.width && uy < self.height {
                            let color = terrain_colors
                                .get(uy)
                                .and_then(|row| row.get(ux))
                                .copied()
                                .unwrap_or([0.0, 0.0, 0.0]);
                            self.set_explored(ux, uy, color);
                        }
                    }
                }
            }
        }
    }

    pub fn explored_percentage(&self) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }
        let total = self.cell_count();
        let explored: usize = self
            .cells
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.explored)
            .count();
        (explored as f32 / total as f32) * 100.0
    }

    pub fn cell_count(&self) -> usize {
        self.width * self.height
    }
}

// ── MinimapRenderer ──

#[derive(Debug, Clone, PartialEq)]
pub struct MinimapRenderer {
    pub visible: bool,
    pub position: [f32; 2],
    pub size: f32,
    pub border_color: [f32; 4],
    pub player_color: [f32; 4],
    pub poi_color: [f32; 4],
    pub enemy_color: [f32; 4],
}

impl MinimapRenderer {
    pub fn new() -> Self {
        Self {
            visible: true,
            position: [0.0, 0.0],
            size: 200.0,
            border_color: [1.0, 1.0, 1.0, 0.8],
            player_color: [0.0, 1.0, 0.0, 1.0],
            poi_color: [1.0, 1.0, 0.0, 1.0],
            enemy_color: [1.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn cell_to_screen(
        &self,
        cell_x: usize,
        cell_y: usize,
        map_width: usize,
        map_height: usize,
    ) -> [f32; 2] {
        if map_width == 0 || map_height == 0 {
            return self.position;
        }
        let cell_w = self.size / map_width as f32;
        let cell_h = self.size / map_height as f32;
        let screen_x = self.position[0] + cell_x as f32 * cell_w;
        let screen_y = self.position[1] + cell_y as f32 * cell_h;
        [screen_x, screen_y]
    }

    pub fn is_click_on_minimap(&self, screen_x: f32, screen_y: f32) -> bool {
        screen_x >= self.position[0]
            && screen_x <= self.position[0] + self.size
            && screen_y >= self.position[1]
            && screen_y <= self.position[1] + self.size
    }

    pub fn render_cell(cell: &MinimapCell) -> [f32; 3] {
        if !cell.explored {
            return MinimapCell::fog_color();
        }
        if cell.has_player {
            return [0.0, 1.0, 0.0];
        }
        if cell.has_enemy {
            return [1.0, 0.0, 0.0];
        }
        if cell.has_poi {
            return [1.0, 1.0, 0.0];
        }
        cell.terrain_color
    }
}

// ── MinimapSystem ──

pub struct MinimapSystem {
    pub data: MinimapData,
    pub renderer: MinimapRenderer,
    pub reveal_radius: usize,
}

impl MinimapSystem {
    pub fn new(map_width: usize, map_height: usize, scale: f32) -> Self {
        Self {
            data: MinimapData::new(map_width, map_height, scale),
            renderer: MinimapRenderer::new(),
            reveal_radius: 5,
        }
    }

    pub fn update(
        &mut self,
        player_world_x: f32,
        player_world_y: f32,
        terrain_colors: &Vec<Vec<[f32; 3]>>,
    ) {
        self.data.clear_enemies();
        self.data.update_player_position(player_world_x, player_world_y);
        self.data.reveal_radius(
            self.data.player_x,
            self.data.player_y,
            self.reveal_radius,
            terrain_colors,
        );
    }

    pub fn add_enemy_marker(&mut self, world_x: f32, world_y: f32) {
        let cell_x = (world_x / self.data.scale).round() as usize;
        let cell_y = (world_y / self.data.scale).round() as usize;
        self.data.mark_enemy(cell_x, cell_y);
    }

    pub fn add_poi_marker(&mut self, world_x: f32, world_y: f32) {
        let cell_x = (world_x / self.data.scale).round() as usize;
        let cell_y = (world_y / self.data.scale).round() as usize;
        self.data.mark_poi(cell_x, cell_y);
    }

    pub fn explored_percentage(&self) -> f32 {
        self.data.explored_percentage()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_cell_defaults() {
        let cell = MinimapCell::new();
        assert!(!cell.explored);
        assert_eq!(cell.terrain_color, [0.0, 0.0, 0.0]);
        assert!(!cell.has_poi);
        assert!(!cell.has_player);
        assert!(!cell.has_enemy);
    }

    #[test]
    fn minimap_cell_unexplored() {
        let cell = MinimapCell::unexplored();
        assert!(!cell.explored);
        assert_eq!(cell.terrain_color, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn minimap_cell_fog_color() {
        assert_eq!(MinimapCell::fog_color(), [0.1, 0.1, 0.15]);
    }

    #[test]
    fn minimap_data_creation() {
        let data = MinimapData::new(10, 8, 1.0);
        assert_eq!(data.width, 10);
        assert_eq!(data.height, 8);
        assert_eq!(data.player_x, 5);
        assert_eq!(data.player_y, 4);
        assert_eq!(data.scale, 1.0);
        for row in &data.cells {
            for cell in row {
                assert!(!cell.explored);
            }
        }
        assert_eq!(data.cell_count(), 80);
    }

    #[test]
    fn minimap_data_set_explored() {
        let mut data = MinimapData::new(10, 10, 1.0);
        data.set_explored(3, 4, terrain_plains());
        let cell = data.get(3, 4).unwrap();
        assert!(cell.explored);
        assert_eq!(cell.terrain_color, [0.3, 0.6, 0.2]);

        // Out of bounds does nothing
        data.set_explored(20, 20, terrain_desert());
        assert!(data.get(20, 20).is_none());
    }

    #[test]
    fn minimap_data_update_player() {
        let mut data = MinimapData::new(20, 20, 2.0);
        data.update_player_position(10.0, 6.0);
        assert_eq!(data.player_x, 5);
        assert_eq!(data.player_y, 3);

        // Clamp beyond bounds
        data.update_player_position(100.0, 100.0);
        assert_eq!(data.player_x, 19);
        assert_eq!(data.player_y, 19);
    }

    #[test]
    fn minimap_data_reveal_radius() {
        let mut data = MinimapData::new(20, 20, 1.0);
        let terrain = vec![vec![terrain_forest(); 20]; 20];
        data.reveal_radius(10, 10, 2, &terrain);

        // Center should be explored
        assert!(data.get(10, 10).unwrap().explored);

        // Within radius
        assert!(data.get(11, 10).unwrap().explored);
        assert!(data.get(10, 11).unwrap().explored);

        // Corner of bounding box but outside circle (dx=2, dy=2, dist > 2)
        assert!(!data.get(12, 12).unwrap().explored);

        // Far away should be unexplored
        assert!(!data.get(0, 0).unwrap().explored);
    }

    #[test]
    fn minimap_data_explored_percentage() {
        let mut data = MinimapData::new(10, 10, 1.0);
        assert_eq!(data.explored_percentage(), 0.0);

        data.set_explored(0, 0, terrain_plains());
        assert_eq!(data.explored_percentage(), 1.0);

        // 0x0 map
        let empty = MinimapData::new(0, 0, 1.0);
        assert_eq!(empty.explored_percentage(), 0.0);
    }

    #[test]
    fn minimap_data_mark_poi_and_enemy() {
        let mut data = MinimapData::new(10, 10, 1.0);
        data.mark_poi(2, 3);
        assert!(data.get(2, 3).unwrap().has_poi);

        data.mark_enemy(5, 5);
        assert!(data.get(5, 5).unwrap().has_enemy);

        data.clear_enemies();
        assert!(!data.get(5, 5).unwrap().has_enemy);
        // POI should remain
        assert!(data.get(2, 3).unwrap().has_poi);
    }

    #[test]
    fn minimap_renderer_cell_to_screen() {
        let renderer = MinimapRenderer::new().with_position(100.0, 50.0).with_size(200.0);
        let pos = renderer.cell_to_screen(0, 0, 10, 10);
        assert_eq!(pos, [100.0, 50.0]);

        let pos = renderer.cell_to_screen(5, 5, 10, 10);
        assert_eq!(pos, [200.0, 150.0]);

        let pos = renderer.cell_to_screen(9, 9, 10, 10);
        assert_eq!(pos[0], 100.0 + 9.0 * 20.0);
        assert_eq!(pos[1], 50.0 + 9.0 * 20.0);

        // Zero map
        let pos = renderer.cell_to_screen(0, 0, 0, 0);
        assert_eq!(pos, [100.0, 50.0]);
    }

    #[test]
    fn minimap_renderer_render_cell() {
        // Unexplored → fog
        let cell = MinimapCell::new();
        assert_eq!(MinimapRenderer::render_cell(&cell), MinimapCell::fog_color());

        // Explored, no markers → terrain color
        let mut cell = MinimapCell::new();
        cell.explored = true;
        cell.terrain_color = terrain_water();
        assert_eq!(MinimapRenderer::render_cell(&cell), [0.2, 0.3, 0.7]);

        // Has player (highest priority)
        cell.has_player = true;
        assert_eq!(MinimapRenderer::render_cell(&cell), [0.0, 1.0, 0.0]);

        // Has enemy
        cell.has_player = false;
        cell.has_enemy = true;
        assert_eq!(MinimapRenderer::render_cell(&cell), [1.0, 0.0, 0.0]);

        // Has POI
        cell.has_enemy = false;
        cell.has_poi = true;
        assert_eq!(MinimapRenderer::render_cell(&cell), [1.0, 1.0, 0.0]);
    }

    #[test]
    fn minimap_renderer_click_detection() {
        let renderer = MinimapRenderer::new().with_position(100.0, 50.0).with_size(200.0);

        // Inside
        assert!(renderer.is_click_on_minimap(150.0, 100.0));
        assert!(renderer.is_click_on_minimap(100.0, 50.0));
        assert!(renderer.is_click_on_minimap(300.0, 250.0));

        // Outside
        assert!(!renderer.is_click_on_minimap(50.0, 100.0));
        assert!(!renderer.is_click_on_minimap(150.0, 20.0));
        assert!(!renderer.is_click_on_minimap(350.0, 100.0));
    }

    #[test]
    fn minimap_renderer_toggle() {
        let mut renderer = MinimapRenderer::new();
        assert!(renderer.visible);
        renderer.toggle();
        assert!(!renderer.visible);
        renderer.toggle();
        assert!(renderer.visible);
    }

    #[test]
    fn minimap_system_update() {
        let mut system = MinimapSystem::new(20, 20, 1.0);
        let terrain = vec![vec![terrain_plains(); 20]; 20];

        system.update(10.0, 10.0, &terrain);

        assert_eq!(system.data.player_x, 10);
        assert_eq!(system.data.player_y, 10);

        // Cells around player should be explored
        assert!(system.data.get(10, 10).unwrap().explored);
        assert!(system.data.get(12, 10).unwrap().explored);

        // Far cells unexplored
        assert!(!system.data.get(0, 0).unwrap().explored);
    }

    #[test]
    fn minimap_system_enemy_marker() {
        let mut system = MinimapSystem::new(20, 20, 2.0);
        system.add_enemy_marker(10.0, 6.0);
        assert!(system.data.get(5, 3).unwrap().has_enemy);

        system.add_poi_marker(4.0, 8.0);
        assert!(system.data.get(2, 4).unwrap().has_poi);
    }

    #[test]
    fn minimap_system_explored_percentage() {
        let mut system = MinimapSystem::new(10, 10, 1.0);
        assert_eq!(system.explored_percentage(), 0.0);

        let terrain = vec![vec![terrain_desert(); 10]; 10];
        system.update(5.0, 5.0, &terrain);
        assert!(system.explored_percentage() > 0.0);
    }

    #[test]
    fn terrain_colors() {
        assert_eq!(terrain_plains(), [0.3, 0.6, 0.2]);
        assert_eq!(terrain_forest(), [0.1, 0.4, 0.1]);
        assert_eq!(terrain_mountain(), [0.5, 0.45, 0.4]);
        assert_eq!(terrain_desert(), [0.8, 0.7, 0.4]);
        assert_eq!(terrain_water(), [0.2, 0.3, 0.7]);
        assert_eq!(terrain_town(), [0.7, 0.6, 0.3]);

        // All distinct
        let colors = [
            terrain_plains(),
            terrain_forest(),
            terrain_mountain(),
            terrain_desert(),
            terrain_water(),
            terrain_town(),
        ];
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "terrain colors {} and {} are not distinct", i, j);
            }
        }
    }

    #[test]
    fn minimap_data_get_out_of_bounds() {
        let data = MinimapData::new(5, 5, 1.0);
        assert!(data.get(0, 0).is_some());
        assert!(data.get(4, 4).is_some());
        assert!(data.get(5, 0).is_none());
        assert!(data.get(0, 5).is_none());
    }

    #[test]
    fn minimap_renderer_builder_pattern() {
        let r = MinimapRenderer::new()
            .with_position(10.0, 20.0)
            .with_size(300.0);
        assert_eq!(r.position, [10.0, 20.0]);
        assert_eq!(r.size, 300.0);
        assert!(r.visible);
    }
}

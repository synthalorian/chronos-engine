//! Fog of war system with visibility grid and explore states.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Unexplored,
    Explored,
    Visible,
}

#[derive(Debug, Clone)]
pub struct FogGrid {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    cells: Vec<Visibility>,
}

impl FogGrid {
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        let total = (width as usize) * (height as usize);
        FogGrid {
            width,
            height,
            cell_size,
            cells: vec![Visibility::Unexplored; total],
        }
    }

    pub fn get(&self, gx: u32, gy: u32) -> Visibility {
        if gx >= self.width || gy >= self.height {
            return Visibility::Unexplored;
        }
        self.cells[(gy as usize) * (self.width as usize) + (gx as usize)]
    }

    pub fn set(&mut self, gx: u32, gy: u32, vis: Visibility) {
        if gx >= self.width || gy >= self.height {
            return;
        }
        self.cells[(gy as usize) * (self.width as usize) + (gx as usize)] = vis;
    }

    pub fn world_to_grid(&self, wx: f32, wy: f32) -> (u32, u32) {
        (
            (wx / self.cell_size).floor().max(0.0) as u32,
            (wy / self.cell_size).floor().max(0.0) as u32,
        )
    }

    pub fn reveal_circle(&mut self, cx: f32, cy: f32, radius: f32) {
        let r_cells = (radius / self.cell_size).ceil() as i32;
        let (gcx, gcy) = self.world_to_grid(cx, cy);
        let gcx = gcx as i32;
        let gcy = gcy as i32;
        let r2 = (radius * radius) / (self.cell_size * self.cell_size);

        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                if dx * dx + dy * dy > r2 as i32 {
                    continue;
                }
                let gx = (gcx + dx) as u32;
                let gy = (gcy + dy) as u32;
                if gx < self.width && gy < self.height {
                    let idx = (gy as usize) * (self.width as usize) + (gx as usize);
                    self.cells[idx] = Visibility::Visible;
                }
            }
        }
    }

    pub fn reveal_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let (gx0, gy0) = self.world_to_grid(x, y);
        let (gx1, gy1) = self.world_to_grid(x + w, y + h);
        for gy in gy0..=gy1.min(self.height - 1) {
            for gx in gx0..=gx1.min(self.width - 1) {
                let idx = (gy as usize) * (self.width as usize) + (gx as usize);
                self.cells[idx] = Visibility::Visible;
            }
        }
    }

    pub fn demote_visible_to_explored(&mut self) {
        for cell in &mut self.cells {
            if *cell == Visibility::Visible {
                *cell = Visibility::Explored;
            }
        }
    }

    pub fn is_visible(&self, wx: f32, wy: f32) -> bool {
        let (gx, gy) = self.world_to_grid(wx, wy);
        self.get(gx, gy) == Visibility::Visible
    }

    pub fn is_explored(&self, wx: f32, wy: f32) -> bool {
        let (gx, gy) = self.world_to_grid(wx, wy);
        self.get(gx, gy) != Visibility::Unexplored
    }

    pub fn visible_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|&&c| c == Visibility::Visible)
            .count()
    }

    pub fn explored_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|&&c| c != Visibility::Unexplored)
            .count()
    }

    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FogRevealer {
    pub entity: u32,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub line_of_sight: bool,
}

pub struct FogOfWar {
    pub grid: FogGrid,
    pub revealers: Vec<FogRevealer>,
    pub block_segments: Vec<WallSegment>,
}

#[derive(Debug, Clone, Copy)]
pub struct WallSegment {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl FogOfWar {
    pub fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        let gw = (world_width / cell_size).ceil() as u32;
        let gh = (world_height / cell_size).ceil() as u32;
        FogOfWar {
            grid: FogGrid::new(gw, gh, cell_size),
            revealers: Vec::new(),
            block_segments: Vec::new(),
        }
    }

    pub fn add_revealer(&mut self, revealer: FogRevealer) {
        self.revealers.push(revealer);
    }

    pub fn remove_revealer(&mut self, entity: u32) {
        self.revealers.retain(|r| r.entity != entity);
    }

    pub fn update_revealer(&mut self, entity: u32, x: f32, y: f32) {
        if let Some(r) = self.revealers.iter_mut().find(|r| r.entity == entity) {
            r.x = x;
            r.y = y;
        }
    }

    pub fn add_wall(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.block_segments.push(WallSegment { x0, y0, x1, y1 });
    }

    pub fn compute(&mut self) {
        self.grid.demote_visible_to_explored();

        let revealers: Vec<FogRevealer> = self.revealers.clone();
        let walls = self.block_segments.clone();

        for revealer in &revealers {
            if revealer.line_of_sight && !walls.is_empty() {
                self.reveal_with_los(revealer.x, revealer.y, revealer.radius, &walls);
            } else {
                self.grid
                    .reveal_circle(revealer.x, revealer.y, revealer.radius);
            }
        }
    }

    fn reveal_with_los(&mut self, cx: f32, cy: f32, radius: f32, walls: &[WallSegment]) {
        let r_cells = (radius / self.grid.cell_size).ceil() as i32;
        let (gcx, gcy) = self.grid.world_to_grid(cx, cy);
        let r_cells_f = r_cells as f32;

        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let dist2 = (dx * dx + dy * dy) as f32;
                if dist2 > r_cells_f * r_cells_f {
                    continue;
                }

                let gx = (gcx as i32 + dx) as u32;
                let gy = (gcy as i32 + dy) as u32;
                if gx >= self.grid.width || gy >= self.grid.height {
                    continue;
                }

                let target_x = gx as f32 * self.grid.cell_size + self.grid.cell_size * 0.5;
                let target_y = gy as f32 * self.grid.cell_size + self.grid.cell_size * 0.5;

                if has_line_of_sight(cx, cy, target_x, target_y, walls) {
                    let idx = (gy as usize) * (self.grid.width as usize) + (gx as usize);
                    self.grid.cells[idx] = Visibility::Visible;
                }
            }
        }
    }
}

fn has_line_of_sight(x0: f32, y0: f32, x1: f32, y1: f32, walls: &[WallSegment]) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    for seg in walls {
        if ray_segment_intersect(x0, y0, dx, dy, seg.x0, seg.y0, seg.x1, seg.y1) {
            return false;
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn ray_segment_intersect(
    rx: f32,
    ry: f32,
    rdx: f32,
    rdy: f32,
    sx0: f32,
    sy0: f32,
    sx1: f32,
    sy1: f32,
) -> bool {
    let sdx = sx1 - sx0;
    let sdy = sy1 - sy0;
    let denom = rdx * sdy - rdy * sdx;
    if denom.abs() < 1e-8 {
        return false;
    }
    let t = ((sx0 - rx) * sdy - (sy0 - ry) * sdx) / denom;
    let u = ((sx0 - rx) * rdy - (sy0 - ry) * rdx) / denom;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

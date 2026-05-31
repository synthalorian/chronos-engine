#[cfg(feature = "game")]
use super::terrain::TerrainGrid;

#[allow(clippy::type_complexity)]
type ClosedNode = (usize, usize, Option<(usize, usize)>);

#[derive(Debug, Clone)]
pub struct NavigationPath {
    pub waypoints: Vec<[f32; 3]>,
    pub current_index: usize,
}

impl NavigationPath {
    pub fn new(waypoints: Vec<[f32; 3]>) -> Self {
        NavigationPath {
            waypoints,
            current_index: 0,
        }
    }

    pub fn advance(&mut self) -> Option<[f32; 3]> {
        if self.current_index < self.waypoints.len() {
            let wp = self.waypoints[self.current_index];
            self.current_index += 1;
            Some(wp)
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&[f32; 3]> {
        self.waypoints.get(self.current_index)
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len()
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PathNode {
    x: usize,
    y: usize,
    g: f32,
    h: f32,
    f: f32,
    parent: Option<(usize, usize)>,
}

impl PathNode {
    fn new(x: usize, y: usize, g: f32, h: f32, parent: Option<(usize, usize)>) -> Self {
        PathNode {
            x,
            y,
            g,
            h,
            f: g + h,
            parent,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pathfinder {
    pub grid_width: usize,
    pub grid_height: usize,
}

impl Pathfinder {
    pub fn new(width: usize, height: usize) -> Self {
        Pathfinder {
            grid_width: width,
            grid_height: height,
        }
    }

    fn heuristic(&self, x: usize, y: usize, gx: usize, gy: usize) -> f32 {
        let dx = (x as f32) - (gx as f32);
        let dy = (y as f32) - (gy as f32);
        dx.abs() + dy.abs()
    }

    fn reconstruct_path(
        &self,
        closed: &[ClosedNode],
        end_x: usize,
        end_y: usize,
        terrain: &TerrainGrid,
        cell_size: f32,
    ) -> Vec<[f32; 3]> {
        let mut path_coords = Vec::new();
        let mut current = Some((end_x, end_y));

        while let Some((cx, cy)) = current {
            path_coords.push((cx, cy));
            if let Some(entry) = closed.iter().find(|(x, y, _)| *x == cx && *y == cy) {
                current = entry.2;
            } else {
                break;
            }
        }

        path_coords.reverse();

        let mut waypoints = Vec::with_capacity(path_coords.len());
        for (gx, gy) in path_coords {
            let (wx, wz) = TerrainGrid::grid_to_world(gx, gy, cell_size);
            let height = terrain.get_height(gx, gy);
            waypoints.push([wx, height, wz]);
        }

        waypoints
    }

    pub fn find_path(
        &self,
        terrain: &TerrainGrid,
        start: (usize, usize),
        end: (usize, usize),
        cell_size: f32,
    ) -> Option<NavigationPath> {
        let (sx, sy) = start;
        let (ex, ey) = end;

        // Out of bounds check
        if sx >= self.grid_width
            || sy >= self.grid_height
            || ex >= self.grid_width
            || ey >= self.grid_height
        {
            return None;
        }

        // Start or end not walkable
        if !terrain.is_walkable(sx, sy) || !terrain.is_walkable(ex, ey) {
            return None;
        }

        // Start == end: single-point path
        if sx == ex && sy == ey {
            let (wx, wz) = TerrainGrid::grid_to_world(sx, sy, cell_size);
            let h = terrain.get_height(sx, sy);
            return Some(NavigationPath::new(vec![[wx, h, wz]]));
        }

        let mut open: Vec<PathNode> = Vec::new();
        let mut closed: Vec<ClosedNode> = Vec::new();

        let start_h = self.heuristic(sx, sy, ex, ey);
        open.push(PathNode::new(sx, sy, 0.0, start_h, None));

        let neighbors: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

        while !open.is_empty() {
            // Find node with lowest f score
            let mut best_idx = 0;
            for i in 1..open.len() {
                if open[i].f < open[best_idx].f {
                    best_idx = i;
                }
            }
            let current = open.remove(best_idx);

            // Goal reached
            if current.x == ex && current.y == ey {
                closed.push((current.x, current.y, current.parent));
                let waypoints = self.reconstruct_path(&closed, ex, ey, terrain, cell_size);
                return Some(NavigationPath::new(waypoints));
            }

            closed.push((current.x, current.y, current.parent));

            for (dx, dy) in &neighbors {
                let nx = (current.x as i32 + dx) as usize;
                let ny = (current.y as i32 + dy) as usize;

                // Bounds check (handle underflow from negative i32 cast)
                if (dx < &0 && current.x == 0) || (dy < &0 && current.y == 0) {
                    continue;
                }
                if nx >= self.grid_width || ny >= self.grid_height {
                    continue;
                }

                if !terrain.is_walkable(nx, ny) {
                    continue;
                }

                // Skip if already in closed list
                if closed.iter().any(|(cx, cy, _)| *cx == nx && *cy == ny) {
                    continue;
                }

                let move_cost = if matches!(
                    terrain.get_tile(nx, ny),
                    Some(super::terrain::TerrainTile::Hill)
                ) {
                    2.0
                } else {
                    1.0
                };

                let tentative_g = current.g + move_cost;

                // Check if already in open with lower g
                let existing = open.iter().find(|n| n.x == nx && n.y == ny);
                if let Some(existing) = existing {
                    if tentative_g >= existing.g {
                        continue;
                    }
                }

                // Remove existing entry if we found a better path
                open.retain(|n| !(n.x == nx && n.y == ny));

                let nh = self.heuristic(nx, ny, ex, ey);
                open.push(PathNode::new(
                    nx,
                    ny,
                    tentative_g,
                    nh,
                    Some((current.x, current.y)),
                ));
            }
        }

        // No path found
        None
    }

    pub fn find_path_world(
        &self,
        terrain: &TerrainGrid,
        start_world: [f32; 3],
        end_world: [f32; 3],
        cell_size: f32,
    ) -> Option<NavigationPath> {
        let start = TerrainGrid::world_to_grid(start_world[0], start_world[2], cell_size);
        let end = TerrainGrid::world_to_grid(end_world[0], end_world[2], cell_size);
        self.find_path(terrain, start, end, cell_size)
    }
}

#[cfg(test)]
mod tests {
    use super::super::terrain::TerrainTile;
    use super::*;

    fn flat_terrain(w: usize, h: usize) -> TerrainGrid {
        TerrainGrid::new(w, h)
    }

    #[test]
    fn path_on_empty_terrain() {
        let terrain = flat_terrain(10, 10);
        let pf = Pathfinder::new(10, 10);
        let path = pf.find_path(&terrain, (0, 0), (5, 5), 1.0);
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.waypoints.len(), 11); // Manhattan distance + 1
    }

    #[test]
    fn path_blocked_by_wall() {
        let mut terrain = flat_terrain(10, 10);
        // Wall across the middle, with one gap
        for x in 0..10 {
            terrain.set_tile(x, 5, TerrainTile::Wall);
        }
        terrain.set_tile(5, 5, TerrainTile::Flat); // gap

        let pf = Pathfinder::new(10, 10);
        let path = pf.find_path(&terrain, (0, 0), (0, 9), 1.0);
        assert!(path.is_some());
        let p = path.unwrap();
        // Path must go through the gap at (5, 5)
        assert!(p.waypoints.iter().any(|&[x, _, z]| {
            let gx = (x / 1.0) as usize;
            let gz = (z / 1.0) as usize;
            gx == 5 && gz == 5
        }));
    }

    #[test]
    fn no_path_when_fully_blocked() {
        let mut terrain = flat_terrain(5, 5);
        // Full wall across row 2
        for x in 0..5 {
            terrain.set_tile(x, 2, TerrainTile::Wall);
        }
        let pf = Pathfinder::new(5, 5);
        let path = pf.find_path(&terrain, (0, 0), (0, 4), 1.0);
        assert!(path.is_none());
    }

    #[test]
    fn path_correct_start_and_end() {
        let terrain = flat_terrain(10, 10);
        let pf = Pathfinder::new(10, 10);
        let path = pf.find_path(&terrain, (2, 3), (7, 8), 1.0).unwrap();

        let start = path.waypoints.first().unwrap();
        let (expected_wx, expected_wz) = TerrainGrid::grid_to_world(2, 3, 1.0);
        assert!((start[0] - expected_wx).abs() < 0.01);
        assert!((start[2] - expected_wz).abs() < 0.01);

        let end = path.waypoints.last().unwrap();
        let (expected_wx, expected_wz) = TerrainGrid::grid_to_world(7, 8, 1.0);
        assert!((end[0] - expected_wx).abs() < 0.01);
        assert!((end[2] - expected_wz).abs() < 0.01);
    }

    #[test]
    fn navigation_path_traversal() {
        let waypoints = vec![[1.0, 0.0, 1.0], [2.0, 0.0, 2.0], [3.0, 0.0, 3.0]];
        let mut path = NavigationPath::new(waypoints);

        assert!(!path.is_complete());
        assert_eq!(*path.peek().unwrap(), [1.0, 0.0, 1.0]);

        assert_eq!(path.advance(), Some([1.0, 0.0, 1.0]));
        assert_eq!(path.advance(), Some([2.0, 0.0, 2.0]));
        assert!(!path.is_complete());

        assert_eq!(path.advance(), Some([3.0, 0.0, 3.0]));
        assert!(path.is_complete());
        assert!(path.peek().is_none());
        assert!(path.advance().is_none());

        path.reset();
        assert!(!path.is_complete());
        assert_eq!(path.advance(), Some([1.0, 0.0, 1.0]));
    }

    #[test]
    fn start_equals_end() {
        let terrain = flat_terrain(5, 5);
        let pf = Pathfinder::new(5, 5);
        let path = pf.find_path(&terrain, (2, 2), (2, 2), 1.0);
        assert!(path.is_some());
        assert_eq!(path.unwrap().waypoints.len(), 1);
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let terrain = flat_terrain(5, 5);
        let pf = Pathfinder::new(5, 5);
        assert!(pf.find_path(&terrain, (10, 10), (0, 0), 1.0).is_none());
        assert!(pf.find_path(&terrain, (0, 0), (10, 10), 1.0).is_none());
    }
}

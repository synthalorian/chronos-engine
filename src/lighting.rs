//! 2D lighting with point, directional, spot, and area lights plus shadow casting.

#[derive(Debug, Clone, Copy)]
pub enum LightType {
    Point { radius: f32, intensity: f32, color: [f32; 4] },
    Directional { direction: [f32; 2], intensity: f32, color: [f32; 4] },
    Spot { direction: [f32; 2], angle: f32, radius: f32, intensity: f32, color: [f32; 4] },
    Area { width: f32, height: f32, intensity: f32, color: [f32; 4] },
}

#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub entity: u32,
    pub position: [f32; 2],
    pub light_type: LightType,
    pub enabled: bool,
    pub layer_mask: u32,
}

impl Light {
    pub fn point(entity: u32, x: f32, y: f32, radius: f32, intensity: f32) -> Self {
        Light {
            entity,
            position: [x, y],
            light_type: LightType::Point { radius, intensity, color: [1.0, 1.0, 1.0, 1.0] },
            enabled: true,
            layer_mask: !0,
        }
    }

    pub fn directional(entity: u32, dx: f32, dy: f32, intensity: f32) -> Self {
        let len = (dx * dx + dy * dy).sqrt().max(1e-8);
        Light {
            entity,
            position: [0.0, 0.0],
            light_type: LightType::Directional {
                direction: [dx / len, dy / len],
                intensity,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            enabled: true,
            layer_mask: !0,
        }
    }

    pub fn spot(entity: u32, x: f32, y: f32, dx: f32, dy: f32, angle: f32, radius: f32) -> Self {
        let len = (dx * dx + dy * dy).sqrt().max(1e-8);
        Light {
            entity,
            position: [x, y],
            light_type: LightType::Spot {
                direction: [dx / len, dy / len],
                angle,
                radius,
                intensity: 1.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            enabled: true,
            layer_mask: !0,
        }
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        match &mut self.light_type {
            LightType::Point { color, .. } => *color = [r, g, b, a],
            LightType::Directional { color, .. } => *color = [r, g, b, a],
            LightType::Spot { color, .. } => *color = [r, g, b, a],
            LightType::Area { color, .. } => *color = [r, g, b, a],
        }
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LineSegment {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

impl LineSegment {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        LineSegment { start: [x0, y0], end: [x1, y1] }
    }
}

#[derive(Debug, Clone)]
pub struct ShadowCaster {
    pub segments: Vec<LineSegment>,
}

impl ShadowCaster {
    pub fn new() -> Self {
        ShadowCaster { segments: Vec::new() }
    }

    pub fn add_segment(&mut self, seg: LineSegment) {
        self.segments.push(seg);
    }

    pub fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let tl = [x, y];
        let tr = [x + w, y];
        let br = [x + w, y + h];
        let bl = [x, y + h];
        self.segments.push(LineSegment { start: tl, end: tr });
        self.segments.push(LineSegment { start: tr, end: br });
        self.segments.push(LineSegment { start: br, end: bl });
        self.segments.push(LineSegment { start: bl, end: tl });
    }
}

pub struct LightMap {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    pub values: Vec<f32>,
}

impl LightMap {
    pub fn new(width: u32, height: u32, cell_size: f32) -> Self {
        let total = (width as usize) * (height as usize);
        LightMap {
            width,
            height,
            cell_size,
            values: vec![0.0; total],
        }
    }

    pub fn clear(&mut self) {
        for v in &mut self.values {
            *v = 0.0;
        }
    }

    pub fn get_intensity(&self, x: f32, y: f32) -> f32 {
        let gx = (x / self.cell_size) as usize;
        let gy = (y / self.cell_size) as usize;
        if gx < self.width as usize && gy < self.height as usize {
            self.values[gy * (self.width as usize) + gx]
        } else {
            0.0
        }
    }

    fn add_intensity(&mut self, gx: usize, gy: usize, intensity: f32) {
        if gx < self.width as usize && gy < self.height as usize {
            let idx = gy * (self.width as usize) + gx;
            self.values[idx] = (self.values[idx] + intensity).min(1.0);
        }
    }
}

pub struct VisibilityPolygon;

impl VisibilityPolygon {
    pub fn compute(origin: [f32; 2], segments: &[LineSegment], max_radius: f32) -> Vec<[f32; 2]> {
        let mut angles: Vec<f32> = Vec::new();

        for seg in segments {
            for point in &[seg.start, seg.end] {
                let dx = point[0] - origin[0];
                let dy = point[1] - origin[1];
                let angle = dy.atan2(dx);
                angles.push(angle - 0.0001);
                angles.push(angle);
                angles.push(angle + 0.0001);
            }
        }

        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut polygon = Vec::new();

        for &angle in &angles {
            let dx = angle.cos();
            let dy = angle.sin();

            let mut closest_t = max_radius;
            let mut hit = false;

            for seg in segments {
                let rdx = dx;
                let rdy = dy;
                let sdx = seg.end[0] - seg.start[0];
                let sdy = seg.end[1] - seg.start[1];

                let denom = rdx * sdy - rdy * sdx;
                if denom.abs() < 1e-8 {
                    continue;
                }

                let t = ((seg.start[0] - origin[0]) * sdy - (seg.start[1] - origin[1]) * sdx) / denom;
                let u = ((seg.start[0] - origin[0]) * rdy - (seg.start[1] - origin[1]) * rdx) / denom;

                if t > 0.0 && u >= 0.0 && u <= 1.0 && t < closest_t {
                    closest_t = t;
                    hit = true;
                }
            }

            if closest_t < max_radius || hit {
                polygon.push([
                    origin[0] + dx * closest_t,
                    origin[1] + dy * closest_t,
                ]);
            } else {
                polygon.push([
                    origin[0] + dx * max_radius,
                    origin[1] + dy * max_radius,
                ]);
            }
        }

        polygon
    }
}

pub struct LightingSystem {
    pub lights: Vec<Light>,
    pub shadow_casters: Vec<ShadowCaster>,
    pub ambient_color: [f32; 4],
    pub ambient_intensity: f32,
}

impl LightingSystem {
    pub fn new() -> Self {
        LightingSystem {
            lights: Vec::new(),
            shadow_casters: Vec::new(),
            ambient_color: [1.0, 1.0, 1.0, 1.0],
            ambient_intensity: 0.15,
        }
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    pub fn remove_light(&mut self, entity: u32) {
        self.lights.retain(|l| l.entity != entity);
    }

    pub fn add_shadow_caster(&mut self, caster: ShadowCaster) {
        self.shadow_casters.push(caster);
    }

    pub fn compute_lighting(&self, map: &mut LightMap) {
        map.clear();

        for cell_idx in 0..map.values.len() {
            map.values[cell_idx] = self.ambient_intensity;
        }

        let all_segments: Vec<LineSegment> = self.shadow_casters.iter()
            .flat_map(|sc| sc.segments.iter().copied())
            .collect();

        for light in &self.lights {
            if !light.enabled {
                continue;
            }
            match &light.light_type {
                LightType::Point { radius, intensity, color } => {
                    self.add_point_light(map, light.position, *radius, *intensity, *color, &all_segments);
                }
                LightType::Directional { direction, intensity, color } => {
                    self.add_directional_light(map, *direction, *intensity, *color);
                }
                LightType::Spot { direction, angle, radius, intensity, color } => {
                    self.add_spot_light(map, light.position, *direction, *angle, *radius, *intensity, *color, &all_segments);
                }
                LightType::Area { width, height, intensity, color } => {
                    self.add_area_light(map, light.position, *width, *height, *intensity, *color);
                }
            }
        }
    }

    fn add_point_light(
        &self, map: &mut LightMap,
        pos: [f32; 2], radius: f32, intensity: f32, color: [f32; 4],
        segments: &[LineSegment],
    ) {
        let r_cells = (radius / map.cell_size).ceil() as i32;
        let (gcx, gcy) = (
            (pos[0] / map.cell_size) as i32,
            (pos[1] / map.cell_size) as i32,
        );

        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let gx = (gcx + dx) as usize;
                let gy = (gcy + dy) as usize;

                let wx = gx as f32 * map.cell_size + map.cell_size * 0.5;
                let wy = gy as f32 * map.cell_size + map.cell_size * 0.5;
                let dist = ((wx - pos[0]).powi(2) + (wy - pos[1]).powi(2)).sqrt();

                if dist > radius {
                    continue;
                }

                let has_los = !segments.is_empty() && self.check_los(pos, [wx, wy], segments);
                if segments.is_empty() || has_los {
                    let atten = 1.0 - (dist / radius);
                    let light_val = intensity * atten * color[0];
                    map.add_intensity(gx, gy, light_val);
                }
            }
        }
    }

    fn add_directional_light(
        &self, map: &mut LightMap,
        _direction: [f32; 2], intensity: f32, color: [f32; 4],
    ) {
        for gy in 0..(map.height as usize) {
            for gx in 0..(map.width as usize) {
                map.add_intensity(gx, gy, intensity * 0.3 * color[0]);
            }
        }
    }

    fn add_spot_light(
        &self, map: &mut LightMap,
        pos: [f32; 2], dir: [f32; 2], angle: f32, radius: f32,
        intensity: f32, color: [f32; 4], segments: &[LineSegment],
    ) {
        let r_cells = (radius / map.cell_size).ceil() as i32;
        let (gcx, gcy) = (
            (pos[0] / map.cell_size) as i32,
            (pos[1] / map.cell_size) as i32,
        );
        let cos_half = (angle * 0.5).cos();

        for dy in -r_cells..=r_cells {
            for dx in -r_cells..=r_cells {
                let gx = (gcx + dx) as usize;
                let gy = (gcy + dy) as usize;

                let wx = gx as f32 * map.cell_size + map.cell_size * 0.5;
                let wy = gy as f32 * map.cell_size + map.cell_size * 0.5;
                let diff_x = wx - pos[0];
                let diff_y = wy - pos[1];
                let dist = (diff_x * diff_x + diff_y * diff_y).sqrt();

                if dist > radius || dist < 1e-8 {
                    continue;
                }

                let dot = (diff_x * dir[0] + diff_y * dir[1]) / dist;
                if dot < cos_half {
                    continue;
                }

                let has_los = self.check_los(pos, [wx, wy], segments);
                if has_los {
                    let atten = (1.0 - dist / radius) * ((dot - cos_half) / (1.0 - cos_half)).max(0.0);
                    map.add_intensity(gx, gy, intensity * atten * color[0]);
                }
            }
        }
    }

    fn add_area_light(
        &self, map: &mut LightMap,
        pos: [f32; 2], width: f32, height: f32, intensity: f32, color: [f32; 4],
    ) {
        let gx0 = ((pos[0] - width * 0.5) / map.cell_size).floor().max(0.0) as usize;
        let gy0 = ((pos[1] - height * 0.5) / map.cell_size).floor().max(0.0) as usize;
        let gx1 = ((pos[0] + width * 0.5) / map.cell_size).ceil() as usize;
        let gy1 = ((pos[1] + height * 0.5) / map.cell_size).ceil() as usize;

        for gy in gy0..=gy1.min(map.height as usize - 1) {
            for gx in gx0..=gx1.min(map.width as usize - 1) {
                map.add_intensity(gx, gy, intensity * color[0]);
            }
        }
    }

    fn check_los(&self, from: [f32; 2], to: [f32; 2], segments: &[LineSegment]) -> bool {
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        for seg in segments {
            let sdx = seg.end[0] - seg.start[0];
            let sdy = seg.end[1] - seg.start[1];
            let denom = dx * sdy - dy * sdx;
            if denom.abs() < 1e-8 {
                continue;
            }
            let t = ((seg.start[0] - from[0]) * sdy - (seg.start[1] - from[1]) * sdx) / denom;
            let u = ((seg.start[0] - from[0]) * dy - (seg.start[1] - from[1]) * dx) / denom;
            if t > 0.001 && t < 0.999 && u > 0.0 && u < 1.0 {
                return false;
            }
        }
        true
    }
}

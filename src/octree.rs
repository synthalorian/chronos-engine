/// Octree — 3D spatial index for efficient collision detection and point queries.

// ──────────────────────────────────────────────
// Bounding Boxes
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB3D {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

impl AABB3D {
    pub fn new(min_x: f32, min_y: f32, min_z: f32, max_x: f32, max_y: f32, max_z: f32) -> Self {
        AABB3D { min_x, min_y, min_z, max_x, max_y, max_z }
    }

    pub fn center(&self) -> (f32, f32, f32) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
            (self.min_z + self.max_z) / 2.0,
        )
    }

    pub fn contains_point(&self, px: f32, py: f32, pz: f32) -> bool {
        px >= self.min_x && px <= self.max_x && py >= self.min_y && py <= self.max_y && pz >= self.min_z && pz <= self.max_z
    }

    pub fn overlaps_sphere(&self, cx: f32, cy: f32, cz: f32, cr: f32) -> bool {
        let closest_x = cx.clamp(self.min_x, self.max_x);
        let closest_y = cy.clamp(self.min_y, self.max_y);
        let closest_z = cz.clamp(self.min_z, self.max_z);
        let dx = cx - closest_x;
        let dy = cy - closest_y;
        let dz = cz - closest_z;
        dx * dx + dy * dy + dz * dz <= cr * cr
    }

    pub fn overlaps_aabb(&self, other: &AABB3D) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
            && self.min_z < other.max_z
            && self.max_z > other.min_z
    }
}

// ──────────────────────────────────────────────
// Octree Object
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct OctreeObject {
    pub entity: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
}

// ──────────────────────────────────────────────
// Octree
// ──────────────────────────────────────────────

#[derive(Debug)]
pub struct Octree {
    bounds: AABB3D,
    capacity: usize,
    max_depth: usize,
    depth: usize,
    objects: Vec<OctreeObject>,
    children: Vec<Option<Octree>>,
}

impl Octree {
    pub fn new(bounds: AABB3D, capacity: usize, max_depth: usize) -> Self {
        Octree {
            bounds,
            capacity,
            max_depth,
            depth: 0,
            objects: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn insert(&mut self, obj: OctreeObject) {
        if !self.children.is_empty() {
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_sphere(obj.x, obj.y, obj.z, obj.radius) {
                        c.insert(obj);
                        return;
                    }
                }
            }
        }

        if self.objects.len() >= self.capacity && self.depth < self.max_depth {
            self.subdivide();
        }

        if !self.children.is_empty() {
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_sphere(obj.x, obj.y, obj.z, obj.radius) {
                        c.insert(obj);
                        return;
                    }
                }
            }
        }

        self.objects.push(obj);
    }

    fn subdivide(&mut self) {
        let half_w = (self.bounds.max_x - self.bounds.min_x) / 2.0;
        let half_h = (self.bounds.max_y - self.bounds.min_y) / 2.0;
        let half_d = (self.bounds.max_z - self.bounds.min_z) / 2.0;
        let cx = self.bounds.min_x + half_w;
        let cy = self.bounds.min_y + half_h;
        let cz = self.bounds.min_z + half_d;

        let octants = [
            AABB3D::new(self.bounds.min_x, self.bounds.min_y, self.bounds.min_z, cx, cy, cz),
            AABB3D::new(cx, self.bounds.min_y, self.bounds.min_z, self.bounds.max_x, cy, cz),
            AABB3D::new(self.bounds.min_x, cy, self.bounds.min_z, cx, self.bounds.max_y, cz),
            AABB3D::new(cx, cy, self.bounds.min_z, self.bounds.max_x, self.bounds.max_y, cz),
            AABB3D::new(self.bounds.min_x, self.bounds.min_y, cz, cx, cy, self.bounds.max_z),
            AABB3D::new(cx, self.bounds.min_y, cz, self.bounds.max_x, cy, self.bounds.max_z),
            AABB3D::new(self.bounds.min_x, cy, cz, cx, self.bounds.max_y, self.bounds.max_z),
            AABB3D::new(cx, cy, cz, self.bounds.max_x, self.bounds.max_y, self.bounds.max_z),
        ];

        for octant in octants {
            let child = Octree {
                bounds: octant,
                capacity: self.capacity,
                max_depth: self.max_depth,
                depth: self.depth + 1,
                objects: Vec::new(),
                children: Vec::new(),
            };
            self.children.push(Some(child));
        }

        let objects = std::mem::take(&mut self.objects);
        for obj in objects {
            let mut inserted = false;
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_sphere(obj.x, obj.y, obj.z, obj.radius) {
                        c.insert(obj);
                        inserted = true;
                        break;
                    }
                }
            }
            if !inserted {
                self.objects.push(obj);
            }
        }
    }

    pub fn query_sphere(&self, x: f32, y: f32, z: f32, radius: f32) -> Vec<&OctreeObject> {
        let mut results = Vec::new();
        if !self.bounds.overlaps_sphere(x, y, z, radius) {
            return results;
        }

        for obj in &self.objects {
            let dx = obj.x - x;
            let dy = obj.y - y;
            let dz = obj.z - z;
            if dx * dx + dy * dy + dz * dz <= radius * radius {
                results.push(obj);
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                results.extend(c.query_sphere(x, y, z, radius));
            }
        }

        results
    }

    pub fn query_aabb(&self, aabb: &AABB3D) -> Vec<&OctreeObject> {
        let mut results = Vec::new();
        if !self.bounds.overlaps_aabb(aabb) {
            return results;
        }

        for obj in &self.objects {
            let obj_aabb = AABB3D::new(
                obj.x - obj.radius,
                obj.y - obj.radius,
                obj.z - obj.radius,
                obj.x + obj.radius,
                obj.y + obj.radius,
                obj.z + obj.radius,
            );
            if self.bounds.overlaps_aabb(&obj_aabb) {
                results.push(obj);
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                results.extend(c.query_aabb(aabb));
            }
        }

        results
    }

    pub fn query_ray(&self, ray: &Ray3D) -> Vec<RayHit3D> {
        let mut hits = Vec::new();

        for obj in &self.objects {
            if let Some(hit) = self.raycast_sphere(ray, obj.x, obj.y, obj.z, obj.radius) {
                hits.push(RayHit3D {
                    entity: obj.entity,
                    distance: hit.distance,
                    hit_x: hit.hit_x,
                    hit_y: hit.hit_y,
                    hit_z: hit.hit_z,
                });
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                hits.extend(c.query_ray(ray));
            }
        }

        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        hits
    }

    fn raycast_sphere(&self, ray: &Ray3D, cx: f32, cy: f32, cz: f32, cr: f32) -> Option<RayHit3D> {
        let ox = ray.origin_x - cx;
        let oy = ray.origin_y - cy;
        let oz = ray.origin_z - cz;
        let a = ray.dir_x * ray.dir_x + ray.dir_y * ray.dir_y + ray.dir_z * ray.dir_z;
        let b = 2.0 * (ox * ray.dir_x + oy * ray.dir_y + oz * ray.dir_z);
        let c = ox * ox + oy * oy + oz * oz - cr * cr;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        let t = if t1 > 0.0 { t1 } else if t2 > 0.0 { t2 } else { return None };

        Some(RayHit3D {
            entity: 0,
            distance: t,
            hit_x: ray.origin_x + ray.dir_x * t,
            hit_y: ray.origin_y + ray.dir_y * t,
            hit_z: ray.origin_z + ray.dir_z * t,
        })
    }

    pub fn query_collisions(&self) -> Vec<(u32, u32)> {
        let all_objects = self.collect_all_objects();
        let mut seen = std::collections::HashSet::new();
        let mut pairs = Vec::new();

        for obj in &all_objects {
            let search_radius = obj.radius * 2.0;
            let nearby = self.query_sphere(obj.x, obj.y, obj.z, search_radius);
            for other in nearby {
                if other.entity == obj.entity {
                    continue;
                }
                let (lo, hi) = if obj.entity < other.entity {
                    (obj.entity, other.entity)
                } else {
                    (other.entity, obj.entity)
                };
                if seen.contains(&(lo, hi)) {
                    continue;
                }
                let dx = obj.x - other.x;
                let dy = obj.y - other.y;
                let dz = obj.z - other.z;
                let combined_radius = obj.radius + other.radius;
                if dx * dx + dy * dy + dz * dz <= combined_radius * combined_radius {
                    seen.insert((lo, hi));
                    pairs.push((lo, hi));
                }
            }
        }

        pairs
    }

    fn collect_all_objects(&self) -> Vec<OctreeObject> {
        let mut all = self.objects.clone();
        for child in &self.children {
            if let Some(ref c) = child {
                all.extend(c.collect_all_objects());
            }
        }
        all
    }

    pub fn rebuild(&mut self) {
        self.objects.clear();
        self.children.clear();
    }

    pub fn extend(&mut self, objects: impl IntoIterator<Item = OctreeObject>) {
        for obj in objects {
            self.insert(obj);
        }
    }
}

// ──────────────────────────────────────────────
// Ray
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Ray3D {
    pub origin_x: f32,
    pub origin_y: f32,
    pub origin_z: f32,
    pub dir_x: f32,
    pub dir_y: f32,
    pub dir_z: f32,
}

impl Ray3D {
    pub fn from_to(from_x: f32, from_y: f32, from_z: f32, to_x: f32, to_y: f32, to_z: f32) -> Self {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let dz = to_z - from_z;
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        Ray3D {
            origin_x: from_x,
            origin_y: from_y,
            origin_z: from_z,
            dir_x: dx / len,
            dir_y: dy / len,
            dir_z: dz / len,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RayHit3D {
    pub entity: u32,
    pub distance: f32,
    pub hit_x: f32,
    pub hit_y: f32,
    pub hit_z: f32,
}

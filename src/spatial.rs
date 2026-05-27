/// Quadtree — 2D spatial index for efficient collision detection and point queries.

// ──────────────────────────────────────────────
// Bounding Boxes
// ──────────────────────────────────────────────

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    /// Minimum X (left edge).
    pub x: f32,
    /// Minimum Y (top edge).
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl AABB {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        AABB { x, y, w, h }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    /// Check if this AABB overlaps a circle (bounding box of circle vs AABB).
    pub fn overlaps_circle(&self, cx: f32, cy: f32, cr: f32) -> bool {
        let closest_x = cx.clamp(self.x, self.x + self.w);
        let closest_y = cy.clamp(self.y, self.y + self.h);
        let dx = cx - closest_x;
        let dy = cy - closest_y;
        dx * dx + dy * dy <= cr * cr
    }

    /// Check if two AABBs overlap.
    pub fn overlaps_aabb(&self, other: &AABB) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Expand this AABB to include a point.
    pub fn expand_to_fit(&mut self, x: f32, y: f32) {
        let min_x = self.x.min(x);
        let min_y = self.y.min(y);
        let max_x = (self.x + self.w).max(x);
        let max_y = (self.y + self.h).max(y);
        self.x = min_x;
        self.y = min_y;
        self.w = max_x - min_x;
        self.h = max_y - min_y;
    }
}

// ──────────────────────────────────────────────
// Quadtree Object
// ──────────────────────────────────────────────

/// An object stored in the quadtree.
#[derive(Debug, Clone, Copy)]
pub struct QuadtreeObject {
    /// Entity index.
    pub entity: u32,
    /// Position X.
    pub x: f32,
    /// Position Y.
    pub y: f32,
    /// Collision radius (for circle narrow-phase).
    pub radius: f32,
}

// ──────────────────────────────────────────────
// Quadtree
// ──────────────────────────────────────────────

/// A 2D spatial index using recursive AABB subdivision.
///
/// Each node can hold up to `capacity` objects. When a node is full, it splits
/// into four children and redistributes its objects. This gives O(log n) insertion
/// and O(k + log n) query where k is the number of overlapping objects.
///
/// Usage:
/// ```ignore
/// let mut qt = Quadtree::new(AABB::new(0.0, 0.0, 200.0, 200.0), 4, 4);
/// qt.insert(QuadtreeObject { entity: 1, x: 10.0, y: 20.0, radius: 5.0 });
/// let nearby = qt.query_circle(10.0, 20.0, 10.0);
/// ```
#[derive(Debug)]
pub struct Quadtree {
    bounds: AABB,
    capacity: usize,
    max_depth: usize,
    depth: usize,
    objects: Vec<QuadtreeObject>,
    children: Vec<Option<Quadtree>>,
}

impl Quadtree {
    /// Create a new quadtree with the given world bounds.
    pub fn new(bounds: AABB, capacity: usize, max_depth: usize) -> Self {
        Quadtree {
            bounds,
            capacity,
            max_depth,
            depth: 0,
            objects: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Insert an object into the quadtree.
    pub fn insert(&mut self, obj: QuadtreeObject) {
        // If this node has children, try to insert into them
        if !self.children.is_empty() {
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_circle(obj.x, obj.y, obj.radius) {
                        c.insert(obj);
                        return;
                    }
                }
            }
        }

        // If this node is full and can split, split it
        if self.objects.len() >= self.capacity && self.depth < self.max_depth {
            self.subdivide();
        }

        // Try to insert into children again (after possible split)
        if !self.children.is_empty() {
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_circle(obj.x, obj.y, obj.radius) {
                        c.insert(obj);
                        return;
                    }
                }
            }
        }

        // Otherwise, add to this node's objects
        self.objects.push(obj);
    }

    /// Subdivide this node into four children and redistribute objects.
    fn subdivide(&mut self) {
        let half_w = self.bounds.w / 2.0;
        let half_h = self.bounds.h / 2.0;
        let center_x = self.bounds.x + half_w;
        let center_y = self.bounds.y + half_h;

        let quadrants = [
            AABB::new(self.bounds.x, self.bounds.y, half_w, half_h),
            AABB::new(center_x, self.bounds.y, half_w, half_h),
            AABB::new(self.bounds.x, center_y, half_w, half_h),
            AABB::new(center_x, center_y, half_w, half_h),
        ];

        for q in quadrants {
            let child = Quadtree {
                bounds: q,
                capacity: self.capacity,
                max_depth: self.max_depth,
                depth: self.depth + 1,
                objects: Vec::new(),
                children: Vec::new(),
            };
            self.children.push(Some(child));
        }

        // Redistribute existing objects
        let objects = std::mem::take(&mut self.objects);
        for obj in objects {
            let mut inserted = false;
            for child in &mut self.children {
                if let Some(ref mut c) = child {
                    if c.bounds.overlaps_circle(obj.x, obj.y, obj.radius) {
                        c.insert(obj);
                        inserted = true;
                        break;
                    }
                }
            }
            // If no child accepted it, put it back
            if !inserted {
                self.objects.push(obj);
            }
        }
    }

    /// Query all objects whose bounds overlap the given circle.
    pub fn query_circle(&self, x: f32, y: f32, radius: f32) -> Vec<&QuadtreeObject> {
        let mut results = Vec::new();
        if !self.bounds.overlaps_circle(x, y, radius) {
            return results;
        }

        for obj in &self.objects {
            if (obj.x - x).powi(2) + (obj.y - y).powi(2) <= radius * radius {
                results.push(obj);
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                results.extend(c.query_circle(x, y, radius));
            }
        }

        results
    }

    /// Query all objects within the given AABB.
    pub fn query_aabb(&self, aabb: &AABB) -> Vec<&QuadtreeObject> {
        let mut results = Vec::new();
        if !self.bounds.overlaps_aabb(aabb) {
            return results;
        }

        for obj in &self.objects {
            let obj_aabb = AABB::new(
                obj.x - obj.radius,
                obj.y - obj.radius,
                obj.radius * 2.0,
                obj.radius * 2.0,
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

    /// Query all objects at a specific point (for raycasting / mouse-picking).
    pub fn query_point(&self, x: f32, y: f32) -> Vec<&QuadtreeObject> {
        let mut results = Vec::new();
        if !self.bounds.contains_point(x, y) {
            return results;
        }

        for obj in &self.objects {
            let dx = obj.x - x;
            let dy = obj.y - y;
            if dx * dx + dy * dy <= obj.radius * obj.radius {
                results.push(obj);
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                results.extend(c.query_point(x, y));
            }
        }

        results
    }

    /// Find all unique collision pairs in the entire quadtree.
    ///
    /// Collects every object from all nodes, then for each object queries
    /// the root with a search radius large enough to find all potential
    /// collision partners. Deduplicates by ensuring entity_a < entity_b.
    ///
    /// MUST be called on the root node. Calling on a child node will miss
    /// cross-subtree pairs.
    pub fn query_collisions(&self) -> Vec<(u32, u32)> {
        let all_objects = self.collect_all_objects();
        let mut seen = std::collections::HashSet::new();
        let mut pairs = Vec::new();

        for obj in &all_objects {
            let search_radius = obj.radius * 2.0;
            let nearby = self.query_circle(obj.x, obj.y, search_radius);
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
                let combined_radius = obj.radius + other.radius;
                if dx * dx + dy * dy <= combined_radius * combined_radius {
                    seen.insert((lo, hi));
                    pairs.push((lo, hi));
                }
            }
        }

        pairs
    }

    fn collect_all_objects(&self) -> Vec<QuadtreeObject> {
        let mut all = self.objects.clone();
        for child in &self.children {
            if let Some(ref c) = child {
                all.extend(c.collect_all_objects());
            }
        }
        all
    }

    /// Rebuild the quadtree from a list of position data.
    /// This is the primary API for game code: build once per tick.
    pub fn rebuild(&mut self) {
        self.objects.clear();
        self.children.clear();
    }

    /// Add pre-built objects to the quadtree.
    pub fn extend(&mut self, objects: impl IntoIterator<Item = QuadtreeObject>) {
        for obj in objects {
            self.insert(obj);
        }
    }
}

// ──────────────────────────────────────────────
// Raycast
// ──────────────────────────────────────────────

/// A ray for line-segment queries through the quadtree.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Origin X.
    pub x: f32,
    /// Origin Y.
    pub y: f32,
    /// Direction X (normalized).
    pub dx: f32,
    /// Direction Y (normalized).
    pub dy: f32,
}

impl Ray {
    /// Create a ray from origin to target point (direction is normalized).
    pub fn from_to(from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> Self {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let len = (dx * dx + dy * dy).sqrt();
        Ray {
            x: from_x,
            y: from_y,
            dx: dx / len,
            dy: dy / len,
        }
    }
}

/// Result of a raycast hit.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    /// Entity that was hit.
    pub entity: u32,
    /// Distance from ray origin to hit point.
    pub distance: f32,
    /// Hit position X.
    pub hit_x: f32,
    /// Hit position Y.
    pub hit_y: f32,
}

impl Quadtree {
    /// Cast a ray through the quadtree, returning hits sorted by distance.
    ///
    /// For each object, computes the closest intersection point on the circle
    /// and returns objects hit within the ray's path.
    pub fn raycast(&self, ray: Ray) -> Vec<RaycastHit> {
        let mut hits = Vec::new();

        for obj in &self.objects {
            if let Some(hit) = self.raycast_circle(ray, obj.x, obj.y, obj.radius) {
                hits.push(hit);
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                hits.extend(c.raycast(ray));
            }
        }

        // Sort by distance
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        hits
    }

    /// Check if a ray intersects a circle and return the hit point if so.
    fn raycast_circle(
        &self,
        ray: Ray,
        cx: f32,
        cy: f32,
        cr: f32,
    ) -> Option<RaycastHit> {
        // Ray-circle intersection: solve |O + tD - C|^2 = r^2
        let ox = ray.x - cx;
        let oy = ray.y - cy;
        let a = ray.dx * ray.dx + ray.dy * ray.dy; // = 1.0 since normalized
        let b = 2.0 * (ox * ray.dx + oy * ray.dy);
        let c = ox * ox + oy * oy - cr * cr;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        // Pick the closest intersection in front of the ray
        let t = if t1 > 0.0 { t1 } else if t2 > 0.0 { t2 } else { return None };

        Some(RaycastHit {
            entity: 0, // placeholder, set by caller
            distance: t,
            hit_x: ray.x + ray.dx * t,
            hit_y: ray.y + ray.dy * t,
        })
    }

    /// Raycast with entity indices. Returns hits sorted by distance.
    pub fn raycast_with_entities(&self, ray: Ray) -> Vec<(u32, RaycastHit)> {
        let mut hits = Vec::new();

        for obj in &self.objects {
            if let Some(hit) = self.raycast_circle(ray, obj.x, obj.y, obj.radius) {
                hits.push((obj.entity, hit));
            }
        }

        for child in &self.children {
            if let Some(ref c) = child {
                hits.extend(c.raycast_with_entities(ray));
            }
        }

        hits.sort_by(|a, b| a.1.distance.partial_cmp(&b.1.distance).unwrap());
        hits
    }
}

//! 2D physics with rigid bodies, collision detection, raycasting, and contact solver.

use std::ops::{Add, Mul, Neg, Sub};

// ──────────────────────────────────────────────
// Vec2
// ──────────────────────────────────────────────

/// 2D vector with x and y components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn zero() -> Self {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn normalize(self) -> Vec2 {
        let len = self.length();
        if len > 1e-8 {
            Vec2 {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Vec2::zero()
        }
    }

    pub fn distance(self, other: Vec2) -> f32 {
        (self - other).length()
    }

    /// Returns the perpendicular vector (-y, x).
    pub fn perp(self) -> Vec2 {
        Vec2 {
            x: -self.y,
            y: self.x,
        }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2 {
            x: self.x * s,
            y: self.y * s,
        }
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

// ──────────────────────────────────────────────
// Collider2D
// ──────────────────────────────────────────────

/// 2D collision shape: circle or axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub enum Collider2D {
    Circle { radius: f32 },
    AABB { half_extents: [f32; 2] },
}

impl Collider2D {
    pub fn circle(radius: f32) -> Self {
        Collider2D::Circle { radius }
    }

    pub fn aabb(hx: f32, hy: f32) -> Self {
        Collider2D::AABB {
            half_extents: [hx, hy],
        }
    }

    /// Returns true if `point` lies inside this collider placed at `pos`.
    pub fn contains_point(&self, pos: Vec2, point: Vec2) -> bool {
        match self {
            Collider2D::Circle { radius } => (point - pos).length_squared() <= radius * radius,
            Collider2D::AABB { half_extents } => {
                point.x >= pos.x - half_extents[0]
                    && point.x <= pos.x + half_extents[0]
                    && point.y >= pos.y - half_extents[1]
                    && point.y <= pos.y + half_extents[1]
            }
        }
    }

    /// Returns true if this collider (at `my_pos`) overlaps `other` (at `other_pos`).
    pub fn intersects(&self, my_pos: Vec2, other: &Collider2D, other_pos: Vec2) -> bool {
        match (self, other) {
            (Collider2D::Circle { radius: ra }, Collider2D::Circle { radius: rb }) => {
                my_pos.distance(other_pos) < ra + rb
            }
            (Collider2D::AABB { half_extents: ha }, Collider2D::AABB { half_extents: hb }) => {
                let ox = (my_pos.x + ha[0]).min(other_pos.x + hb[0])
                    - (my_pos.x - ha[0]).max(other_pos.x - hb[0]);
                let oy = (my_pos.y + ha[1]).min(other_pos.y + hb[1])
                    - (my_pos.y - ha[1]).max(other_pos.y - hb[1]);
                ox > 0.0 && oy > 0.0
            }
            (Collider2D::Circle { radius }, Collider2D::AABB { half_extents }) => {
                circle_aabb_overlap(my_pos, *radius, other_pos, *half_extents)
            }
            (Collider2D::AABB { half_extents }, Collider2D::Circle { radius }) => {
                circle_aabb_overlap(other_pos, *radius, my_pos, *half_extents)
            }
        }
    }
}

#[inline]
fn circle_aabb_overlap(circle_pos: Vec2, radius: f32, aabb_pos: Vec2, half: [f32; 2]) -> bool {
    let cx = circle_pos
        .x
        .clamp(aabb_pos.x - half[0], aabb_pos.x + half[0]);
    let cy = circle_pos
        .y
        .clamp(aabb_pos.y - half[1], aabb_pos.y + half[1]);
    let dx = circle_pos.x - cx;
    let dy = circle_pos.y - cy;
    dx * dx + dy * dy <= radius * radius
}

// ──────────────────────────────────────────────
// RigidBody2D
// ──────────────────────────────────────────────

/// A 2D rigid body with mass, velocity, and optional collider.
#[derive(Debug, Clone, Copy)]
pub struct RigidBody2D {
    pub entity: u32,
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub mass: f32,
    pub restitution: f32,
    pub friction: f32,
    pub is_static: bool,
    pub gravity_scale: f32,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub collider: Option<Collider2D>,
}

impl RigidBody2D {
    pub fn new(entity: u32) -> Self {
        RigidBody2D {
            entity,
            position: Vec2::zero(),
            velocity: Vec2::zero(),
            acceleration: Vec2::zero(),
            mass: 1.0,
            restitution: 0.5,
            friction: 0.3,
            is_static: false,
            gravity_scale: 1.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            collider: None,
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    pub fn with_velocity(mut self, x: f32, y: f32) -> Self {
        self.velocity = Vec2::new(x, y);
        self
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_restitution(mut self, r: f32) -> Self {
        self.restitution = r;
        self
    }

    pub fn with_friction(mut self, f: f32) -> Self {
        self.friction = f;
        self
    }

    pub fn make_static(mut self) -> Self {
        self.is_static = true;
        self.mass = 0.0;
        self
    }

    pub fn with_gravity_scale(mut self, s: f32) -> Self {
        self.gravity_scale = s;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_collider(mut self, collider: Collider2D) -> Self {
        self.collider = Some(collider);
        self
    }

    pub fn inverse_mass(&self) -> f32 {
        if self.is_static || self.mass <= 0.0 {
            0.0
        } else {
            1.0 / self.mass
        }
    }

    pub fn kinetic_energy(&self) -> f32 {
        if self.is_static || self.mass <= 0.0 {
            return 0.0;
        }
        0.5 * self.mass * self.velocity.length_squared()
    }
}

// ──────────────────────────────────────────────
// Contact2D
// ──────────────────────────────────────────────

/// Describes a collision contact between two bodies.
#[derive(Debug, Clone, Copy)]
pub struct Contact2D {
    pub body_a: u32,
    pub body_b: u32,
    /// Normal pointing from body A toward body B.
    pub normal: Vec2,
    /// Penetration depth.
    pub depth: f32,
    /// Contact point in world space.
    pub point: Vec2,
}

// ──────────────────────────────────────────────
// Ray2D / RayHit2D
// ──────────────────────────────────────────────

/// A ray defined by origin and direction.
#[derive(Debug, Clone, Copy)]
pub struct Ray2D {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl Ray2D {
    pub fn new(origin: Vec2, direction: Vec2) -> Self {
        Ray2D { origin, direction }
    }

    /// Returns the point at parameter `t` along the ray.
    pub fn at(&self, t: f32) -> Vec2 {
        self.origin + self.direction * t
    }
}

/// Result of a raycast hit against a body.
#[derive(Debug, Clone, Copy)]
pub struct RayHit2D {
    pub entity: u32,
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}

// ──────────────────────────────────────────────
// PhysicsWorld2D
// ──────────────────────────────────────────────

/// 2D physics world that manages bodies, steps simulation, and resolves collisions.
pub struct PhysicsWorld2D {
    pub bodies: Vec<RigidBody2D>,
    pub gravity: Vec2,
}

impl Default for PhysicsWorld2D {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsWorld2D {
    pub fn new() -> Self {
        PhysicsWorld2D {
            bodies: Vec::new(),
            gravity: Vec2::new(0.0, -9.81),
        }
    }

    /// Adds a body and returns its index.
    pub fn add_body(&mut self, body: RigidBody2D) -> usize {
        let idx = self.bodies.len();
        self.bodies.push(body);
        idx
    }

    pub fn remove_body(&mut self, index: usize) {
        if index < self.bodies.len() {
            self.bodies.remove(index);
        }
    }

    pub fn get_body(&self, index: usize) -> Option<&RigidBody2D> {
        self.bodies.get(index)
    }

    pub fn get_body_mut(&mut self, index: usize) -> Option<&mut RigidBody2D> {
        self.bodies.get_mut(index)
    }

    pub fn find_body_by_entity(&self, entity: u32) -> Option<usize> {
        self.bodies.iter().position(|b| b.entity == entity)
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn clear(&mut self) {
        self.bodies.clear();
    }

    /// Full physics step: gravity, integration, collision detection & resolution.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        // 1. Apply gravity (semi-implicit Euler: update velocity first)
        for body in &mut self.bodies {
            if body.is_static {
                continue;
            }
            body.velocity = body.velocity + self.gravity * body.gravity_scale * dt;
        }

        // 2. Integrate position using the updated velocity
        for body in &mut self.bodies {
            if body.is_static {
                continue;
            }
            body.position = body.position + body.velocity * dt;
            body.rotation += body.angular_velocity * dt;
        }

        // 3. Detect collisions
        let contacts = self.detect_collisions();

        // 4. Resolve collisions (impulse + friction)
        for contact in &contacts {
            self.resolve_collision(contact);
        }
    }

    /// Broad + narrow phase: check all body pairs with colliders.
    pub fn detect_collisions(&self) -> Vec<Contact2D> {
        let mut contacts = Vec::new();
        let n = self.bodies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &self.bodies[i];
                let b = &self.bodies[j];
                let ca = match a.collider {
                    Some(ref c) => c,
                    None => continue,
                };
                let cb = match b.collider {
                    Some(ref c) => c,
                    None => continue,
                };

                let contact = match (ca, cb) {
                    (Collider2D::Circle { radius: ra }, Collider2D::Circle { radius: rb }) => {
                        circle_vs_circle(a.position, *ra, b.position, *rb)
                    }
                    (
                        Collider2D::AABB { half_extents: ha },
                        Collider2D::AABB { half_extents: hb },
                    ) => aabb_vs_aabb(a.position, *ha, b.position, *hb),
                    (Collider2D::Circle { radius }, Collider2D::AABB { half_extents }) => {
                        circle_vs_aabb(a.position, *radius, b.position, *half_extents)
                    }
                    (Collider2D::AABB { half_extents }, Collider2D::Circle { radius }) => {
                        circle_vs_aabb(b.position, *radius, a.position, *half_extents).map(
                            |mut c| {
                                c.normal = -c.normal;
                                c
                            },
                        )
                    }
                };

                if let Some(mut contact) = contact {
                    contact.body_a = a.entity;
                    contact.body_b = b.entity;
                    contacts.push(contact);
                }
            }
        }
        contacts
    }

    /// Impulse-based collision resolution with friction.
    pub fn resolve_collision(&mut self, contact: &Contact2D) {
        let ia = match self.find_body_by_entity(contact.body_a) {
            Some(i) => i,
            None => return,
        };
        let ib = match self.find_body_by_entity(contact.body_b) {
            Some(i) => i,
            None => return,
        };

        let inv_a = self.bodies[ia].inverse_mass();
        let inv_b = self.bodies[ib].inverse_mass();
        let total_inv = inv_a + inv_b;
        if total_inv <= 0.0 {
            return;
        }

        let normal = contact.normal;
        let depth = contact.depth;
        let vel_a = self.bodies[ia].velocity;
        let vel_b = self.bodies[ib].velocity;
        let rest = self.bodies[ia].restitution.min(self.bodies[ib].restitution);
        let friction_coeff = (self.bodies[ia].friction + self.bodies[ib].friction) * 0.5;

        // Positional correction — push bodies apart
        let correction = depth / total_inv;
        let pos_a = self.bodies[ia].position - normal * correction * inv_a;
        let pos_b = self.bodies[ib].position + normal * correction * inv_b;
        self.bodies[ia].position = pos_a;
        self.bodies[ib].position = pos_b;

        // Impulse response
        let rel_vel = vel_b - vel_a;
        let vel_along_normal = rel_vel.dot(normal);
        if vel_along_normal > 0.0 {
            return; // Already separating
        }

        let j = -(1.0 + rest) * vel_along_normal / total_inv;
        let impulse = normal * j;

        let new_vel_a = vel_a - impulse * inv_a;
        let new_vel_b = vel_b + impulse * inv_b;

        // Friction
        let tangent = rel_vel - normal * rel_vel.dot(normal);
        let t_len = tangent.length();
        if t_len > 1e-8 {
            let t = tangent * (1.0 / t_len);
            let jt = -rel_vel.dot(t) / total_inv;
            let friction_impulse = if jt.abs() < j * friction_coeff {
                t * jt
            } else {
                t * (-j * friction_coeff)
            };
            self.bodies[ia].velocity = new_vel_a - friction_impulse * inv_a;
            self.bodies[ib].velocity = new_vel_b + friction_impulse * inv_b;
        } else {
            self.bodies[ia].velocity = new_vel_a;
            self.bodies[ib].velocity = new_vel_b;
        }
    }

    /// Cast a ray against all bodies, returning the closest hit.
    pub fn raycast(&self, ray: &Ray2D, max_distance: f32) -> Option<RayHit2D> {
        let mut closest: Option<RayHit2D> = None;

        for body in &self.bodies {
            let collider = match body.collider {
                Some(ref c) => c,
                None => continue,
            };

            let hit = match collider {
                Collider2D::Circle { radius } => ray_vs_circle(ray, body.position, *radius)
                    .filter(|(t, _)| *t >= 0.0 && *t <= max_distance)
                    .map(|(t, normal)| RayHit2D {
                        entity: body.entity,
                        point: ray.at(t),
                        normal,
                        distance: t,
                    }),
                Collider2D::AABB { half_extents } => ray_vs_aabb(ray, body.position, *half_extents)
                    .filter(|(t, _)| *t >= 0.0 && *t <= max_distance)
                    .map(|(t, normal)| RayHit2D {
                        entity: body.entity,
                        point: ray.at(t),
                        normal,
                        distance: t,
                    }),
            };

            if let Some(h) = hit {
                #[allow(clippy::unnecessary_map_or)]
                if closest.as_ref().map_or(true, |c| h.distance < c.distance) {
                    closest = Some(h);
                }
            }
        }

        closest
    }
}

// ──────────────────────────────────────────────
// Collision Detection Functions
// ──────────────────────────────────────────────

/// Circle vs circle collision. Returns contact with normal from A toward B.
pub fn circle_vs_circle(pos_a: Vec2, r_a: f32, pos_b: Vec2, r_b: f32) -> Option<Contact2D> {
    let diff = pos_b - pos_a;
    let dist = diff.length();
    let combined = r_a + r_b;
    if dist < combined && dist > 1e-8 {
        let normal = diff * (1.0 / dist);
        Some(Contact2D {
            body_a: 0,
            body_b: 0,
            normal,
            depth: combined - dist,
            point: pos_a + normal * r_a,
        })
    } else {
        None
    }
}

/// AABB vs AABB collision. Returns contact with normal from A toward B.
pub fn aabb_vs_aabb(
    pos_a: Vec2,
    half_a: [f32; 2],
    pos_b: Vec2,
    half_b: [f32; 2],
) -> Option<Contact2D> {
    let overlap_x = (pos_a.x + half_a[0]).min(pos_b.x + half_b[0])
        - (pos_a.x - half_a[0]).max(pos_b.x - half_b[0]);
    let overlap_y = (pos_a.y + half_a[1]).min(pos_b.y + half_b[1])
        - (pos_a.y - half_a[1]).max(pos_b.y - half_b[1]);

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    // Minimum penetration axis gives the collision normal
    let (depth, normal) = if overlap_x <= overlap_y {
        let n = if pos_a.x < pos_b.x {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(-1.0, 0.0)
        };
        (overlap_x, n)
    } else {
        let n = if pos_a.y < pos_b.y {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::new(0.0, -1.0)
        };
        (overlap_y, n)
    };

    Some(Contact2D {
        body_a: 0,
        body_b: 0,
        normal,
        depth,
        point: (pos_a + pos_b) * 0.5,
    })
}

/// Circle vs AABB collision. Returns contact with normal from circle (A) toward AABB (B).
pub fn circle_vs_aabb(
    circle_pos: Vec2,
    radius: f32,
    aabb_pos: Vec2,
    half: [f32; 2],
) -> Option<Contact2D> {
    let min = aabb_pos - Vec2::new(half[0], half[1]);
    let max = aabb_pos + Vec2::new(half[0], half[1]);

    let closest = Vec2::new(
        circle_pos.x.clamp(min.x, max.x),
        circle_pos.y.clamp(min.y, max.y),
    );

    let diff = circle_pos - closest;
    let dist_sq = diff.length_squared();

    if dist_sq > 1e-8 && dist_sq < radius * radius {
        // Circle center is outside AABB but overlapping
        let dist = dist_sq.sqrt();
        let normal = diff * (-1.0 / dist); // from circle toward closest on AABB
        Some(Contact2D {
            body_a: 0,
            body_b: 0,
            normal,
            depth: radius - dist,
            point: closest,
        })
    } else if dist_sq <= 1e-8 {
        // Circle center is inside AABB — find nearest face
        let dx_min = circle_pos.x - min.x;
        let dx_max = max.x - circle_pos.x;
        let dy_min = circle_pos.y - min.y;
        let dy_max = max.y - circle_pos.y;

        let min_d = dx_min.min(dx_max).min(dy_min).min(dy_max);

        // Normal = inward normal of nearest face (points from face toward interior)
        let normal = if min_d == dx_min {
            Vec2::new(1.0, 0.0)
        } else if min_d == dx_max {
            Vec2::new(-1.0, 0.0)
        } else if min_d == dy_min {
            Vec2::new(0.0, 1.0)
        } else {
            Vec2::new(0.0, -1.0)
        };

        Some(Contact2D {
            body_a: 0,
            body_b: 0,
            normal,
            depth: radius + min_d,
            point: closest,
        })
    } else {
        None
    }
}

// ──────────────────────────────────────────────
// Ray Casting Functions
// ──────────────────────────────────────────────

/// Ray vs circle. Returns (t, normal) at the hit point.
pub fn ray_vs_circle(ray: &Ray2D, center: Vec2, radius: f32) -> Option<(f32, Vec2)> {
    let oc = ray.origin - center;
    let a = ray.direction.dot(ray.direction);
    let b = 2.0 * oc.dot(ray.direction);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        return None;
    };

    let point = ray.at(t);
    let normal = (point - center).normalize();

    Some((t, normal))
}

/// Ray vs AABB. Returns (t, normal) at the hit point using the slab method.
pub fn ray_vs_aabb(ray: &Ray2D, center: Vec2, half: [f32; 2]) -> Option<(f32, Vec2)> {
    if ray.direction.length_squared() < 1e-16 {
        return None;
    }

    let min = center - Vec2::new(half[0], half[1]);
    let max = center + Vec2::new(half[0], half[1]);

    let mut tmin: f32 = f32::NEG_INFINITY;
    let mut tmax: f32 = f32::INFINITY;
    let mut entry_normal = Vec2::zero();
    let mut exit_normal = Vec2::zero();

    // X slab
    if ray.direction.x.abs() < 1e-8 {
        if ray.origin.x < min.x || ray.origin.x > max.x {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.direction.x;
        let (t_near, t_far, n_near) = if inv_d >= 0.0 {
            (
                (min.x - ray.origin.x) * inv_d,
                (max.x - ray.origin.x) * inv_d,
                Vec2::new(-1.0, 0.0),
            )
        } else {
            (
                (max.x - ray.origin.x) * inv_d,
                (min.x - ray.origin.x) * inv_d,
                Vec2::new(1.0, 0.0),
            )
        };
        if t_near > tmin {
            tmin = t_near;
            entry_normal = n_near;
        }
        if t_far < tmax {
            tmax = t_far;
            exit_normal = -n_near;
        }
        if tmin > tmax {
            return None;
        }
    }

    // Y slab
    if ray.direction.y.abs() < 1e-8 {
        if ray.origin.y < min.y || ray.origin.y > max.y {
            return None;
        }
    } else {
        let inv_d = 1.0 / ray.direction.y;
        let (t_near, t_far, n_near) = if inv_d >= 0.0 {
            (
                (min.y - ray.origin.y) * inv_d,
                (max.y - ray.origin.y) * inv_d,
                Vec2::new(0.0, -1.0),
            )
        } else {
            (
                (max.y - ray.origin.y) * inv_d,
                (min.y - ray.origin.y) * inv_d,
                Vec2::new(0.0, 1.0),
            )
        };
        if t_near > tmin {
            tmin = t_near;
            entry_normal = n_near;
        }
        if t_far < tmax {
            tmax = t_far;
            exit_normal = -n_near;
        }
        if tmin > tmax {
            return None;
        }
    }

    if tmax < 0.0 {
        return None;
    }

    let (t, normal) = if tmin >= 0.0 {
        (tmin, entry_normal)
    } else {
        (tmax, exit_normal)
    };

    Some((t, normal))
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Vec2 tests ──

    #[test]
    fn test_vec2_arithmetic() {
        let a = Vec2::new(3.0, 4.0);
        let b = Vec2::new(1.0, 2.0);

        let sum = a + b;
        assert_eq!(sum, Vec2::new(4.0, 6.0));

        let diff = a - b;
        assert_eq!(diff, Vec2::new(2.0, 2.0));

        let scaled = a * 2.0;
        assert_eq!(scaled, Vec2::new(6.0, 8.0));

        let neg = -a;
        assert_eq!(neg, Vec2::new(-3.0, -4.0));
    }

    #[test]
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);

        // Zero vector normalizes to zero
        let z = Vec2::zero().normalize();
        assert_eq!(z, Vec2::zero());
    }

    #[test]
    fn test_vec2_dot_product() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert!((a.dot(b)).abs() < 1e-6); // Perpendicular

        let c = Vec2::new(1.0, 0.0);
        assert!((c.dot(c) - 1.0).abs() < 1e-6); // Self
    }

    #[test]
    fn test_vec2_perpendicular() {
        let v = Vec2::new(1.0, 0.0);
        let p = v.perp();
        assert_eq!(p, Vec2::new(0.0, 1.0));

        let v2 = Vec2::new(3.0, 4.0);
        let p2 = v2.perp();
        assert_eq!(p2, Vec2::new(-4.0, 3.0));
        // Perpendicular should have zero dot product
        assert!(v2.dot(p2).abs() < 1e-6);
    }

    #[test]
    fn test_vec2_distance() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-6);
    }

    // ── Collider2D tests ──

    #[test]
    fn test_collider_circle_contains_point() {
        let c = Collider2D::circle(5.0);
        let pos = Vec2::new(0.0, 0.0);
        assert!(c.contains_point(pos, Vec2::new(3.0, 0.0)));
        assert!(c.contains_point(pos, Vec2::new(0.0, 5.0))); // On edge
        assert!(!c.contains_point(pos, Vec2::new(6.0, 0.0)));
    }

    #[test]
    fn test_collider_aabb_contains_point() {
        let c = Collider2D::aabb(2.0, 3.0);
        let pos = Vec2::new(5.0, 5.0);
        assert!(c.contains_point(pos, Vec2::new(5.0, 5.0))); // Center
        assert!(c.contains_point(pos, Vec2::new(6.0, 7.0))); // On edge
        assert!(!c.contains_point(pos, Vec2::new(8.0, 5.0)));
    }

    #[test]
    fn test_collider_intersects_circle_circle() {
        let ca = Collider2D::circle(1.0);
        let cb = Collider2D::circle(1.0);
        let pos_a = Vec2::new(0.0, 0.0);
        let pos_b = Vec2::new(1.5, 0.0);
        assert!(ca.intersects(pos_a, &cb, pos_b)); // Overlapping

        let pos_c = Vec2::new(3.0, 0.0);
        assert!(!ca.intersects(pos_a, &cb, pos_c)); // Separated
    }

    #[test]
    fn test_collider_intersects_aabb_aabb() {
        let ca = Collider2D::aabb(1.0, 1.0);
        let cb = Collider2D::aabb(1.0, 1.0);
        assert!(ca.intersects(Vec2::new(0.0, 0.0), &cb, Vec2::new(1.5, 0.0)));
        assert!(!ca.intersects(Vec2::new(0.0, 0.0), &cb, Vec2::new(3.0, 0.0)));
    }

    #[test]
    fn test_collider_intersects_circle_aabb() {
        let circle = Collider2D::circle(1.0);
        let aabb = Collider2D::aabb(1.0, 1.0);
        assert!(circle.intersects(Vec2::new(0.0, 0.0), &aabb, Vec2::new(1.5, 0.0)));
        assert!(aabb.intersects(Vec2::new(1.5, 0.0), &circle, Vec2::new(0.0, 0.0)));
        assert!(!circle.intersects(Vec2::new(0.0, 0.0), &aabb, Vec2::new(5.0, 0.0)));
    }

    // ── RigidBody2D tests ──

    #[test]
    fn test_rigid_body_builder() {
        let body = RigidBody2D::new(42)
            .with_position(1.0, 2.0)
            .with_velocity(3.0, 4.0)
            .with_mass(5.0)
            .with_restitution(0.8)
            .with_friction(0.2)
            .with_gravity_scale(0.5)
            .with_rotation(1.57);

        assert_eq!(body.entity, 42);
        assert_eq!(body.position, Vec2::new(1.0, 2.0));
        assert_eq!(body.velocity, Vec2::new(3.0, 4.0));
        assert!((body.mass - 5.0).abs() < 1e-6);
        assert!((body.restitution - 0.8).abs() < 1e-6);
        assert!((body.friction - 0.2).abs() < 1e-6);
        assert!((body.gravity_scale - 0.5).abs() < 1e-6);
        assert!((body.rotation - 1.57).abs() < 1e-6);
        assert!(!body.is_static);
    }

    #[test]
    fn test_rigid_body_inverse_mass() {
        let dynamic = RigidBody2D::new(1).with_mass(2.0);
        assert!((dynamic.inverse_mass() - 0.5).abs() < 1e-6);

        let static_body = RigidBody2D::new(2).make_static();
        assert!((static_body.inverse_mass()).abs() < 1e-6);

        let zero_mass = RigidBody2D::new(3).with_mass(0.0);
        assert!((zero_mass.inverse_mass()).abs() < 1e-6);
    }

    #[test]
    fn test_rigid_body_kinetic_energy() {
        let body = RigidBody2D::new(1).with_mass(2.0).with_velocity(3.0, 4.0);
        // KE = 0.5 * 2.0 * (9 + 16) = 25.0
        assert!((body.kinetic_energy() - 25.0).abs() < 1e-6);

        let static_body = RigidBody2D::new(2).make_static().with_velocity(3.0, 4.0);
        assert!((static_body.kinetic_energy()).abs() < 1e-6);
    }

    // ── PhysicsWorld2D tests ──

    #[test]
    fn test_world_add_remove_find() {
        let mut world = PhysicsWorld2D::new();
        let idx0 = world.add_body(RigidBody2D::new(10));
        let idx1 = world.add_body(RigidBody2D::new(20));
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(world.body_count(), 2);

        assert_eq!(world.find_body_by_entity(10), Some(0));
        assert_eq!(world.find_body_by_entity(20), Some(1));
        assert_eq!(world.find_body_by_entity(99), None);

        assert!(world.get_body(0).is_some());
        assert!(world.get_body(2).is_none());

        world.remove_body(0);
        assert_eq!(world.body_count(), 1);
        assert_eq!(world.find_body_by_entity(10), None);
        assert_eq!(world.find_body_by_entity(20), Some(0)); // shifted

        world.clear();
        assert_eq!(world.body_count(), 0);
    }

    #[test]
    fn test_gravity_integration() {
        let mut world = PhysicsWorld2D::new();
        world.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 10.0)
                .with_gravity_scale(1.0),
        );

        // Step once with dt=1.0
        world.step(1.0);
        let body = world.get_body(0).unwrap();
        // velocity = (0, -9.81) * 1.0 = (0, -9.81)
        assert!((body.velocity.y - (-9.81)).abs() < 1e-4);
        // position = (0, 10) + (0, -9.81) * 1.0 = (0, 0.19)
        assert!((body.position.y - 0.19).abs() < 0.1);
    }

    // ── Collision detection tests ──

    #[test]
    fn test_collision_circle_circle() {
        let contact = circle_vs_circle(Vec2::new(0.0, 0.0), 1.0, Vec2::new(1.5, 0.0), 1.0).unwrap();
        assert!((contact.depth - 0.5).abs() < 1e-6);
        assert!((contact.normal.x - 1.0).abs() < 1e-6);
        assert!(contact.normal.y.abs() < 1e-6);

        // Non-overlapping
        assert!(circle_vs_circle(Vec2::new(0.0, 0.0), 1.0, Vec2::new(3.0, 0.0), 1.0).is_none());
    }

    #[test]
    fn test_collision_aabb_aabb() {
        let contact = aabb_vs_aabb(
            Vec2::new(0.0, 0.0),
            [1.0, 1.0],
            Vec2::new(1.5, 0.0),
            [1.0, 1.0],
        )
        .unwrap();
        assert!((contact.depth - 0.5).abs() < 1e-6);
        // A is left of B, so normal points from A to B = right = (1, 0)
        assert!((contact.normal.x - 1.0).abs() < 1e-6);
        assert!(contact.normal.y.abs() < 1e-6);

        // Non-overlapping
        assert!(aabb_vs_aabb(
            Vec2::new(0.0, 0.0),
            [1.0, 1.0],
            Vec2::new(5.0, 0.0),
            [1.0, 1.0]
        )
        .is_none());
    }

    #[test]
    fn test_collision_circle_aabb() {
        // Circle to the left, overlapping AABB
        let contact =
            circle_vs_aabb(Vec2::new(-0.5, 0.0), 1.0, Vec2::new(1.0, 0.0), [1.0, 1.0]).unwrap();
        assert!(contact.depth > 0.0);
        // Normal should point from circle toward AABB (positive x direction)
        assert!(contact.normal.x > 0.0);

        // Non-overlapping
        assert!(
            circle_vs_aabb(Vec2::new(-5.0, 0.0), 1.0, Vec2::new(1.0, 0.0), [1.0, 1.0]).is_none()
        );
    }

    #[test]
    fn test_collision_resolution() {
        let mut world = PhysicsWorld2D::new();
        world.gravity = Vec2::zero(); // No gravity for this test

        world.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 0.0)
                .with_velocity(1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world.add_body(
            RigidBody2D::new(2)
                .with_position(0.8, 0.0)
                .with_velocity(-1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.0)
                .with_collider(Collider2D::circle(0.5)),
        );

        world.step(1.0 / 60.0);

        // After resolution, bodies should be separating (velocities reversed or zeroed)
        let a = world.get_body(0).unwrap();
        let b = world.get_body(1).unwrap();

        // A should now be moving left or stopped, B right or stopped
        assert!(a.velocity.x <= 0.0, "A should be moving left or stopped");
        assert!(b.velocity.x >= 0.0, "B should be moving right or stopped");
    }

    #[test]
    fn test_static_body_no_move() {
        let mut world = PhysicsWorld2D::new();
        world.gravity = Vec2::zero();

        world.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 0.0)
                .with_velocity(1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(1.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world.add_body(
            RigidBody2D::new(2)
                .with_position(0.8, 0.0)
                .make_static()
                .with_restitution(1.0)
                .with_collider(Collider2D::circle(0.5)),
        );

        world.step(1.0 / 60.0);

        let a = world.get_body(0).unwrap();
        let b = world.get_body(1).unwrap();

        // Dynamic body should bounce back
        assert!(a.velocity.x < 0.0, "Dynamic body should bounce back");

        // Static body should not have moved
        assert!(
            (b.position.x - 0.8).abs() < 1e-4,
            "Static body should not move"
        );
        assert!(
            b.velocity.x.abs() < 1e-6,
            "Static body should have zero velocity"
        );
    }

    // ── Raycasting tests ──

    #[test]
    fn test_raycast_circle() {
        let ray = Ray2D::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
        let result = ray_vs_circle(&ray, Vec2::new(0.0, 0.0), 1.0);
        assert!(result.is_some());
        let (t, normal) = result.unwrap();
        assert!((t - 4.0).abs() < 1e-4); // Hit at x = -1.0, distance from origin = 4.0
        assert!(normal.x < 0.0); // Normal points outward from circle (toward ray origin)

        // Miss
        let ray2 = Ray2D::new(Vec2::new(-5.0, 3.0), Vec2::new(1.0, 0.0));
        assert!(ray_vs_circle(&ray2, Vec2::new(0.0, 0.0), 1.0).is_none());
    }

    #[test]
    fn test_raycast_aabb() {
        let ray = Ray2D::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
        let result = ray_vs_aabb(&ray, Vec2::new(0.0, 0.0), [1.0, 1.0]);
        assert!(result.is_some());
        let (t, normal) = result.unwrap();
        assert!((t - 4.0).abs() < 1e-4); // Hit at x = -1.0 (left face)
        assert!((normal.x - (-1.0)).abs() < 1e-4); // Normal points left (outward from left face)

        // Miss
        let ray2 = Ray2D::new(Vec2::new(-5.0, 3.0), Vec2::new(1.0, 0.0));
        assert!(ray_vs_aabb(&ray2, Vec2::new(0.0, 0.0), [1.0, 1.0]).is_none());
    }

    #[test]
    fn test_raycast_world() {
        let mut world = PhysicsWorld2D::new();
        world.add_body(
            RigidBody2D::new(1)
                .with_position(5.0, 0.0)
                .with_collider(Collider2D::circle(1.0)),
        );

        let ray = Ray2D::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let hit = world.raycast(&ray, 100.0);
        assert!(hit.is_some());
        let h = hit.unwrap();
        assert_eq!(h.entity, 1);
        assert!((h.distance - 4.0).abs() < 1e-4);
    }

    // ── Restitution test ──

    #[test]
    fn test_restitution_bouncy_vs_nonbouncy() {
        // Use a dynamic body hitting a static wall to isolate restitution effect.
        // With restitution=1.0 the body rebounds at full speed; with 0.0 it stops.

        // Bouncy (restitution = 1.0)
        let mut world_bouncy = PhysicsWorld2D::new();
        world_bouncy.gravity = Vec2::zero();
        world_bouncy.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 0.0)
                .with_velocity(1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(1.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world_bouncy.add_body(
            RigidBody2D::new(2)
                .with_position(0.8, 0.0)
                .make_static()
                .with_restitution(1.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world_bouncy.step(1.0 / 60.0);
        let bouncy_vel = world_bouncy.get_body(0).unwrap().velocity.x;

        // Non-bouncy (restitution = 0.0)
        let mut world_flat = PhysicsWorld2D::new();
        world_flat.gravity = Vec2::zero();
        world_flat.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 0.0)
                .with_velocity(1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world_flat.add_body(
            RigidBody2D::new(2)
                .with_position(0.8, 0.0)
                .make_static()
                .with_restitution(0.0)
                .with_collider(Collider2D::circle(0.5)),
        );
        world_flat.step(1.0 / 60.0);
        let flat_vel = world_flat.get_body(0).unwrap().velocity.x;

        // Bouncy should rebound (negative velocity); flat should stop or move slowly
        assert!(
            bouncy_vel < flat_vel,
            "Bouncy ({}) should rebound more than flat ({})",
            bouncy_vel,
            flat_vel
        );
        // Bouncy body should be moving backward
        assert!(bouncy_vel < -0.5, "Bouncy body should rebound strongly");
    }

    // ── Multiple bodies test ──

    #[test]
    fn test_multiple_bodies_collision() {
        let mut world = PhysicsWorld2D::new();
        world.gravity = Vec2::zero();

        // Three bodies close enough to collide in one frame
        world.add_body(
            RigidBody2D::new(1)
                .with_position(-0.4, 0.0)
                .with_velocity(1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.5)
                .with_collider(Collider2D::circle(0.3)),
        );
        world.add_body(
            RigidBody2D::new(2)
                .with_position(0.0, 0.0)
                .with_velocity(0.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.5)
                .with_collider(Collider2D::circle(0.3)),
        );
        world.add_body(
            RigidBody2D::new(3)
                .with_position(0.4, 0.0)
                .with_velocity(-1.0, 0.0)
                .with_mass(1.0)
                .with_restitution(0.5)
                .with_collider(Collider2D::circle(0.3)),
        );

        world.step(1.0 / 60.0);

        let a = world.get_body(0).unwrap();
        let b = world.get_body(1).unwrap();
        let c = world.get_body(2).unwrap();

        // Body A should slow down or reverse (was moving right at 1.0)
        assert!(
            a.velocity.x < 1.0,
            "A should slow down, got {}",
            a.velocity.x
        );
        // Body C should slow down or reverse (was moving left at -1.0)
        assert!(
            c.velocity.x > -1.0,
            "C should slow down, got {}",
            c.velocity.x
        );
        // Middle body B should have been pushed
        assert!(b.velocity.x.abs() > 0.0 || b.position.x.abs() > 0.01);
    }

    // ── Edge case tests ──

    #[test]
    fn test_edge_case_zero_dt() {
        let mut world = PhysicsWorld2D::new();
        world.add_body(
            RigidBody2D::new(1)
                .with_position(5.0, 5.0)
                .with_velocity(1.0, 0.0),
        );

        let pos_before = world.get_body(0).unwrap().position;
        world.step(0.0);
        let pos_after = world.get_body(0).unwrap().position;
        assert_eq!(pos_before, pos_after);
    }

    #[test]
    fn test_edge_case_overlapping_bodies() {
        let mut world = PhysicsWorld2D::new();
        world.gravity = Vec2::zero();

        // Two bodies at the exact same position
        world.add_body(
            RigidBody2D::new(1)
                .with_position(0.0, 0.0)
                .with_mass(1.0)
                .with_collider(Collider2D::circle(1.0)),
        );
        world.add_body(
            RigidBody2D::new(2)
                .with_position(0.0, 0.0)
                .with_mass(1.0)
                .with_collider(Collider2D::circle(1.0)),
        );

        // Should not panic
        world.step(1.0 / 60.0);
        assert_eq!(world.body_count(), 2);
    }

    #[test]
    fn test_edge_case_zero_mass() {
        let body = RigidBody2D::new(1).with_mass(0.0);
        assert!((body.inverse_mass()).abs() < 1e-8);
        assert!((body.kinetic_energy()).abs() < 1e-8);
    }

    #[test]
    fn test_raycast_max_distance() {
        let mut world = PhysicsWorld2D::new();
        world.add_body(
            RigidBody2D::new(1)
                .with_position(10.0, 0.0)
                .with_collider(Collider2D::circle(1.0)),
        );

        let ray = Ray2D::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        // Too far
        assert!(world.raycast(&ray, 5.0).is_none());
        // Close enough
        assert!(world.raycast(&ray, 100.0).is_some());
    }
}

//! 3D physics with rigid bodies, collision detection, and constraints.

#[derive(Debug, Clone, Copy)]
pub struct RigidBody3D {
    pub entity: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
    pub mass: f32,
    pub restitution: f32,
    pub friction: f32,
    pub is_static: bool,
    pub gravity_scale: f32,
}

impl RigidBody3D {
    pub fn new(entity: u32) -> Self {
        RigidBody3D {
            entity,
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            mass: 1.0,
            restitution: 0.5,
            friction: 0.3,
            is_static: false,
            gravity_scale: 1.0,
        }
    }

    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    pub fn with_velocity(mut self, x: f32, y: f32, z: f32) -> Self {
        self.velocity = [x, y, z];
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

    pub fn inverse_mass(&self) -> f32 {
        if self.is_static || self.mass <= 0.0 {
            0.0
        } else {
            1.0 / self.mass
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Collider3D {
    Sphere { radius: f32 },
    AABB { half_extents: [f32; 3] },
}

impl Collider3D {
    pub fn sphere(radius: f32) -> Self {
        Collider3D::Sphere { radius }
    }

    pub fn aabb(hx: f32, hy: f32, hz: f32) -> Self {
        Collider3D::AABB { half_extents: [hx, hy, hz] }
    }

    pub fn contains_point(&self, point: [f32; 3], position: [f32; 3]) -> bool {
        match self {
            Collider3D::Sphere { radius } => {
                let dx = point[0] - position[0];
                let dy = point[1] - position[1];
                let dz = point[2] - position[2];
                dx * dx + dy * dy + dz * dz <= radius * radius
            }
            Collider3D::AABB { half_extents } => {
                let min_x = position[0] - half_extents[0];
                let max_x = position[0] + half_extents[0];
                let min_y = position[1] - half_extents[1];
                let max_y = position[1] + half_extents[1];
                let min_z = position[2] - half_extents[2];
                let max_z = position[2] + half_extents[2];
                point[0] >= min_x && point[0] <= max_x
                    && point[1] >= min_y && point[1] <= max_y
                    && point[2] >= min_z && point[2] <= max_z
            }
        }
    }

    pub fn intersects(
        &self,
        other: &Collider3D,
        self_pos: [f32; 3],
        other_pos: [f32; 3],
    ) -> Option<Contact3D> {
        match (self, other) {
            (Collider3D::Sphere { radius: ra }, Collider3D::Sphere { radius: rb }) => {
                let diff = sub3(other_pos, self_pos);
                let dist = len3(diff);
                let combined = ra + rb;
                if dist < combined && dist > 1e-8 {
                    let n = scale3(diff, 1.0 / dist);
                    Some(Contact3D {
                        entity_a: 0,
                        entity_b: 0,
                        normal: n,
                        depth: combined - dist,
                        point: add3(self_pos, scale3(n, *ra)),
                    })
                } else {
                    None
                }
            }
            (Collider3D::Sphere { radius }, Collider3D::AABB { half_extents }) => {
                sphere_aabb_contact(0, 0, self_pos, *radius, other_pos, *half_extents)
            }
            (Collider3D::AABB { half_extents }, Collider3D::Sphere { radius }) => {
                sphere_aabb_contact(0, 0, self_pos, *radius, other_pos, *half_extents).map(|mut c| {
                    c.normal = neg3(c.normal);
                    c
                })
            }
            (Collider3D::AABB { half_extents: ha }, Collider3D::AABB { half_extents: hb }) => {
                aabb_aabb_contact(0, 0, self_pos, *ha, other_pos, *hb)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Contact3D {
    pub entity_a: u32,
    pub entity_b: u32,
    pub normal: [f32; 3],
    pub depth: f32,
    pub point: [f32; 3],
}

pub trait Constraint3D {
    fn entities(&self) -> (u32, u32);
    fn solve(&self, bodies: &mut [RigidBody3D], dt: f32);
}

#[derive(Debug, Clone, Copy)]
pub struct DistanceConstraint {
    pub entity_a: u32,
    pub entity_b: u32,
    pub target_distance: f32,
    pub stiffness: f32,
}

impl DistanceConstraint {
    pub fn new(a: u32, b: u32, distance: f32, stiffness: f32) -> Self {
        DistanceConstraint {
            entity_a: a,
            entity_b: b,
            target_distance: distance,
            stiffness,
        }
    }
}

impl Constraint3D for DistanceConstraint {
    fn entities(&self) -> (u32, u32) {
        (self.entity_a, self.entity_b)
    }

    fn solve(&self, bodies: &mut [RigidBody3D], _dt: f32) {
        let (ia, ib) = match (find_body(bodies, self.entity_a), find_body(bodies, self.entity_b)) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };

        let diff = sub3(bodies[ib].position, bodies[ia].position);
        let dist = len3(diff);
        if dist < 1e-8 {
            return;
        }
        let n = scale3(diff, 1.0 / dist);
        let error = dist - self.target_distance;
        let inv_a = bodies[ia].inverse_mass();
        let inv_b = bodies[ib].inverse_mass();
        let total_inv = inv_a + inv_b;
        if total_inv <= 0.0 {
            return;
        }
        let correction = error * self.stiffness / total_inv;
        if inv_a > 0.0 {
            bodies[ia].position = add3(bodies[ia].position, scale3(n, -correction * inv_a));
        }
        if inv_b > 0.0 {
            bodies[ib].position = add3(bodies[ib].position, scale3(n, correction * inv_b));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PointConstraint {
    pub entity: u32,
    pub anchor: [f32; 3],
    pub stiffness: f32,
}

impl PointConstraint {
    pub fn new(entity: u32, anchor: [f32; 3], stiffness: f32) -> Self {
        PointConstraint { entity, anchor, stiffness }
    }
}

impl Constraint3D for PointConstraint {
    fn entities(&self) -> (u32, u32) {
        (self.entity, self.entity)
    }

    fn solve(&self, bodies: &mut [RigidBody3D], _dt: f32) {
        let i = match find_body(bodies, self.entity) {
            Some(i) => i,
            None => return,
        };
        if bodies[i].is_static {
            return;
        }
        let diff = sub3(self.anchor, bodies[i].position);
        bodies[i].position = add3(bodies[i].position, scale3(diff, self.stiffness));
    }
}

pub struct PhysicsWorld3D {
    pub bodies: Vec<RigidBody3D>,
    pub colliders: Vec<(u32, Collider3D)>,
    pub gravity: [f32; 3],
    pub constraints: Vec<Box<dyn Constraint3D>>,
}

impl PhysicsWorld3D {
    pub fn new() -> Self {
        PhysicsWorld3D {
            bodies: Vec::new(),
            colliders: Vec::new(),
            gravity: [0.0, -9.81, 0.0],
            constraints: Vec::new(),
        }
    }

    pub fn add_body(&mut self, body: RigidBody3D) {
        self.bodies.push(body);
    }

    pub fn add_collider(&mut self, entity: u32, collider: Collider3D) {
        self.colliders.push((entity, collider));
    }

    pub fn add_constraint(&mut self, constraint: Box<dyn Constraint3D>) {
        self.constraints.push(constraint);
    }

    pub fn step(&mut self, dt: f32) {
        self.apply_gravity(dt);
        self.integrate(dt);
        let contacts = self.detect_collisions();
        self.resolve_collisions(&contacts, dt);
        for constraint in &self.constraints {
            constraint.solve(&mut self.bodies, dt);
        }
    }

    pub fn apply_gravity(&mut self, dt: f32) {
        for body in &mut self.bodies {
            if body.is_static {
                continue;
            }
            body.acceleration = scale3(self.gravity, body.gravity_scale);
            body.velocity = add3(body.velocity, scale3(body.acceleration, dt));
        }
    }

    pub fn integrate(&mut self, dt: f32) {
        for body in &mut self.bodies {
            if body.is_static {
                continue;
            }
            body.position = add3(body.position, scale3(body.velocity, dt));
        }
    }

    pub fn detect_collisions(&self) -> Vec<Contact3D> {
        let mut contacts = Vec::new();
        let n = self.colliders.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (ea, ca) = &self.colliders[i];
                let (eb, cb) = &self.colliders[j];
                let pa = self.body_position(*ea);
                let pb = self.body_position(*eb);
                if let Some(mut contact) = ca.intersects(cb, pa, pb) {
                    contact.entity_a = *ea;
                    contact.entity_b = *eb;
                    contacts.push(contact);
                }
            }
        }
        contacts
    }

    fn body_position(&self, entity: u32) -> [f32; 3] {
        self.bodies.iter()
            .find(|b| b.entity == entity)
            .map(|b| b.position)
            .unwrap_or([0.0, 0.0, 0.0])
    }

    pub fn resolve_collisions(&mut self, contacts: &[Contact3D], _dt: f32) {
        for contact in contacts {
            let (ia, ib) = match (
                find_body(&self.bodies, contact.entity_a),
                find_body(&self.bodies, contact.entity_b),
            ) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            let inv_a = self.bodies[ia].inverse_mass();
            let inv_b = self.bodies[ib].inverse_mass();
            let total_inv = inv_a + inv_b;
            if total_inv <= 0.0 {
                continue;
            }

            let correction = contact.depth / total_inv;
            if inv_a > 0.0 {
                self.bodies[ia].position = sub3(
                    self.bodies[ia].position,
                    scale3(contact.normal, correction * inv_a),
                );
            }
            if inv_b > 0.0 {
                self.bodies[ib].position = add3(
                    self.bodies[ib].position,
                    scale3(contact.normal, correction * inv_b),
                );
            }

            let rel_vel = sub3(self.bodies[ib].velocity, self.bodies[ia].velocity);
            let vel_along_normal = dot3(rel_vel, contact.normal);
            if vel_along_normal > 0.0 {
                continue;
            }

            let e = self.bodies[ia].restitution.min(self.bodies[ib].restitution);
            let j = -(1.0 + e) * vel_along_normal / total_inv;

            let impulse = scale3(contact.normal, j);
            if inv_a > 0.0 {
                self.bodies[ia].velocity = sub3(self.bodies[ia].velocity, scale3(impulse, inv_a));
            }
            if inv_b > 0.0 {
                self.bodies[ib].velocity = add3(self.bodies[ib].velocity, scale3(impulse, inv_b));
            }

            let tangent = sub3(rel_vel, scale3(contact.normal, dot3(rel_vel, contact.normal)));
            let t_len = len3(tangent);
            if t_len > 1e-8 {
                let t = scale3(tangent, 1.0 / t_len);
                let friction_coeff = (self.bodies[ia].friction + self.bodies[ib].friction) * 0.5;
                let jt = -dot3(rel_vel, t) / total_inv;
                let friction_impulse = if jt.abs() < j * friction_coeff {
                    scale3(t, jt)
                } else {
                    scale3(t, -j * friction_coeff)
                };

                if inv_a > 0.0 {
                    self.bodies[ia].velocity = sub3(self.bodies[ia].velocity, scale3(friction_impulse, inv_a));
                }
                if inv_b > 0.0 {
                    self.bodies[ib].velocity = add3(self.bodies[ib].velocity, scale3(friction_impulse, inv_b));
                }
            }
        }
    }
}

fn find_body(bodies: &[RigidBody3D], entity: u32) -> Option<usize> {
    bodies.iter().position(|b| b.entity == entity)
}

fn sphere_aabb_contact(
    sphere_entity: u32, aabb_entity: u32,
    sphere_pos: [f32; 3], radius: f32,
    aabb_pos: [f32; 3], half_extents: [f32; 3],
) -> Option<Contact3D> {
    let min = sub3(aabb_pos, half_extents);
    let max = add3(aabb_pos, half_extents);
    let closest = [
        sphere_pos[0].clamp(min[0], max[0]),
        sphere_pos[1].clamp(min[1], max[1]),
        sphere_pos[2].clamp(min[2], max[2]),
    ];
    let diff = sub3(sphere_pos, closest);
    let dist = len3(diff);
    if dist < radius && dist > 1e-8 {
        let n = scale3(diff, 1.0 / dist);
        Some(Contact3D {
            entity_a: sphere_entity,
            entity_b: aabb_entity,
            normal: n,
            depth: radius - dist,
            point: closest,
        })
    } else if dist < 1e-8 {
        let dx_min = sphere_pos[0] - min[0];
        let dx_max = max[0] - sphere_pos[0];
        let dy_min = sphere_pos[1] - min[1];
        let dy_max = max[1] - sphere_pos[1];
        let dz_min = sphere_pos[2] - min[2];
        let dz_max = max[2] - sphere_pos[2];
        let min_d = dx_min.min(dx_max).min(dy_min).min(dy_max).min(dz_min).min(dz_max);
        let n = if min_d == dx_min { [-1.0, 0.0, 0.0] }
                else if min_d == dx_max { [1.0, 0.0, 0.0] }
                else if min_d == dy_min { [0.0, -1.0, 0.0] }
                else if min_d == dy_max { [0.0, 1.0, 0.0] }
                else if min_d == dz_min { [0.0, 0.0, -1.0] }
                else { [0.0, 0.0, 1.0] };
        Some(Contact3D {
            entity_a: sphere_entity,
            entity_b: aabb_entity,
            normal: n,
            depth: radius,
            point: closest,
        })
    } else {
        None
    }
}

fn aabb_aabb_contact(
    ea: u32, eb: u32,
    pa: [f32; 3], ha: [f32; 3],
    pb: [f32; 3], hb: [f32; 3],
) -> Option<Contact3D> {
    let min_a = sub3(pa, ha);
    let max_a = add3(pa, ha);
    let min_b = sub3(pb, hb);
    let max_b = add3(pb, hb);

    let overlap_x = (max_a[0].min(max_b[0]) - min_a[0].max(min_b[0])).max(0.0);
    let overlap_y = (max_a[1].min(max_b[1]) - min_a[1].max(min_b[1])).max(0.0);
    let overlap_z = (max_a[2].min(max_b[2]) - min_a[2].max(min_b[2])).max(0.0);

    if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    let (depth, normal) = if overlap_x <= overlap_y && overlap_x <= overlap_z {
        let n = if pa[0] < pb[0] { [-1.0, 0.0, 0.0] } else { [1.0, 0.0, 0.0] };
        (overlap_x, n)
    } else if overlap_y <= overlap_z {
        let n = if pa[1] < pb[1] { [0.0, -1.0, 0.0] } else { [0.0, 1.0, 0.0] };
        (overlap_y, n)
    } else {
        let n = if pa[2] < pb[2] { [0.0, 0.0, -1.0] } else { [0.0, 0.0, 1.0] };
        (overlap_z, n)
    };

    Some(Contact3D {
        entity_a: ea,
        entity_b: eb,
        normal,
        depth,
        point: scale3(add3(pa, pb), 0.5),
    })
}

#[inline]
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn scale3(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn len3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

#[inline]
fn neg3(v: [f32; 3]) -> [f32; 3] {
    [-v[0], -v[1], -v[2]]
}

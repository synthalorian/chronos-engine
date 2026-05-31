//! Particle system for visual effects.
//!
//! Provides particle emitter components and a system for
//! updating particle lifetimes, velocities, and spawning.

use crate::component::Position;
use crate::system::{EventBus, System};
use crate::world::World;

/// A particle emitter attached to an entity.
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Rate of particle emission (particles per second).
    pub rate: f32,
    /// Accumulated emission time.
    pub accumulator: f32,
    /// Particle lifetime in seconds.
    pub lifetime: f32,
    /// Initial speed range (min, max).
    pub speed_range: (f32, f32),
    /// Direction angle range in radians (min, max).
    pub angle_range: (f32, f32),
    /// Particle size in world units.
    pub size: f32,
    /// Starting color (r, g, b, a).
    pub color_start: [f32; 4],
    /// Ending color (r, g, b, a).
    pub color_end: [f32; 4],
    /// Maximum particles alive at once.
    pub max_particles: u32,
    /// Whether to use world gravity.
    pub gravity: f32,
}

impl ParticleEmitter {
    pub fn new(rate: f32, lifetime: f32) -> Self {
        ParticleEmitter {
            rate,
            accumulator: 0.0,
            lifetime,
            speed_range: (20.0, 60.0),
            angle_range: (0.0, std::f32::consts::TAU),
            size: 4.0,
            color_start: [1.0, 0.8, 0.2, 1.0],
            color_end: [1.0, 0.2, 0.0, 0.0],
            max_particles: 100,
            gravity: 0.0,
        }
    }

    pub fn explosion() -> Self {
        ParticleEmitter {
            rate: 200.0,
            accumulator: 0.0,
            lifetime: 0.5,
            speed_range: (40.0, 120.0),
            angle_range: (0.0, std::f32::consts::TAU),
            size: 3.0,
            color_start: [1.0, 0.6, 0.1, 1.0],
            color_end: [0.5, 0.1, 0.0, 0.0],
            max_particles: 50,
            gravity: 50.0,
        }
    }

    pub fn smoke() -> Self {
        ParticleEmitter {
            rate: 10.0,
            accumulator: 0.0,
            lifetime: 2.0,
            speed_range: (5.0, 15.0),
            angle_range: (-0.3, 0.3),
            size: 6.0,
            color_start: [0.5, 0.5, 0.5, 0.6],
            color_end: [0.3, 0.3, 0.3, 0.0],
            max_particles: 30,
            gravity: -10.0,
        }
    }

    pub fn trail() -> Self {
        ParticleEmitter {
            rate: 30.0,
            accumulator: 0.0,
            lifetime: 0.3,
            speed_range: (2.0, 8.0),
            angle_range: (-0.5, 0.5),
            size: 2.0,
            color_start: [0.3, 0.6, 1.0, 0.8],
            color_end: [0.1, 0.2, 0.5, 0.0],
            max_particles: 20,
            gravity: 0.0,
        }
    }
}

/// Per-particle state.
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    /// Remaining lifetime in seconds.
    pub lifetime: f32,
    /// Total lifetime (for computing t).
    pub total_lifetime: f32,
    /// Velocity X.
    pub vx: f32,
    /// Velocity Y.
    pub vy: f32,
    /// Size in world units.
    pub size: f32,
    /// Color interpolation start.
    pub color_start: [f32; 4],
    /// Color interpolation end.
    pub color_end: [f32; 4],
    /// Gravity applied per second.
    pub gravity: f32,
}

impl Particle {
    pub fn t(&self) -> f32 {
        1.0 - (self.lifetime / self.total_lifetime).clamp(0.0, 1.0)
    }

    pub fn current_color(&self) -> [f32; 4] {
        let t = self.t();
        [
            self.color_start[0] + (self.color_end[0] - self.color_start[0]) * t,
            self.color_start[1] + (self.color_end[1] - self.color_start[1]) * t,
            self.color_start[2] + (self.color_end[2] - self.color_start[2]) * t,
            self.color_start[3] + (self.color_end[3] - self.color_start[3]) * t,
        ]
    }
}

/// System that updates particle lifetimes and velocities.
pub struct ParticleSystem {
    pub name: String,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            name: "particle".to_string(),
        }
    }
}

impl System for ParticleSystem {
    fn update(&mut self, world: &mut World, _events: &mut EventBus, dt: f64) {
        let dt = dt as f32;

        let particles: Vec<(u32, f32, f32, f32, f32)> = world
            .query_mut::<Particle>()
            .map(|(e, p)| (e.index(), p.lifetime, p.vx, p.vy, p.gravity))
            .collect();

        for (idx, lifetime, vx, vy, gravity) in particles {
            let entity = world.entity_from_index(idx);
            if !world.entity_exists(entity) {
                continue;
            }

            let new_lifetime = lifetime - dt;
            if new_lifetime <= 0.0 {
                world.destroy_entity(entity);
                continue;
            }

            if let Some(particle) = world.get_component_mut::<Particle>(entity) {
                particle.lifetime = new_lifetime;
                particle.vy += gravity * dt;
            }

            if let Some(pos) = world.get_component_mut::<Position>(entity) {
                pos.x += vx * dt;
                pos.y += vy * dt;
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

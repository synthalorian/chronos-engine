#[cfg(feature = "game")]

use std::collections::HashMap;

// ── EffectType ──

/// Identifies the kind of visual effect to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    MeleeHit,
    RangedHit,
    MagicHit,
    Heal,
    LevelUp,
    Death,
    LootDrop,
    GoldPickup,
    ShieldBreak,
    ManaRegen,
    XpGain,
    DustTrail,
    BloodSplatter,
    FireBurst,
    IceShatter,
    LightningStrike,
}

impl EffectType {
    /// Human-readable name for the effect type.
    pub fn name(&self) -> &str {
        match self {
            EffectType::MeleeHit => "Melee Hit",
            EffectType::RangedHit => "Ranged Hit",
            EffectType::MagicHit => "Magic Hit",
            EffectType::Heal => "Heal",
            EffectType::LevelUp => "Level Up",
            EffectType::Death => "Death",
            EffectType::LootDrop => "Loot Drop",
            EffectType::GoldPickup => "Gold Pickup",
            EffectType::ShieldBreak => "Shield Break",
            EffectType::ManaRegen => "Mana Regen",
            EffectType::XpGain => "XP Gain",
            EffectType::DustTrail => "Dust Trail",
            EffectType::BloodSplatter => "Blood Splatter",
            EffectType::FireBurst => "Fire Burst",
            EffectType::IceShatter => "Ice Shatter",
            EffectType::LightningStrike => "Lightning Strike",
        }
    }

    /// Default duration of the effect in seconds.
    pub fn duration(&self) -> f32 {
        match self {
            EffectType::MeleeHit => 0.3,
            EffectType::RangedHit => 0.2,
            EffectType::MagicHit => 0.5,
            EffectType::Heal => 0.6,
            EffectType::LevelUp => 1.5,
            EffectType::Death => 1.0,
            EffectType::LootDrop => 0.4,
            EffectType::GoldPickup => 0.3,
            EffectType::ShieldBreak => 0.5,
            EffectType::ManaRegen => 0.4,
            EffectType::XpGain => 0.5,
            EffectType::DustTrail => 0.8,
            EffectType::BloodSplatter => 0.4,
            EffectType::FireBurst => 0.6,
            EffectType::IceShatter => 0.5,
            EffectType::LightningStrike => 0.3,
        }
    }
}

// ── EffectParams ──

/// Positional and visual parameters for spawning an effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParams {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub scale: f32,
    /// 0.0–1.0, affects particle count and size.
    pub intensity: f32,
    pub color_override: Option<[f32; 4]>,
}

impl EffectParams {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        EffectParams {
            x,
            y,
            z,
            scale: 1.0,
            intensity: 1.0,
            color_override: None,
        }
    }

    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = s;
        self
    }

    pub fn with_intensity(mut self, i: f32) -> Self {
        self.intensity = i;
        self
    }

    pub fn with_color(mut self, c: [f32; 4]) -> Self {
        self.color_override = Some(c);
        self
    }
}

// ── ParticleProfile ──

/// Describes the particle characteristics for a visual effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleProfile {
    pub count: u32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub angle_min: f32,
    pub angle_max: f32,
    pub size: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub gravity: f32,
    pub lifetime: f32,
}

impl ParticleProfile {
    /// Return a new profile with `count` and `size` scaled by `scale`.
    pub fn scaled(&self, scale: f32) -> ParticleProfile {
        ParticleProfile {
            count: ((self.count as f32) * scale).round().max(1.0) as u32,
            size: self.size * scale,
            ..*self
        }
    }
}

// ── EffectPreset ──

/// Factory for game-specific particle profiles.
pub struct EffectPreset;

impl EffectPreset {
    /// Fast orange/red sparks with upward spread.
    pub fn melee_hit() -> ParticleProfile {
        ParticleProfile {
            count: 15,
            speed_min: 3.0,
            speed_max: 8.0,
            angle_min: 60.0,
            angle_max: 120.0,
            size: 0.1,
            color_start: [1.0, 0.5, 0.0, 1.0],
            color_end: [0.8, 0.1, 0.0, 0.0],
            gravity: -2.0,
            lifetime: 0.3,
        }
    }

    /// Fast yellow/white particles in a narrow cone.
    pub fn ranged_hit() -> ParticleProfile {
        ParticleProfile {
            count: 8,
            speed_min: 5.0,
            speed_max: 10.0,
            angle_min: 75.0,
            angle_max: 105.0,
            size: 0.08,
            color_start: [1.0, 1.0, 0.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            gravity: 0.0,
            lifetime: 0.2,
        }
    }

    /// Medium-speed purple/blue radial burst.
    pub fn magic_hit() -> ParticleProfile {
        ParticleProfile {
            count: 20,
            speed_min: 2.0,
            speed_max: 5.0,
            angle_min: 0.0,
            angle_max: 360.0,
            size: 0.12,
            color_start: [0.5, 0.0, 1.0, 1.0],
            color_end: [0.2, 0.4, 1.0, 0.0],
            gravity: 0.0,
            lifetime: 0.5,
        }
    }

    /// Slow green particles rising in a column.
    pub fn heal() -> ParticleProfile {
        ParticleProfile {
            count: 25,
            speed_min: 0.5,
            speed_max: 1.5,
            angle_min: 80.0,
            angle_max: 100.0,
            size: 0.1,
            color_start: [0.0, 1.0, 0.3, 1.0],
            color_end: [0.2, 0.8, 0.4, 0.0],
            gravity: -1.0,
            lifetime: 0.6,
        }
    }

    /// Medium gold/white particles expanding in a ring.
    pub fn level_up() -> ParticleProfile {
        ParticleProfile {
            count: 50,
            speed_min: 2.0,
            speed_max: 4.0,
            angle_min: 0.0,
            angle_max: 360.0,
            size: 0.15,
            color_start: [1.0, 0.85, 0.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            gravity: -0.5,
            lifetime: 1.5,
        }
    }

    /// Fast dark red/gray radial burst.
    pub fn death() -> ParticleProfile {
        ParticleProfile {
            count: 40,
            speed_min: 3.0,
            speed_max: 7.0,
            angle_min: 0.0,
            angle_max: 360.0,
            size: 0.12,
            color_start: [0.5, 0.0, 0.0, 1.0],
            color_end: [0.3, 0.3, 0.3, 0.0],
            gravity: 0.5,
            lifetime: 1.0,
        }
    }

    /// Medium golden particles rising upward.
    pub fn loot_drop() -> ParticleProfile {
        ParticleProfile {
            count: 12,
            speed_min: 1.0,
            speed_max: 3.0,
            angle_min: 70.0,
            angle_max: 110.0,
            size: 0.09,
            color_start: [1.0, 0.85, 0.0, 1.0],
            color_end: [0.8, 0.6, 0.0, 0.0],
            gravity: -1.0,
            lifetime: 0.4,
        }
    }

    /// Slow brown/tan particles with low spread.
    pub fn dust_trail() -> ParticleProfile {
        ParticleProfile {
            count: 5,
            speed_min: 0.5,
            speed_max: 1.0,
            angle_min: 0.0,
            angle_max: 30.0,
            size: 0.15,
            color_start: [0.6, 0.4, 0.2, 1.0],
            color_end: [0.8, 0.7, 0.5, 0.0],
            gravity: -0.3,
            lifetime: 0.8,
        }
    }

    /// Fast orange/red particles bursting upward.
    pub fn fire_burst() -> ParticleProfile {
        ParticleProfile {
            count: 30,
            speed_min: 3.0,
            speed_max: 6.0,
            angle_min: 50.0,
            angle_max: 130.0,
            size: 0.14,
            color_start: [1.0, 0.5, 0.0, 1.0],
            color_end: [0.8, 0.1, 0.0, 0.0],
            gravity: -1.5,
            lifetime: 0.6,
        }
    }

    /// Medium cyan/white particles scattering radially.
    pub fn ice_shatter() -> ParticleProfile {
        ParticleProfile {
            count: 25,
            speed_min: 2.0,
            speed_max: 5.0,
            angle_min: 0.0,
            angle_max: 360.0,
            size: 0.1,
            color_start: [0.0, 0.8, 1.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            gravity: 0.5,
            lifetime: 0.5,
        }
    }

    /// Return the preset profile for any effect type.
    pub fn get(effect_type: EffectType) -> ParticleProfile {
        match effect_type {
            EffectType::MeleeHit => Self::melee_hit(),
            EffectType::RangedHit => Self::ranged_hit(),
            EffectType::MagicHit => Self::magic_hit(),
            EffectType::Heal => Self::heal(),
            EffectType::LevelUp => Self::level_up(),
            EffectType::Death => Self::death(),
            EffectType::LootDrop => Self::loot_drop(),
            EffectType::GoldPickup => ParticleProfile {
                count: 10,
                speed_min: 1.0,
                speed_max: 3.0,
                angle_min: 70.0,
                angle_max: 110.0,
                size: 0.08,
                color_start: [1.0, 0.85, 0.0, 1.0],
                color_end: [1.0, 0.7, 0.0, 0.0],
                gravity: -1.0,
                lifetime: 0.3,
            },
            EffectType::ShieldBreak => ParticleProfile {
                count: 20,
                speed_min: 3.0,
                speed_max: 6.0,
                angle_min: 0.0,
                angle_max: 360.0,
                size: 0.15,
                color_start: [0.3, 0.5, 1.0, 1.0],
                color_end: [0.1, 0.3, 0.8, 0.0],
                gravity: 0.0,
                lifetime: 0.5,
            },
            EffectType::ManaRegen => ParticleProfile {
                count: 15,
                speed_min: 0.5,
                speed_max: 2.0,
                angle_min: 80.0,
                angle_max: 100.0,
                size: 0.1,
                color_start: [0.3, 0.3, 1.0, 1.0],
                color_end: [0.5, 0.0, 1.0, 0.0],
                gravity: -0.5,
                lifetime: 0.4,
            },
            EffectType::XpGain => ParticleProfile {
                count: 8,
                speed_min: 1.0,
                speed_max: 3.0,
                angle_min: 75.0,
                angle_max: 105.0,
                size: 0.06,
                color_start: [1.0, 1.0, 0.5, 1.0],
                color_end: [0.8, 0.8, 0.2, 0.0],
                gravity: -1.0,
                lifetime: 0.5,
            },
            EffectType::DustTrail => Self::dust_trail(),
            EffectType::BloodSplatter => ParticleProfile {
                count: 18,
                speed_min: 2.0,
                speed_max: 5.0,
                angle_min: 0.0,
                angle_max: 360.0,
                size: 0.08,
                color_start: [0.7, 0.0, 0.0, 1.0],
                color_end: [0.4, 0.0, 0.0, 0.0],
                gravity: 1.0,
                lifetime: 0.4,
            },
            EffectType::FireBurst => Self::fire_burst(),
            EffectType::IceShatter => Self::ice_shatter(),
            EffectType::LightningStrike => ParticleProfile {
                count: 12,
                speed_min: 8.0,
                speed_max: 15.0,
                angle_min: 250.0,
                angle_max: 290.0,
                size: 0.05,
                color_start: [1.0, 1.0, 0.8, 1.0],
                color_end: [0.8, 0.8, 1.0, 0.0],
                gravity: 0.0,
                lifetime: 0.3,
            },
        }
    }
}

// ── ActiveEffect ──

/// A running visual effect instance tracked by the effect system.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveEffect {
    pub id: u32,
    pub effect_type: EffectType,
    pub params: EffectParams,
    pub elapsed: f32,
    pub duration: f32,
}

impl ActiveEffect {
    pub fn new(id: u32, effect_type: EffectType, params: EffectParams) -> Self {
        ActiveEffect {
            id,
            effect_type,
            params,
            elapsed: 0.0,
            duration: effect_type.duration(),
        }
    }

    /// Advance elapsed time by `delta` seconds. Returns `true` if still alive.
    pub fn tick(&mut self, delta: f32) -> bool {
        self.elapsed += delta;
        self.elapsed < self.duration
    }

    /// Playback progress from 0.0 (start) to 1.0 (complete).
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).min(1.0)
    }
}

// ── EffectSystem ──

/// Manages all active visual effects in the game world.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSystem {
    pub active_effects: HashMap<u32, ActiveEffect>,
    pub next_id: u32,
    pub max_effects: usize,
}

impl EffectSystem {
    pub fn new() -> Self {
        EffectSystem {
            active_effects: HashMap::new(),
            next_id: 1,
            max_effects: 200,
        }
    }

    /// Spawn a new effect. If at capacity, the oldest effect is evicted.
    /// Returns the assigned effect id.
    pub fn spawn(&mut self, effect_type: EffectType, params: EffectParams) -> u32 {
        if self.active_effects.len() >= self.max_effects {
            let oldest_id = *self
                .active_effects
                .keys()
                .min()
                .expect("active_effects non-empty at max_effects");
            self.active_effects.remove(&oldest_id);
        }

        let id = self.next_id;
        self.next_id += 1;
        let effect = ActiveEffect::new(id, effect_type, params);
        self.active_effects.insert(id, effect);
        id
    }

    /// Tick all active effects, removing those that have finished.
    /// Returns the number of effects removed.
    pub fn update(&mut self, delta: f32) -> usize {
        let dead: Vec<u32> = self
            .active_effects
            .iter_mut()
            .filter_map(|(id, effect)| {
                if !effect.tick(delta) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        let removed = dead.len();
        for id in &dead {
            self.active_effects.remove(id);
        }
        removed
    }

    /// Look up an active effect by id.
    pub fn get(&self, id: u32) -> Option<&ActiveEffect> {
        self.active_effects.get(&id)
    }

    /// Cancel and remove a specific effect. Returns the removed effect if it existed.
    pub fn cancel(&mut self, id: u32) -> Option<ActiveEffect> {
        self.active_effects.remove(&id)
    }

    /// Number of currently active effects.
    pub fn active_count(&self) -> usize {
        self.active_effects.len()
    }

    /// Return all active effects within `radius` of the given point.
    pub fn effects_at(&self, x: f32, y: f32, z: f32, radius: f32) -> Vec<&ActiveEffect> {
        let r2 = radius * radius;
        self.active_effects
            .values()
            .filter(|e| {
                let dx = e.params.x - x;
                let dy = e.params.y - y;
                let dz = e.params.z - z;
                dx * dx + dy * dy + dz * dz <= r2
            })
            .collect()
    }

    /// Remove all active effects.
    pub fn clear_all(&mut self) {
        self.active_effects.clear();
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_type_properties() {
        assert_eq!(EffectType::MeleeHit.name(), "Melee Hit");
        assert_eq!(EffectType::RangedHit.name(), "Ranged Hit");
        assert_eq!(EffectType::MagicHit.name(), "Magic Hit");
        assert_eq!(EffectType::Heal.name(), "Heal");
        assert_eq!(EffectType::LevelUp.name(), "Level Up");
        assert_eq!(EffectType::Death.name(), "Death");
        assert_eq!(EffectType::LootDrop.name(), "Loot Drop");
        assert_eq!(EffectType::GoldPickup.name(), "Gold Pickup");
        assert_eq!(EffectType::ShieldBreak.name(), "Shield Break");
        assert_eq!(EffectType::ManaRegen.name(), "Mana Regen");
        assert_eq!(EffectType::XpGain.name(), "XP Gain");
        assert_eq!(EffectType::DustTrail.name(), "Dust Trail");
        assert_eq!(EffectType::BloodSplatter.name(), "Blood Splatter");
        assert_eq!(EffectType::FireBurst.name(), "Fire Burst");
        assert_eq!(EffectType::IceShatter.name(), "Ice Shatter");
        assert_eq!(EffectType::LightningStrike.name(), "Lightning Strike");

        assert!((EffectType::MeleeHit.duration() - 0.3).abs() < f32::EPSILON);
        assert!((EffectType::RangedHit.duration() - 0.2).abs() < f32::EPSILON);
        assert!((EffectType::MagicHit.duration() - 0.5).abs() < f32::EPSILON);
        assert!((EffectType::Heal.duration() - 0.6).abs() < f32::EPSILON);
        assert!((EffectType::LevelUp.duration() - 1.5).abs() < f32::EPSILON);
        assert!((EffectType::Death.duration() - 1.0).abs() < f32::EPSILON);
        assert!((EffectType::LootDrop.duration() - 0.4).abs() < f32::EPSILON);
        assert!((EffectType::GoldPickup.duration() - 0.3).abs() < f32::EPSILON);
        assert!((EffectType::ShieldBreak.duration() - 0.5).abs() < f32::EPSILON);
        assert!((EffectType::ManaRegen.duration() - 0.4).abs() < f32::EPSILON);
        assert!((EffectType::XpGain.duration() - 0.5).abs() < f32::EPSILON);
        assert!((EffectType::DustTrail.duration() - 0.8).abs() < f32::EPSILON);
        assert!((EffectType::BloodSplatter.duration() - 0.4).abs() < f32::EPSILON);
        assert!((EffectType::FireBurst.duration() - 0.6).abs() < f32::EPSILON);
        assert!((EffectType::IceShatter.duration() - 0.5).abs() < f32::EPSILON);
        assert!((EffectType::LightningStrike.duration() - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn effect_params_builder() {
        let p = EffectParams::new(1.0, 2.0, 3.0);
        assert!((p.x - 1.0).abs() < f32::EPSILON);
        assert!((p.y - 2.0).abs() < f32::EPSILON);
        assert!((p.z - 3.0).abs() < f32::EPSILON);
        assert!((p.scale - 1.0).abs() < f32::EPSILON);
        assert!((p.intensity - 1.0).abs() < f32::EPSILON);
        assert!(p.color_override.is_none());

        let p = p.with_scale(2.5).with_intensity(0.7).with_color([1.0, 0.0, 0.0, 1.0]);
        assert!((p.scale - 2.5).abs() < f32::EPSILON);
        assert!((p.intensity - 0.7).abs() < f32::EPSILON);
        assert_eq!(p.color_override, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn particle_profile_scaled() {
        let profile = ParticleProfile {
            count: 10,
            speed_min: 1.0,
            speed_max: 3.0,
            angle_min: 0.0,
            angle_max: 360.0,
            size: 0.2,
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [0.0, 0.0, 0.0, 0.0],
            gravity: 0.0,
            lifetime: 0.5,
        };
        let scaled = profile.scaled(2.0);
        assert_eq!(scaled.count, 20);
        assert!((scaled.size - 0.4).abs() < f32::EPSILON);
        // Non-scaled fields unchanged
        assert!((scaled.speed_min - 1.0).abs() < f32::EPSILON);
        assert!((scaled.gravity - 0.0).abs() < f32::EPSILON);
        assert!((scaled.lifetime - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn effect_preset_melee_hit() {
        let p = EffectPreset::melee_hit();
        assert_eq!(p.count, 15);
        assert!(p.speed_min > 0.0);
        assert!(p.speed_max > p.speed_min);
        assert!((p.lifetime - 0.3).abs() < f32::EPSILON);
        // Orange start
        assert!(p.color_start[0] > 0.5 && p.color_start[1] > 0.0);
    }

    #[test]
    fn effect_preset_level_up() {
        let p = EffectPreset::level_up();
        assert_eq!(p.count, 50);
        assert!((p.lifetime - 1.5).abs() < f32::EPSILON);
        // Gold/white colors
        assert!(p.color_start[0] > 0.8 && p.color_start[1] > 0.7);
        // Full radial
        assert!((p.angle_min).abs() < f32::EPSILON);
        assert!((p.angle_max - 360.0).abs() < f32::EPSILON);
    }

    #[test]
    fn effect_preset_get_all() {
        let all_types = [
            EffectType::MeleeHit,
            EffectType::RangedHit,
            EffectType::MagicHit,
            EffectType::Heal,
            EffectType::LevelUp,
            EffectType::Death,
            EffectType::LootDrop,
            EffectType::GoldPickup,
            EffectType::ShieldBreak,
            EffectType::ManaRegen,
            EffectType::XpGain,
            EffectType::DustTrail,
            EffectType::BloodSplatter,
            EffectType::FireBurst,
            EffectType::IceShatter,
            EffectType::LightningStrike,
        ];

        for et in &all_types {
            let p = EffectPreset::get(*et);
            assert!(p.count > 0, "count > 0 for {:?}", et);
            assert!(p.speed_max >= p.speed_min, "speed_max >= speed_min for {:?}", et);
            assert!(p.size > 0.0, "size > 0 for {:?}", et);
        }

        // Verify named presets match get()
        assert_eq!(EffectPreset::get(EffectType::MeleeHit), EffectPreset::melee_hit());
        assert_eq!(EffectPreset::get(EffectType::RangedHit), EffectPreset::ranged_hit());
        assert_eq!(EffectPreset::get(EffectType::MagicHit), EffectPreset::magic_hit());
        assert_eq!(EffectPreset::get(EffectType::Heal), EffectPreset::heal());
        assert_eq!(EffectPreset::get(EffectType::LevelUp), EffectPreset::level_up());
        assert_eq!(EffectPreset::get(EffectType::Death), EffectPreset::death());
        assert_eq!(EffectPreset::get(EffectType::LootDrop), EffectPreset::loot_drop());
        assert_eq!(EffectPreset::get(EffectType::DustTrail), EffectPreset::dust_trail());
        assert_eq!(EffectPreset::get(EffectType::FireBurst), EffectPreset::fire_burst());
        assert_eq!(EffectPreset::get(EffectType::IceShatter), EffectPreset::ice_shatter());
    }

    #[test]
    fn active_effect_tick_alive() {
        let mut e = ActiveEffect::new(1, EffectType::MeleeHit, EffectParams::new(0.0, 0.0, 0.0));
        assert_eq!(e.id, 1);
        assert_eq!(e.effect_type, EffectType::MeleeHit);
        assert!((e.elapsed).abs() < f32::EPSILON);
        assert!((e.duration - 0.3).abs() < f32::EPSILON);

        assert!(e.tick(0.1));  // elapsed=0.1 < 0.3 → alive
        assert!((e.elapsed - 0.1).abs() < f32::EPSILON);
        assert!(e.tick(0.1));  // elapsed=0.2 < 0.3 → alive
        assert!((e.elapsed - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn active_effect_tick_dies() {
        let mut e = ActiveEffect::new(1, EffectType::MeleeHit, EffectParams::new(0.0, 0.0, 0.0));
        assert!(e.tick(0.2));    // 0.2 < 0.3 → alive
        assert!(!e.tick(0.2));   // 0.4 >= 0.3 → dead
        assert!(!e.tick(0.1));   // stays dead
        assert!((e.progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn effect_system_spawn() {
        let mut sys = EffectSystem::new();
        let id = sys.spawn(EffectType::Heal, EffectParams::new(10.0, 20.0, 0.0));
        assert_eq!(id, 1);
        assert_eq!(sys.next_id, 2);
        assert_eq!(sys.active_count(), 1);

        let effect = sys.get(id).unwrap();
        assert_eq!(effect.effect_type, EffectType::Heal);
        assert!((effect.params.x - 10.0).abs() < f32::EPSILON);
        assert!((effect.params.y - 20.0).abs() < f32::EPSILON);

        let id2 = sys.spawn(EffectType::Death, EffectParams::new(5.0, 5.0, 1.0));
        assert_eq!(id2, 2);
        assert_eq!(sys.active_count(), 2);
    }

    #[test]
    fn effect_system_update_removes_dead() {
        let mut sys = EffectSystem::new();
        sys.spawn(EffectType::MeleeHit, EffectParams::new(0.0, 0.0, 0.0)); // duration 0.3
        sys.spawn(EffectType::LevelUp, EffectParams::new(1.0, 1.0, 0.0));  // duration 1.5
        assert_eq!(sys.active_count(), 2);

        // Tick past MeleeHit duration but not LevelUp
        let removed = sys.update(0.5);
        assert_eq!(removed, 1);
        assert_eq!(sys.active_count(), 1);
        assert!(sys.get(1).is_none()); // MeleeHit gone
        assert!(sys.get(2).is_some());  // LevelUp still alive
    }

    #[test]
    fn effect_system_cancel() {
        let mut sys = EffectSystem::new();
        let id = sys.spawn(EffectType::Heal, EffectParams::new(0.0, 0.0, 0.0));
        assert_eq!(sys.active_count(), 1);

        let removed = sys.cancel(id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().effect_type, EffectType::Heal);
        assert_eq!(sys.active_count(), 0);

        // Cancel non-existent returns None
        assert!(sys.cancel(999).is_none());
    }

    #[test]
    fn effect_system_max_effects() {
        let mut sys = EffectSystem::new();
        sys.max_effects = 3;

        let id1 = sys.spawn(EffectType::MeleeHit, EffectParams::new(0.0, 0.0, 0.0));
        let id2 = sys.spawn(EffectType::RangedHit, EffectParams::new(0.0, 0.0, 0.0));
        let id3 = sys.spawn(EffectType::MagicHit, EffectParams::new(0.0, 0.0, 0.0));
        assert_eq!(sys.active_count(), 3);

        // Spawning a 4th should evict id1 (oldest)
        let id4 = sys.spawn(EffectType::Heal, EffectParams::new(0.0, 0.0, 0.0));
        assert_eq!(sys.active_count(), 3);
        assert!(sys.get(id1).is_none()); // evicted
        assert!(sys.get(id2).is_some());
        assert!(sys.get(id3).is_some());
        assert!(sys.get(id4).is_some());
    }

    #[test]
    fn effect_system_effects_at() {
        let mut sys = EffectSystem::new();
        sys.spawn(EffectType::Heal, EffectParams::new(0.0, 0.0, 0.0));
        sys.spawn(EffectType::Death, EffectParams::new(10.0, 0.0, 0.0));
        sys.spawn(EffectType::FireBurst, EffectParams::new(3.0, 4.0, 0.0));

        // Radius 5 from origin: Heal (dist=0), FireBurst (dist=5)
        let nearby = sys.effects_at(0.0, 0.0, 0.0, 5.0);
        assert_eq!(nearby.len(), 2);

        // Radius 1 from origin: only Heal
        let close = sys.effects_at(0.0, 0.0, 0.0, 1.0);
        assert_eq!(close.len(), 1);
        assert_eq!(close[0].effect_type, EffectType::Heal);

        // Large radius gets all
        let all = sys.effects_at(0.0, 0.0, 0.0, 100.0);
        assert_eq!(all.len(), 3);
    }
}

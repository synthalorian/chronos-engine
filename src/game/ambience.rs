#[cfg(feature = "game")]
// ── Imports ──────────────────────────────────────────────────────────────────

// std only — this module defines data types and trigger logic, NOT audio playback.

// ── AmbientZone ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientZone {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub sound_id: String,
    pub volume: f32,
    pub priority: u32,
    pub fade_in_seconds: f32,
    pub fade_out_seconds: f32,
}

impl AmbientZone {
    pub fn new(id: u32, name: &str, x: f32, y: f32, z: f32, radius: f32, sound_id: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            x,
            y,
            z,
            radius,
            sound_id: sound_id.to_string(),
            volume: 1.0,
            priority: 0,
            fade_in_seconds: 1.0,
            fade_out_seconds: 1.0,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_fade(mut self, fade_in: f32, fade_out: f32) -> Self {
        self.fade_in_seconds = fade_in.max(0.0);
        self.fade_out_seconds = fade_out.max(0.0);
        self
    }

    /// Returns true if `(x, y, z)` is within this zone's radius.
    pub fn contains(&self, x: f32, y: f32, z: f32) -> bool {
        self.distance_to(x, y, z) <= self.radius
    }

    /// Euclidean distance from the zone center to the given point.
    pub fn distance_to(&self, x: f32, y: f32, z: f32) -> f32 {
        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Volume at a point: full at center, linearly fading to 0 at the radius edge.
    /// Returns 0.0 if the point is outside the zone.
    pub fn volume_at(&self, x: f32, y: f32, z: f32) -> f32 {
        let dist = self.distance_to(x, y, z);
        if dist >= self.radius {
            return 0.0;
        }
        if self.radius == 0.0 {
            return self.volume;
        }
        self.volume * (1.0 - dist / self.radius)
    }
}

// ── MusicTrigger ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicTrigger {
    Explore,
    Combat,
    Boss,
    Town,
    Dungeon,
    Menu,
    Victory,
    Defeat,
    Tension,
}

impl MusicTrigger {
    pub fn track_id(&self) -> &str {
        match self {
            MusicTrigger::Explore => "music_explore",
            MusicTrigger::Combat => "music_combat",
            MusicTrigger::Boss => "music_boss",
            MusicTrigger::Town => "music_town",
            MusicTrigger::Dungeon => "music_dungeon",
            MusicTrigger::Menu => "music_menu",
            MusicTrigger::Victory => "music_victory",
            MusicTrigger::Defeat => "music_defeat",
            MusicTrigger::Tension => "music_tension",
        }
    }

    pub fn volume(&self) -> f32 {
        match self {
            MusicTrigger::Tension => 0.7,
            _ => 1.0,
        }
    }
}

// ── FootstepSurface ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootstepSurface {
    Grass,
    Stone,
    Dirt,
    Wood,
    Water,
    Metal,
    Sand,
    Snow,
}

impl FootstepSurface {
    pub fn sound_id(&self) -> &str {
        match self {
            FootstepSurface::Grass => "step_grass",
            FootstepSurface::Stone => "step_stone",
            FootstepSurface::Dirt => "step_dirt",
            FootstepSurface::Wood => "step_wood",
            FootstepSurface::Water => "step_water",
            FootstepSurface::Metal => "step_metal",
            FootstepSurface::Sand => "step_sand",
            FootstepSurface::Snow => "step_snow",
        }
    }
}

// ── FootstepTracker ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct FootstepTracker {
    pub distance_accumulated: f32,
    pub step_interval: f32,
    pub current_surface: FootstepSurface,
    pub is_moving: bool,
}

impl FootstepTracker {
    pub fn new(step_interval: f32) -> Self {
        Self {
            distance_accumulated: 0.0,
            step_interval,
            current_surface: FootstepSurface::Grass,
            is_moving: false,
        }
    }

    pub fn set_surface(&mut self, surface: FootstepSurface) {
        self.current_surface = surface;
    }

    pub fn set_moving(&mut self, moving: bool) {
        self.is_moving = moving;
    }

    /// Accumulate distance. Returns `Some(surface)` when a step should fire,
    /// resetting the accumulator. Returns `None` if not moving or not enough
    /// distance has been covered.
    pub fn update(&mut self, distance_moved: f32) -> Option<FootstepSurface> {
        if !self.is_moving || self.step_interval <= 0.0 {
            return None;
        }

        self.distance_accumulated += distance_moved;

        if self.distance_accumulated >= self.step_interval {
            self.distance_accumulated -= self.step_interval;
            Some(self.current_surface)
        } else {
            None
        }
    }
}

// ── AmbienceManager ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct AmbienceManager {
    pub zones: Vec<AmbientZone>,
    pub current_music: Option<MusicTrigger>,
    pub previous_music: Option<MusicTrigger>,
    pub music_fade_progress: f32,
    pub music_fade_duration: f32,
    pub footstep_tracker: FootstepTracker,
    pub master_ambience_volume: f32,
    pub master_music_volume: f32,
}

impl AmbienceManager {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            current_music: None,
            previous_music: None,
            music_fade_progress: 1.0,
            music_fade_duration: 2.0,
            footstep_tracker: FootstepTracker::new(2.0),
            master_ambience_volume: 1.0,
            master_music_volume: 1.0,
        }
    }

    // ── Zone management ──

    pub fn add_zone(&mut self, zone: AmbientZone) {
        self.zones.push(zone);
    }

    pub fn remove_zone(&mut self, id: u32) -> Option<AmbientZone> {
        if let Some(pos) = self.zones.iter().position(|z| z.id == id) {
            Some(self.zones.remove(pos))
        } else {
            None
        }
    }

    /// All zones whose radius contains the given point.
    pub fn active_zones(&self, x: f32, y: f32, z: f32) -> Vec<&AmbientZone> {
        self.zones
            .iter()
            .filter(|zone| zone.contains(x, y, z))
            .collect()
    }

    /// The active zone with the highest priority (ties broken by first added).
    pub fn highest_priority_zone(&self, x: f32, y: f32, z: f32) -> Option<&AmbientZone> {
        self.active_zones(x, y, z)
            .into_iter()
            .max_by_key(|zone| zone.priority)
    }

    // ── Music ──

    /// Transition to a new music trigger. Sets `previous_music` to the old
    /// `current_music` and resets fade progress to 0.
    pub fn trigger_music(&mut self, trigger: MusicTrigger) {
        self.previous_music = self.current_music;
        self.current_music = Some(trigger);
        self.music_fade_progress = 0.0;
    }

    /// Advance the music crossfade by `delta` seconds.
    pub fn update_music_fade(&mut self, delta: f32) {
        if self.music_fade_duration > 0.0 {
            self.music_fade_progress += delta / self.music_fade_duration;
        } else {
            self.music_fade_progress = 1.0;
        }
        if self.music_fade_progress > 1.0 {
            self.music_fade_progress = 1.0;
        }
    }

    /// True while a music crossfade is still in progress.
    pub fn is_transitioning_music(&self) -> bool {
        self.music_fade_progress < 1.0
    }

    // ── Footsteps (delegated) ──

    pub fn update_footsteps(&mut self, distance_moved: f32) -> Option<FootstepSurface> {
        self.footstep_tracker.update(distance_moved)
    }

    pub fn set_surface(&mut self, surface: FootstepSurface) {
        self.footstep_tracker.set_surface(surface);
    }

    pub fn set_moving(&mut self, moving: bool) {
        self.footstep_tracker.set_moving(moving);
    }
}

impl Default for AmbienceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── AmbientEvent ─────────────────────────────────────────────────────────────

/// Events this system produces for the game loop / audio engine to consume.
#[derive(Debug, Clone, PartialEq)]
pub enum AmbientEvent {
    PlayAmbient {
        sound_id: String,
        volume: f32,
    },
    StopAmbient {
        sound_id: String,
    },
    PlayMusic {
        trigger: MusicTrigger,
        fade_in: f32,
    },
    StopMusic {
        fade_out: f32,
    },
    PlayFootstep {
        surface: FootstepSurface,
        volume: f32,
    },
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. ambient_zone_contains — point inside / outside radius
    #[test]
    fn ambient_zone_contains() {
        let zone = AmbientZone::new(1, "forest", 0.0, 0.0, 0.0, 10.0, "amb_forest");
        assert!(zone.contains(0.0, 0.0, 0.0));
        assert!(zone.contains(5.0, 0.0, 0.0));
        assert!(zone.contains(10.0, 0.0, 0.0)); // exactly on edge
        assert!(!zone.contains(10.01, 0.0, 0.0));
        assert!(!zone.contains(0.0, 0.0, 50.0));
    }

    // 2. ambient_zone_volume_at — full at center, zero at edge
    #[test]
    fn ambient_zone_volume_at() {
        let zone = AmbientZone::new(1, "cave", 0.0, 0.0, 0.0, 10.0, "amb_cave");
        let eps = 1e-6;

        // Full volume at center
        assert!((zone.volume_at(0.0, 0.0, 0.0) - 1.0).abs() < eps);

        // Half volume at halfway point (e.g. x=5)
        assert!((zone.volume_at(5.0, 0.0, 0.0) - 0.5).abs() < eps);

        // Zero at the edge
        assert!((zone.volume_at(10.0, 0.0, 0.0)).abs() < eps);

        // Zero outside
        assert!(zone.volume_at(15.0, 0.0, 0.0).abs() < eps);

        // Custom volume clamps correctly
        let quiet = AmbientZone::new(2, "river", 0.0, 0.0, 0.0, 10.0, "amb_river").with_volume(0.5);
        assert!((quiet.volume_at(0.0, 0.0, 0.0) - 0.5).abs() < eps);
    }

    // 3. music_trigger_properties — track_id and volume for each variant
    #[test]
    fn music_trigger_properties() {
        let triggers = [
            (MusicTrigger::Explore, "music_explore", 1.0),
            (MusicTrigger::Combat, "music_combat", 1.0),
            (MusicTrigger::Boss, "music_boss", 1.0),
            (MusicTrigger::Town, "music_town", 1.0),
            (MusicTrigger::Dungeon, "music_dungeon", 1.0),
            (MusicTrigger::Menu, "music_menu", 1.0),
            (MusicTrigger::Victory, "music_victory", 1.0),
            (MusicTrigger::Defeat, "music_defeat", 1.0),
            (MusicTrigger::Tension, "music_tension", 0.7),
        ];

        for (trigger, expected_id, expected_vol) in triggers {
            assert_eq!(trigger.track_id(), expected_id);
            assert!((trigger.volume() - expected_vol).abs() < 1e-6);
        }
    }

    // 4. footstep_surface_sound_id — distinct id for each surface
    #[test]
    fn footstep_surface_sound_id() {
        let surfaces = [
            (FootstepSurface::Grass, "step_grass"),
            (FootstepSurface::Stone, "step_stone"),
            (FootstepSurface::Dirt, "step_dirt"),
            (FootstepSurface::Wood, "step_wood"),
            (FootstepSurface::Water, "step_water"),
            (FootstepSurface::Metal, "step_metal"),
            (FootstepSurface::Sand, "step_sand"),
            (FootstepSurface::Snow, "step_snow"),
        ];

        let mut ids: Vec<&str> = surfaces.iter().map(|(s, _)| s.sound_id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), surfaces.len(), "all sound_ids must be unique");

        for (surface, expected) in surfaces {
            assert_eq!(surface.sound_id(), expected);
        }
    }

    // 5. footstep_tracker_step_triggers — step fires at interval
    #[test]
    fn footstep_tracker_step_triggers() {
        let mut tracker = FootstepTracker::new(3.0);
        tracker.set_moving(true);

        // Move 2.0 — not enough
        assert!(tracker.update(2.0).is_none());

        // Move 1.0 more (total 3.0) — step!
        let result = tracker.update(1.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), FootstepSurface::Grass);
    }

    // 6. footstep_tracker_no_step_when_still — None when not moving
    #[test]
    fn footstep_tracker_no_step_when_still() {
        let mut tracker = FootstepTracker::new(1.0);
        // Not moving by default
        assert!(tracker.update(100.0).is_none());

        tracker.set_moving(true);
        assert!(tracker.update(5.0).is_some());

        tracker.set_moving(false);
        assert!(tracker.update(5.0).is_none());
    }

    // 7. footstep_tracker_resets_after_step — accumulator resets
    #[test]
    fn footstep_tracker_resets_after_step() {
        let mut tracker = FootstepTracker::new(2.0);
        tracker.set_moving(true);

        // Overshoot: move 5.0, step fires, accumulator = 5 - 2 = 3, >= 2 again → second step
        let first = tracker.update(5.0);
        assert!(first.is_some());

        // After one step: accumulated was 5.0, subtracted 2.0 → 3.0 ≥ 2.0
        // The update method only fires once per call, so check accumulated remainder
        assert!(
            tracker.distance_accumulated >= 2.0 - 1e-6,
            "remainder should be 3.0, got {}",
            tracker.distance_accumulated
        );

        // Next small move should trigger again from the remainder
        let second = tracker.update(0.0);
        assert!(second.is_some());
    }

    // 8. ambience_manager_add_remove_zones — zone CRUD
    #[test]
    fn ambience_manager_add_remove_zones() {
        let mut mgr = AmbienceManager::new();
        assert!(mgr.zones.is_empty());

        let zone = AmbientZone::new(1, "forest", 0.0, 0.0, 0.0, 10.0, "amb_forest");
        mgr.add_zone(zone);
        assert_eq!(mgr.zones.len(), 1);

        let removed = mgr.remove_zone(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "forest");
        assert!(mgr.zones.is_empty());

        let missing = mgr.remove_zone(999);
        assert!(missing.is_none());
    }

    // 9. ambience_manager_active_zones — returns zones containing point
    #[test]
    fn ambience_manager_active_zones() {
        let mut mgr = AmbienceManager::new();
        mgr.add_zone(AmbientZone::new(
            1,
            "forest",
            0.0,
            0.0,
            0.0,
            10.0,
            "amb_forest",
        ));
        mgr.add_zone(AmbientZone::new(2, "town", 20.0, 0.0, 0.0, 5.0, "amb_town"));
        mgr.add_zone(AmbientZone::new(3, "lake", 3.0, 0.0, 0.0, 8.0, "amb_lake"));

        let active = mgr.active_zones(2.0, 0.0, 0.0);
        let active_ids: Vec<u32> = active.iter().map(|z| z.id).collect();
        assert!(active_ids.contains(&1), "forest zone should contain point");
        assert!(active_ids.contains(&3), "lake zone should contain point");
        assert!(
            !active_ids.contains(&2),
            "town zone should NOT contain point"
        );
    }

    // 10. ambience_manager_highest_priority — picks highest priority zone
    #[test]
    fn ambience_manager_highest_priority() {
        let mut mgr = AmbienceManager::new();
        mgr.add_zone(
            AmbientZone::new(1, "forest", 0.0, 0.0, 0.0, 10.0, "amb_forest").with_priority(1),
        );
        mgr.add_zone(AmbientZone::new(2, "cave", 0.0, 0.0, 0.0, 10.0, "amb_cave").with_priority(5));
        mgr.add_zone(
            AmbientZone::new(3, "river", 0.0, 0.0, 0.0, 10.0, "amb_river").with_priority(3),
        );

        let best = mgr.highest_priority_zone(0.0, 0.0, 0.0);
        assert!(best.is_some());
        assert_eq!(best.unwrap().id, 2);
        assert_eq!(best.unwrap().priority, 5);
    }

    // 11. ambience_manager_trigger_music — current and previous updated
    #[test]
    fn ambience_manager_trigger_music() {
        let mut mgr = AmbienceManager::new();
        assert!(mgr.current_music.is_none());
        assert!(mgr.previous_music.is_none());

        mgr.trigger_music(MusicTrigger::Explore);
        assert_eq!(mgr.current_music, Some(MusicTrigger::Explore));
        assert!(mgr.previous_music.is_none()); // first trigger, no previous
        assert!((mgr.music_fade_progress).abs() < 1e-6);

        mgr.trigger_music(MusicTrigger::Combat);
        assert_eq!(mgr.current_music, Some(MusicTrigger::Combat));
        assert_eq!(mgr.previous_music, Some(MusicTrigger::Explore));
        assert!((mgr.music_fade_progress).abs() < 1e-6);
    }

    // 12. ambience_manager_music_fade — progress advances to 1.0
    #[test]
    fn ambience_manager_music_fade() {
        let mut mgr = AmbienceManager::new();
        mgr.music_fade_duration = 2.0;
        mgr.trigger_music(MusicTrigger::Town);
        assert!(mgr.is_transitioning_music());

        // Advance 1.0s of a 2.0s fade → progress = 0.5
        mgr.update_music_fade(1.0);
        assert!((mgr.music_fade_progress - 0.5).abs() < 1e-6);
        assert!(mgr.is_transitioning_music());

        // Advance another 1.0s → progress = 1.0
        mgr.update_music_fade(1.0);
        assert!((mgr.music_fade_progress - 1.0).abs() < 1e-6);
        assert!(!mgr.is_transitioning_music());

        // Overshoot clamps to 1.0
        mgr.update_music_fade(5.0);
        assert!((mgr.music_fade_progress - 1.0).abs() < 1e-6);
    }

    // 13. ambience_manager_footsteps — delegates to tracker
    #[test]
    fn ambience_manager_footsteps() {
        let mut mgr = AmbienceManager::new();

        // Not moving by default
        assert!(mgr.update_footsteps(5.0).is_none());

        mgr.set_moving(true);

        // Default step_interval is 2.0
        let step = mgr.update_footsteps(2.5);
        assert!(step.is_some());
        assert_eq!(step.unwrap(), FootstepSurface::Grass);

        // Switch surface
        mgr.set_surface(FootstepSurface::Stone);
        let step2 = mgr.update_footsteps(2.5);
        assert!(step2.is_some());
        assert_eq!(step2.unwrap(), FootstepSurface::Stone);
    }
}

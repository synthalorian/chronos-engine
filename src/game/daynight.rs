#[cfg(feature = "game")]
// ── TimePhase ────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePhase {
    Dawn,      // 5:00 - 7:00
    Morning,   // 7:00 - 12:00
    Noon,      // 12:00 - 14:00
    Afternoon, // 14:00 - 17:00
    Dusk,      // 17:00 - 19:00
    Night,     // 19:00 - 5:00
}

impl TimePhase {
    pub fn name(&self) -> &str {
        match self {
            TimePhase::Dawn => "Dawn",
            TimePhase::Morning => "Morning",
            TimePhase::Noon => "Noon",
            TimePhase::Afternoon => "Afternoon",
            TimePhase::Dusk => "Dusk",
            TimePhase::Night => "Night",
        }
    }

    pub fn from_hour(hour: f32) -> Self {
        if (5.0..7.0).contains(&hour) {
            TimePhase::Dawn
        } else if (7.0..12.0).contains(&hour) {
            TimePhase::Morning
        } else if (12.0..14.0).contains(&hour) {
            TimePhase::Noon
        } else if (14.0..17.0).contains(&hour) {
            TimePhase::Afternoon
        } else if (17.0..19.0).contains(&hour) {
            TimePhase::Dusk
        } else {
            TimePhase::Night
        }
    }
}

// ── LightingParams ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightingParams {
    pub ambient: [f32; 3],
    pub sun_direction: [f32; 3],
    pub sun_intensity: f32,
    pub fog_density: f32,
    pub sky_color: [f32; 3],
}

impl LightingParams {
    pub fn lerp(a: &LightingParams, b: &LightingParams, t: f32) -> LightingParams {
        let lerp_f32 = |v1: f32, v2: f32, t: f32| v1 + (v2 - v1) * t;
        let lerp_arr = |a: &[f32; 3], b: &[f32; 3], t: f32| -> [f32; 3] {
            [
                lerp_f32(a[0], b[0], t),
                lerp_f32(a[1], b[1], t),
                lerp_f32(a[2], b[2], t),
            ]
        };

        LightingParams {
            ambient: lerp_arr(&a.ambient, &b.ambient, t),
            sun_direction: lerp_arr(&a.sun_direction, &b.sun_direction, t),
            sun_intensity: lerp_f32(a.sun_intensity, b.sun_intensity, t),
            fog_density: lerp_f32(a.fog_density, b.fog_density, t),
            sky_color: lerp_arr(&a.sky_color, &b.sky_color, t),
        }
    }
}

// ── TimeOfDay ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeOfDay {
    pub hour: f32,
    pub day: u32,
}

impl TimeOfDay {
    pub fn new(hour: f32, day: u32) -> Self {
        TimeOfDay {
            hour: hour % 24.0,
            day,
        }
    }

    pub fn phase(&self) -> TimePhase {
        TimePhase::from_hour(self.hour)
    }

    pub fn is_daytime(&self) -> bool {
        self.hour >= 6.0 && self.hour < 18.0
    }

    pub fn is_nighttime(&self) -> bool {
        !self.is_daytime()
    }

    pub fn advance(&mut self, delta_hours: f32) {
        self.hour += delta_hours;
        while self.hour >= 24.0 {
            self.hour -= 24.0;
            self.day += 1;
        }
        while self.hour < 0.0 {
            self.hour += 24.0;
            self.day = self.day.saturating_sub(1);
        }
    }

    pub fn fraction_of_day(&self) -> f32 {
        self.hour / 24.0
    }
}

// ── DayNightCycle ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DayNightCycle {
    pub time: TimeOfDay,
    pub time_scale: f32,
    pub paused: bool,
    pub lighting: LightingParams,
}

impl DayNightCycle {
    pub fn new() -> Self {
        let time = TimeOfDay::new(8.0, 1);
        let lighting = Self::lighting_for_phase(time.phase());
        DayNightCycle {
            time,
            time_scale: 1.0,
            paused: false,
            lighting,
        }
    }

    pub fn with_time_scale(mut self, scale: f32) -> Self {
        self.time_scale = scale;
        self
    }

    pub fn with_starting_hour(mut self, hour: f32) -> Self {
        self.time.hour = hour % 24.0;
        self.lighting = Self::lighting_for_phase(self.time.phase());
        self
    }

    pub fn update(&mut self, delta_seconds: f32) {
        if !self.paused {
            self.time.advance(delta_seconds * self.time_scale);
            self.lighting = Self::lighting_for_phase(self.time.phase());
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_time(&mut self, hour: f32) {
        self.time.hour = hour % 24.0;
        self.lighting = Self::lighting_for_phase(self.time.phase());
    }

    pub fn phase(&self) -> TimePhase {
        self.time.phase()
    }

    pub fn lighting_for_phase(phase: TimePhase) -> LightingParams {
        match phase {
            TimePhase::Dawn => LightingParams {
                ambient: [0.4, 0.35, 0.5],
                sun_direction: [0.5, 0.3, 0.7],
                sun_intensity: 0.3,
                fog_density: 0.1,
                sky_color: [0.8, 0.5, 0.3],
            },
            TimePhase::Morning => LightingParams {
                ambient: [0.6, 0.6, 0.5],
                sun_direction: [0.3, 0.7, 0.5],
                sun_intensity: 0.6,
                fog_density: 0.05,
                sky_color: [0.6, 0.75, 0.95],
            },
            TimePhase::Noon => LightingParams {
                ambient: [0.8, 0.8, 0.7],
                sun_direction: [0.0, 1.0, 0.0],
                sun_intensity: 1.0,
                fog_density: 0.02,
                sky_color: [0.5, 0.7, 1.0],
            },
            TimePhase::Afternoon => LightingParams {
                ambient: [0.7, 0.65, 0.5],
                sun_direction: [0.4, 0.5, 0.3],
                sun_intensity: 0.7,
                fog_density: 0.05,
                sky_color: [0.7, 0.6, 0.5],
            },
            TimePhase::Dusk => LightingParams {
                ambient: [0.4, 0.3, 0.4],
                sun_direction: [0.6, 0.2, 0.5],
                sun_intensity: 0.3,
                fog_density: 0.15,
                sky_color: [0.9, 0.4, 0.2],
            },
            TimePhase::Night => LightingParams {
                ambient: [0.1, 0.1, 0.2],
                sun_direction: [0.0, -1.0, 0.0],
                sun_intensity: 0.05,
                fog_density: 0.3,
                sky_color: [0.05, 0.05, 0.15],
            },
        }
    }
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_phase_from_hour() {
        assert_eq!(TimePhase::from_hour(1.0), TimePhase::Night);
        assert_eq!(TimePhase::from_hour(5.5), TimePhase::Dawn);
        assert_eq!(TimePhase::from_hour(9.0), TimePhase::Morning);
        assert_eq!(TimePhase::from_hour(13.0), TimePhase::Noon);
        assert_eq!(TimePhase::from_hour(15.0), TimePhase::Afternoon);
        assert_eq!(TimePhase::from_hour(18.0), TimePhase::Dusk);
        assert_eq!(TimePhase::from_hour(22.0), TimePhase::Night);
        assert_eq!(TimePhase::from_hour(0.0), TimePhase::Night);
    }

    #[test]
    fn time_phase_boundary_cases() {
        assert_eq!(TimePhase::from_hour(5.0), TimePhase::Dawn);
        assert_eq!(TimePhase::from_hour(7.0), TimePhase::Morning);
        assert_eq!(TimePhase::from_hour(12.0), TimePhase::Noon);
        assert_eq!(TimePhase::from_hour(14.0), TimePhase::Afternoon);
        assert_eq!(TimePhase::from_hour(17.0), TimePhase::Dusk);
        assert_eq!(TimePhase::from_hour(19.0), TimePhase::Night);
        assert_eq!(TimePhase::from_hour(24.0), TimePhase::Night);
        assert_eq!(TimePhase::from_hour(23.999), TimePhase::Night);
    }

    #[test]
    fn time_of_day_advance() {
        let mut tod = TimeOfDay::new(23.0, 3);
        tod.advance(2.0);
        assert_eq!(tod.hour, 1.0);
        assert_eq!(tod.day, 4);

        // multi-day wrap
        let mut tod2 = TimeOfDay::new(22.0, 1);
        tod2.advance(50.0); // 22 + 50 = 72 => 72 / 24 = 3 days, 72 - 72 = 0
        assert_eq!(tod2.hour, 0.0);
        assert_eq!(tod2.day, 4);
    }

    #[test]
    fn time_of_day_is_daytime() {
        assert!(TimeOfDay::new(6.0, 1).is_daytime());
        assert!(TimeOfDay::new(12.0, 1).is_daytime());
        assert!(TimeOfDay::new(17.9, 1).is_daytime());
        assert!(!TimeOfDay::new(5.0, 1).is_daytime());
        assert!(!TimeOfDay::new(18.0, 1).is_daytime());
        assert!(!TimeOfDay::new(23.0, 1).is_daytime());
        assert!(!TimeOfDay::new(0.0, 1).is_daytime());
    }

    #[test]
    fn daynight_cycle_update() {
        let mut cycle = DayNightCycle::new().with_time_scale(2.0);
        let start_hour = cycle.time.hour;
        cycle.update(3.0);
        assert!((cycle.time.hour - (start_hour + 6.0)).abs() < f32::EPSILON);
        assert!(!cycle.paused);
    }

    #[test]
    fn daynight_cycle_pause() {
        let mut cycle = DayNightCycle::new();
        cycle.toggle_pause();
        assert!(cycle.paused);

        let hour_before = cycle.time.hour;
        cycle.update(10.0);
        assert!((cycle.time.hour - hour_before).abs() < f32::EPSILON);

        cycle.toggle_pause();
        assert!(!cycle.paused);
        cycle.update(1.0);
        assert!((cycle.time.hour - (hour_before + 1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn daynight_cycle_set_time() {
        let mut cycle = DayNightCycle::new();
        let day_before = cycle.time.day;
        cycle.set_time(22.0);
        assert!((cycle.time.hour - 22.0).abs() < f32::EPSILON);
        assert_eq!(cycle.time.day, day_before);
        assert_eq!(cycle.phase(), TimePhase::Night);
    }

    #[test]
    fn lighting_params_lerp() {
        let a = LightingParams {
            ambient: [0.0, 0.0, 0.0],
            sun_direction: [1.0, 0.0, 0.0],
            sun_intensity: 0.0,
            fog_density: 0.0,
            sky_color: [0.0, 0.0, 0.0],
        };
        let b = LightingParams {
            ambient: [1.0, 1.0, 1.0],
            sun_direction: [0.0, 1.0, 0.0],
            sun_intensity: 1.0,
            fog_density: 1.0,
            sky_color: [1.0, 1.0, 1.0],
        };

        let mid = LightingParams::lerp(&a, &b, 0.5);
        assert!((mid.ambient[0] - 0.5).abs() < 1e-6);
        assert!((mid.sun_direction[1] - 0.5).abs() < 1e-6);
        assert!((mid.sun_intensity - 0.5).abs() < 1e-6);
        assert!((mid.fog_density - 0.5).abs() < 1e-6);
        assert!((mid.sky_color[2] - 0.5).abs() < 1e-6);

        let at_zero = LightingParams::lerp(&a, &b, 0.0);
        assert_eq!(at_zero, a);

        let at_one = LightingParams::lerp(&a, &b, 1.0);
        assert_eq!(at_one, b);
    }

    #[test]
    fn lighting_for_phase() {
        let phases = [
            TimePhase::Dawn,
            TimePhase::Morning,
            TimePhase::Noon,
            TimePhase::Afternoon,
            TimePhase::Dusk,
            TimePhase::Night,
        ];

        for &phase in &phases {
            let params = DayNightCycle::lighting_for_phase(phase);
            assert!(params.sun_intensity >= 0.0 && params.sun_intensity <= 1.0);
            assert!(params.fog_density >= 0.0 && params.fog_density <= 1.0);
        }

        // Night should be darker than noon
        let night = DayNightCycle::lighting_for_phase(TimePhase::Night);
        let noon = DayNightCycle::lighting_for_phase(TimePhase::Noon);
        assert!(night.sun_intensity < noon.sun_intensity);
        assert!(night.fog_density > noon.fog_density);
    }

    #[test]
    fn fraction_of_day() {
        assert!((TimeOfDay::new(0.0, 1).fraction_of_day() - 0.0).abs() < f32::EPSILON);
        assert!((TimeOfDay::new(6.0, 1).fraction_of_day() - 0.25).abs() < f32::EPSILON);
        assert!((TimeOfDay::new(12.0, 1).fraction_of_day() - 0.5).abs() < f32::EPSILON);
        assert!((TimeOfDay::new(18.0, 1).fraction_of_day() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn default_impl_matches_new() {
        let from_new = DayNightCycle::new();
        let from_default = DayNightCycle::default();
        assert_eq!(from_new.time, from_default.time);
        assert_eq!(from_new.time_scale, from_default.time_scale);
        assert_eq!(from_new.paused, from_default.paused);
        assert_eq!(from_new.lighting, from_default.lighting);
    }
}

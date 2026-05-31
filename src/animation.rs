#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Animation state machine, blend trees, sprite animation, and timeline system.
//!
//! This module extends the skeletal animation layer (`skeletal.rs`) with:
//! - **State machines** that drive animation transitions via parameters
//! - **Blend trees** for 1D and 2D animation blending
//! - **Sprite animation** for frame-by-frame 2D animation
//! - **Timeline** for keyframed property animation with interpolation

// ──────────────────────────────────────────────
// Animation Parameters
// ──────────────────────────────────────────────

/// A named parameter that drives animation state machine transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimParam {
    /// A boolean parameter (e.g., "is_grounded", "is_attacking").
    Bool { name: String, value: bool },
    /// A float parameter (e.g., "speed", "health").
    Float { name: String, value: f32 },
    /// A trigger parameter that fires once and resets (e.g., "jump", "shoot").
    Trigger { name: String, triggered: bool },
    /// An integer parameter (e.g., "combo_count", "weapon_type").
    Int { name: String, value: i32 },
}

impl AnimParam {
    /// Returns the parameter name.
    pub fn name(&self) -> &str {
        match self {
            AnimParam::Bool { name, .. } => name,
            AnimParam::Float { name, .. } => name,
            AnimParam::Trigger { name, .. } => name,
            AnimParam::Int { name, .. } => name,
        }
    }
}

// ──────────────────────────────────────────────
// Animation Conditions
// ──────────────────────────────────────────────

/// A condition that must be satisfied for a transition to fire.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimCondition {
    /// True when the named bool parameter equals the given value.
    BoolEquals { param: String, value: bool },
    /// True when the named float parameter exceeds the threshold.
    FloatGreaterThan { param: String, threshold: f32 },
    /// True when the named float parameter is below the threshold.
    FloatLessThan { param: String, threshold: f32 },
    /// True when the named trigger parameter has been activated.
    TriggerActive { param: String },
    /// True when the named int parameter equals the given value.
    IntEquals { param: String, value: i32 },
}

impl AnimCondition {
    /// Evaluate this condition against a set of parameters.
    pub fn evaluate(&self, params: &[AnimParam]) -> bool {
        match self {
            AnimCondition::BoolEquals { param, value } => params.iter().any(|p| {
                if let AnimParam::Bool { name, value: v } = p {
                    name == param && v == value
                } else {
                    false
                }
            }),
            AnimCondition::FloatGreaterThan { param, threshold } => params.iter().any(|p| {
                if let AnimParam::Float { name, value } = p {
                    name == param && *value > *threshold
                } else {
                    false
                }
            }),
            AnimCondition::FloatLessThan { param, threshold } => params.iter().any(|p| {
                if let AnimParam::Float { name, value } = p {
                    name == param && *value < *threshold
                } else {
                    false
                }
            }),
            AnimCondition::TriggerActive { param } => params.iter().any(|p| {
                if let AnimParam::Trigger { name, triggered } = p {
                    name == param && *triggered
                } else {
                    false
                }
            }),
            AnimCondition::IntEquals { param, value } => params.iter().any(|p| {
                if let AnimParam::Int { name, value: v } = p {
                    name == param && v == value
                } else {
                    false
                }
            }),
        }
    }
}

// ──────────────────────────────────────────────
// Animation Transitions
// ──────────────────────────────────────────────

/// A transition from one state to another, gated by conditions.
#[derive(Debug, Clone)]
pub struct AnimTransition {
    /// Name of the target state to transition into.
    pub target_state: String,
    /// All conditions that must be true for this transition to fire.
    pub conditions: Vec<AnimCondition>,
    /// Crossfade blend duration in seconds.
    pub duration: f32,
    /// Whether this transition can be interrupted by a higher-priority transition.
    pub can_interrupt: bool,
}

impl AnimTransition {
    /// Create a new transition to the given target state.
    pub fn new(target: &str) -> Self {
        AnimTransition {
            target_state: target.to_string(),
            conditions: Vec::new(),
            duration: 0.2,
            can_interrupt: true,
        }
    }

    /// Add a condition to this transition.
    pub fn with_condition(mut self, condition: AnimCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the crossfade duration in seconds.
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Set whether this transition can be interrupted.
    pub fn with_interrupt(mut self, can_interrupt: bool) -> Self {
        self.can_interrupt = can_interrupt;
        self
    }

    /// Check if all conditions are met given the current parameters.
    pub fn all_conditions_met(&self, params: &[AnimParam]) -> bool {
        self.conditions.iter().all(|c| c.evaluate(params))
    }
}

// ──────────────────────────────────────────────
// Blend Trees
// ──────────────────────────────────────────────

/// How a blend tree interpolates between its children.
#[derive(Debug, Clone, PartialEq)]
pub enum BlendType {
    /// 1D blending along a single parameter (e.g., speed: idle → walk → run).
    Blend1D { parameter: String },
    /// 2D blending along two parameters (e.g., speed + direction).
    Blend2D { param_x: String, param_y: String },
}

/// A single child node in a blend tree, referencing an animation clip.
#[derive(Debug, Clone)]
pub struct BlendChild {
    /// Index into the animation clip array.
    pub clip_index: usize,
    /// Position in blend space. 1D uses index 0; 2D uses both.
    pub position: [f32; 2],
}

impl BlendChild {
    /// Create a 1D blend child at the given position.
    pub fn new_1d(clip_index: usize, position: f32) -> Self {
        BlendChild {
            clip_index,
            position: [position, 0.0],
        }
    }

    /// Create a 2D blend child at the given (x, y) position.
    pub fn new_2d(clip_index: usize, x: f32, y: f32) -> Self {
        BlendChild {
            clip_index,
            position: [x, y],
        }
    }
}

/// A blend tree that interpolates between multiple animation clips
/// based on one or two parameters.
#[derive(Debug, Clone)]
pub struct BlendTree {
    /// The blending mode (1D or 2D).
    pub blend_type: BlendType,
    /// Child nodes with their clip indices and blend-space positions.
    pub children: Vec<BlendChild>,
}

impl BlendTree {
    /// Create a new 1D blend tree driven by the given parameter.
    pub fn new_1d(parameter: &str) -> Self {
        BlendTree {
            blend_type: BlendType::Blend1D {
                parameter: parameter.to_string(),
            },
            children: Vec::new(),
        }
    }

    /// Create a new 2D blend tree driven by two parameters.
    pub fn new_2d(param_x: &str, param_y: &str) -> Self {
        BlendTree {
            blend_type: BlendType::Blend2D {
                param_x: param_x.to_string(),
                param_y: param_y.to_string(),
            },
            children: Vec::new(),
        }
    }

    /// Add a child node to the blend tree.
    pub fn add_child(&mut self, child: BlendChild) {
        self.children.push(child);
    }

    /// Sample the blend tree given the current parameters.
    ///
    /// Returns `(clip_index, weight)` — the clip to use and its blend weight.
    /// For 1D blending, finds the two nearest children and interpolates.
    pub fn sample(&self, params: &[AnimParam]) -> (usize, f32) {
        if self.children.is_empty() {
            return (0, 1.0);
        }
        if self.children.len() == 1 {
            return (self.children[0].clip_index, 1.0);
        }

        match &self.blend_type {
            BlendType::Blend1D { parameter } => {
                let value = params
                    .iter()
                    .find_map(|p| {
                        if let AnimParam::Float { name, value } = p {
                            if name == parameter {
                                Some(*value)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                // Sort children by position[0] to find neighbors
                let mut sorted: Vec<&BlendChild> = self.children.iter().collect();
                sorted.sort_by(|a, b| {
                    a.position[0]
                        .partial_cmp(&b.position[0])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Clamp to first/last
                if value <= sorted[0].position[0] {
                    return (sorted[0].clip_index, 1.0);
                }
                if value >= sorted[sorted.len() - 1].position[0] {
                    return (sorted[sorted.len() - 1].clip_index, 1.0);
                }

                // Find the two bracketing children
                for i in 0..sorted.len() - 1 {
                    let lo = sorted[i].position[0];
                    let hi = sorted[i + 1].position[0];
                    if value >= lo && value <= hi {
                        let range = hi - lo;
                        let t = if range.abs() < 1e-8 {
                            0.0
                        } else {
                            (value - lo) / range
                        };
                        // Return the lower child with weight (1-t), or upper with weight t
                        // We return the dominant clip and its weight
                        if t <= 0.5 {
                            return (sorted[i].clip_index, 1.0 - t);
                        } else {
                            return (sorted[i + 1].clip_index, t);
                        }
                    }
                }
                (sorted[0].clip_index, 1.0)
            }
            BlendType::Blend2D { param_x, param_y } => {
                let x = params
                    .iter()
                    .find_map(|p| {
                        if let AnimParam::Float { name, value } = p {
                            if name == param_x {
                                Some(*value)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                let y = params
                    .iter()
                    .find_map(|p| {
                        if let AnimParam::Float { name, value } = p {
                            if name == param_y {
                                Some(*value)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                // Find nearest child by distance
                let mut best_idx = 0;
                let mut best_dist = f32::MAX;
                for (i, child) in self.children.iter().enumerate() {
                    let dx = child.position[0] - x;
                    let dy = child.position[1] - y;
                    let dist = dx * dx + dy * dy;
                    if dist < best_dist {
                        best_dist = dist;
                        best_idx = i;
                    }
                }
                (self.children[best_idx].clip_index, 1.0)
            }
        }
    }
}

// ──────────────────────────────────────────────
// Animation States
// ──────────────────────────────────────────────

/// A single state within the animation state machine.
#[derive(Debug, Clone)]
pub struct AnimState {
    /// Human-readable state name (must be unique within the machine).
    pub name: String,
    /// Index into the animation clip array. `None` if this state uses a blend tree.
    pub clip_index: Option<usize>,
    /// Optional blend tree for 1D/2D blending. Used when `clip_index` is `None`.
    pub blend_tree: Option<BlendTree>,
    /// Outgoing transitions from this state.
    pub transitions: Vec<AnimTransition>,
    /// Playback speed multiplier (1.0 = normal speed).
    pub speed: f32,
    /// Whether the clip loops when it reaches the end.
    pub looping: bool,
}

impl AnimState {
    /// Create a new state with a single clip.
    pub fn new(name: &str, clip_index: usize) -> Self {
        AnimState {
            name: name.to_string(),
            clip_index: Some(clip_index),
            blend_tree: None,
            transitions: Vec::new(),
            speed: 1.0,
            looping: true,
        }
    }

    /// Create a new state driven by a blend tree (no single clip).
    pub fn new_blend(name: &str, blend_tree: BlendTree) -> Self {
        AnimState {
            name: name.to_string(),
            clip_index: None,
            blend_tree: Some(blend_tree),
            transitions: Vec::new(),
            speed: 1.0,
            looping: true,
        }
    }

    /// Add an outgoing transition.
    pub fn with_transition(mut self, transition: AnimTransition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Set the playback speed.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Set whether this state loops.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

// ──────────────────────────────────────────────
// State Machine Update Result
// ──────────────────────────────────────────────

/// The result of updating the state machine for one tick.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimStateUpdate {
    /// Currently playing a state at the given local time.
    Playing { state: String, time: f32 },
    /// Currently crossfading between two states.
    Transitioning {
        from: String,
        to: String,
        progress: f32,
    },
    /// The animation has finished (non-looping state reached the end).
    Finished,
}

// ──────────────────────────────────────────────
// Animation State Machine
// ──────────────────────────────────────────────

/// The core animation state machine that drives transitions based on parameters.
///
/// The machine maintains a set of named parameters and a list of states.
/// Each state has outgoing transitions with conditions. When conditions are met,
/// the machine crossfades to the target state over the transition's duration.
#[derive(Debug, Clone)]
pub struct AnimStateMachine {
    /// All states in the machine.
    pub states: Vec<AnimState>,
    /// Index of the currently active state.
    pub current_state_index: usize,
    /// Parameters that drive transition conditions.
    pub parameters: Vec<AnimParam>,
    /// Progress of the current crossfade transition (0.0 to 1.0).
    pub transition_progress: f32,
    /// The state index we're transitioning FROM (if crossfading).
    pub transitioning_from: Option<usize>,
    /// The duration of the current transition in seconds.
    transition_duration: f32,
    /// Local time within the current state's clip.
    pub local_time: f32,
    /// Duration of the current clip (for looping/finished detection).
    clip_duration: f32,
}

impl AnimStateMachine {
    /// Create a new, empty state machine.
    pub fn new() -> Self {
        AnimStateMachine {
            states: Vec::new(),
            current_state_index: 0,
            parameters: Vec::new(),
            transition_progress: 0.0,
            transitioning_from: None,
            transition_duration: 0.0,
            local_time: 0.0,
            clip_duration: 0.0,
        }
    }

    /// Add a state and return its index.
    pub fn add_state(&mut self, state: AnimState) -> usize {
        let idx = self.states.len();
        self.states.push(state);
        idx
    }

    /// Set a parameter by name. Updates the existing parameter or adds a new one.
    pub fn set_parameter(&mut self, name: &str, param: AnimParam) {
        if let Some(existing) = self.parameters.iter_mut().find(|p| p.name() == name) {
            *existing = param;
        } else {
            self.parameters.push(param);
        }
    }

    /// Get the current state.
    pub fn current_state(&self) -> &AnimState {
        &self.states[self.current_state_index]
    }

    /// Get the clip index for the current state, if it has one.
    pub fn current_clip_index(&self) -> Option<usize> {
        self.current_state().clip_index
    }

    /// Immediately force a transition to the named state (no crossfade).
    pub fn force_state(&mut self, name: &str) {
        if let Some(idx) = self.states.iter().position(|s| s.name == name) {
            self.current_state_index = idx;
            self.local_time = 0.0;
            self.transition_progress = 0.0;
            self.transitioning_from = None;
            self.transition_duration = 0.0;
        }
    }

    /// Reset the state machine to its initial state.
    pub fn reset(&mut self) {
        self.current_state_index = 0;
        self.local_time = 0.0;
        self.transition_progress = 0.0;
        self.transitioning_from = None;
        self.transition_duration = 0.0;
        self.clip_duration = 0.0;
        // Reset all triggers
        for param in &mut self.parameters {
            if let AnimParam::Trigger { triggered, .. } = param {
                *triggered = false;
            }
        }
    }

    /// Advance the state machine by `dt` seconds.
    ///
    /// Checks transitions, advances crossfades, and updates local time.
    pub fn update(&mut self, dt: f32) -> AnimStateUpdate {
        if self.states.is_empty() {
            return AnimStateUpdate::Finished;
        }

        let state = &self.states[self.current_state_index];

        // If we're mid-transition, advance the crossfade
        if self.transitioning_from.is_some() {
            let dur = if self.transition_duration > 0.0 {
                self.transition_duration
            } else {
                0.2
            };
            self.transition_progress += dt / dur;

            if self.transition_progress >= 1.0 {
                // Transition complete
                self.transition_progress = 0.0;
                self.transitioning_from = None;
                self.transition_duration = 0.0;
                self.local_time = 0.0;
            }

            // Re-borrow after transition completion
            let state = &self.states[self.current_state_index];
            let speed = state.speed;
            self.local_time += dt * speed;

            if let Some(from_idx) = self.transitioning_from {
                let from_name = self.states[from_idx].name.clone();
                let to_name = self.states[self.current_state_index].name.clone();
                return AnimStateUpdate::Transitioning {
                    from: from_name,
                    to: to_name,
                    progress: self.transition_progress.min(1.0),
                };
            }

            // Transition just completed this frame
            return AnimStateUpdate::Playing {
                state: state.name.clone(),
                time: self.local_time,
            };
        }

        // Not transitioning — check for new transitions
        let transitions: Vec<AnimTransition> = state.transitions.clone();
        for transition in &transitions {
            if transition.all_conditions_met(&self.parameters) {
                if let Some(target_idx) = self
                    .states
                    .iter()
                    .position(|s| s.name == transition.target_state)
                {
                    if target_idx != self.current_state_index {
                        // Begin transition
                        self.transitioning_from = Some(self.current_state_index);
                        self.current_state_index = target_idx;
                        self.transition_progress = 0.0;
                        self.transition_duration = transition.duration;
                        self.local_time = 0.0;

                        // Consume triggers
                        for param in &mut self.parameters {
                            if let AnimParam::Trigger { triggered, .. } = param {
                                *triggered = false;
                            }
                        }

                        let from_name = self.states[self.transitioning_from.expect("transitioning_from should be Some when in Transitioning state")]
                            .name
                            .clone();
                        let to_name = self.states[self.current_state_index].name.clone();
                        return AnimStateUpdate::Transitioning {
                            from: from_name,
                            to: to_name,
                            progress: 0.0,
                        };
                    }
                }
            }
        }

        // No transition — advance local time
        let speed = self.states[self.current_state_index].speed;
        self.local_time += dt * speed;

        let looping = self.states[self.current_state_index].looping;
        if self.clip_duration > 0.0 && self.local_time >= self.clip_duration {
            if looping {
                self.local_time -= self.clip_duration;
            } else {
                self.local_time = self.clip_duration;
                return AnimStateUpdate::Finished;
            }
        }

        AnimStateUpdate::Playing {
            state: self.states[self.current_state_index].name.clone(),
            time: self.local_time,
        }
    }

    /// Set the clip duration for finished/looping detection.
    pub fn set_clip_duration(&mut self, duration: f32) {
        self.clip_duration = duration;
    }
}

impl Default for AnimStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Sprite Animation
// ──────────────────────────────────────────────

/// A single frame in a sprite animation.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteFrame {
    /// Index into the sprite sheet / texture atlas.
    pub sprite_index: usize,
    /// How long this frame displays, in seconds.
    pub duration: f32,
}

impl SpriteFrame {
    /// Create a new sprite frame.
    pub fn new(sprite_index: usize, duration: f32) -> Self {
        SpriteFrame {
            sprite_index,
            duration,
        }
    }
}

/// Events emitted by sprite animation playback.
#[derive(Debug, Clone, PartialEq)]
pub enum SpriteAnimationEvent {
    /// The frame changed to the given index.
    FrameChanged(usize),
    /// The animation finished (non-looping).
    Finished,
    /// The animation looped back to the start.
    Looped,
}

/// A frame-by-frame sprite animation player.
#[derive(Debug, Clone)]
pub struct SpriteAnimation {
    /// All frames in this animation.
    pub frames: Vec<SpriteFrame>,
    /// Index of the currently displayed frame.
    pub current_frame: usize,
    /// How long the current frame has been displayed.
    pub frame_time: f32,
    /// Accumulated time since animation start.
    pub elapsed: f32,
    /// Whether the animation loops.
    pub looping: bool,
    /// Playback speed multiplier.
    pub speed: f32,
}

impl SpriteAnimation {
    /// Create a new sprite animation from a list of frames.
    pub fn new(frames: Vec<SpriteFrame>) -> Self {
        SpriteAnimation {
            frames,
            current_frame: 0,
            frame_time: 0.0,
            elapsed: 0.0,
            looping: true,
            speed: 1.0,
        }
    }

    /// Advance the animation by `dt` seconds.
    ///
    /// Returns `Some(&SpriteFrame)` with the current frame (which may have advanced).
    /// Returns `None` if the animation is finished (non-looping, past the last frame).
    pub fn update(&mut self, dt: f32) -> Option<&SpriteFrame> {
        if self.frames.is_empty() {
            return None;
        }

        self.frame_time += dt * self.speed;
        self.elapsed += dt * self.speed;

        let current_duration = self.frames[self.current_frame].duration;
        if self.frame_time >= current_duration {
            self.frame_time -= current_duration;
            self.current_frame += 1;

            if self.current_frame >= self.frames.len() {
                if self.looping {
                    self.current_frame = 0;
                } else {
                    self.current_frame = self.frames.len() - 1;
                    return None;
                }
            }
        }

        Some(&self.frames[self.current_frame])
    }

    /// Reset the animation to the first frame.
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.frame_time = 0.0;
        self.elapsed = 0.0;
    }

    /// Whether the animation has finished playing (non-looping only).
    pub fn is_finished(&self) -> bool {
        if self.looping || self.frames.is_empty() {
            return false;
        }
        self.current_frame == self.frames.len() - 1
            && self.frame_time > 0.0
            && self.elapsed >= self.total_duration()
    }

    /// Total duration of the animation in seconds.
    pub fn total_duration(&self) -> f32 {
        self.frames.iter().map(|f| f.duration).sum()
    }

    /// Set whether the animation loops.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Set the playback speed.
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

// ──────────────────────────────────────────────
// Timeline System
// ──────────────────────────────────────────────

/// Interpolation mode for keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interpolation {
    /// No interpolation — hold value until next keyframe.
    Step,
    /// Linear interpolation between keyframes.
    Linear,
    /// Smooth Hermite interpolation (ease in/out).
    Smoothstep,
}

/// The value stored in a keyframe.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyframeValue {
    /// A single float value.
    Float(f32),
    /// A 3D vector.
    Vec3([f32; 3]),
    /// An RGBA color.
    Color([f32; 4]),
    /// A named event trigger.
    Event(String),
}

/// A single keyframe on a timeline track.
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Time in seconds when this keyframe occurs.
    pub time: f32,
    /// The value at this keyframe.
    pub value: KeyframeValue,
    /// How to interpolate from the previous keyframe to this one.
    pub interpolation: Interpolation,
}

impl Keyframe {
    /// Create a new keyframe with linear interpolation.
    pub fn new(time: f32, value: KeyframeValue) -> Self {
        Keyframe {
            time,
            value,
            interpolation: Interpolation::Linear,
        }
    }

    /// Create a step keyframe (no interpolation).
    pub fn step(time: f32, value: KeyframeValue) -> Self {
        Keyframe {
            time,
            value,
            interpolation: Interpolation::Step,
        }
    }

    /// Create a smoothstep keyframe.
    pub fn smooth(time: f32, value: KeyframeValue) -> Self {
        Keyframe {
            time,
            value,
            interpolation: Interpolation::Smoothstep,
        }
    }
}

/// A single track on the timeline, containing keyframes for one property.
#[derive(Debug, Clone)]
pub struct TimelineTrack {
    /// Human-readable track name.
    pub name: String,
    /// Keyframes sorted by time.
    pub keyframes: Vec<Keyframe>,
}

impl TimelineTrack {
    /// Create a new, empty track.
    pub fn new(name: &str) -> Self {
        TimelineTrack {
            name: name.to_string(),
            keyframes: Vec::new(),
        }
    }

    /// Add a keyframe (inserts in time order).
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        let pos = self.keyframes.partition_point(|k| k.time < keyframe.time);
        self.keyframes.insert(pos, keyframe);
    }

    /// Sample the track at the given time, returning the interpolated value.
    pub fn sample(&self, time: f32) -> Option<KeyframeValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        if time <= self.keyframes[0].time {
            return Some(self.keyframes[0].value.clone());
        }

        if time >= self.keyframes[self.keyframes.len() - 1].time {
            return Some(self.keyframes[self.keyframes.len() - 1].value.clone());
        }

        // Find the segment we're in
        let idx = self.keyframes.partition_point(|k| k.time <= time);
        if idx == 0 {
            return Some(self.keyframes[0].value.clone());
        }

        let prev = &self.keyframes[idx - 1];
        let next = &self.keyframes[idx];
        let range = next.time - prev.time;
        let t = if range.abs() < 1e-8 {
            0.0
        } else {
            (time - prev.time) / range
        };

        match next.interpolation {
            Interpolation::Step => Some(prev.value.clone()),
            Interpolation::Linear => Some(interpolate_value(&prev.value, &next.value, t)),
            Interpolation::Smoothstep => {
                let s = smoothstep(t);
                Some(interpolate_value(&prev.value, &next.value, s))
            }
        }
    }
}

/// A sampled value from a timeline track at a given time.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSample {
    /// Name of the track this sample came from.
    pub track_name: String,
    /// The interpolated value.
    pub value: KeyframeValue,
}

/// The timeline drives keyframed property animation with multiple tracks.
#[derive(Debug, Clone)]
pub struct Timeline {
    /// All tracks on this timeline.
    pub tracks: Vec<TimelineTrack>,
    /// Total duration of the timeline in seconds.
    pub duration: f32,
    /// Current playback time.
    pub time: f32,
    /// Whether the timeline is currently playing.
    pub playing: bool,
    /// Whether the timeline loops when it reaches the end.
    pub looping: bool,
    /// Playback speed multiplier.
    pub speed: f32,
}

impl Timeline {
    /// Create a new timeline with the given total duration.
    pub fn new(duration: f32) -> Self {
        Timeline {
            tracks: Vec::new(),
            duration,
            time: 0.0,
            playing: false,
            looping: false,
            speed: 1.0,
        }
    }

    /// Add a track and return its index.
    pub fn add_track(&mut self, track: TimelineTrack) -> usize {
        let idx = self.tracks.len();
        self.tracks.push(track);
        idx
    }

    /// Advance the timeline by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !self.playing {
            return;
        }
        self.time += dt * self.speed;
        if self.time >= self.duration {
            if self.looping {
                self.time -= self.duration;
            } else {
                self.time = self.duration;
                self.playing = false;
            }
        }
    }

    /// Sample all tracks at the current time.
    pub fn sample(&self) -> Vec<TimelineSample> {
        self.tracks
            .iter()
            .filter_map(|track| {
                track.sample(self.time).map(|value| TimelineSample {
                    track_name: track.name.clone(),
                    value,
                })
            })
            .collect()
    }

    /// Seek to a specific time.
    pub fn seek(&mut self, time: f32) {
        self.time = time.clamp(0.0, self.duration);
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pause playback (time is preserved).
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Stop playback and reset time to zero.
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
    }

    /// Whether the timeline is currently playing.
    pub fn is_playing(&self) -> bool {
        self.playing
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Smoothstep interpolation (Hermite).
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Interpolate between two keyframe values.
fn interpolate_value(a: &KeyframeValue, b: &KeyframeValue, t: f32) -> KeyframeValue {
    match (a, b) {
        (KeyframeValue::Float(va), KeyframeValue::Float(vb)) => {
            KeyframeValue::Float(va + (vb - va) * t)
        }
        (KeyframeValue::Vec3(va), KeyframeValue::Vec3(vb)) => KeyframeValue::Vec3([
            va[0] + (vb[0] - va[0]) * t,
            va[1] + (vb[1] - va[1]) * t,
            va[2] + (vb[2] - va[2]) * t,
        ]),
        (KeyframeValue::Color(va), KeyframeValue::Color(vb)) => KeyframeValue::Color([
            va[0] + (vb[0] - va[0]) * t,
            va[1] + (vb[1] - va[1]) * t,
            va[2] + (vb[2] - va[2]) * t,
            va[3] + (vb[3] - va[3]) * t,
        ]),
        // For mismatched types or events, return the target at t > 0.5
        (_, vb) => {
            if t > 0.5 {
                vb.clone()
            } else {
                a.clone()
            }
        }
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- AnimParam Tests ---

    #[test]
    fn test_animparam_creation_and_matching() {
        let p1 = AnimParam::Bool {
            name: "alive".to_string(),
            value: true,
        };
        let p2 = AnimParam::Float {
            name: "speed".to_string(),
            value: 5.5,
        };
        let p3 = AnimParam::Trigger {
            name: "jump".to_string(),
            triggered: true,
        };
        let p4 = AnimParam::Int {
            name: "combo".to_string(),
            value: 3,
        };

        assert_eq!(p1.name(), "alive");
        assert_eq!(p2.name(), "speed");
        assert_eq!(p3.name(), "jump");
        assert_eq!(p4.name(), "combo");

        assert_eq!(
            p1,
            AnimParam::Bool {
                name: "alive".to_string(),
                value: true
            }
        );
    }

    // --- AnimCondition Tests ---

    #[test]
    fn test_condition_bool_equals() {
        let params = vec![AnimParam::Bool {
            name: "grounded".to_string(),
            value: true,
        }];
        let cond = AnimCondition::BoolEquals {
            param: "grounded".to_string(),
            value: true,
        };
        assert!(cond.evaluate(&params));

        let cond_false = AnimCondition::BoolEquals {
            param: "grounded".to_string(),
            value: false,
        };
        assert!(!cond_false.evaluate(&params));
    }

    #[test]
    fn test_condition_float_greater_than() {
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 3.5,
        }];
        let cond = AnimCondition::FloatGreaterThan {
            param: "speed".to_string(),
            threshold: 3.0,
        };
        assert!(cond.evaluate(&params));

        let cond_high = AnimCondition::FloatGreaterThan {
            param: "speed".to_string(),
            threshold: 4.0,
        };
        assert!(!cond_high.evaluate(&params));
    }

    #[test]
    fn test_condition_float_less_than() {
        let params = vec![AnimParam::Float {
            name: "health".to_string(),
            value: 25.0,
        }];
        let cond = AnimCondition::FloatLessThan {
            param: "health".to_string(),
            threshold: 50.0,
        };
        assert!(cond.evaluate(&params));

        let cond_low = AnimCondition::FloatLessThan {
            param: "health".to_string(),
            threshold: 10.0,
        };
        assert!(!cond_low.evaluate(&params));
    }

    #[test]
    fn test_condition_trigger_active() {
        let params = vec![AnimParam::Trigger {
            name: "shoot".to_string(),
            triggered: true,
        }];
        let cond = AnimCondition::TriggerActive {
            param: "shoot".to_string(),
        };
        assert!(cond.evaluate(&params));

        let params_off = vec![AnimParam::Trigger {
            name: "shoot".to_string(),
            triggered: false,
        }];
        assert!(!cond.evaluate(&params_off));
    }

    #[test]
    fn test_condition_int_equals() {
        let params = vec![AnimParam::Int {
            name: "weapon".to_string(),
            value: 2,
        }];
        let cond = AnimCondition::IntEquals {
            param: "weapon".to_string(),
            value: 2,
        };
        assert!(cond.evaluate(&params));

        let cond_wrong = AnimCondition::IntEquals {
            param: "weapon".to_string(),
            value: 5,
        };
        assert!(!cond_wrong.evaluate(&params));
    }

    // --- AnimTransition Tests ---

    #[test]
    fn test_transition_single_condition() {
        let params = vec![AnimParam::Bool {
            name: "grounded".to_string(),
            value: true,
        }];
        let transition = AnimTransition::new("idle").with_condition(AnimCondition::BoolEquals {
            param: "grounded".to_string(),
            value: true,
        });
        assert!(transition.all_conditions_met(&params));
    }

    #[test]
    fn test_transition_multiple_conditions() {
        let params = vec![
            AnimParam::Bool {
                name: "grounded".to_string(),
                value: true,
            },
            AnimParam::Float {
                name: "speed".to_string(),
                value: 5.0,
            },
        ];
        let transition = AnimTransition::new("run")
            .with_condition(AnimCondition::BoolEquals {
                param: "grounded".to_string(),
                value: true,
            })
            .with_condition(AnimCondition::FloatGreaterThan {
                param: "speed".to_string(),
                threshold: 3.0,
            });
        assert!(transition.all_conditions_met(&params));

        let params_slow = vec![
            AnimParam::Bool {
                name: "grounded".to_string(),
                value: true,
            },
            AnimParam::Float {
                name: "speed".to_string(),
                value: 1.0,
            },
        ];
        assert!(!transition.all_conditions_met(&params_slow));
    }

    // --- AnimState Tests ---

    #[test]
    fn test_anim_state_creation_and_transitions() {
        let state = AnimState::new("idle", 0)
            .with_transition(AnimTransition::new("walk").with_condition(
                AnimCondition::FloatGreaterThan {
                    param: "speed".to_string(),
                    threshold: 0.5,
                },
            ))
            .with_looping(true)
            .with_speed(1.0);

        assert_eq!(state.name, "idle");
        assert_eq!(state.clip_index, Some(0));
        assert_eq!(state.transitions.len(), 1);
        assert!(state.looping);
    }

    // --- AnimStateMachine Tests ---

    #[test]
    fn test_state_machine_bool_trigger_transition() {
        let mut sm = AnimStateMachine::new();
        sm.add_state(AnimState::new("idle", 0).with_transition(
            AnimTransition::new("jump").with_condition(AnimCondition::TriggerActive {
                param: "jump".to_string(),
            }),
        ));
        sm.add_state(AnimState::new("jump", 1));

        sm.set_parameter(
            "jump",
            AnimParam::Trigger {
                name: "jump".to_string(),
                triggered: true,
            },
        );

        assert_eq!(sm.current_state().name, "idle");
        let result = sm.update(0.016);
        assert!(
            matches!(result, AnimStateUpdate::Transitioning { ref from, ref to, .. } if from == "idle" && to == "jump")
        );
    }

    #[test]
    fn test_state_machine_float_threshold_transition() {
        let mut sm = AnimStateMachine::new();
        sm.add_state(AnimState::new("idle", 0).with_transition(
            AnimTransition::new("walk").with_condition(AnimCondition::FloatGreaterThan {
                param: "speed".to_string(),
                threshold: 0.5,
            }),
        ));
        sm.add_state(AnimState::new("walk", 1));

        // Speed below threshold — should stay idle
        sm.set_parameter(
            "speed",
            AnimParam::Float {
                name: "speed".to_string(),
                value: 0.2,
            },
        );
        let result = sm.update(0.016);
        assert!(matches!(result, AnimStateUpdate::Playing { ref state, .. } if state == "idle"));

        // Speed above threshold — should transition
        sm.set_parameter(
            "speed",
            AnimParam::Float {
                name: "speed".to_string(),
                value: 1.0,
            },
        );
        let result = sm.update(0.016);
        assert!(
            matches!(result, AnimStateUpdate::Transitioning { ref from, ref to, .. } if from == "idle" && to == "walk")
        );
    }

    #[test]
    fn test_state_machine_crossfade_progress() {
        let mut sm = AnimStateMachine::new();
        sm.add_state(
            AnimState::new("idle", 0).with_transition(
                AnimTransition::new("walk")
                    .with_condition(AnimCondition::BoolEquals {
                        param: "moving".to_string(),
                        value: true,
                    })
                    .with_duration(0.3),
            ),
        );
        sm.add_state(AnimState::new("walk", 1));

        sm.set_parameter(
            "moving",
            AnimParam::Bool {
                name: "moving".to_string(),
                value: true,
            },
        );
        sm.update(0.016); // Start transition

        // Advance partway through crossfade
        let result = sm.update(0.1);
        if let AnimStateUpdate::Transitioning { progress, .. } = result {
            assert!(
                progress > 0.0 && progress < 1.0,
                "progress should be between 0 and 1, got {}",
                progress
            );
        } else {
            panic!("Expected Transitioning, got {:?}", result);
        }

        // Advance past the end — transition should complete
        let _result = sm.update(0.3);
        // After transition completes, next update should be Playing
        let result2 = sm.update(0.016);
        assert!(matches!(result2, AnimStateUpdate::Playing { ref state, .. } if state == "walk"));
    }

    #[test]
    fn test_state_machine_force_state() {
        let mut sm = AnimStateMachine::new();
        sm.add_state(AnimState::new("idle", 0));
        sm.add_state(AnimState::new("run", 1));
        sm.add_state(AnimState::new("jump", 2));

        assert_eq!(sm.current_state().name, "idle");
        sm.force_state("jump");
        assert_eq!(sm.current_state().name, "jump");
        assert_eq!(sm.local_time, 0.0);
        assert_eq!(sm.transition_progress, 0.0);
    }

    #[test]
    fn test_state_machine_reset() {
        let mut sm = AnimStateMachine::new();
        sm.add_state(AnimState::new("idle", 0));
        sm.add_state(AnimState::new("run", 1));

        sm.set_parameter(
            "jump",
            AnimParam::Trigger {
                name: "jump".to_string(),
                triggered: true,
            },
        );
        sm.force_state("run");
        sm.local_time = 5.0;

        sm.reset();
        assert_eq!(sm.current_state_index, 0);
        assert_eq!(sm.local_time, 0.0);

        // Triggers should be reset
        for param in &sm.parameters {
            if let AnimParam::Trigger { triggered, .. } = param {
                assert!(!triggered);
            }
        }
    }

    // --- BlendTree Tests ---

    #[test]
    fn test_blend_tree_1d_sampling() {
        let mut tree = BlendTree::new_1d("speed");
        tree.add_child(BlendChild::new_1d(0, 0.0)); // idle at speed 0
        tree.add_child(BlendChild::new_1d(1, 3.0)); // walk at speed 3
        tree.add_child(BlendChild::new_1d(2, 6.0)); // run at speed 6

        // At speed 0, should get idle
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 0.0,
        }];
        let (clip, weight) = tree.sample(&params);
        assert_eq!(clip, 0);
        assert!((weight - 1.0).abs() < 0.01);

        // At speed 3, should get walk
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 3.0,
        }];
        let (clip, _weight) = tree.sample(&params);
        assert_eq!(clip, 1);

        // At speed 6, should get run
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 6.0,
        }];
        let (clip, _weight) = tree.sample(&params);
        assert_eq!(clip, 2);

        // At speed 1.5, between idle and walk — should lean idle
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 1.5,
        }];
        let (clip, _weight) = tree.sample(&params);
        assert_eq!(clip, 0); // t = 0.5, returns lower child

        // At speed 4.5, between walk and run — t=0.5, returns lower child (walk)
        let params = vec![AnimParam::Float {
            name: "speed".to_string(),
            value: 4.5,
        }];
        let (clip, _weight) = tree.sample(&params);
        assert_eq!(clip, 1); // t = 0.5, lower child (walk at index 1)
    }

    #[test]
    fn test_blend_tree_2d_sampling() {
        let mut tree = BlendTree::new_2d("speed", "angle");
        tree.add_child(BlendChild::new_2d(0, 0.0, 0.0)); // idle
        tree.add_child(BlendChild::new_2d(1, 1.0, 0.0)); // forward
        tree.add_child(BlendChild::new_2d(2, -1.0, 0.0)); // backward
        tree.add_child(BlendChild::new_2d(3, 0.0, 1.0)); // strafe right

        let params = vec![
            AnimParam::Float {
                name: "speed".to_string(),
                value: 1.0,
            },
            AnimParam::Float {
                name: "angle".to_string(),
                value: 0.0,
            },
        ];
        let (clip, _) = tree.sample(&params);
        assert_eq!(clip, 1); // Nearest to (1,0)

        let params = vec![
            AnimParam::Float {
                name: "speed".to_string(),
                value: 0.0,
            },
            AnimParam::Float {
                name: "angle".to_string(),
                value: 0.9,
            },
        ];
        let (clip, _) = tree.sample(&params);
        assert_eq!(clip, 3); // Nearest to (0,1)
    }

    // --- SpriteAnimation Tests ---

    #[test]
    fn test_sprite_animation_frame_advancement() {
        let frames = vec![
            SpriteFrame::new(0, 0.1),
            SpriteFrame::new(1, 0.1),
            SpriteFrame::new(2, 0.1),
        ];
        let mut anim = SpriteAnimation::new(frames);

        assert_eq!(anim.current_frame, 0);
        anim.update(0.1); // Advance past frame 0
        assert_eq!(anim.current_frame, 1);
        anim.update(0.1); // Advance past frame 1
        assert_eq!(anim.current_frame, 2);
    }

    #[test]
    fn test_sprite_animation_looping() {
        let frames = vec![SpriteFrame::new(0, 0.1), SpriteFrame::new(1, 0.1)];
        let mut anim = SpriteAnimation::new(frames).with_looping(true);

        anim.update(0.1); // frame 0 → 1
        anim.update(0.1); // frame 1 → loop back to 0
        assert_eq!(anim.current_frame, 0);
        assert!(anim.update(0.016).is_some()); // Still playing
    }

    #[test]
    fn test_sprite_animation_non_looping_finishes() {
        let frames = vec![SpriteFrame::new(0, 0.1), SpriteFrame::new(1, 0.1)];
        let mut anim = SpriteAnimation::new(frames).with_looping(false);

        let result = anim.update(0.1);
        assert!(result.is_some());
        assert_eq!(anim.current_frame, 1);

        let result = anim.update(0.1);
        assert!(result.is_none());
    }

    #[test]
    fn test_sprite_animation_speed_variation() {
        let frames = vec![SpriteFrame::new(0, 0.2), SpriteFrame::new(1, 0.2)];
        let mut anim = SpriteAnimation::new(frames).with_speed(2.0);

        anim.update(0.1); // At speed 2x, 0.1 * 2 = 0.2, which equals frame duration
        assert_eq!(anim.current_frame, 1);
    }

    #[test]
    fn test_sprite_animation_total_duration() {
        let frames = vec![
            SpriteFrame::new(0, 0.1),
            SpriteFrame::new(1, 0.2),
            SpriteFrame::new(2, 0.3),
        ];
        let anim = SpriteAnimation::new(frames);
        assert!((anim.total_duration() - 0.6).abs() < 0.001);
    }

    // --- Timeline Tests ---

    #[test]
    fn test_timeline_step_interpolation() {
        let mut track = TimelineTrack::new("opacity");
        track.add_keyframe(Keyframe::step(0.0, KeyframeValue::Float(0.0)));
        track.add_keyframe(Keyframe::step(0.5, KeyframeValue::Float(1.0)));

        // Before second keyframe, should hold first value
        let val = track.sample(0.25).unwrap();
        assert_eq!(val, KeyframeValue::Float(0.0));

        // At second keyframe, should hold second value
        let val = track.sample(0.5).unwrap();
        assert_eq!(val, KeyframeValue::Float(1.0));
    }

    #[test]
    fn test_timeline_linear_interpolation() {
        let mut track = TimelineTrack::new("position");
        track.add_keyframe(Keyframe::new(0.0, KeyframeValue::Float(0.0)));
        track.add_keyframe(Keyframe::new(1.0, KeyframeValue::Float(10.0)));

        let val = track.sample(0.5).unwrap();
        assert_eq!(val, KeyframeValue::Float(5.0));

        let val = track.sample(0.25).unwrap();
        assert_eq!(val, KeyframeValue::Float(2.5));
    }

    #[test]
    fn test_timeline_smoothstep_interpolation() {
        let mut track = TimelineTrack::new("scale");
        track.add_keyframe(Keyframe::smooth(0.0, KeyframeValue::Float(0.0)));
        track.add_keyframe(Keyframe::smooth(1.0, KeyframeValue::Float(1.0)));

        // At t=0.5, smoothstep(0.5) = 0.5, so value should be 0.5
        let val = track.sample(0.5).unwrap();
        if let KeyframeValue::Float(v) = val {
            assert!((v - 0.5).abs() < 0.01);
        }

        // At t=0.25, smoothstep(0.25) = 0.15625
        let val = track.sample(0.25).unwrap();
        if let KeyframeValue::Float(v) = val {
            assert!(v < 0.25, "smoothstep should ease in: got {}", v);
        }
    }

    #[test]
    fn test_timeline_vec3_interpolation() {
        let mut track = TimelineTrack::new("pos");
        track.add_keyframe(Keyframe::new(0.0, KeyframeValue::Vec3([0.0, 0.0, 0.0])));
        track.add_keyframe(Keyframe::new(1.0, KeyframeValue::Vec3([10.0, 20.0, 30.0])));

        let val = track.sample(0.5).unwrap();
        assert_eq!(val, KeyframeValue::Vec3([5.0, 10.0, 15.0]));
    }

    #[test]
    fn test_timeline_seek_and_play_pause_stop() {
        let mut tl = Timeline::new(5.0);
        tl.add_track(TimelineTrack::new("test"));

        assert!(!tl.is_playing());

        tl.play();
        assert!(tl.is_playing());

        tl.update(1.0);
        assert!((tl.time - 1.0).abs() < 0.001);

        tl.pause();
        assert!(!tl.is_playing());
        tl.update(1.0); // Should not advance
        assert!((tl.time - 1.0).abs() < 0.001);

        tl.seek(3.0);
        assert!((tl.time - 3.0).abs() < 0.001);

        tl.stop();
        assert!(!tl.is_playing());
        assert_eq!(tl.time, 0.0);
    }

    #[test]
    fn test_timeline_looping() {
        let mut tl = Timeline::new(2.0);
        tl.looping = true;
        tl.play();

        tl.update(2.5);
        assert!(
            (tl.time - 0.5).abs() < 0.001,
            "time should wrap to 0.5, got {}",
            tl.time
        );
        assert!(tl.is_playing());
    }

    #[test]
    fn test_timeline_non_looping_finishes() {
        let mut tl = Timeline::new(2.0);
        tl.looping = false;
        tl.play();

        tl.update(3.0);
        assert!(!tl.is_playing());
        assert!((tl.time - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_timeline_sample_all_tracks() {
        let mut tl = Timeline::new(2.0);

        let mut track_a = TimelineTrack::new("alpha");
        track_a.add_keyframe(Keyframe::new(0.0, KeyframeValue::Float(0.0)));
        track_a.add_keyframe(Keyframe::new(2.0, KeyframeValue::Float(1.0)));
        tl.add_track(track_a);

        let mut track_b = TimelineTrack::new("color");
        track_b.add_keyframe(Keyframe::new(
            0.0,
            KeyframeValue::Color([0.0, 0.0, 0.0, 1.0]),
        ));
        track_b.add_keyframe(Keyframe::new(
            2.0,
            KeyframeValue::Color([1.0, 1.0, 1.0, 1.0]),
        ));
        tl.add_track(track_b);

        tl.seek(1.0);
        let samples = tl.sample();
        assert_eq!(samples.len(), 2);
    }
}

//! Developer tools overlay for the Chronos Engine.
//!
//! Provides runtime inspection and debugging: entity inspector, stats panel,
//! dev console with commands, and a hierarchical scene tree. Feature-gated
//! behind `render` so it ships only when the renderer is included.

#[cfg(feature = "render")]
use crate::{
    component::{
        CircleRadius, Damage, Dead, Gravity, Grounded, Health, Position, RigidBody, Sprite,
        Transform, Velocity,
    },
    Entity, World,
};

use std::time::Instant;

// ──────────────────────────────────────────────
// Data types
// ──────────────────────────────────────────────

/// Severity level for console log messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// A single timestamped log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: f64,
    pub level: LogLevel,
    pub message: String,
}

/// Snapshot of engine performance stats.
#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub fps: f32,
    pub entity_count: usize,
    pub draw_calls: u32,
    pub frame_time_ms: f32,
}

/// Describes one component attached to an entity.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

/// Full inspection report for a single entity.
#[derive(Debug, Clone)]
pub struct InspectionReport {
    pub entity: Entity,
    pub components: Vec<ComponentInfo>,
}

/// One row in the scene tree.
#[derive(Debug, Clone)]
pub struct SceneEntry {
    pub entity: Entity,
    pub component_summary: String,
}

// ──────────────────────────────────────────────
// StatsPanel
// ──────────────────────────────────────────────

/// Tracks FPS and engine statistics via a rolling frame-time buffer.
#[derive(Debug)]
pub struct StatsPanel {
    frame_times: Vec<f32>,
    cursor: usize,
    sample_count: usize,
    capacity: usize,
    entity_count: usize,
    draw_calls: u32,
}

impl StatsPanel {
    const DEFAULT_CAPACITY: usize = 60;

    pub fn new() -> Self {
        StatsPanel {
            frame_times: vec![0.0; Self::DEFAULT_CAPACITY],
            cursor: 0,
            sample_count: 0,
            capacity: Self::DEFAULT_CAPACITY,
            entity_count: 0,
            draw_calls: 0,
        }
    }

    /// Record a new frame delta (in milliseconds).
    pub fn update_fps(&mut self, dt: f32) {
        self.frame_times[self.cursor] = dt;
        self.cursor = (self.cursor + 1) % self.capacity;
        if self.sample_count < self.capacity {
            self.sample_count += 1;
        }
    }

    pub fn set_entity_count(&mut self, count: usize) {
        self.entity_count = count;
    }

    pub fn set_draw_calls(&mut self, count: u32) {
        self.draw_calls = count;
    }

    pub fn get_stats(&self) -> Stats {
        let sum: f32 = self.frame_times[..self.sample_count].iter().copied().sum();
        let avg = if self.sample_count > 0 {
            sum / self.sample_count as f32
        } else {
            16.667 // assume ~60fps before data arrives
        };
        let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };
        Stats {
            fps,
            entity_count: self.entity_count,
            draw_calls: self.draw_calls,
            frame_time_ms: avg,
        }
    }
}

impl Default for StatsPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// DevConsole
// ──────────────────────────────────────────────

/// In-engine console with command parsing and a capped log buffer.
#[derive(Debug)]
pub struct DevConsole {
    log: Vec<LogEntry>,
    max_entries: usize,
    epoch: Instant,
    stats: Stats,
}

impl DevConsole {
    const DEFAULT_MAX_ENTRIES: usize = 1000;

    pub fn new() -> Self {
        DevConsole {
            log: Vec::new(),
            max_entries: Self::DEFAULT_MAX_ENTRIES,
            epoch: Instant::now(),
            stats: Stats {
                fps: 0.0,
                entity_count: 0,
                draw_calls: 0,
                frame_time_ms: 0.0,
            },
        }
    }

    /// Push a message into the log, evicting the oldest entry if at capacity.
    pub fn log(&mut self, message: &str) {
        self.log_with_level(LogLevel::Info, message);
    }

    pub fn log_with_level(&mut self, level: LogLevel, message: &str) {
        let elapsed = self.epoch.elapsed().as_secs_f64();
        if self.log.len() >= self.max_entries {
            self.log.remove(0);
        }
        self.log.push(LogEntry {
            timestamp: elapsed,
            level,
            message: message.to_owned(),
        });
    }

    /// Parse and execute a console command.
    pub fn submit(&mut self, input: &str) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }
        self.log_with_level(LogLevel::Info, &format!("> {}", trimmed));

        match trimmed {
            "help" => {
                self.log_with_level(LogLevel::Info, "Available commands: help, clear, entities, fps");
            }
            "clear" => {
                self.log.clear();
            }
            "entities" => {
                self.log_with_level(
                    LogLevel::Info,
                    &format!("Entity count: {}", self.stats.entity_count),
                );
            }
            "fps" => {
                self.log_with_level(
                    LogLevel::Info,
                    &format!(
                        "FPS: {:.1} | frame time: {:.2}ms",
                        self.stats.fps, self.stats.frame_time_ms
                    ),
                );
            }
            _ => {
                self.log_with_level(LogLevel::Warn, &format!("Unknown command: {}", trimmed));
            }
        }
    }

    pub fn get_log(&self) -> &[LogEntry] {
        &self.log
    }

    /// Filter log entries by level.
    pub fn get_log_filtered(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.log.iter().filter(|e| e.level == level).collect()
    }

    pub fn clear(&mut self) {
        self.log.clear();
    }

    /// Feed the console current stats so commands like `fps` and `entities` can report them.
    pub fn update_stats(&mut self, stats: Stats) {
        self.stats = stats;
    }
}

impl Default for DevConsole {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// EntityInspector
// ──────────────────────────────────────────────

/// Reflects all known component types on a single entity.
#[derive(Debug)]
pub struct EntityInspector {
    report: Option<InspectionReport>,
}

impl EntityInspector {
    pub fn new() -> Self {
        EntityInspector { report: None }
    }

    /// Scan `entity` for every known component type and build a report.
    pub fn inspect(&mut self, world: &World, entity: Entity) {
        let mut components = Vec::new();

        if world.has_component::<Position>(entity) {
            if let Some(p) = world.get_component::<Position>(entity) {
                components.push(ComponentInfo {
                    name: "Position".into(),
                    fields: vec![
                        ("x".into(), format!("{:.2}", p.x)),
                        ("y".into(), format!("{:.2}", p.y)),
                    ],
                });
            }
        }

        if world.has_component::<Velocity>(entity) {
            if let Some(v) = world.get_component::<Velocity>(entity) {
                components.push(ComponentInfo {
                    name: "Velocity".into(),
                    fields: vec![
                        ("x".into(), format!("{:.2}", v.x)),
                        ("y".into(), format!("{:.2}", v.y)),
                    ],
                });
            }
        }

        if world.has_component::<Health>(entity) {
            if let Some(h) = world.get_component::<Health>(entity) {
                components.push(ComponentInfo {
                    name: "Health".into(),
                    fields: vec![
                        ("current".into(), h.current.to_string()),
                        ("max".into(), h.max.to_string()),
                    ],
                });
            }
        }

        if world.has_component::<Damage>(entity) {
            if let Some(d) = world.get_component::<Damage>(entity) {
                components.push(ComponentInfo {
                    name: "Damage".into(),
                    fields: vec![("value".into(), d.0.to_string())],
                });
            }
        }

        if world.has_component::<Dead>(entity) {
            components.push(ComponentInfo {
                name: "Dead".into(),
                fields: vec![("marker".into(), "true".into())],
            });
        }

        if world.has_component::<Transform>(entity) {
            if let Some(t) = world.get_component::<Transform>(entity) {
                components.push(ComponentInfo {
                    name: "Transform".into(),
                    fields: vec![
                        ("x".into(), format!("{:.2}", t.x)),
                        ("y".into(), format!("{:.2}", t.y)),
                        ("z".into(), format!("{:.2}", t.z)),
                        ("rotation".into(), format!("{:.2}", t.rotation)),
                        ("scale".into(), format!("{:.2}", t.scale)),
                    ],
                });
            }
        }

        if world.has_component::<Sprite>(entity) {
            if let Some(s) = world.get_component::<Sprite>(entity) {
                components.push(ComponentInfo {
                    name: "Sprite".into(),
                    fields: vec![
                        ("symbol".into(), s.symbol.to_string()),
                        ("color_r".into(), s.color.0.to_string()),
                        ("color_g".into(), s.color.1.to_string()),
                        ("color_b".into(), s.color.2.to_string()),
                        ("layer".into(), s.layer.to_string()),
                    ],
                });
            }
        }

        if world.has_component::<CircleRadius>(entity) {
            if let Some(c) = world.get_component::<CircleRadius>(entity) {
                components.push(ComponentInfo {
                    name: "CircleRadius".into(),
                    fields: vec![("radius".into(), format!("{:.2}", c.0))],
                });
            }
        }

        if world.has_component::<RigidBody>(entity) {
            if let Some(rb) = world.get_component::<RigidBody>(entity) {
                components.push(ComponentInfo {
                    name: "RigidBody".into(),
                    fields: vec![
                        ("mass".into(), format!("{:.2}", rb.mass)),
                        ("damping".into(), format!("{:.2}", rb.damping)),
                        ("restitution".into(), format!("{:.2}", rb.restitution)),
                    ],
                });
            }
        }

        if world.has_component::<Grounded>(entity) {
            components.push(ComponentInfo {
                name: "Grounded".into(),
                fields: vec![("marker".into(), "true".into())],
            });
        }

        if world.has_component::<Gravity>(entity) {
            if let Some(g) = world.get_component::<Gravity>(entity) {
                components.push(ComponentInfo {
                    name: "Gravity".into(),
                    fields: vec![
                        ("x".into(), format!("{:.2}", g.x)),
                        ("y".into(), format!("{:.2}", g.y)),
                    ],
                });
            }
        }

        self.report = Some(InspectionReport { entity, components });
    }

    pub fn get_report(&self) -> Option<&InspectionReport> {
        self.report.as_ref()
    }
}

impl Default for EntityInspector {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// SceneTree
// ──────────────────────────────────────────────

/// Maintains a flat list of entities with a summary of their components.
#[derive(Debug)]
pub struct SceneTree {
    entries: Vec<SceneEntry>,
    selected: Option<usize>,
}

impl SceneTree {
    pub fn new() -> Self {
        SceneTree {
            entries: Vec::new(),
            selected: None,
        }
    }

    /// Rebuild the tree by scanning all living entities in `world`.
    pub fn rebuild(&mut self, world: &World) {
        self.entries.clear();
        self.selected = None;

        for i in 0..world.entity_capacity() {
            let entity = world.entity_from_index(i as u32);
            if !world.entity_exists(entity) {
                continue;
            }

            let summary = build_component_summary(world, entity);
            self.entries.push(SceneEntry {
                entity,
                component_summary: summary,
            });
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected = Some(index);
        }
    }

    pub fn get_selected(&self) -> Option<Entity> {
        self.selected.and_then(|i| self.entries.get(i).map(|e| e.entity))
    }

    pub fn entries(&self) -> &[SceneEntry] {
        &self.entries
    }
}

impl Default for SceneTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a comma-separated summary of which known components exist on `entity`.
fn build_component_summary(world: &World, entity: Entity) -> String {
    let mut names: Vec<&str> = Vec::new();

    macro_rules! check {
        ($ty:ty, $label:literal) => {
            if world.has_component::<$ty>(entity) {
                names.push($label);
            }
        };
    }

    check!(Position, "Position");
    check!(Velocity, "Velocity");
    check!(Health, "Health");
    check!(Damage, "Damage");
    check!(Dead, "Dead");
    check!(Transform, "Transform");
    check!(Sprite, "Sprite");
    check!(CircleRadius, "CircleRadius");
    check!(RigidBody, "RigidBody");
    check!(Grounded, "Grounded");
    check!(Gravity, "Gravity");

    if names.is_empty() {
        "(empty)".into()
    } else {
        names.join(", ")
    }
}

// ──────────────────────────────────────────────
// DevOverlay
// ──────────────────────────────────────────────

/// Top-level container that owns and drives all dev-tool subsystems.
#[derive(Debug)]
pub struct DevOverlay {
    pub entity_inspector: EntityInspector,
    pub stats_panel: StatsPanel,
    pub console: DevConsole,
    pub scene_tree: SceneTree,
    pub visible: bool,
    pub selected_entity: Option<Entity>,
}

impl DevOverlay {
    pub fn new() -> Self {
        DevOverlay {
            entity_inspector: EntityInspector::new(),
            stats_panel: StatsPanel::new(),
            console: DevConsole::new(),
            scene_tree: SceneTree::new(),
            visible: false,
            selected_entity: None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Tick all subsystems. `dt` is the frame delta in milliseconds.
    pub fn update(&mut self, world: &World, dt: f32) {
        self.stats_panel.update_fps(dt);
        self.stats_panel.set_entity_count(world.entity_count());

        let stats = self.stats_panel.get_stats();
        self.console.update_stats(stats);

        if let Some(entity) = self.selected_entity {
            if world.entity_exists(entity) {
                self.entity_inspector.inspect(world, entity);
            } else {
                self.selected_entity = None;
            }
        }
    }

    /// Return renderable data for the UI layer to draw. Returns `None` when hidden.
    pub fn render(&self) -> Option<OverlayRenderData<'_>> {
        if !self.visible {
            return None;
        }
        Some(OverlayRenderData {
            stats: self.stats_panel.get_stats(),
            inspection: self.entity_inspector.get_report(),
            log: self.console.get_log(),
            scene_entries: self.scene_tree.entries(),
            selected_entity: self.selected_entity,
        })
    }
}

impl Default for DevOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowed snapshot of everything the renderer needs to draw for one frame.
#[derive(Debug)]
pub struct OverlayRenderData<'a> {
    pub stats: Stats,
    pub inspection: Option<&'a InspectionReport>,
    pub log: &'a [LogEntry],
    pub scene_entries: &'a [SceneEntry],
    pub selected_entity: Option<Entity>,
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Position, Velocity, Health, Damage, Dead, Transform, Gravity};
    use crate::component::{Sprite, CircleRadius, RigidBody, Grounded};
    use crate::World;

    // ── Stats tracking ──

    #[test]
    fn stats_fps_tracking_averages_over_ring_buffer() {
        let mut panel = StatsPanel::new();
        // Feed 60 frames at exactly 16.667ms each (~60fps)
        for _ in 0..60 {
            panel.update_fps(16.667);
        }
        let stats = panel.get_stats();
        assert!((stats.fps - 60.0).abs() < 1.0, "FPS should be ~60, got {}", stats.fps);
        assert!((stats.frame_time_ms - 16.667).abs() < 0.5);
    }

    // ── Dev console ──

    #[test]
    fn console_log_submit_and_clear() {
        let mut con = DevConsole::new();
        con.log("hello");
        con.log("world");
        assert_eq!(con.get_log().len(), 2);

        con.submit("help");
        // submit logs the echo "> help" plus the help response = 2 extra entries
        assert_eq!(con.get_log().len(), 4);

        con.submit("clear");
        // clear itself: "> clear" echo, then the log is cleared
        assert!(con.get_log().is_empty());
    }

    #[test]
    fn console_unknown_command() {
        let mut con = DevConsole::new();
        con.submit("foobar");
        let log = con.get_log();
        assert_eq!(log[log.len() - 1].level, LogLevel::Warn);
        assert!(log[log.len() - 1].message.contains("Unknown command: foobar"));
    }

    #[test]
    fn console_drops_oldest_at_capacity() {
        let mut con = DevConsole::new();
        con.max_entries = 5;
        for i in 0..10 {
            con.log(&format!("msg {}", i));
        }
        assert_eq!(con.get_log().len(), 5);
        assert_eq!(con.get_log()[0].message, "msg 5");
    }

    // ── Scene tree ──

    #[test]
    fn scene_tree_rebuild_and_select() {
        let mut world = World::new();
        let e1 = world.create_entity();
        world.add_component(e1, Position::new(1.0, 2.0));
        world.add_component(e1, Velocity::new(0.5, -0.5));

        let e2 = world.create_entity();
        world.add_component(e2, Health::new(100));

        let mut tree = SceneTree::new();
        tree.rebuild(&world);

        assert_eq!(tree.entries().len(), 2);
        assert!(tree.entries()[0].component_summary.contains("Position"));
        assert!(tree.entries()[0].component_summary.contains("Velocity"));
        assert!(tree.entries()[1].component_summary.contains("Health"));

        tree.select(1);
        assert_eq!(tree.get_selected(), Some(e2));

        // Out-of-bounds select is a no-op
        tree.select(99);
        assert_eq!(tree.get_selected(), Some(e2)); // unchanged
    }

    // ── Entity inspector ──

    #[test]
    fn inspector_reads_all_component_fields() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(10.0, 20.0));
        world.add_component(entity, Health::new(75));
        world.add_component(entity, Damage(5));
        world.add_component(entity, Dead);
        world.add_component(entity, Gravity::new(0.0, 9.8));

        let mut inspector = EntityInspector::new();
        inspector.inspect(&world, entity);

        let report = inspector.get_report().expect("report should exist");
        assert_eq!(report.entity, entity);
        assert_eq!(report.components.len(), 5);

        let names: Vec<&str> = report.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Position"));
        assert!(names.contains(&"Health"));
        assert!(names.contains(&"Damage"));
        assert!(names.contains(&"Dead"));
        assert!(names.contains(&"Gravity"));

        // Verify field values
        let pos = report.components.iter().find(|c| c.name == "Position").unwrap();
        assert_eq!(pos.fields[0].0, "x");
        assert_eq!(pos.fields[0].1, "10.00");
    }

    // ── Log level filtering ──

    #[test]
    fn log_level_filter_returns_only_matching() {
        let mut con = DevConsole::new();
        con.log_with_level(LogLevel::Info, "info msg");
        con.log_with_level(LogLevel::Warn, "warn msg");
        con.log_with_level(LogLevel::Error, "error msg");
        con.log_with_level(LogLevel::Debug, "debug msg");

        let warns = con.get_log_filtered(LogLevel::Warn);
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].message, "warn msg");

        let infos = con.get_log_filtered(LogLevel::Info);
        assert_eq!(infos.len(), 1);
    }

    // ── Toggle visibility ──

    #[test]
    fn overlay_toggle_and_render_visibility() {
        let mut overlay = DevOverlay::new();
        assert!(!overlay.is_visible());
        assert!(overlay.render().is_none());

        overlay.toggle();
        assert!(overlay.is_visible());
        assert!(overlay.render().is_some());

        overlay.toggle();
        assert!(!overlay.is_visible());
    }

    // ── Inspector on nonexistent entity ──

    #[test]
    fn inspector_empty_entity_produces_empty_report() {
        let mut world = World::new();
        let entity = world.create_entity(); // no components

        let mut inspector = EntityInspector::new();
        inspector.inspect(&world, entity);

        let report = inspector.get_report().expect("report should exist");
        assert!(report.components.is_empty());
    }

    // ── Component summary for fully-loaded entity ──

    #[test]
    fn component_summary_includes_all_types() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position::new(0.0, 0.0));
        world.add_component(entity, Velocity::new(1.0, 1.0));
        world.add_component(entity, Health::new(50));
        world.add_component(entity, Damage(10));
        world.add_component(entity, Dead);
        world.add_component(entity, Transform::new(0.0, 0.0, 0.0));
        world.add_component(entity, Sprite::new('@', 255, 255, 255));
        world.add_component(entity, CircleRadius::new(1.0));
        world.add_component(entity, RigidBody::dynamic(1.0));
        world.add_component(entity, Grounded);
        world.add_component(entity, Gravity::down(9.8));

        let summary = build_component_summary(&world, entity);
        assert!(summary.contains("Position"));
        assert!(summary.contains("Velocity"));
        assert!(summary.contains("Health"));
        assert!(summary.contains("Damage"));
        assert!(summary.contains("Dead"));
        assert!(summary.contains("Transform"));
        assert!(summary.contains("Sprite"));
        assert!(summary.contains("CircleRadius"));
        assert!(summary.contains("RigidBody"));
        assert!(summary.contains("Grounded"));
        assert!(summary.contains("Gravity"));
    }
}

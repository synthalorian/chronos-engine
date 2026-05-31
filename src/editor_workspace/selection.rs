//! Viewport selection system for the Chronos Engine editor.
//!
//! Handles click-pick (closest entity within a radius), box-select
//! (all entities inside a screen-space rectangle), and hover highlight.

use crate::component::{Position, Transform};
use crate::entity::Entity;
use crate::world::World;

use super::{PickResult, SelectionRect};

// ──────────────────────────────────────────────
// ViewportSelector
// ──────────────────────────────────────────────

/// Manages viewport selection state — click-pick, box-select, and hover highlight.
pub struct ViewportSelector {
    /// Entity currently under the cursor (hover highlight).
    pub hovered_entity: Option<Entity>,
    /// Screen-space start position of an in-progress box select.
    pub box_select_start: Option<[f32; 2]>,
    /// Whether a box-select drag is active.
    pub box_selecting: bool,
    /// Pixel radius for click-pick tolerance.
    pub pick_radius: f32,
    /// Whether the pointer is in a dragging state.
    pub is_dragging: bool,
    /// Current mouse position (updated each frame during box select).
    current_mouse: [f32; 2],
}

impl ViewportSelector {
    /// Create a new selector with default settings.
    pub fn new() -> Self {
        ViewportSelector {
            hovered_entity: None,
            box_select_start: None,
            box_selecting: false,
            pick_radius: 5.0,
            is_dragging: false,
            current_mouse: [0.0, 0.0],
        }
    }

    // ── Generic pick ──────────────────────────

    /// Pick the closest entity to `screen_pos` using a caller-supplied projection.
    ///
    /// `project_fn` maps an entity to its screen-space position, returning `None`
    /// if the entity cannot be projected (e.g. off-screen or missing spatial data).
    pub fn pick(
        &self,
        screen_pos: [f32; 2],
        entities: &[Entity],
        project_fn: impl Fn(Entity) -> Option<[f32; 2]>,
    ) -> Option<PickResult> {
        let mut best: Option<(Entity, f32, [f32; 2])> = None;

        for &entity in entities {
            let projected = match project_fn(entity) {
                Some(pos) => pos,
                None => continue,
            };

            let dx = projected[0] - screen_pos[0];
            let dy = projected[1] - screen_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > self.pick_radius {
                continue;
            }

            let is_closer = best
                .as_ref()
                .is_none_or(|(_, best_dist, _)| dist < *best_dist);

            if is_closer {
                best = Some((entity, dist, projected));
            }
        }

        best.map(|(entity, distance, projected)| PickResult {
            entity,
            distance,
            world_pos: [projected[0], projected[1], 0.0],
        })
    }

    /// Return all entities whose projected screen position falls within `rect`.
    pub fn pick_box(
        &self,
        rect: &SelectionRect,
        entities: &[Entity],
        project_fn: impl Fn(Entity) -> Option<[f32; 2]>,
    ) -> Vec<Entity> {
        entities
            .iter()
            .filter_map(|&entity| {
                let pos = project_fn(entity)?;
                if rect.contains(pos[0], pos[1]) {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect()
    }

    // ── World-aware click picking ─────────────

    /// Click-pick against all entities in a `World` using 2D orthographic
    /// projection.
    ///
    /// The projection is: `screen = (world_pos - camera) * zoom`
    /// with the viewport center assumed to be `[0, 0]`.
    pub fn handle_click(
        &mut self,
        screen_pos: [f32; 2],
        world: &World,
        camera: [f32; 2],
        zoom: f32,
    ) -> Option<Entity> {
        let entities = world.all_entities();
        let mut best: Option<(Entity, f32)> = None;

        for entity in &entities {
            // Try Position first, fall back to Transform.
            let world_pos: [f32; 2] = if let Some(pos) = world.get_component::<Position>(*entity) {
                [pos.x, pos.y]
            } else if let Some(xform) = world.get_component::<Transform>(*entity) {
                [xform.x, xform.y]
            } else {
                continue;
            };

            let screen = world_to_screen_2d(world_pos, camera, zoom);
            let dx = screen[0] - screen_pos[0];
            let dy = screen[1] - screen_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > self.pick_radius {
                continue;
            }

            let is_closer = best.as_ref().is_none_or(|(_, d)| dist < *d);

            if is_closer {
                best = Some((*entity, dist));
            }
        }

        self.hovered_entity = best.as_ref().map(|(e, _)| *e);
        best.map(|(entity, _)| entity)
    }

    // ── Box-select state machine ──────────────

    /// Begin a box-select drag at the given screen position.
    pub fn begin_box_select(&mut self, screen_pos: [f32; 2]) {
        self.box_select_start = Some(screen_pos);
        self.current_mouse = screen_pos;
        self.box_selecting = true;
        self.is_dragging = true;
    }

    /// Update the box-select drag with the current mouse position.
    ///
    /// Returns the current `SelectionRect` if a box select is in progress.
    pub fn update_box_select(&mut self, screen_pos: [f32; 2]) -> Option<SelectionRect> {
        if !self.box_selecting {
            return None;
        }
        self.current_mouse = screen_pos;
        self.box_select_start
            .map(|start| SelectionRect::new(start[0], start[1], screen_pos[0], screen_pos[1]))
    }

    /// Finish the box-select drag and return the final `SelectionRect`.
    pub fn end_box_select(&mut self) -> Option<SelectionRect> {
        if !self.box_selecting {
            return None;
        }
        let rect = self.box_select_start.map(|start| {
            SelectionRect::new(
                start[0],
                start[1],
                self.current_mouse[0],
                self.current_mouse[1],
            )
        });
        self.box_selecting = false;
        self.is_dragging = false;
        self.box_select_start = None;
        rect
    }

    // ── Rendering ─────────────────────────────

    /// Draw the box-select rectangle using an egui painter.
    ///
    /// Only draws when a box-select is actively in progress.
    #[cfg(feature = "editor")]
    pub fn render_box_select(&self, painter: &egui::Painter) {
        if !self.box_selecting {
            return;
        }
        let start = match self.box_select_start {
            Some(s) => s,
            None => return,
        };

        let min = egui::Pos2::new(
            start[0].min(self.current_mouse[0]),
            start[1].min(self.current_mouse[1]),
        );
        let max = egui::Pos2::new(
            start[0].max(self.current_mouse[0]),
            start[1].max(self.current_mouse[1]),
        );
        let rect = egui::Rect::from_min_max(min, max);

        // Semi-transparent blue fill.
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(100, 149, 237, 50),
        );
        // Solid blue stroke.
        painter.rect_stroke(rect, 0.0, (1.0, egui::Color32::from_rgb(100, 149, 237)));
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Project a 2D world position to screen space using orthographic projection.
///
/// `screen = (world - camera) * zoom`
///
/// The viewport center is assumed to be the origin.
pub fn world_to_screen_2d(world: [f32; 2], camera: [f32; 2], zoom: f32) -> [f32; 2] {
    [(world[0] - camera[0]) * zoom, (world[1] - camera[1]) * zoom]
}

impl Default for ViewportSelector {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Position;

    // ── Helper ────────────────────────────────

    fn entity(id: u32) -> Entity {
        Entity::new(id, 0)
    }

    // 1. new() defaults
    #[test]
    fn new_selector_has_correct_defaults() {
        let sel = ViewportSelector::new();
        assert!(sel.hovered_entity.is_none());
        assert!(sel.box_select_start.is_none());
        assert!(!sel.box_selecting);
        assert!(!sel.is_dragging);
        assert!((sel.pick_radius - 5.0).abs() < f32::EPSILON);
    }

    // 2. pick closest entity
    #[test]
    fn pick_finds_closest_entity_within_radius() {
        let sel = ViewportSelector::new();
        let entities = vec![entity(0), entity(1), entity(2)];

        // Entity 0 at (10, 0), entity 1 at (20, 0), entity 2 at (100, 0).
        let project = |e: Entity| match e.index() {
            0 => Some([10.0_f32, 0.0]),
            1 => Some([20.0_f32, 0.0]),
            2 => Some([100.0_f32, 0.0]),
            _ => None,
        };

        let result = sel.pick([11.0, 0.0], &entities, project).unwrap();
        assert_eq!(result.entity, entity(0));
        assert!((result.distance - 1.0).abs() < f32::EPSILON);
    }

    // 3. pick no entity when too far
    #[test]
    fn pick_returns_none_when_all_entities_too_far() {
        let sel = ViewportSelector::new();
        let entities = vec![entity(0)];

        let project = |_: Entity| Some([1000.0_f32, 1000.0]);

        let result = sel.pick([0.0, 0.0], &entities, project);
        assert!(result.is_none());
    }

    // 4. pick_box multiple entities
    #[test]
    fn pick_box_finds_entities_inside_rect() {
        let sel = ViewportSelector::new();
        let entities = vec![entity(0), entity(1), entity(2)];

        let project = |e: Entity| match e.index() {
            0 => Some([5.0_f32, 5.0]),
            1 => Some([15.0_f32, 15.0]),
            2 => Some([50.0_f32, 50.0]),
            _ => None,
        };

        let rect = SelectionRect::new(0.0, 0.0, 20.0, 20.0);
        let found = sel.pick_box(&rect, &entities, project);
        assert_eq!(found.len(), 2);
        assert!(found.contains(&entity(0)));
        assert!(found.contains(&entity(1)));
    }

    // 5. pick_box empty rect
    #[test]
    fn pick_box_returns_empty_for_empty_rect() {
        let sel = ViewportSelector::new();
        let entities = vec![entity(0)];

        let project = |_: Entity| Some([100.0_f32, 100.0]);

        let rect = SelectionRect::new(0.0, 0.0, 1.0, 1.0);
        let found = sel.pick_box(&rect, &entities, project);
        assert!(found.is_empty());
    }

    // 6. begin/update/end box select flow
    #[test]
    fn box_select_full_flow() {
        let mut sel = ViewportSelector::new();

        // Begin
        sel.begin_box_select([10.0, 20.0]);
        assert!(sel.box_selecting);
        assert_eq!(sel.box_select_start, Some([10.0, 20.0]));

        // Update — move to (50, 60)
        let rect = sel.update_box_select([50.0, 60.0]).unwrap();
        assert!((rect.x_min - 10.0).abs() < f32::EPSILON);
        assert!((rect.y_min - 20.0).abs() < f32::EPSILON);
        assert!((rect.x_max - 50.0).abs() < f32::EPSILON);
        assert!((rect.y_max - 60.0).abs() < f32::EPSILON);

        // End
        let final_rect = sel.end_box_select().unwrap();
        assert!((final_rect.x_min - 10.0).abs() < f32::EPSILON);
        assert!(!sel.box_selecting);
        assert!(sel.box_select_start.is_none());
    }

    // 7. SelectionRect contains() method
    #[test]
    fn selection_rect_contains_point() {
        let rect = SelectionRect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains(50.0, 50.0));
        assert!(rect.contains(0.0, 0.0));
        assert!(rect.contains(100.0, 100.0));
        assert!(!rect.contains(-1.0, 50.0));
        assert!(!rect.contains(50.0, 101.0));
    }

    // 8. world_to_screen_2d projection
    #[test]
    fn world_to_screen_projection() {
        let screen = world_to_screen_2d([100.0, 200.0], [10.0, 20.0], 2.0);
        assert!((screen[0] - 180.0).abs() < f32::EPSILON);
        assert!((screen[1] - 360.0).abs() < f32::EPSILON);
    }

    // 9. handle_click with World integration
    #[test]
    fn handle_click_picks_entity_from_world() {
        let mut world = World::new();
        let e0 = world.create_entity();
        let e1 = world.create_entity();
        world.add_component(e0, Position::new(10.0, 10.0));
        world.add_component(e1, Position::new(500.0, 500.0));

        let mut sel = ViewportSelector::new();
        // camera at origin, zoom 1.0 → screen = world position
        let picked = sel.handle_click([11.0, 10.0], &world, [0.0, 0.0], 1.0);
        assert_eq!(picked, Some(e0));
        assert_eq!(sel.hovered_entity, Some(e0));
    }

    // 10. update/end box select when not selecting returns None
    #[test]
    fn box_select_noop_when_not_selecting() {
        let mut sel = ViewportSelector::new();
        assert!(sel.update_box_select([10.0, 10.0]).is_none());
        assert!(sel.end_box_select().is_none());
    }
}

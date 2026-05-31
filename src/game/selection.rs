use super::components::{Selectable, Selected};
use crate::component::Position;
#[cfg(feature = "game")]
use crate::{Entity, World};

#[derive(Debug, Clone, Copy)]
pub struct SelectionBox {
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
}

impl SelectionBox {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let min_x = self.start_x.min(self.end_x);
        let max_x = self.start_x.max(self.end_x);
        let min_y = self.start_y.min(self.end_y);
        let max_y = self.start_y.max(self.end_y);
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    pub fn width(&self) -> f32 {
        (self.end_x - self.start_x).abs()
    }

    pub fn height(&self) -> f32 {
        (self.end_y - self.start_y).abs()
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

#[derive(Debug, Clone)]
pub struct SelectionManager {
    pub selected_entities: Vec<Entity>,
    pub selection_box: Option<SelectionBox>,
    pub max_selection: usize,
}

impl SelectionManager {
    pub fn new() -> Self {
        SelectionManager {
            selected_entities: Vec::new(),
            selection_box: None,
            max_selection: 50,
        }
    }

    pub fn select_single(&mut self, entity: Entity, world: &mut World) {
        self.deselect_all(world);
        if !self.is_selected(entity) {
            world.add_component(entity, Selected);
            self.selected_entities.push(entity);
        }
    }

    pub fn select_entities(&mut self, entities: Vec<Entity>, world: &mut World) {
        self.deselect_all(world);
        for entity in entities {
            if self.selected_entities.len() >= self.max_selection {
                break;
            }
            if !self.is_selected(entity) {
                world.add_component(entity, Selected);
                self.selected_entities.push(entity);
            }
        }
    }

    pub fn deselect_all(&mut self, world: &mut World) {
        for entity in &self.selected_entities {
            world.remove_component::<Selected>(*entity);
        }
        self.selected_entities.clear();
    }

    pub fn begin_box_select(&mut self, x: f32, y: f32) {
        self.selection_box = Some(SelectionBox {
            start_x: x,
            start_y: y,
            end_x: x,
            end_y: y,
        });
    }

    pub fn update_box_select(&mut self, x: f32, y: f32) {
        if let Some(ref mut sb) = self.selection_box {
            sb.end_x = x;
            sb.end_y = y;
        }
    }

    pub fn finish_box_select(&mut self, world: &mut World) {
        let sb = match self.selection_box {
            Some(sb) => sb,
            None => return,
        };

        let positions: Vec<(Entity, Position)> =
            world.query::<Position>().map(|(e, p)| (e, *p)).collect();

        let mut matched = Vec::new();
        for (entity, pos) in &positions {
            if world.has_component::<Selectable>(*entity) && sb.contains(pos.x, pos.y) {
                matched.push(*entity);
            }
        }

        self.deselect_all(world);

        for entity in matched {
            if self.selected_entities.len() >= self.max_selection {
                break;
            }
            if !self.is_selected(entity) {
                world.add_component(entity, Selected);
                self.selected_entities.push(entity);
            }
        }

        self.selection_box = None;
    }

    pub fn toggle_select(&mut self, entity: Entity, world: &mut World) {
        if self.is_selected(entity) {
            world.remove_component::<Selected>(entity);
            self.selected_entities.retain(|&e| e != entity);
        } else if self.selected_entities.len() < self.max_selection {
            world.add_component(entity, Selected);
            self.selected_entities.push(entity);
        }
    }

    pub fn selected_count(&self) -> usize {
        self.selected_entities.len()
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Position;

    #[test]
    fn select_single_and_deselect() {
        let mut world = World::new();
        let mut mgr = SelectionManager::new();

        let e = world.create_entity();
        world.add_component(e, Selectable);
        world.add_component(e, Position::new(5.0, 5.0));

        mgr.select_single(e, &mut world);
        assert_eq!(mgr.selected_count(), 1);
        assert!(mgr.is_selected(e));
        assert!(world.has_component::<Selected>(e));

        mgr.deselect_all(&mut world);
        assert_eq!(mgr.selected_count(), 0);
        assert!(!world.has_component::<Selected>(e));
    }

    #[test]
    fn box_select_finds_entities_in_range() {
        let mut world = World::new();
        let mut mgr = SelectionManager::new();

        let inside = world.create_entity();
        world.add_component(inside, Selectable);
        world.add_component(inside, Position::new(5.0, 5.0));

        let outside = world.create_entity();
        world.add_component(outside, Selectable);
        world.add_component(outside, Position::new(50.0, 50.0));

        mgr.begin_box_select(0.0, 0.0);
        mgr.update_box_select(10.0, 10.0);
        mgr.finish_box_select(&mut world);

        assert!(mgr.is_selected(inside));
        assert!(!mgr.is_selected(outside));
    }

    #[test]
    fn max_selection_limit() {
        let mut world = World::new();
        let mut mgr = SelectionManager::new();
        mgr.max_selection = 2;

        let _entities: Vec<Entity> = (0..5)
            .map(|_| {
                let e = world.create_entity();
                world.add_component(e, Selectable);
                world.add_component(e, Position::new(0.0, 0.0));
                e
            })
            .collect();

        mgr.begin_box_select(-10.0, -10.0);
        mgr.update_box_select(10.0, 10.0);
        mgr.finish_box_select(&mut world);

        assert_eq!(mgr.selected_count(), 2);
    }

    #[test]
    fn toggle_select() {
        let mut world = World::new();
        let mut mgr = SelectionManager::new();

        let e = world.create_entity();
        world.add_component(e, Selectable);

        mgr.toggle_select(e, &mut world);
        assert!(mgr.is_selected(e));
        assert!(world.has_component::<Selected>(e));

        mgr.toggle_select(e, &mut world);
        assert!(!mgr.is_selected(e));
        assert!(!world.has_component::<Selected>(e));
    }
}

#[cfg(feature = "game")]
use std::collections::HashMap;
use std::f32::consts::PI;

use super::components::{MoveTarget, NavigationAgent, SquadMember};
use crate::{Entity, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SquadId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Formation {
    Line,
    Column,
    Circle,
    Wedge,
}

impl Formation {
    pub fn offsets(&self, count: usize, spacing: f32) -> Vec<[f32; 3]> {
        if count == 0 {
            return Vec::new();
        }
        match self {
            Formation::Line => {
                let half = (count as f32 - 1.0) / 2.0;
                (0..count)
                    .map(|i| {
                        let x = (i as f32 - half) * spacing;
                        [x, 0.0, 0.0]
                    })
                    .collect()
            }
            Formation::Column => (0..count).map(|i| [0.0, 0.0, i as f32 * spacing]).collect(),
            Formation::Circle => {
                if count == 1 {
                    return vec![[0.0, 0.0, 0.0]];
                }
                let radius = spacing * count as f32 / (2.0 * PI);
                (0..count)
                    .map(|i| {
                        let angle = 2.0 * PI * i as f32 / count as f32;
                        [angle.cos() * radius, 0.0, angle.sin() * radius]
                    })
                    .collect()
            }
            Formation::Wedge => (0..count)
                .map(|i| {
                    let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                    let rank = (i / 2 + 1) as f32;
                    let x = rank * spacing * side;
                    let z = rank * spacing;
                    [x, 0.0, z]
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Squad {
    pub id: SquadId,
    pub members: Vec<Entity>,
    pub formation: Formation,
    pub leader: Option<Entity>,
}

impl Squad {
    pub fn set_formation(&mut self, formation: Formation) {
        self.formation = formation;
    }
}

#[derive(Debug, Clone)]
pub struct SquadManager {
    next_id: u32,
    squads: HashMap<u32, Squad>,
}

impl SquadManager {
    pub fn new() -> Self {
        SquadManager {
            next_id: 0,
            squads: HashMap::new(),
        }
    }

    pub fn create_squad(&mut self, members: Vec<Entity>) -> SquadId {
        let id = SquadId(self.next_id);
        self.next_id += 1;

        let leader = members.first().copied();
        let squad = Squad {
            id,
            members: members.clone(),
            formation: Formation::Line,
            leader,
        };

        self.squads.insert(id.0, squad);
        id
    }

    pub fn disband_squad(&mut self, id: SquadId, world: &mut World) {
        if let Some(squad) = self.squads.remove(&id.0) {
            for entity in &squad.members {
                world.remove_component::<SquadMember>(*entity);
            }
        }
    }

    pub fn move_squad(&mut self, id: SquadId, target: [f32; 3], world: &mut World) {
        let squad = match self.squads.get(&id.0) {
            Some(s) => s,
            None => return,
        };
        let offsets = squad.formation.offsets(squad.members.len(), 2.0);
        let members = squad.members.clone();

        for (i, entity) in members.iter().enumerate() {
            let offset = offsets.get(i).copied().unwrap_or([0.0, 0.0, 0.0]);
            let dest = [
                target[0] + offset[0],
                target[1] + offset[1],
                target[2] + offset[2],
            ];

            let mt = MoveTarget::new(dest[0], dest[1], dest[2]);
            if world.has_component::<MoveTarget>(*entity) {
                if let Some(existing) = world.get_component_mut::<MoveTarget>(*entity) {
                    existing.x = dest[0];
                    existing.y = dest[1];
                    existing.z = dest[2];
                }
            } else {
                world.add_component(*entity, mt);
            }

            let nav = NavigationAgent::new(3.0);
            if world.has_component::<NavigationAgent>(*entity) {
                if let Some(existing) = world.get_component_mut::<NavigationAgent>(*entity) {
                    existing.path = nav.path;
                    existing.path_index = 0;
                    existing.speed = nav.speed;
                }
            } else {
                world.add_component(*entity, nav);
            }
        }
    }

    pub fn get_squad(&self, id: SquadId) -> Option<&Squad> {
        self.squads.get(&id.0)
    }

    pub fn get_squad_mut(&mut self, id: SquadId) -> Option<&mut Squad> {
        self.squads.get_mut(&id.0)
    }

    pub fn add_member(&mut self, id: SquadId, entity: Entity, world: &mut World) {
        if let Some(squad) = self.squads.get_mut(&id.0) {
            if !squad.members.contains(&entity) {
                squad.members.push(entity);
                world.add_component(entity, SquadMember::new(id.0));
                if squad.leader.is_none() {
                    squad.leader = Some(entity);
                }
            }
        }
    }

    pub fn remove_member(&mut self, id: SquadId, entity: Entity, world: &mut World) {
        if let Some(squad) = self.squads.get_mut(&id.0) {
            squad.members.retain(|&e| e != entity);
            if squad.leader == Some(entity) {
                squad.leader = squad.members.first().copied();
            }
            world.remove_component::<SquadMember>(entity);
        }
    }
}

impl Default for SquadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squad_creation() {
        let mut world = World::new();
        let mut mgr = SquadManager::new();

        let e1 = world.create_entity();
        let e2 = world.create_entity();
        let id = mgr.create_squad(vec![e1, e2]);

        let squad = mgr.get_squad(id).expect("squad exists");
        assert_eq!(squad.members.len(), 2);
        assert_eq!(squad.leader, Some(e1));
        assert_eq!(squad.formation, Formation::Line);
    }

    #[test]
    fn formation_line_offsets() {
        let offsets = Formation::Line.offsets(3, 2.0);
        assert_eq!(offsets.len(), 3);
        assert!((offsets[0][0] - (-2.0)).abs() < 1e-4);
        assert!((offsets[1][0] - 0.0).abs() < 1e-4);
        assert!((offsets[2][0] - 2.0).abs() < 1e-4);
    }

    #[test]
    fn formation_column_offsets() {
        let offsets = Formation::Column.offsets(3, 1.5);
        assert_eq!(offsets.len(), 3);
        assert!((offsets[0][2] - 0.0).abs() < 1e-4);
        assert!((offsets[1][2] - 1.5).abs() < 1e-4);
        assert!((offsets[2][2] - 3.0).abs() < 1e-4);
    }

    #[test]
    fn formation_circle_offsets() {
        let offsets = Formation::Circle.offsets(4, 2.0);
        assert_eq!(offsets.len(), 4);
        let r = 2.0 * 4.0 / (2.0 * PI);
        for off in &offsets {
            let dist = (off[0] * off[0] + off[2] * off[2]).sqrt();
            assert!((dist - r).abs() < 1e-4);
        }
    }

    #[test]
    fn formation_wedge_offsets() {
        let offsets = Formation::Wedge.offsets(4, 2.0);
        assert_eq!(offsets.len(), 4);
        assert!((offsets[0][0] - 2.0).abs() < 1e-4);
        assert!((offsets[0][2] - 2.0).abs() < 1e-4);
        assert!((offsets[1][0] - (-2.0)).abs() < 1e-4);
        assert!((offsets[1][2] - 2.0).abs() < 1e-4);
    }

    #[test]
    fn move_squad_assigns_targets() {
        let mut world = World::new();
        let mut mgr = SquadManager::new();

        let e1 = world.create_entity();
        let e2 = world.create_entity();
        let id = mgr.create_squad(vec![e1, e2]);

        mgr.move_squad(id, [10.0, 0.0, 10.0], &mut world);

        assert!(world.has_component::<MoveTarget>(e1));
        assert!(world.has_component::<MoveTarget>(e2));
        assert!(world.has_component::<NavigationAgent>(e1));
        assert!(world.has_component::<NavigationAgent>(e2));

        let nav = world.get_component::<NavigationAgent>(e1).unwrap();
        assert!((nav.speed - 3.0).abs() < f32::EPSILON);
        assert!(nav.path.is_none());
    }

    #[test]
    fn disband_removes_components() {
        let mut world = World::new();
        let mut mgr = SquadManager::new();

        let e1 = world.create_entity();
        let id = mgr.create_squad(vec![e1]);
        world.add_component(e1, SquadMember::new(id.0));

        mgr.disband_squad(id, &mut world);

        assert!(!world.has_component::<SquadMember>(e1));
        assert!(mgr.get_squad(id).is_none());
    }

    #[test]
    fn add_remove_members() {
        let mut world = World::new();
        let mut mgr = SquadManager::new();

        let e1 = world.create_entity();
        let e2 = world.create_entity();
        let e3 = world.create_entity();

        let id = mgr.create_squad(vec![e1]);
        mgr.add_member(id, e2, &mut world);
        mgr.add_member(id, e3, &mut world);

        let squad = mgr.get_squad(id).unwrap();
        assert_eq!(squad.members.len(), 3);
        assert!(world.has_component::<SquadMember>(e2));
        assert!(world.has_component::<SquadMember>(e3));

        mgr.remove_member(id, e2, &mut world);
        let squad = mgr.get_squad(id).unwrap();
        assert_eq!(squad.members.len(), 2);
        assert!(!squad.members.contains(&e2));
        assert!(!world.has_component::<SquadMember>(e2));
    }
}

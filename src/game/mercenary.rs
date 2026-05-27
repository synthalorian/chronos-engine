#[cfg(feature = "game")]

use crate::{Entity, World};
use crate::component::{Health, Transform, Velocity};
use super::components::*;

/// Preset mercenary archetypes with distinct stat profiles.
#[derive(Debug, Clone, Copy)]
pub enum MercenaryTemplate {
    Warrior,
    Archer,
    Mage,
    Scout,
}

impl MercenaryTemplate {
    /// Base stat layout: (strength, dexterity, intelligence, vitality).
    fn base_stats(&self) -> (u32, u32, u32, u32) {
        match self {
            MercenaryTemplate::Warrior => (15, 8, 5, 12),
            MercenaryTemplate::Archer => (8, 15, 6, 8),
            MercenaryTemplate::Mage => (5, 8, 18, 6),
            MercenaryTemplate::Scout => (8, 14, 8, 8),
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            MercenaryTemplate::Warrior => "Warrior",
            MercenaryTemplate::Archer => "Archer",
            MercenaryTemplate::Mage => "Mage",
            MercenaryTemplate::Scout => "Scout",
        }
    }
}

/// Factory for spawning pre-configured mercenary entities into a [`World`].
pub struct MercenaryFactory;

impl MercenaryFactory {
    /// Creates a fully equipped mercenary entity and inserts it into `world`.
    pub fn create_mercenary(
        world: &mut World,
        template: MercenaryTemplate,
        position: [f32; 3],
    ) -> Entity {
        let entity = world.create_entity();

        let (str, dex, int, vit) = template.base_stats();
        let stats = MercenaryStats::new(template.default_name())
            .with_stats(str, dex, int, vit);

        world.add_component(entity, Transform::new(position[0], position[1], position[2]));
        world.add_component(entity, Velocity::new(0.0, 0.0));
        world.add_component(entity, Health::new(vit * 10));
        world.add_component(entity, stats);
        world.add_component(entity, Selectable);
        world.add_component(entity, Team::Player);
        world.add_component(entity, AggroRadius::new(8.0));
        world.add_component(entity, HealthBar::new(1.0, 0.1));

        entity
    }

    /// Creates a squad of mercenaries arranged around a center position.
    pub fn create_squad(
        world: &mut World,
        templates: &[MercenaryTemplate],
        center: [f32; 3],
    ) -> Vec<Entity> {
        let squad_id = world.create_entity().index();
        let mut entities = Vec::with_capacity(templates.len());

        for (i, template) in templates.iter().enumerate() {
            let angle = (i as f32) * (std::f32::consts::TAU / templates.len() as f32);
            let offset_x = angle.cos() * 2.0;
            let offset_z = angle.sin() * 2.0;
            let pos = [center[0] + offset_x, center[1], center[2] + offset_z];

            let entity = Self::create_mercenary(world, *template, pos);
            world.add_component(entity, SquadMember::new(squad_id));
            entities.push(entity);
        }

        entities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_world() -> World {
        World::new()
    }

    #[test]
    fn factory_creates_entity_with_correct_stats() {
        let mut world = test_world();
        let entity = MercenaryFactory::create_mercenary(
            &mut world,
            MercenaryTemplate::Warrior,
            [0.0, 0.0, 0.0],
        );

        assert!(world.has_component::<MercenaryStats>(entity));
        assert!(world.has_component::<Team>(entity));
        assert!(world.has_component::<Selectable>(entity));

        let stats = world.get_component::<MercenaryStats>(entity).expect("stats");
        assert_eq!(stats.strength, 15);
        assert_eq!(stats.vitality, 12);
    }

    #[test]
    fn template_stats_are_different() {
        let warrior = MercenaryTemplate::Warrior.base_stats();
        let mage = MercenaryTemplate::Mage.base_stats();
        let archer = MercenaryTemplate::Archer.base_stats();
        let scout = MercenaryTemplate::Scout.base_stats();

        assert_ne!(warrior, mage);
        assert_ne!(warrior, archer);
        assert_ne!(mage, scout);
        assert_ne!(archer, scout);
    }

    #[test]
    fn squad_creation() {
        let mut world = test_world();
        let templates = [
            MercenaryTemplate::Warrior,
            MercenaryTemplate::Archer,
            MercenaryTemplate::Mage,
        ];
        let entities = MercenaryFactory::create_squad(&mut world, &templates, [0.0, 0.0, 0.0]);

        assert_eq!(entities.len(), 3);
        for entity in &entities {
            assert!(world.has_component::<SquadMember>(*entity));
        }
    }
}

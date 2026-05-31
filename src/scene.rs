#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Scene/level serialization module.
//!
//! Provides `Scene` — a named collection of `EntityPrefab` templates that can be
//! serialized to and loaded from JSON files. Prefabs can be spawned into a `World`
//! to instantiate entities with their predefined components.

use crate::component::{
    CircleRadius, Damage, Dead, Gravity, Grounded, Health, Position, RigidBody, Sprite, Transform,
    Velocity,
};
use crate::entity::Entity;
use crate::world::World;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// A single component stored in a scene as a type-erased JSON value.
///
/// Each variant wraps the corresponding `serde_json::Value` so the JSON
/// structure is human-readable and hand-editable. Serializes as a single-key
/// object (e.g. `{"Position": {"x": 1.0, "y": 2.0}}`).
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentValue {
    /// A 2D position component.
    Position(serde_json::Value),
    /// A 2D velocity component.
    Velocity(serde_json::Value),
    /// A health component with current/max values.
    Health(serde_json::Value),
    /// A damage tag component (raw u32).
    Damage(serde_json::Value),
    /// A 3D transform component.
    Transform(serde_json::Value),
    /// A sprite/render component.
    Sprite(serde_json::Value),
    /// A circle collision radius.
    CircleRadius(serde_json::Value),
    /// A rigid body with mass, damping, and restitution.
    RigidBody(serde_json::Value),
    /// A tag component marking an entity as grounded.
    Grounded(serde_json::Value),
    /// A gravity direction component.
    Gravity(serde_json::Value),
    /// A tag component marking an entity as dead.
    Dead(serde_json::Value),
}

impl fmt::Display for ComponentValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentValue::Position(_) => write!(f, "Position"),
            ComponentValue::Velocity(_) => write!(f, "Velocity"),
            ComponentValue::Health(_) => write!(f, "Health"),
            ComponentValue::Damage(_) => write!(f, "Damage"),
            ComponentValue::Transform(_) => write!(f, "Transform"),
            ComponentValue::Sprite(_) => write!(f, "Sprite"),
            ComponentValue::CircleRadius(_) => write!(f, "CircleRadius"),
            ComponentValue::RigidBody(_) => write!(f, "RigidBody"),
            ComponentValue::Grounded(_) => write!(f, "Grounded"),
            ComponentValue::Gravity(_) => write!(f, "Gravity"),
            ComponentValue::Dead(_) => write!(f, "Dead"),
        }
    }
}

impl Serialize for ComponentValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let (key, value) = match self {
            ComponentValue::Position(v) => ("Position", v),
            ComponentValue::Velocity(v) => ("Velocity", v),
            ComponentValue::Health(v) => ("Health", v),
            ComponentValue::Damage(v) => ("Damage", v),
            ComponentValue::Transform(v) => ("Transform", v),
            ComponentValue::Sprite(v) => ("Sprite", v),
            ComponentValue::CircleRadius(v) => ("CircleRadius", v),
            ComponentValue::RigidBody(v) => ("RigidBody", v),
            ComponentValue::Grounded(v) => ("Grounded", v),
            ComponentValue::Gravity(v) => ("Gravity", v),
            ComponentValue::Dead(v) => ("Dead", v),
        };
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(key, value)?;
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for ComponentValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        ComponentValue::from_json_value(value).map_err(serde::de::Error::custom)
    }
}

impl ComponentValue {
    fn from_json_value(value: serde_json::Value) -> Result<Self, String> {
        match value {
            serde_json::Value::Object(map) => {
                if map.len() != 1 {
                    return Err(format!(
                        "ComponentValue must be a single-key object, got {} keys",
                        map.len()
                    ));
                }

                let (key, data) = map
            .into_iter()
            .next()
            .expect("component map must have at least one entry for single-key object");
                match key.as_str() {
                    "Position" => {
                        let x = data["x"].as_f64().ok_or("Position requires a numeric x")?;
                        let y = data["y"].as_f64().ok_or("Position requires a numeric y")?;
                        Ok(ComponentValue::Position(
                            serde_json::json!({"x": x, "y": y}),
                        ))
                    }
                    "Velocity" => {
                        let x = data["x"].as_f64().ok_or("Velocity requires a numeric x")?;
                        let y = data["y"].as_f64().ok_or("Velocity requires a numeric y")?;
                        Ok(ComponentValue::Velocity(
                            serde_json::json!({"x": x, "y": y}),
                        ))
                    }
                    "Health" => {
                        let current = data["current"]
                            .as_u64()
                            .ok_or("Health requires a numeric current")?;
                        let max = data["max"]
                            .as_u64()
                            .ok_or("Health requires a numeric max")?;
                        Ok(ComponentValue::Health(
                            serde_json::json!({"current": current, "max": max}),
                        ))
                    }
                    "Damage" => {
                        let v = data.as_u64().ok_or("Damage requires a numeric value")?;
                        Ok(ComponentValue::Damage(serde_json::json!(v)))
                    }
                    "Transform" => {
                        let x = data["x"].as_f64().ok_or("Transform requires a numeric x")?;
                        let y = data["y"].as_f64().ok_or("Transform requires a numeric y")?;
                        let z = data["z"].as_f64().ok_or("Transform requires a numeric z")?;
                        let rotation = data["rotation"]
                            .as_f64()
                            .ok_or("Transform requires a numeric rotation")?;
                        let scale = data["scale"]
                            .as_f64()
                            .ok_or("Transform requires a numeric scale")?;
                        Ok(ComponentValue::Transform(serde_json::json!({
                            "x": x, "y": y, "z": z, "rotation": rotation, "scale": scale
                        })))
                    }
                    "Sprite" => {
                        let symbol = data["symbol"]
                            .as_str()
                            .ok_or("Sprite requires a string symbol")?;
                        let color = data["color"]
                            .as_array()
                            .ok_or("Sprite requires an array color")?;
                        let r = color[0]
                            .as_u64()
                            .ok_or("Sprite color requires numeric values")?
                            as u8;
                        let g = color[1]
                            .as_u64()
                            .ok_or("Sprite color requires numeric values")?
                            as u8;
                        let b = color[2]
                            .as_u64()
                            .ok_or("Sprite color requires numeric values")?
                            as u8;
                        let layer = data["layer"]
                            .as_i64()
                            .ok_or("Sprite requires a numeric layer")?;
                        Ok(ComponentValue::Sprite(serde_json::json!({
                            "symbol": symbol, "color": [r, g, b], "layer": layer
                        })))
                    }
                    "CircleRadius" => {
                        let v = data
                            .as_f64()
                            .ok_or("CircleRadius requires a numeric value")?;
                        Ok(ComponentValue::CircleRadius(serde_json::json!(v)))
                    }
                    "RigidBody" => {
                        let mass = data["mass"]
                            .as_f64()
                            .ok_or("RigidBody requires a numeric mass")?;
                        let damping = data["damping"]
                            .as_f64()
                            .ok_or("RigidBody requires a numeric damping")?;
                        let restitution = data["restitution"]
                            .as_f64()
                            .ok_or("RigidBody requires a numeric restitution")?;
                        Ok(ComponentValue::RigidBody(serde_json::json!({
                            "mass": mass, "damping": damping, "restitution": restitution
                        })))
                    }
                    "Grounded" => match data {
                        serde_json::Value::Null | serde_json::Value::Object(_) => {
                            Ok(ComponentValue::Grounded(serde_json::json!({})))
                        }
                        _ => Err("Grounded expects an empty object".into()),
                    },
                    "Gravity" => {
                        let x = data["x"].as_f64().ok_or("Gravity requires a numeric x")?;
                        let y = data["y"].as_f64().ok_or("Gravity requires a numeric y")?;
                        Ok(ComponentValue::Gravity(serde_json::json!({"x": x, "y": y})))
                    }
                    "Dead" => match data {
                        serde_json::Value::Null | serde_json::Value::Object(_) => {
                            Ok(ComponentValue::Dead(serde_json::json!({})))
                        }
                        _ => Err("Dead expects an empty object".into()),
                    },
                    _ => Err(format!("Unknown component type: {}", key)),
                }
            }
            other => Err(format!("ComponentValue must be an object, got {}", other)),
        }
    }
}

/// A template for an entity with a predefined set of components.
///
/// Prefabs are the building blocks of scenes — they define what components
/// an entity should have and with what values. A single prefab can be
/// spawned multiple times into a world to create many entities of the
/// same type (e.g., multiple enemies with identical stats).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPrefab {
    /// The name of this prefab (used for lookups).
    pub name: String,
    /// The list of components that this prefab provides.
    pub components: Vec<ComponentValue>,
}

/// A scene — a named collection of entity prefabs.
///
/// Scenes represent levels or worlds that can be saved to disk and loaded
/// at runtime. Each scene contains one or more `EntityPrefab` templates
/// that define what entities exist in the scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// The human-readable name of this scene.
    pub name: String,
    /// The schema version (for forward/backward compatibility).
    pub version: u32,
    /// The collection of entity prefabs in this scene.
    pub prefabs: Vec<EntityPrefab>,
}

impl Scene {
    /// Create a new empty scene with the given name.
    pub fn new(name: &str) -> Self {
        Scene {
            name: name.to_string(),
            version: 1,
            prefabs: Vec::new(),
        }
    }

    /// Add an entity prefab to this scene.
    pub fn add_prefab(&mut self, prefab: EntityPrefab) {
        self.prefabs.push(prefab);
    }

    /// Find a prefab by name, returning a reference to it.
    pub fn find_prefab(&self, name: &str) -> Option<&EntityPrefab> {
        self.prefabs.iter().find(|p| p.name == name)
    }

    /// Serialize the scene to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse a scene from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Write the scene to a file as pretty-printed JSON.
    ///
    /// The file is created or overwritten at the given path.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Serialization failed: {}", e),
            )
        })?;
        fs::write(path, json)
    }

    /// Load a scene from a JSON file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse scene JSON: {}", e),
            )
        })
    }

    /// Spawn an entity from a prefab into the given world.
    ///
    /// Creates a new entity and attaches all the prefab's components.
    /// Returns the newly created entity.
    ///
    /// # Errors
    /// Returns an error if a component value cannot be deserialized
    /// into the expected type.
    pub fn spawn_prefab(&self, world: &mut World, prefab_name: &str) -> Result<Entity, SceneError> {
        let prefab = self
            .find_prefab(prefab_name)
            .ok_or_else(|| SceneError::PrefabNotFound(prefab_name.to_string()))?;

        let entity = world.create_entity();
        for component in &prefab.components {
            let component_value = component.clone();
            spawn_component(world, entity, component_value)?;
        }
        Ok(entity)
    }
}

/// Error type for scene operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneError {
    /// The specified prefab was not found in the scene.
    PrefabNotFound(String),
    /// A component value could not be deserialized into the expected type.
    ComponentDeserialize(String),
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SceneError::PrefabNotFound(name) => write!(f, "Prefab not found: {}", name),
            SceneError::ComponentDeserialize(msg) => {
                write!(f, "Component deserialize error: {}", msg)
            }
        }
    }
}

impl std::error::Error for SceneError {}

/// Spawn a single component value into a world.
///
/// This is a helper function that converts a `ComponentValue` into the
/// appropriate concrete component type and adds it to the entity.
///
/// # Errors
/// Returns a `SceneError::ComponentDeserialize` if the value cannot be
/// converted into the expected component type.
pub fn spawn_component(
    world: &mut World,
    entity: Entity,
    value: ComponentValue,
) -> Result<(), SceneError> {
    match value {
        ComponentValue::Position(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Position expects an object".into())
            })?;
            let x = obj["x"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Position x must be a number".into())
            })?;
            let y = obj["y"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Position y must be a number".into())
            })?;
            world.add_component(entity, Position::new(x as f32, y as f32));
        }
        ComponentValue::Velocity(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Velocity expects an object".into())
            })?;
            let x = obj["x"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Velocity x must be a number".into())
            })?;
            let y = obj["y"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Velocity y must be a number".into())
            })?;
            world.add_component(entity, Velocity::new(x as f32, y as f32));
        }
        ComponentValue::Health(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Health expects an object".into())
            })?;
            let current = obj["current"].as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Health current must be a number".into())
            })?;
            let max = obj["max"].as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Health max must be a number".into())
            })?;
            world.add_component(entity, Health::new(max as u32));
            // Set current after construction since Health::new sets current = max
            if let Some(h) = world.get_component_mut::<Health>(entity) {
                h.current = current as u32;
            }
        }
        ComponentValue::Damage(data) => {
            let value = data.as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Damage must be a number".into())
            })?;
            world.add_component(entity, Damage(value as u32));
        }
        ComponentValue::Transform(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Transform expects an object".into())
            })?;
            let x = obj["x"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Transform x must be a number".into())
            })?;
            let y = obj["y"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Transform y must be a number".into())
            })?;
            let z = obj["z"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Transform z must be a number".into())
            })?;
            world.add_component(entity, Transform::new(x as f32, y as f32, z as f32));
        }
        ComponentValue::Sprite(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite expects an object".into())
            })?;
            let symbol = obj["symbol"].as_str().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite symbol must be a string".into())
            })?;
            let color = obj["color"].as_array().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite color must be an array".into())
            })?;
            let r = color[0].as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite color r must be a number".into())
            })? as u8;
            let g = color[1].as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite color g must be a number".into())
            })? as u8;
            let b = color[2].as_u64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite color b must be a number".into())
            })? as u8;
            let layer = obj["layer"].as_i64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Sprite layer must be a number".into())
            })? as i32;
            world.add_component(
                entity,
                Sprite::new(symbol.chars().next().unwrap_or('?'), r, g, b),
            );
            if let Some(s) = world.get_component_mut::<Sprite>(entity) {
                s.layer = layer;
            }
        }
        ComponentValue::CircleRadius(data) => {
            let value = data.as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("CircleRadius must be a number".into())
            })?;
            world.add_component(entity, CircleRadius::new(value as f32));
        }
        ComponentValue::RigidBody(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("RigidBody expects an object".into())
            })?;
            let mass = obj["mass"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("RigidBody mass must be a number".into())
            })?;
            let damping = obj["damping"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("RigidBody damping must be a number".into())
            })?;
            let restitution = obj["restitution"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("RigidBody restitution must be a number".into())
            })?;
            world.add_component(
                entity,
                RigidBody::new(mass as f32, damping as f32, restitution as f32),
            );
        }
        ComponentValue::Grounded(_) => {
            world.add_component(entity, Grounded);
        }
        ComponentValue::Gravity(data) => {
            let obj = data.as_object().ok_or_else(|| {
                SceneError::ComponentDeserialize("Gravity expects an object".into())
            })?;
            let x = obj["x"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Gravity x must be a number".into())
            })?;
            let y = obj["y"].as_f64().ok_or_else(|| {
                SceneError::ComponentDeserialize("Gravity y must be a number".into())
            })?;
            world.add_component(entity, Gravity::new(x as f32, y as f32));
        }
        ComponentValue::Dead(_) => {
            world.add_component(entity, Dead);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal scene JSON that the engine should understand.
    const TEST_SCENE_JSON: &str = r#"{
  "name": "test_level",
  "version": 1,
  "prefabs": [
    {
      "name": "player",
      "components": [
        { "Position": { "x": 10.0, "y": 20.0 } },
        { "Velocity": { "x": 1.0, "y": 0.0 } },
        { "Health": { "current": 100, "max": 100 } }
      ]
    },
    {
      "name": "enemy",
      "components": [
        { "Position": { "x": 50.0, "y": 50.0 } },
        { "Health": { "current": 50, "max": 50 } },
        { "Damage": 15 }
      ]
    }
  ]
}"#;

    #[test]
    fn test_scene_new() {
        let scene = Scene::new("my_scene");
        assert_eq!(scene.name, "my_scene");
        assert_eq!(scene.version, 1);
        assert!(scene.prefabs.is_empty());
    }

    #[test]
    fn test_scene_to_json_roundtrip() {
        let mut scene = Scene::new("roundtrip_test");
        scene.add_prefab(EntityPrefab {
            name: "hero".into(),
            components: vec![
                ComponentValue::Position(serde_json::json!({"x": 0.0, "y": 0.0})),
                ComponentValue::Health(serde_json::json!({"current": 200, "max": 200})),
            ],
        });

        let json = scene.to_json().expect("to_json should succeed");
        let loaded = Scene::from_json(&json).expect("from_json should succeed");

        assert_eq!(loaded.name, "roundtrip_test");
        assert_eq!(loaded.prefabs.len(), 1);
        assert_eq!(loaded.prefabs[0].name, "hero");
    }

    #[test]
    fn test_scene_from_json() {
        let scene = Scene::from_json(TEST_SCENE_JSON).expect("valid scene JSON");

        assert_eq!(scene.name, "test_level");
        assert_eq!(scene.version, 1);
        assert_eq!(scene.prefabs.len(), 2);
        assert_eq!(scene.prefabs[0].name, "player");
        assert_eq!(scene.prefabs[0].components.len(), 3);
        assert_eq!(scene.prefabs[1].name, "enemy");
        assert_eq!(scene.prefabs[1].components.len(), 3);
    }

    #[test]
    fn test_scene_spawn_prefab() {
        let scene = Scene::from_json(TEST_SCENE_JSON).expect("valid scene JSON");
        let mut world = World::new();

        let entity = scene
            .spawn_prefab(&mut world, "player")
            .expect("spawn should succeed");

        assert!(world.entity_exists(entity));
        assert!(world.has_component::<Position>(entity));
        assert!(world.has_component::<Health>(entity));
        assert!(world.has_component::<Velocity>(entity));

        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 10.0).abs() < f32::EPSILON);
        assert!((pos.y - 20.0).abs() < f32::EPSILON);

        let health = world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.current, 100);
        assert_eq!(health.max, 100);
    }

    #[test]
    fn test_scene_spawn_enemy_prefab() {
        let scene = Scene::from_json(TEST_SCENE_JSON).expect("valid scene JSON");
        let mut world = World::new();

        let entity = scene
            .spawn_prefab(&mut world, "enemy")
            .expect("spawn should succeed");

        assert!(world.entity_exists(entity));

        let pos = world.get_component::<Position>(entity).unwrap();
        assert!((pos.x - 50.0).abs() < f32::EPSILON);

        let health = world.get_component::<Health>(entity).unwrap();
        assert_eq!(health.current, 50);
        assert_eq!(health.max, 50);

        let damage = world.get_component::<Damage>(entity).unwrap();
        assert_eq!(damage.0, 15);
    }

    #[test]
    fn test_scene_spawn_prefab_not_found() {
        let scene = Scene::from_json(TEST_SCENE_JSON).expect("valid scene JSON");
        let mut world = World::new();

        let result = scene.spawn_prefab(&mut world, "nonexistent");
        assert!(matches!(result, Err(SceneError::PrefabNotFound(_))));
    }

    #[test]
    fn test_scene_add_and_find_prefab() {
        let mut scene = Scene::new("find_test");
        scene.add_prefab(EntityPrefab {
            name: "boss".into(),
            components: vec![
                ComponentValue::Position(serde_json::json!({"x": 100.0, "y": 200.0})),
                ComponentValue::Gravity(serde_json::json!({"x": 0.0, "y": 9.8})),
            ],
        });

        let found = scene.find_prefab("boss");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "boss");
        assert_eq!(found.unwrap().components.len(), 2);

        assert!(scene.find_prefab("missing").is_none());
    }

    #[test]
    fn test_scene_save_and_load_file() {
        let tmp = std::env::temp_dir().join("chronos_test_scene.json");
        let mut scene = Scene::new("file_test");
        scene.add_prefab(EntityPrefab {
            name: "spawn_point".into(),
            components: vec![
                ComponentValue::Position(serde_json::json!({"x": 0.0, "y": 0.0})),
                ComponentValue::RigidBody(
                    serde_json::json!({"mass": 1.0, "damping": 0.0, "restitution": 0.0}),
                ),
            ],
        });

        scene.save(&tmp).expect("save should succeed");

        let loaded = Scene::from_file(&tmp).expect("from_file should succeed");
        assert_eq!(loaded.name, "file_test");
        assert_eq!(loaded.prefabs.len(), 1);
        assert_eq!(loaded.prefabs[0].name, "spawn_point");

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_component_value_display() {
        assert_eq!(
            format!("{}", ComponentValue::Position(serde_json::json!({}))),
            "Position"
        );
        assert_eq!(
            format!("{}", ComponentValue::Velocity(serde_json::json!({}))),
            "Velocity"
        );
        assert_eq!(
            format!("{}", ComponentValue::Grounded(serde_json::json!({}))),
            "Grounded"
        );
    }

    #[test]
    fn test_scene_version_preserved() {
        let scene = Scene::from_json(TEST_SCENE_JSON).expect("valid scene JSON");
        assert_eq!(scene.version, 1);

        let json = scene.to_json().expect("to_json should succeed");
        // Parse the version field directly to ensure it's preserved in the JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["version"], 1);
    }
}

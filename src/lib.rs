//! Chronos Engine — A custom ECS game engine.
//!
//! Core ECS with generational entity IDs, type-erased component storage,
//! archetype tracking, system pipeline, event bus, game loop, spatial
//! indexing, tile maps, particles, fog of war, skeletal animation,
//! 2D/3D rendering, lighting, and post-processing.

pub mod entity;
pub mod component;
pub mod input;
pub mod spatial;
pub mod storage;
pub mod world;
pub mod system;
pub mod tilemap;
pub mod particle;
pub mod obj_loader;
pub mod octree;
pub mod physics3d;
pub mod lighting;
pub mod skeletal;
pub mod fog_of_war;

#[cfg(feature = "render")]
pub mod render;

#[cfg(feature = "render")]
pub mod texture;

#[cfg(feature = "render")]
pub mod font;

#[cfg(feature = "render")]
pub mod ui;

#[cfg(feature = "render")]
pub mod render3d;

#[cfg(feature = "render")]
pub mod postprocess;

#[cfg(feature = "serialize")]
pub mod scene;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "dev-tools")]
pub mod asset;

#[cfg(feature = "render")]
pub mod editor;

#[cfg(feature = "game")]
pub mod game;

pub use entity::Entity;
pub use component::{
    Component, Position, Velocity, Health, Damage, Dead, Transform, Sprite,
    CircleRadius, RigidBody, Grounded, Gravity,
};
pub use storage::{ComponentStorage, StorageRegistry};
pub use world::World;
pub use system::{
    System, MovementSystem, HealthSystem, CollisionSystem, DeathCleanupSystem,
    DebugRenderSystem, SystemPhase, GameLoop, TickScheduler, Event, EventBus,
    GravitySystem, PlatformerSystem, RaycastSystem,
};
pub use spatial::{Quadtree, QuadtreeObject, AABB, Ray, RaycastHit};
pub use tilemap::{TileMap, TileChunk, Tile};
pub use particle::{ParticleEmitter, Particle, ParticleSystem};
pub use obj_loader::ObjMesh;
pub use octree::{Octree, OctreeObject, AABB3D, Ray3D, RayHit3D};
pub use physics3d::{PhysicsWorld3D, RigidBody3D, Collider3D, Contact3D, Constraint3D, DistanceConstraint, PointConstraint};
pub use lighting::{Light, LightType, LightingSystem, LightMap, ShadowCaster, LineSegment, VisibilityPolygon};
pub use skeletal::{Skeleton, Joint, JointPose, SkeletonPose, AnimationClip, AnimationChannel, AnimationPlayer, AnimationBlender};
pub use fog_of_war::{FogOfWar, FogGrid, FogRevealer, Visibility};
pub use input::{
    KeyCode, MouseButton, GamepadButton, GamepadAxis,
    InputSource, InputEvent,
    Binding, ActionBinding, ActionState,
    InputContext, AxisBinding, InputManager,
};

#[cfg(feature = "render")]
pub use render::{Renderer, RenderSprite, SpriteBatch, Camera};

#[cfg(feature = "render")]
pub use texture::{TextureAtlas, AtlasFrame, FpsCounter};

#[cfg(feature = "render")]
pub use font::BitmapFont;

#[cfg(feature = "render")]
pub use ui::{Button, Slider, Label, Panel, UiContext, WidgetState, WidgetStyle, Rect};

#[cfg(feature = "render")]
pub use render3d::{Renderer3D, PerspectiveCamera, Mesh3D, Vertex3D, Transform3D};

#[cfg(feature = "render")]
pub use postprocess::{PostProcessor, ColorGradeParams};

#[cfg(feature = "serialize")]
pub use scene::{Scene, EntityPrefab, ComponentValue, SceneError, spawn_component};

#[cfg(feature = "audio")]
pub use audio::{
    AudioEngine, AudioError, VolumeControl, SpatialAudio, SoundBuffer,
    SfxPlayer, MusicPlayer, MusicState,
};

#[cfg(feature = "dev-tools")]
pub use asset::{
    Asset, AssetId, AssetError, AssetRegistry, HotReloadWatcher, AssetLoader,
};

#[cfg(feature = "render")]
pub use editor::{
    DevOverlay, EntityInspector, InspectionReport, ComponentInfo,
    StatsPanel, Stats, DevConsole, LogEntry, LogLevel,
    SceneTree, SceneEntry, OverlayRenderData,
};

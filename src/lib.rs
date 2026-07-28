//! Chronos Engine — A custom ECS game engine.
//!
//! Core ECS with generational entity IDs, type-erased component storage,
//! archetype tracking, system pipeline, event bus, game loop, spatial
//! indexing, tile maps, particles, fog of war, skeletal animation,
//! 2D/3D rendering, lighting, and post-processing.

pub mod animation;
pub mod component;
pub mod entity;
pub mod fog_of_war;
pub mod general_systems;
pub mod input;
pub mod lighting;
pub mod material;
pub mod obj_loader;
pub mod octree;
pub mod particle;
pub mod physics2d;
pub mod physics3d;
pub mod shader;
pub mod skeletal;
pub mod spatial;
pub mod storage;
pub mod system;
pub mod tilemap;
pub mod world;

#[cfg(feature = "scripting")]
pub mod scripting;

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

#[cfg(feature = "editor")]
pub mod editor_app;

#[cfg(feature = "editor")]
pub mod editor_panels;

#[cfg(feature = "editor")]
pub mod editor_workspace;

#[cfg(feature = "editor")]
pub mod editor_project;

#[cfg(feature = "game")]
pub mod game;

pub use animation::{
    AnimCondition, AnimParam, AnimState, AnimStateMachine, AnimStateUpdate, AnimTransition,
    BlendChild, BlendTree, BlendType, Interpolation, Keyframe, KeyframeValue, SpriteAnimation,
    SpriteAnimationEvent, SpriteFrame, Timeline, TimelineSample, TimelineTrack,
};
pub use component::{
    CircleRadius, Component, Damage, Dead, Gravity, Grounded, Health, Position, RigidBody, Sprite,
    Transform, Velocity,
};
pub use entity::Entity;
pub use fog_of_war::{FogGrid, FogOfWar, FogRevealer, Visibility};
pub use input::{
    ActionBinding, ActionState, AxisBinding, Binding, GamepadAxis, GamepadButton, InputContext,
    InputEvent, InputManager, InputSource, KeyCode, MouseButton,
};
pub use lighting::{
    Light, LightMap, LightType, LightingSystem, LineSegment, ShadowCaster, VisibilityPolygon,
};
pub use material::{
    particle_material, pbr_standard, skybox_material, sprite_material, terrain_material,
    ui_material, unlit, BlendMode, CompiledMaterial, CullMode, MaterialDefinition, MaterialError,
    MaterialProperty, MaterialValue, RenderState,
};
pub use obj_loader::ObjMesh;
pub use octree::{Octree, OctreeObject, Ray3D, RayHit3D, AABB3D};
pub use particle::{Particle, ParticleEmitter, ParticleSystem};
pub use physics2d::{Collider2D, Contact2D, PhysicsWorld2D, Ray2D, RayHit2D, RigidBody2D, Vec2};
pub use physics3d::{
    Collider3D, Constraint3D, Contact3D, DistanceConstraint, PhysicsWorld3D, PointConstraint,
    RigidBody3D,
};
pub use shader::{
    pbr_shader, sprite_shader, unlit_shader, NodeConnection, NodePort, PortDef, PortType,
    ShaderError, ShaderGraph, ShaderInput, ShaderNode, ShaderNodeType, ShaderOutput, ShaderWatcher,
};
pub use skeletal::{
    AnimationBlender, AnimationChannel, AnimationClip, AnimationPlayer, Joint, JointPose, Skeleton,
    SkeletonPose,
};
pub use spatial::{Quadtree, QuadtreeObject, Ray, RaycastHit, AABB};
pub use storage::{ComponentStorage, StorageRegistry};
pub use system::{
    CollisionSystem, DeathCleanupSystem, DebugRenderSystem, Event, EventBus, GameLoop,
    GravitySystem, HealthSystem, MovementSystem, PlatformerSystem, RaycastSystem, System,
    SystemPhase, TickScheduler,
};
pub use tilemap::{Tile, TileChunk, TileMap};
pub use world::World;

#[cfg(feature = "render")]
pub use render::{Camera, RenderSprite, Renderer, SpriteBatch};

#[cfg(feature = "render")]
pub use texture::{AtlasFrame, FpsCounter, TextureAtlas};

#[cfg(feature = "render")]
pub use font::BitmapFont;

#[cfg(feature = "render")]
pub use ui::{Button, Label, Panel, Rect, Slider, UiContext, WidgetState, WidgetStyle};

#[cfg(feature = "render")]
pub use render3d::{Mesh3D, PerspectiveCamera, Renderer3D, Transform3D, Vertex3D};

#[cfg(feature = "render")]
pub use postprocess::{ColorGradeParams, PostProcessor};

#[cfg(feature = "serialize")]
pub use scene::{spawn_component, ComponentValue, EntityPrefab, Scene, SceneError};

#[cfg(feature = "audio")]
pub use audio::{
    AudioEngine, AudioError, MusicPlayer, MusicState, SfxPlayer, SoundBuffer, SpatialAudio,
    VolumeControl,
};

#[cfg(feature = "dev-tools")]
pub use asset::{Asset, AssetError, AssetId, AssetLoader, AssetRegistry, HotReloadWatcher};

#[cfg(feature = "render")]
pub use editor::{
    ComponentInfo, DevConsole, DevOverlay, EntityInspector, InspectionReport, LogEntry, LogLevel,
    OverlayRenderData, SceneEntry, SceneTree, Stats, StatsPanel,
};

#[cfg(feature = "editor")]
pub use editor_app::{EditorApp, EditorError};

#[cfg(feature = "asset-pipeline")]
pub mod import;

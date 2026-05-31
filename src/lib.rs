//! Chronos Engine — A custom ECS game engine.
//!
//! Core ECS with generational entity IDs, type-erased component storage,
//! archetype tracking, system pipeline, event bus, game loop, spatial
//! indexing, tile maps, particles, fog of war, skeletal animation,
//! 2D/3D rendering, lighting, and post-processing.

pub mod error;
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
pub mod platform;
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

#[cfg(feature = "game")]
pub use game::plugin::ChronosCompanyPlugin;
#[cfg(feature = "game")]
pub use game::runner::{ChronosCompanyGame, GameConfig, GameMode, GameState};

#[cfg(feature = "game")]
pub mod demo;

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

pub mod net;
pub mod plugin;
pub mod profiler;

// ── WASM / Web entry point ────────────────────────────────────────────

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use web::run_chronos_web;

/// Convenience prelude that re-exports the most commonly used types.
///
/// # Usage
///
/// ```rust,ignore
/// use chronos_engine::prelude::*;
/// ```
pub mod prelude {
    pub use crate::error::{ChronosResult, ensure, ensure_or, from_display};
    pub use crate::{
        ActionBinding, ActionState, AnimCondition, AnimParam, AnimState, AnimStateMachine,
        AnimStateUpdate, AnimTransition, AnimationBlender, AnimationChannel, AnimationClip,
        AnimationPlayer, AxisBinding, Binding, BlendChild, BlendTree, BlendType, CircleRadius,
        Collider2D, Collider3D, CollisionSystem, Component, Constraint3D, Contact2D, Contact3D,
        Damage, Dead, DeathCleanupSystem, DebugRenderSystem, DistanceConstraint, Entity, Event,
        EventBus, FogGrid, FogOfWar, FogRevealer, GameLoop, GamepadAxis, GamepadButton, Gravity,
        GravitySystem, Grounded, Health, HealthSystem, InputContext, InputEvent, InputManager,
        InputSource, Interpolation, Joint, JointPose, KeyCode, Keyframe, KeyframeValue, Light,
        LightMap, LightType, LightingSystem, LineSegment, MouseButton, MovementSystem, ObjMesh,
        Octree, OctreeObject, Particle, ParticleEmitter, ParticleSystem, PhysicsWorld2D,
        PhysicsWorld3D, PlatformerSystem, PointConstraint, Position, Quadtree, QuadtreeObject, Ray,
        Ray2D, Ray3D, RayHit2D, RayHit3D, RaycastHit, RaycastSystem, RigidBody, RigidBody2D,
        RigidBody3D, ShadowCaster, Skeleton, SkeletonPose, Sprite, SpriteAnimation,
        SpriteAnimationEvent, SpriteFrame, System, SystemPhase, TickScheduler, Tile, TileChunk,
        TileMap, Timeline, TimelineSample, TimelineTrack, Transform, Vec2, Velocity, Visibility,
        VisibilityPolygon, World, AABB, AABB3D,
    };

    #[cfg(feature = "render")]
    pub use crate::{
        AtlasFrame, BitmapFont, Button, Camera, ColorGradeParams, ComponentInfo, DevConsole,
        DevOverlay, EntityInspector, FpsCounter, InspectionReport, Label, LogEntry, LogLevel,
        Mesh3D, OverlayRenderData, Panel, PerspectiveCamera, PostProcessor, Rect, RenderSprite,
        Renderer, Renderer3D, SceneEntry, SceneTree, Slider, SpriteBatch, Stats, StatsPanel,
        TextureAtlas, Transform3D, UiContext, Vertex3D, WidgetState, WidgetStyle,
    };

    #[cfg(feature = "serialize")]
    pub use crate::{spawn_component, ComponentValue, EntityPrefab, Scene, SceneError};

    #[cfg(feature = "audio")]
    pub use crate::{
        AudioEngine, AudioError, MusicPlayer, MusicState, SfxPlayer, SoundBuffer, SpatialAudio,
        VolumeControl,
    };

    #[cfg(feature = "dev-tools")]
    pub use crate::{Asset, AssetError, AssetId, AssetLoader, AssetRegistry, HotReloadWatcher};

    #[cfg(feature = "editor")]
    pub use crate::{EditorApp, EditorError};

    #[cfg(feature = "asset-pipeline")]
    pub use crate::import::{
        AssetCatalog, AssetGuid, AssetKind, AssetMeta, AssetType, Guid, ImportSettings, MetaManager,
    };

    pub use crate::profiler::{CounterStats, FrameProfiler, GpuTimer, TimingSample};

    pub use crate::plugin::editor::{DockZone, EditorPanel, InspectorHook, ToolbarButton};
    pub use crate::plugin::{
        EditorPluginHooks, Plugin, PluginApi, PluginContext, PluginError, PluginLoader,
        PluginManifest, PluginRegistry,
    };

    #[cfg(feature = "game")]
    pub use crate::game::plugin::ChronosCompanyPlugin;
    #[cfg(feature = "game")]
    pub use crate::game::runner::{ChronosCompanyGame, GameConfig, GameMode, GameState};

    #[cfg(feature = "net")]
    pub use crate::net::{
        ConnectionId, ConnectionManager, ConnectionState, ConnectionStats, EntityState,
        EntitySyncConfig, EntitySyncManager, EntityUpdate, EntityUpdateKind, InterestManager,
        InterestMode,        LagCompensation, LagCompensationConfig, Lobby, LobbyError, LockstepConfig,
        LockstepSync, MatchInfo, NatTraversal, NetworkId, NetworkStats, Packet,
        PacketHeader, PlayerInput, ReconciliationCorrection, RollbackManager, RollbackState,
        ServerSnapshot, SnapshotHistory, SyncSnapshot, UdpError, UdpTransport, WorldSnapshot,
        MAX_PACKET_SIZE,
    };

    #[cfg(feature = "voice-chat")]
    pub use crate::net::{AudioPacket, MicMode, VoiceChat, VoiceCodec, VoiceConfig, VoiceError};
}

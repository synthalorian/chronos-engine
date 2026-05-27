<p align="center">
  <img src="icon.png" alt="Chronos Engine" width="256" height="256">
</p>

<h1 align="center">Chronos Engine</h1>

<p align="center">
  <strong>A genre-agnostic ECS game engine in Rust. Zero dependencies (core). Deterministic by design.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021-orange?logo=rust" alt="Rust 2021">
  <img src="https://img.shields.io/badge/lines-13_500+-blue" alt="13,500+ lines">
  <img src="https://img.shields.io/badge/tests-110-brightgreen" alt="110 tests">
  <img src="https://img.shields.io/badge/unsafe-0-red" alt="Zero unsafe">
  <img src="https://img.shields.io/badge/deps-0_(core)-success" alt="Zero core deps">
</p>

---

> *"Write the future in the present while preserving the past."*

Born from the VHS static of 1984. Forged in Rust. Designed for RTS, platformers, RPGs, shooters, sims — whatever you throw at it. Chronos Engine doesn't care about your genre. It cares about entities, components, systems, and **determinism**.

Every byte of storage, every archetype migration, every event dispatch — hand-wired. No Bevy. No Legion. No shortcuts. The engine is **genre-agnostic**: the same core that powers an RTS lockstep simulation also handles a platformer's variable-timestep physics loop, an RPG's event-driven damage pipeline, or a sim's deterministic tick scheduler.

---

## Table of Contents

- [What's Inside](#whats-inside)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Feature Flags](#feature-flags)
- [Project Structure](#project-structure)
- [Systems Reference](#systems-reference)
- [Chronos Company](#chronos-company--first-game)
- [Comparison](#how-it-compares)
- [Roadmap](#roadmap)
- [Architecture Principles](#architecture-principles)
- [License](#license)

---

## What's Inside

**~13,500 lines** of Rust across **36 source files**. **110 tests** (85 unit + 25 integration). `cargo build --features full` compiles clean.

### ECS Core

The foundation. Zero dependencies. Pure `std` library Rust.

| Layer | What | Why It Matters |
|-------|------|---------------|
| **Entity** | Generational IDs with slot reuse — freed slots recycled with incremented generations | Stale handles **never** alias live entities. Use-after-free is impossible by construction. |
| **Component** | Blanket impl trait — any `Send + Sync + 'static` type is a component | No macros, no registration ceremony. Just attach your type. |
| **Storage** | Type-erased `Box<dyn Any>` per-component storage via `TypeId` | Maximally simple. Zero unsafe. Cache-hostile but correct. |
| **Archetypes** | `ArchetypeKey` (sorted `Vec<TypeId>`) → `Archetype` (entity group) | Multi-component queries skip entities that can't match. |
| **World** | Central registry: entity lifecycle, component attach/detach, archetype tracking, slot reuse | One-stop shop for all ECS operations. |

**Built-in components:** `Position`, `Velocity`, `Health`, `Damage`, `Dead`, `Transform`, `Sprite`, `CircleRadius`, `RigidBody`, `Grounded`, `Gravity`.

### Spatial Indexing

| Module | What |
|--------|------|
| **Quadtree** | 2D spatial index — O(n log n) insertion, O(k + log n) range query, cross-subtree collision support |
| **Octree** | 3D spatial index — AABB3D, recursive 8-child subdivision, sphere/AABB/ray queries |
| **AABB / AABB3D** | 2D and 3D bounding boxes with overlap/containment checks |
| **Ray / Ray3D** | Origin + direction for spatial queries with hit detection |

### Systems & Scheduling

| System | What |
|--------|------|
| **MovementSystem** | `Position += Velocity × dt` |
| **HealthSystem** | Damage → Health → Dead pipeline with events |
| **CollisionSystem** | Quadtree broad-phase + circle narrow-phase |
| **GravitySystem** | Gravity acceleration (g×dt) |
| **PlatformerSystem** | Ground check, jump impulse, friction |
| **RaycastSystem** | Point queries and ray casting via spatial index |
| **DeathCleanupSystem** | Remove entities with Dead component |
| **DebugRenderSystem** | Terminal grid renderer — no GPU needed |

| Scheduler | Use Case |
|-----------|----------|
| **GameLoop** | Variable-framerate (platformers, RPGs, FPS) — 5-phase pipeline: PreUpdate → Update → PostUpdate → Cleanup → Render |
| **TickScheduler** | Deterministic fixed-timestep (RTS, strategy, sims) — same inputs → same outputs, every time |

### Event Bus

`EventBus` with `VecDeque<Event>` for cross-system communication. Six event types: `Collision`, `DamageTaken`, `EntityDied`, `EntityDestroyed`, `RayHit`, `Custom`. Systems emit; game code drains between frames.

### Input System

Full multi-device input with context-based action bindings.

| Feature | What |
|---------|------|
| **Keyboard** | 80+ `KeyCode` variants |
| **Mouse** | Position, delta, scroll — accumulated per-frame |
| **Gamepad** | `GamepadButton` + `GamepadAxis` (sticks, triggers) |
| **Action bindings** | Map any input to named actions. Chainable `.or()` API: `"move_forward" → W, ArrowUp, LeftStickUp` |
| **Axis support** | Analog thumbsticks, mouse delta, scroll via `AxisBinding` |
| **Context switching** | Swap input maps per game state (gameplay, menu, console) |
| **State tracking** | `pressed`, `just_pressed`, `just_released`, `held` per action per frame |

### 2D Rendering (`render` feature)

wgpu 23 sprite batch renderer with instanced drawing.

| Module | What |
|--------|------|
| **Renderer** | Full wgpu pipeline, sprite instancing, camera with orthographic projection + screen shake |
| **TextureAtlas** | GPU texture atlas management with frame extraction |
| **BitmapFont** | ASCII grid glyph atlas, kerning, `render_text()` → `Vec<RenderSprite>` |
| **TileMap** | Chunked 16×16 grids with frustum culling |
| **ParticleSystem** | ECS-integrated particles with explosion/smoke/trail presets |
| **PostProcessor** | Color grading pipeline — brightness, contrast, saturation, gamma, vignette, bloom. Presets: default, CRT, noir, sunset |

### 3D Rendering (`render` feature)

| Module | What |
|--------|------|
| **Renderer3D** | Depth buffer, perspective camera, mesh pipeline, directional lighting, back-face culling |
| **Mesh3D** | Cube/plane primitives with vertex normals |
| **Transform3D** | TRS → model matrix computation |
| **ObjLoader** | Wavefront .obj parser — vertex/normal/UV/face parsing, fan triangulation |

### 3D Physics

| Module | What |
|--------|------|
| **PhysicsWorld3D** | Full rigid body simulation — static/dynamic bodies, sphere/AABB colliders |
| **Collision response** | Impulse-based with restitution + friction |
| **Constraints** | `DistanceConstraint`, `PointConstraint` — joint-like connections |
| **Gravity integration** | Per-body gravity with semi-implicit Euler |

### Advanced Systems

| Module | What |
|--------|------|
| **Lighting** | 2D lighting — Point, Directional, Spot, Area lights. Shadow casting via `ShadowCaster`. Visibility polygon computation. `LightMap` for scene-wide lighting. |
| **Skeletal Animation** | Joint hierarchy, `JointPose` (TRS), quaternion SLERP, `AnimationClip` with keyframe channels, `AnimationPlayer`, `AnimationBlender` for cross-fade. Custom mat4 inverse. |
| **Fog of War** | `FogGrid` with Unexplored/Explored/Visible states. `FogRevealer` for line-of-sight. `WallSegment` obstacles block visibility. |
| **UI** | Immediate-mode widgets — Button, Slider, Label, Panel. Hit-testing, `UiContext`, style presets (dark/light/accent). |

### Audio (`audio` feature)

Rodio 0.20 backend.

| Module | What |
|--------|------|
| **AudioEngine** | Device init, output stream management |
| **SfxPlayer** | Load .wav/.ogg, play one-shot sounds with volume |
| **MusicPlayer** | Stream long audio with crossfade support |
| **VolumeControl** | Master, Music, SFX independent channels |
| **SpatialAudio** | Position-based inverse distance attenuation |
| **SoundBuffer** | Byte cache — reuse buffers, avoid per-play allocations |

### Scene System (`serialize` feature)

JSON scene/level serialization with `serde_json`.

| Module | What |
|--------|------|
| **Scene** | Named collection of `EntityPrefab`s with metadata |
| **EntityPrefab** | Template: component list with default values |
| **ComponentValue** | 11 variants matching all built-in components — round-trip JSON |
| **World integration** | `world.spawn_prefab(scene, prefab_name)` |

### Asset Pipeline (`dev-tools` feature)

| Module | What |
|--------|------|
| **Asset trait** | `fn load(path) -> Result<Self>` for any asset type |
| **AssetRegistry** | Path → loaded asset with handle-based access |
| **HotReloadWatcher** | notify 7 file watcher — reload changed assets at runtime |
| **AssetLoader** | File I/O with format detection, combines registry + watcher |

### Developer Overlay (`render` feature)

| Module | What |
|--------|------|
| **DevOverlay** | In-game toggleable dev tools panel |
| **EntityInspector** | Show all components on selected entity with live values (11 component types) |
| **StatsPanel** | FPS ring buffer, entity count, draw calls, memory |
| **DevConsole** | Command parser + capped log output |
| **SceneTree** | Hierarchical entity list with selection |

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                        Game Code                          │
│                  (your game logic here)                    │
├──────────────────────────────────────────────────────────┤
│                     Game Module                           │
│   Mercenaries │ Terrain │ Navigation │ Squads │ Camera   │
├──────────────────────────────────────────────────────────┤
│                   Developer Tools                         │
│   Dev Overlay │ Asset Pipeline │ Hot Reload │ Scene I/O  │
├──────────────────────────────────────────────────────────┤
│                    Subsystem Layer                         │
│  Rendering │ Audio │ Physics │ Lighting │ Skeletal │ Fog  │
├──────────────────────────────────────────────────────────┤
│                   System Pipeline                          │
│  Movement │ Health │ Collision │ Gravity │ Raycast │ AI   │
├──────────────────────────────────────────────────────────┤
│                     ECS Core                              │
│    Entity │ Component │ Storage │ World │ Archetypes      │
├──────────────────────────────────────────────────────────┤
│                Spatial Indexing                            │
│          Quadtree (2D) │ Octree (3D) │ AABB │ Ray         │
├──────────────────────────────────────────────────────────┤
│                   Schedulers                              │
│    GameLoop (variable) │ TickScheduler (deterministic)    │
├──────────────────────────────────────────────────────────┤
│                     EventBus                              │
│    Collision │ Damage │ Death │ RayHit │ Custom Events     │
└──────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# Clone
git clone https://github.com/synthalorian/chronos-engine.git
cd chronos-engine

# Terminal demos (no GPU needed):
cargo run

# GPU demo with rendering (needs a display):
cargo run --features render

# Everything:
cargo run --features full

# Run tests:
cargo test --features full

# Minimal ECS only:
cargo test
```

---

## Feature Flags

| Flag | What | Dependencies |
|------|------|-------------|
| *(default)* | ECS core, systems, spatial indexing, input, tilemaps, particles, lighting, skeletal animation, fog of war | **None** — pure `std` |
| `render` | 2D/3D rendering, UI, post-processing, editor overlay | wgpu 23, winit 0.30, bytemuck, rand, tokio, image |
| `serialize` | Scene/level serialization, entity prefabs | serde, serde_json |
| `audio` | Audio engine, spatial audio, music crossfade | rodio 0.20 |
| `dev-tools` | Asset pipeline, hot reload | notify 7, serde, serde_json |
| `game` | Chronos Company first game | render (transitive) |
| `full` | Everything above | all of the above |

---

## Project Structure

```
chronos-engine/
├── Cargo.toml              # Feature-gated deps
├── README.md               # This file
├── ROADMAP.md              # Full development plan
├── icon.png                # App icon
├── assets/
│   └── icon.png            # Icon source
├── src/
│   ├── lib.rs              # Public API — re-exports all modules
│   ├── entity.rs           # Generational entity IDs
│   ├── component.rs        # Component trait + 11 built-in types
│   ├── input.rs            # Input system — keyboard, mouse, gamepad, action bindings
│   ├── storage.rs          # Type-erased component storage
│   ├── world.rs            # World — entity lifecycle, archetypes, queries
│   ├── system.rs           # 8 systems, GameLoop, TickScheduler, EventBus
│   ├── spatial.rs          # Quadtree, AABB, Ray (2D)
│   ├── octree.rs           # Octree, AABB3D, Ray3D (3D)
│   ├── physics3d.rs        # 3D physics world + constraints
│   ├── tilemap.rs          # Chunked tile map with frustum culling
│   ├── particle.rs         # Particle emitter + presets
│   ├── obj_loader.rs       # Wavefront .obj parser
│   ├── lighting.rs         # 2D lighting + shadow casting
│   ├── skeletal.rs         # Skeletal animation + blending
│   ├── fog_of_war.rs       # Fog of war + line-of-sight
│   ├── render.rs           # 2D sprite batch renderer (render)
│   ├── render3d.rs         # 3D renderer with depth buffer (render)
│   ├── texture.rs          # Texture atlas (render)
│   ├── font.rs             # Bitmap font rendering (render)
│   ├── ui.rs               # UI widgets (render)
│   ├── postprocess.rs      # Post-processing pipeline (render)
│   ├── scene.rs            # Scene/level serialization (serialize)
│   ├── audio.rs            # Audio engine (audio)
│   ├── asset.rs            # Asset pipeline + hot reload (dev-tools)
│   ├── editor.rs           # Developer overlay (render)
│   ├── main.rs             # Battle Arena + RTS + Bullet Hell demos
│   └── game/               # Chronos Company — first game (game)
│       ├── mod.rs           # Game module root
│       ├── components.rs   # Game-specific components (11 types)
│       ├── mercenary.rs    # Mercenary factory + templates
│       ├── terrain.rs      # Terrain grid + heightmap
│       ├── navigation.rs   # A* pathfinding
│       ├── camera.rs       # Tabletop/isometric camera
│       ├── selection.rs    # Unit selection system
│       └── squad.rs        # Squad controller + 4 formations
└── tests/
    └── integration_tests.rs  # 25 integration tests
```

---

## Systems Reference

### MovementSystem
```rust
// Applies velocity to position every tick
Position.x += Velocity.x * dt;
Position.y += Velocity.y * dt;
```

### HealthSystem
```rust
// Damage → Health → Dead pipeline
if entity has Damage {
    health.current -= damage.0;
    emit DamageTaken(entity, amount);
    if health.current <= 0 {
        add Dead component;
        emit EntityDied(entity);
    }
    remove Damage;
}
```

### CollisionSystem
```rust
// Quadtree broad-phase + circle narrow-phase
// Insert all entities with Position + CircleRadius into quadtree
// Query collision pairs → check distance < r1 + r2
// Emit Collision events with cooldown to prevent re-trigger
```

### GravitySystem / PlatformerSystem
```rust
// Gravity: velocity.y += gravity * dt (acceleration, not force)
// Platformer: ground check, jump impulse, ground friction
```

### RaycastSystem
```rust
// Spatial index ray queries
// Returns sorted RaycastHit list (entity, distance, point)
```

---

## Chronos Company — First Game

The first game built on Chronos Engine. **A 3D real-time strategy open-world RPG sandbox.**

You command a band of mercenaries navigating an open world, taking job boards and bounties, moving as a unit, fighting in RTS-style tactical combat. Tabletop/isometric camera view. The world persists. Your company grows.

**Core pillars:**
- Open world traversal (3D, tabletop camera)
- RTS-style unit control (select, move, fight as a squad)
- RPG progression (mercenary stats, equipment, skills)
- Job board / bounty system (mission generation)
- Sandbox freedom (go anywhere, take any contract)

**Implemented (Phase 6A):**
- 11 game-specific components: `Selectable`, `Selected`, `MoveTarget`, `MercenaryStats`, `NavigationAgent`, `Team`, `SquadMember`, `HealthBar`, `AggroRadius`, `LootDrop`, `SelectionRing`
- `MercenaryFactory` with Warrior/Archer/Mage/Scout templates
- `TerrainGrid` with height data, walkability (Flat/Hill/Water/Wall/Path), procedural heightmap
- A* pathfinding on `TerrainGrid` with world-coordinate API
- `TabletopCamera` — spherical orbit, WASD pan, scroll zoom, auto-follow, screen-to-ray picking
- `SelectionManager` — click select, box select, toggle, max selection limit
- `SquadManager` with 4 formations (Line, Column, Circle, Wedge)

---

## How It Compares

| Aspect | Chronos Engine | Bevy | Legion / Hecs |
|--------|---------------|------|---------------|
| **Core Dependencies** | **Zero** — pure std | Heavy (wgpu, many crates) | Moderate |
| **Deterministic Simulation** | **First-class** — `TickScheduler` for lockstep | Not designed for lockstep | Not primary focus |
| **Game Systems** | Built-in — fog of war, skeletal animation, lighting, pathfinding, squad AI | Plugin ecosystem | ECS-only |
| **Physics** | Built-in 2D collision + 3D physics with constraints | Rapier / Avian | None |
| **RTS-Specific** | Tabletop camera, box selection, A*, formations, mercenary stats | No RTS modules | None |
| **Terminal Mode** | `DebugRenderSystem` — runs without GPU | Requires GPU | No rendering |
| **Safety** | **100% safe Rust** — zero `unsafe` in ECS/storage/spatial | Extensive unsafe for perf | Extensive unsafe |
| **Codebase Size** | ~13.5K lines — readable in an afternoon | Very large | Moderate |
| **Learning Curve** | Low — small, well-documented codebase | High — many abstractions | Moderate |

**The key differentiator:** Chronos Engine is a self-contained, zero-dependency ECS with a **complete game systems layer** that compiles with only the Rust standard library, yet scales to full wgpu rendering with 3D pipelines, post-processing, and audio. It prioritizes **deterministic lockstep simulation** for RTS/strategy games — a niche not well-served by existing engines.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full phased development plan.

```
Phase 1 — Core ECS                          ✅ Done
Phase 2 — Systems & Game Loop               ✅ Done
Phase 3 — Spatial Index & Physics           ✅ Done
Phase 4 — Rendering & Advanced Systems      ✅ Done
Phase 5 — Developer Experience              ✅ Done
Phase 6 — Chronos Company (first game)      🔄 WIP (6A done, 6B-6E TODO)
Phase 7 — Scripting & Modding               📋 TODO
Phase 8 — Networking                        📋 TODO
Phase 9 — Polish & Distribution             📋 TODO
```

---

## Architecture Principles

1. **Zero-dependency core** — the ECS itself is `std` only. Optional subsystems pull in their own deps and are feature-gated.

2. **Determinism is a feature** — `TickScheduler` guarantees same inputs → same outputs. This makes replays, lockstep multiplayer, and testing trivial.

3. **No unsafe** — not a single `unsafe` block in the engine. Type-erased storage uses `Box<dyn Any>` + `downcast`, safe by construction.

4. **Systems own nothing** — systems operate on World via queries. No state between ticks (except configuration). Testable, parallelizable, order-independent.

5. **Data-oriented by default** — components are flat data. Systems are transform functions. No inheritance, no virtual dispatch, no hidden state.

6. **Genre-agnostic** — the same ECS core powers RTS lockstep, platformer variable-timestep, RPG event-driven, and shooter pipelines. You bring the game logic.

---

## Stats

| Metric | Value |
|--------|-------|
| Lines of Rust | ~13,500 |
| Source files | 36 |
| Public types | 115+ |
| Unit tests | 85 |
| Integration tests | 25 |
| Total tests | 110 |
| `unsafe` blocks | 0 |
| Core dependencies | 0 |
| Optional dependencies | 8 (wgpu, winit, bytemuck, rand, tokio, image, serde, rodio, notify) |

---

<p align="center">
  Built with 🎹🦞 by <a href="https://github.com/synthalorian">synth</a><br>
  <em>Write the future in the present while preserving the past.</em>
</p>

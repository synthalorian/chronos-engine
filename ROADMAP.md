# Chronos Engine — Development Roadmap

> *"Write the future in the present while preserving the past."*

This is the master plan. Not a wishlist — a staged buildout of a genuinely multi-genre game engine. Each phase is gated: no phase N+1 until phase N ships and holds water.

---

## Legend

```
Done   Complete
WIP    In progress
TODO   Not started
HOLD   Deferred
```

---

## Phase 1 — Core ECS — Done

| Milestone | Status |
|-----------|--------|
| Generational entity IDs with slot reuse | Done |
| Component trait + built-in types | Done |
| Type-erased Box<dyn Any> storage | Done |
| StorageRegistry (TypeId → ComponentStorage) | Done |
| World (entity lifecycle, create/destroy/exists) | Done |
| Archetype tracking with migration | Done |
| Component attach/detach | Done |
| Typed queries (query, query_mut, query_with_all) | Done |

---

## Phase 2 — Systems & Game Loop — Done

| Milestone | Status |
|-----------|--------|
| System trait + MovementSystem | Done |
| HealthSystem (Damage → Health → Dead) | Done |
| CollisionSystem (quadtree-based) | Done |
| GravitySystem, PlatformerSystem | Done |
| RaycastSystem | Done |
| DeathCleanupSystem, DebugRenderSystem | Done |
| EventBus with 6 event types | Done |
| GameLoop (5-phase pipeline) | Done |
| TickScheduler (deterministic fixed-timestep) | Done |
| Battle arena demo + RTS scenario + Bullet Hell demo | Done |

---

## Phase 3 — Spatial Index & Physics — Done

| Milestone | Status |
|-----------|--------|
| Quadtree (2D) with cross-subtree collision fix | Done |
| Octree (3D) with sphere/AABB/ray queries | Done |
| Raycasting via spatial index | Done |
| AABB + circle narrow-phase collision | Done |
| RigidBody, Grounded, Gravity components | Done |
| 3D physics world with impulse response + constraints | Done |

---

## Phase 4 — Rendering & Advanced Systems — Done

### 4A — 2D Rendering

| Milestone | Status |
|-----------|--------|
| wgpu 23 sprite batch renderer | Done |
| Camera (orthographic, screen shake) | Done |
| TextureAtlas + AtlasFrame | Done |
| Tile map (chunked grids, frustum culling) | Done |
| Bitmap font rendering | Done |
| Particle system (explosion/smoke/trail presets) | Done |
| Parallax per sprite (WGSL shader) | Done |
| Layer sorting in SpriteBatch | Done |
| FPS counter | Done |

### 4B — UI System

| Milestone | Status |
|-----------|--------|
| Button, Slider, Label, Panel widgets | Done |
| Hit-testing + UiContext | Done |
| WidgetStyle presets (dark/light/accent) | Done |

### 4C — 3D Rendering

| Milestone | Status |
|-----------|--------|
| Renderer3D with depth buffer | Done |
| PerspectiveCamera (view/projection matrices) | Done |
| Mesh3D (cube/plane primitives) | Done |
| Transform3D (TRS → model matrix) | Done |
| Directional lighting + back-face culling | Done |
| Obj loader (Wavefront .obj parser) | Done |

### 4D — Advanced Systems

| Milestone | Status |
|-----------|--------|
| 2D lighting (Point/Directional/Spot/Area, shadows) | Done |
| Skeletal animation (joints, poses, blending) | Done |
| Fog of war (visibility grid, line-of-sight) | Done |
| Post-processing (color grading, bloom, vignette, CRT/noir/sunset) | Done |

---

## Phase 5 — Developer Experience — Done

**Make the engine usable by anyone. The goal: easier than Unity, Unreal, and Godot.**

This phase transforms Chronos from a powerful engine core into a developer-friendly platform. Every feature here exists to eliminate friction between "I have an idea" and "I have a working game."

### 5A — Input System (input.rs) — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| KeyCode / MouseButton / GamepadButton enums | Full key coverage for keyboard, mouse, gamepad | Done |
| Binding system | Map any input to named actions (e.g., "move_forward" → W, ArrowUp, LeftStickUp). Chainable `.or()` API. | Done |
| InputState tracking | pressed, just_pressed, just_released per action per frame via `ActionState` enum | Done |
| Axis support | Analog values for thumbsticks, mouse delta, scroll wheel. `AxisBinding` with positive/negative sources. | Done |
| InputManager | Orchestrator that processes raw events → action state. Per-frame `end_frame()` transition. | Done |
| Contexts / Maps | Swap input bindings per game state (menu vs gameplay vs console). `InputContext` with reverse source map. | Done |
| Mouse tracking | Position, delta, scroll — accumulated per-frame, reset on `end_frame()`. | Done |
| Gamepad axes | `GamepadAxis` enum (sticks, triggers) with `GamepadAxis` event handling. | Done |
| Unit tests | 7 tests: key binding, multi-source, axis, context switching, mouse, gamepad, chaining. | Done |

### 5B — Audio Engine (audio.rs) — feature: `audio` — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| AudioEngine wrapping rodio | Device init, output stream management | Done |
| SFX playback | Load .wav/.ogg, play one-shot sounds with volume | Done |
| Music streaming | Stream long audio files with crossfade support | Done |
| Volume channels | Master, Music, SFX independent volume control | Done |
| Spatial audio | Position-based attenuation for 3D sound | Done |
| Sound pooling | Reuse buffers, avoid per-play allocations | Done |
| Unit tests | 10 tests: volume control, spatial attenuation, buffer caching, state transitions | Done |

### 5C — Scene / Level System (scene.rs) — feature: `serialize` — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Scene struct | Named collection of EntityPrefabs with metadata | Done |
| EntityPrefab | Template: component list with default values | Done |
| JSON serialization | serde_json for scene files — human-readable, hand-editable | Done |
| Save/Load | `Scene::from_file()` / `Scene::save()` + World integration | Done |
| Component serialization | Serialize Position, Velocity, Health, etc. via custom serde impls | Done |
| Prefab spawning | `world.spawn_prefab(scene, prefab_name)` | Done |
| Unit tests | 10 tests: JSON roundtrip, file I/O, prefab spawn, error cases | Done |

### 5D — Asset Pipeline (asset.rs) — feature: `dev-tools` — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Asset trait | `fn load(path) -> Result<Self>` for any asset type | Done |
| AssetRegistry | Path → loaded asset with handle-based access | Done |
| Hot-reload watcher | notify-based file watcher, reload changed assets at runtime | Done |
| AssetLoader | File I/O with format detection, combines registry + watcher | Done |
| Cache + reference counting | Avoid redundant loads, unload unused assets | Done |
| Unit tests | 10 tests: ID generation, CRUD, type mismatch, watcher, loader integration | Done |

### 5E — Developer Overlay (editor.rs) — feature: `render` — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| DevOverlay | In-game toggleable dev tools panel | Done |
| Entity inspector | Show all components on selected entity with live values | Done |
| Stats panel | FPS, entity count, draw calls, memory | Done |
| Dev console | Command input + log output, accessible anytime | Done |
| Scene tree | Hierarchical entity list with selection | Done |
| Unit tests | 10 tests: FPS tracking, console commands, inspector, scene tree, toggle | Done |

### Feature gates

| Flag | Dependencies | Modules |
|------|-------------|---------|
| `serialize` | serde, serde_json | scene.rs |
| `audio` | rodio | audio.rs |
| `dev-tools` | notify, serde, serde_json | asset.rs, editor.rs |
| Core (always) | std only | input.rs |

---

## Phase 6 — Chronos Company (First Game) — WIP

**The first game built on Chronos Engine. Proves the engine in production.**

### Game Design

**Chronos Company** — a 3D real-time strategy open-world RPG sandbox.

You command a band of mercenaries navigating an open world, taking job boards and bounties, moving as a unit, fighting in RTS-style tactical combat. Tabletop/isometric camera view. The world persists. Your company grows.

**Core pillars:**
- Open world traversal (3D, tabletop camera)
- RTS-style unit control (select, move, fight as a squad)
- RPG progression (mercenary stats, equipment, skills)
- Job board / bounty system (mission generation)
- Sandbox freedom (go anywhere, take any contract)

### 6A — Foundation — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Game components | Selectable, Selected, MoveTarget, MercenaryStats, NavigationAgent, Team, SquadMember, HealthBar, AggroRadius, LootDrop | Done |
| Mercenary factory | MercenaryFactory with Warrior/Archer/Mage/Scout templates, squad creation helper | Done |
| Basic terrain | TerrainGrid with height data, walkability (Flat/Hill/Water/Wall/Path), procedural heightmap | Done |
| Ground navigation | A* pathfinding on TerrainGrid, NavigationPath waypoint traversal, world-coordinate API | Done |
| Camera system | TabletopCamera — spherical orbit, WASD pan, scroll zoom, auto-follow, screen-to-ray picking | Done |
| Squad controller | SelectionManager (click/box select), SquadManager with 4 formations (Line/Column/Circle/Wedge) | Done |
| Unit tests | 23 game tests: components, mercenary factory, terrain, navigation, camera, selection, squad | Done |

### 6B — Combat — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Enemy entities | Spawn, AI patrol, aggro radius, chase, state machine | Done |
| Combat system | Attack, damage (STR/DEX/INT scaling), health bars, death, melee + ranged + magic | Done |
| Ability system | Per-unit abilities with cooldowns, mana, 6 ability types, 4 slots per unit | Done |
| Loot drops | InventoryItem, LootPickup, LootSpawner, gold stacking, auto-pickup, despawn | Done |
| Formation during combat | Squad keeps formation during movement | Done |

### 6C — RPG Systems — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Mercenary stats | STR/DEX/INT/VIT, leveling, XP, stat growth, allocation | Done |
| Equipment system | 7 equipment slots, stat bonuses, equip/unequip, level gating | Done |
| Inventory | Item management, stacking, sorting, filtering, drag-drop | Done |
| Job board | Procedural bounty/contract generation, 6 job types, 5 difficulty tiers | Done |
| Dialogue system | NPC conversations, branching choices, condition gates | Done |
| Faction reputation | Standing with different factions, pricing modifiers | Done |

### 6D — Open World — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| World map | Large traversable area with regions, procedural biomes, exploration | Done |
| Points of interest | Towns, dungeons, camps, shrines, discovery system | Done |
| Day/night cycle | Time progression with 6-phase lighting | Done |
| Encounters | Random battles, ambushes, deterministic spawning, difficulty scaling | Done |
| Save/load | Persistent world state, versioning, checksums, auto-save | Done |
| Minimap | Explored/fog cells, POI markers, enemy markers, terrain colors | Done |

### 6E — Polish — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| HUD overlay | Health/mana/XP bars, tooltips, notifications, squad panel | Done |
| Screen manager | Menu stack, transitions, button layouts, presets | Done |
| Visual effects | 16 effect types, particle profiles, spatial effect system | Done |
| Ambience system | Sound zones, music triggers, footstep tracking | Done |
| Tutorial system | Objectives, sequences, hint registry, guided presets | Done |

---

## The Vision

Chronos Engine is becoming an **open-source, general-purpose game engine** — a real alternative to Unity and Unreal.

**The goal:** A cross-platform desktop editor application (Linux/Windows/macOS) where anyone can build any kind of game. The engine ships with a built-in scripting language, visual editor, asset pipeline, and networking. Chronos Company (our RPG) becomes the proof-of-concept — the first game built *in* Chronos Editor.

**Platforms:**
- Linux (Arch, Ubuntu, Fedora — AppImage + native packages)
- Windows 10/11 (MSI installer)
- macOS 12+ (.app bundle, Universal Binary)

**Technology:**
- Rust engine core (zero unsafe, deterministic ECS)
- wgpu rendering (Vulkan / Metal / DirectX 12)
- egui editor UI (immediate-mode, cross-platform)
- Rhai scripting (Rust-native, no FFI overhead)
- winit windowing (cross-platform)

---

## Phase 7 — Editor Application (~6 weeks)

**The desktop application. Open Chronos, see a window, build a game.**

### 7A — Window & Rendering Foundation

| Milestone | Description | Status |
|-----------|-------------|--------|
| Editor binary | `chronos-editor` crate with main() that opens a winit window | Done |
| wgpu surface | Initialize wgpu adapter/device/queue, render to window | Done |
| egui integration | egui + wgpu backend, immediate-mode UI rendering every frame | Done |
| DPI awareness | Handle HiDPI/Retina scaling across platforms | Done |
| Event loop | winit 0.30 ApplicationHandler event loop → egui input → render, 60fps target | Done |

### 7B — Editor Panels

| Milestone | Description | Status |
|-----------|-------------|--------|
| Scene viewport | CentralPanel with camera controls (orbit/pan/zoom), grid overlay, FPS counter | Done |
| Hierarchy panel | Left SidePanel, entity tree with add/remove, selection sync, search filter | Done |
| Inspector panel | Right SidePanel, component property editor with drag sliders for all 11 component types | Done |
| Asset browser | Bottom SidePanel, file system browser with list/grid views, type detection, navigation | Done |
| Console panel | Bottom SidePanel, log output with severity filters, command input (help/clear/echo/entities) | Done |
| Toolbar | Top panel, Play/Pause/Stop, Translate/Rotate/Scale mode, snap toggle, keyboard shortcuts | Done |
| Menu bar | Top TopBottomPanel, File/Edit/View/Help menus with shortcuts dialog and about dialog | Done |

### 7C — Editor Workspace — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Undo/redo system | Command pattern with dual-stack architecture, type-erased `Box<dyn EditorCommand>`, move/create/destroy/modify commands | Done |
| Grid rendering | Infinite ground grid with axis coloring (X=red, Y=green), configurable spacing, snap-to-grid | Done |
| Gizmo system | Translate/Rotate/Scale gizmos in viewport, mouse drag to produce deltas, axis hit-testing | Done |
| Selection system | Click-pick (ray casting), box select, multi-select via `ViewportSelector` | Done |
| Keyboard shortcuts | Configurable keybindings with Blender-style defaults, `ShortcutMap` with `ShortcutAction` enum | Done |
| Settings dialog | `EditorSettings` struct with rendering/editor/shortcuts tabs, `apply_clamp()` for validation | Done |
| Editor integration | Wire workspace modules into `EditorApp` struct and `render()` loop | Done |
| Panel docking | Resizable, dockable panels. Save/restore layout. | Done |

### 7D — Project Management — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Project format | `.chronos` project directory with manifest (scenes, assets, scripts, settings), serde JSON serialization | Done |
| New project wizard | Template selection (Empty, 2D Platformer, 3D Shooter, RPG) via `ProjectTemplate` enum with preset scenes | Done |
| Open/Save project | `ProjectManager` with open/save/save_as/close/validate, directory structure creation, manifest roundtrip | Done |
| Recent projects | `RecentProject` tracking, welcome screen with template selector + recent list, editor integration | Done |

---

## Phase 8 — Engine Generalization (~4 weeks)

**Extract genre-agnostic systems. Every game type becomes a first-class citizen.**

### 8A — 2D Physics — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| AABB collisions | Axis-aligned bounding box intersection tests | Done |
| Circle collisions | Circle-circle and circle-AABB narrow phase | Done |
| 2D Rigid body | Position, velocity, mass, restitution, friction | Done |
| 2D Physics world | Step simulation, gravity, solver iterations | Done |
| Raycasting 2D | Ray vs AABB/circle queries | Done |
| Contact solver | Impulse-based collision response | Done |

### 8B — Generic Animation — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Animation state machine | States, transitions, parameters (bool/float/trigger) | Done |
| Animation blend tree | 1D/2D blending (idle→walk→run), additive layers | Done |
| Sprite animation | Sprite sheet flipbook, frame events | Done |
| Timeline system | Keyframe interpolation (linear/bezier/step), event tracks | Done |

### 8C — Material & Shader System — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| Material definition | Albedo, normal, metallic, roughness, emissive, opacity | Done |
| Shader graph data | Node-based shader description (data, not visual editor yet) | Done |
| Built-in shaders | Unlit, PBR standard, sprite, particle, UI, skybox, terrain | Done |
| Shader hot-reload | Watch shader files, recompile on save | Done |

### 8D — General Systems — Done

| Milestone | Description | Status |
|-----------|-------------|--------|
| 2D Camera | Orthographic camera with shake, follow, bounds | Done |
| Tilemap system | Chunked tilemap with layers, collision tiles, autotile | Done |
| Pathfinding 2D | A* on tilemap grids with variable cost | Done |
| Audio zones | Spatial audio regions, reverb zones, occlusion | Done |

---

## Phase 9 — Scripting & Modding (~3 weeks) — Done

**Make the engine programmable without touching Rust.**

| Milestone | Description | Status |
|-----------|-------------|--------|
| Rhai engine bridge | Register ECS types (Entity, World, Vec3) with Rhai | Done |
| Script components | `ScriptComponent` — attach Rhai scripts to entities | Done |
| Script lifecycle | `on_start`, `on_update`, `on_destroy`, `on_collision` hooks | Done |
| Script API: entities | Create/destroy entities, attach/detach components from scripts | Done |
| Script API: query | Query entities by component from scripts | Done |
| Script API: input | Read keyboard/mouse/gamepad state from scripts | Done |
| Script API: audio | Play sounds, control music from scripts | Done |
| Script API: events | Emit/listen for custom events from scripts | Done |
| Script API: physics | Raycasts, collision queries, force application | Done |
| Script API: scene | Load scenes, instantiate prefabs from scripts | Done |
| Hot-reload scripts | Watch script files, recompile and hot-swap on save | Done |
| Script debugging | Print statements, error stack traces, editor console integration | Done |
| Mod loading | Programmatic mod loading with ModBuilder, mod metadata | Done |
| Mod API | Expose engine systems to mods, sandboxing | Done |

---

## Phase 10 — Asset Pipeline (~3 weeks)

**Import any asset. Process once, cache forever.**

| Milestone | Description | Status |
|-----------|-------------|--------|
| glTF importer | Scenes, meshes, materials, animations, skins | Done |
| OBJ importer | Wavefront meshes (already exists), improve with materials | TODO |
| Image importer | PNG, JPG, BMP, TGA → GPU textures with mipmaps | Done |
| Audio importer | WAV, OGG, MP3, FLAC → engine audio buffers | Done |
| Font importer | TTF/OTF → bitmap font atlases | Done |
| Asset metadata | `.meta` files alongside assets (import settings, GUIDs) | Done |
| Asset registry | GUID-based lookup, reference counting, garbage collection | Done |
| Asset processing | Background import pipeline, cache processed assets | TODO |
| Thumbnail generation | Automatic thumbnails for asset browser | TODO |
| Asset hot-reload | Watch source files, re-import on change | TODO |

---

## Phase 11 — Cross-Platform Distribution (~3 weeks)

**Ship on every platform. One engine, three targets.**

### 11A — Platform Support

| Milestone | Description | Status |
|-----------|-------------|--------|
| Linux build | AppImage (universal), PKGBUILD for Arch AUR | TODO |
| Windows build | MSVC toolchain, MSI installer | TODO |
| macOS build | Universal binary (x86_64 + aarch64), .app bundle, DMG | TODO |
| Platform abstractions | File paths, dialogs, input handling per-platform | TODO |

### 11B — CI/CD & Quality

| Milestone | Description | Status |
|-----------|-------------|--------|
| GitHub Actions | Build + test on Linux/Windows/macOS for every push | TODO |
| Release automation | Tag → build all platforms → attach to GitHub Release | TODO |
| Error handling | No unwrap/expect in engine code, proper error types everywhere | TODO |
| Profiling | Frame profiler, system timing, performance overlays | TODO |
| Benchmark suite | Entity throughput, component ops/sec, rendering benchmarks | TODO |
| rustdoc | Public API documentation for all engine modules | TODO |
| Clippy clean | Zero clippy warnings across all features | TODO |

### 11C — WASM Target

| Milestone | Description | Status |
|-----------|-------------|--------|
| WASM compilation | wasm-pack build, web-compatible rendering | TODO |
| WebGL2 renderer | wgpu WebGL2 backend for browser support | TODO |
| Web audio | Web Audio API bridge for browser audio | TODO |
| Web input | Keyboard/mouse/touch for browser games | TODO |

---

## Phase 12 — Networking (~4 weeks)

**Multiplayer that works. Deterministic, rollback, lag-free.**

| Milestone | Description | Status |
|-----------|-------------|--------|
| UDP transport | Cross-platform UDP socket with reliability layer | TODO |
| Packet protocol | Channel-based message protocol, serialization | TODO |
| Connection manager | Connect, disconnect, heartbeat, timeout detection | TODO |
| Deterministic lockstep | Synchronized TickScheduler, input buffering | TODO |
| Rollback netcode | GGPO-style state snapshots, re-simulate on late input | TODO |
| Lag compensation | Client-side prediction, server reconciliation | TODO |
| Lobby system | Host/join/list games, NAT traversal | TODO |
| Networked entities | Sync transforms, animations, state across clients | TODO |
| Voice chat | Opus-based voice communication (stretch goal) | TODO |

---

## Phase 13 — Plugin System (~3 weeks)

**Third-party extensions. The ecosystem grows.**

| Milestone | Description | Status |
|-----------|-------------|--------|
| Plugin format | `.chronos-plugin` zip with manifest, WASM or native | TODO |
| Plugin loader | Discover, load, initialize plugins at startup | TODO |
| Plugin API | Engine interfaces exposed to plugins (limited surface) | TODO |
| Editor plugins | Custom panels, inspectors, toolbars from plugins | TODO |
| Template projects | Starter templates: 2D platformer, 3D FPS, RPG, puzzle, racing | TODO |
| CLI tool | `chronos` command: new, build, run, package, publish | TODO |
| Package registry | Plugin/package discovery and installation | TODO |

---

## Phase 14 — Chronos Company Demo (~4 weeks)

**The proof. Build the RPG inside the editor.**

| Milestone | Description | Status |
|-----------|-------------|--------|
| Project setup | Create Chronos Company project in editor, configure settings | TODO |
| World building | Use terrain tools to build the open world map | TODO |
| Character creation | Build mercenaries in editor, set up stats and abilities | TODO |
| Scene composition | Place towns, dungeons, camps, enemies via editor | TODO |
| Script gameplay | Write NPC AI, combat logic, quest system in Rhai scripts | TODO |
| Audio design | Import and place ambient zones, combat music, SFX | TODO |
| UI design | Build HUD, menus, inventory screen with editor UI tools | TODO |
| Effects | Set up particle effects for combat, level-up, environment | TODO |
| Save/load | Wire up save/load system with editor integration | TODO |
| Polish | Post-processing, lighting, day/night cycle fine-tuning | TODO |
| Ship | Package as standalone game, distribute with engine | TODO |

---

## Timeline Summary

```
Phase 1  — Core ECS                          Done
Phase 2  — Systems & Game Loop               Done
Phase 3  — Spatial Index & Physics           Done
Phase 4  — Rendering & Advanced Systems      Done
Phase 5  — Developer Experience              Done
Phase 6  — Chronos Company (game modules)    Done
Phase 7  — Editor Application                Done
Phase 8  — Engine Generalization             Done
Phase 9  — Scripting & Modding               Done
Phase 10 — Asset Pipeline                    TODO
Phase 11 — Cross-Platform Distribution       TODO
Phase 12 — Networking                        TODO
Phase 13 — Plugin System                     TODO
Phase 14 — Chronos Company Demo              TODO
```

---

## Session State (May 27, 2026)

- **~48,000 lines** across **89 source files** (48 engine + 28 game + 8 editor workspace + 2 editor project + 3 editor panels/app/binary + 7 scripting)
- **906 tests passing** (881 unit + 25 integration)
- `cargo build --features full` compiles clean (41 pre-existing warnings only)
- **Phase 1–9 COMPLETE.** Engine core + full RPG game + editor + engine generalization + scripting & modding done.

### What's new this session — Phase 7E + Phase 8 + Phase 9

| Module | File | LOC | Tests | What |
|--------|------|-----|-------|------|
| `docking.rs` | DockNode, DockLayout, DockState, DockZone, DragState, SplitDirection, DockError | 1,356 | 32 | Panel docking system with tree layout, serialize/deserialize, drag-drop |
| `physics2d.rs` | Vec2, Collider2D, RigidBody2D, Contact2D, Ray2D, RayHit2D, PhysicsWorld2D | 1,344 | 24 | Full 2D physics with AABB/circle collisions, impulse solver, raycasting |
| `animation.rs` | AnimStateMachine, AnimState, AnimTransition, BlendTree, SpriteAnimation, Timeline | 1,484 | 29 | Animation state machine, blend trees, sprite flipbook, keyframe timeline |
| `material.rs` | MaterialDefinition, MaterialValue, RenderState, CompiledMaterial | 757 | 14 | Material system with 7 built-in presets (unlit, PBR, sprite, particle, UI, skybox, terrain) |
| `shader.rs` | ShaderGraph, ShaderNode (28 types), ShaderWatcher | 1,075 | 21 | Shader graph data model, WGSL generation, hot-reload watcher |
| `general_systems.rs` | Camera2D, TilemapEx, Pathfinder2D, AudioZoneManager, FootstepSystem | 1,198 | 21 | 2D camera, layered tilemap with autotile, A* pathfinding, audio zones |
| `bridge.rs` | ScriptEngine, Rhai type registration (Vec2, Vec3, Entity, transforms) | 664 | 12 | Rhai engine bridge — register ECS types, compile/eval scripts |
| `component.rs` | ScriptComponent, ScriptHandle, ScriptRegistry | 395 | 10 | Attach Rhai scripts to entities, per-entity state management |
| `lifecycle.rs` | ScriptLifecycle, ScriptContext, ScriptHook (on_start/on_update/on_destroy/on_collision) | 577 | 14 | Script lifecycle hooks with delta time, entity context, collision data |
| `api.rs` | ScriptApi (math, entity, debug, time, input functions for Rhai) | 595 | 12 | Full scripting API — vec math, entity CRUD, debug logging, time, input state |
| `hotreload.rs` | ScriptWatcher, ScriptReloader, ReloadPolicy | 861 | 18 | Polling-based script hot-reload with configurable policies |
| `modloader.rs` | ModLoader, ModMetadata, ModBuilder, ModError | 1,087 | 18 | Programmatic mod loading with metadata, dependency resolution, sandboxing |

### Build & test commands

```bash
cargo build --features full    # Should compile clean
cargo test --features full     # Should pass 906 tests
```

### Engine Stats

| Metric | Value |
|--------|-------|
| Source files | 89 |
| Engine modules | 48 |
| Game modules | 28 |
| Scripting modules | 6 (+ mod.rs) |
| Editor panels | 8 (7 + welcome) |
| Editor workspace modules | 7 (+ mod.rs) |
| Editor project modules | 1 (+ mod.rs) |
| Total tests | 906 |
| `unsafe` blocks | 0 |
| Core dependencies | 0 |

### Next — Phase 10: Asset Pipeline

| Milestone | Description | Status |
|-----------|-------------|--------|
| glTF importer | Scenes, meshes, materials, animations, skins | TODO |
| Image importer | PNG, JPG, BMP, TGA → GPU textures with mipmaps | TODO |
| Audio importer | WAV, OGG, MP3, FLAC → engine audio buffers | TODO |
| Font importer | TTF/OTF → bitmap font atlases | TODO |
| Asset metadata | `.meta` files alongside assets (import settings, GUIDs) | TODO |
| Asset registry | GUID-based lookup, reference counting, garbage collection | TODO |

### New Dependencies (planned)

| Crate | Purpose | Phase |
|-------|---------|-------|
| `gltf` | glTF scene importer | 10 |
| `symphonia` | Multi-format audio decoding | 10 |
| `image` | Image loading/processing | 10 |
| `ab_glyph` | Font rasterization | 10 |

---

## Architecture Principles

1. **Zero-dependency core** — the ECS itself is `std` only. Optional subsystems (rendering, audio, networking) pull in their own deps and are feature-gated.

2. **Determinism is a feature** — the TickScheduler guarantees same inputs → same outputs. This makes replays, lockstep multiplayer, and testing trivial.

3. **No unsafe** — not a single `unsafe` block in the engine. Type-erased storage uses `Box<dyn Any>` + `downcast`, which is safe by construction.

4. **Systems own nothing** — systems operate on World via queries. They don't hold state between ticks (except configuration). This makes them testable, parallelizable, and order-independent.

5. **Data-oriented by default** — components are flat data. Systems are transform functions. No inheritance, no virtual dispatch, no hidden state.

---

Built with 🎹🦞 by synth. Write the future in the present while preserving the past.

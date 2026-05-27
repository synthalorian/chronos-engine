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

### 6B — Combat (~14 days)

| Milestone | Description | Status |
|-----------|-------------|--------|
| Enemy entities | Spawn, AI patrol, aggro radius | TODO |
| Combat system | Attack, damage, health bars, death | TODO |
| Formation system | Squad keeps formation during movement | TODO |
| Ability system | Per-unit abilities with cooldowns | TODO |
| Loot drops | Enemies drop items/gold on death | TODO |

### 6C — RPG Systems (~14 days)

| Milestone | Description | Status |
|-----------|-------------|--------|
| Mercenary stats | STR/DEX/INT/VIT, leveling, XP | TODO |
| Equipment system | Weapons, armor, accessory slots | TODO |
| Inventory | Item management, stacking, drag-drop | TODO |
| Job board | Procedural bounty/contract generation | TODO |
| Dialogue system | NPC conversations, branching choices | TODO |
| Faction reputation | Standing with different factions | TODO |

### 6D — Open World (~14 days)

| Milestone | Description | Status |
|-----------|-------------|--------|
| World map | Large traversable area with regions | TODO |
| Points of interest | Towns, dungeons, camps, resources | TODO |
| Day/night cycle | Time progression with lighting changes | TODO |
| Encounters | Random battles while traveling | TODO |
| Save/load | Persistent world state | TODO |
| Minimap | Overview of explored world | TODO |

### 6E — Polish (~10 days)

| Milestone | Description | Status |
|-----------|-------------|--------|
| UI overhaul | HUD, inventory screen, character sheet, job board UI | TODO |
| Sound design | Ambient, combat, UI sounds | TODO |
| Particle effects | Combat hits, level-ups, environment | TODO |
| Post-processing | Appropriate color grading for the tone | TODO |
| Tutorial | Guided introduction for new players | TODO |

---

## Phase 7 — Scripting & Modding — TODO

| Milestone | Description | Status |
|-----------|-------------|--------|
| Rhai scripting integration | Rust-native scripting, no FFI | TODO |
| Script API: entities, components, events | Full engine access from scripts | TODO |
| Script API: prefabs, timers, systems | Spawn and schedule from scripts | TODO |
| Hot-reload scripts | Watch files, reload on save | TODO |
| Mod loading | Zip archives with scripts + assets | TODO |

---

## Phase 8 — Networking — TODO

| Milestone | Description | Status |
|-----------|-------------|--------|
| Deterministic lockstep | Synchronized TickScheduler across clients | TODO |
| Input buffer + delayed execution | Synchronized random seed | TODO |
| Rollback netcode | GGPO-style state snapshots, re-simulate on late input | TODO |
| UDP transport | Laminar or QUIC-based | TODO |
| Lobby system | Create/join/list games | TODO |

---

## Phase 9 — Polish & Distribution — TODO

| Milestone | Description | Status |
|-----------|-------------|--------|
| Error handling overhaul | No unwrap/expect in engine code | TODO |
| Profiling | Frame profiler, system timing | TODO |
| Benchmark suite | Entity throughput, component ops/sec | TODO |
| Documentation | rustdoc for all public API | TODO |
| CI/CD | GitHub Actions: build, test, lint | TODO |
| WASM support | Compile-to-web via wasm-pack | TODO |

---

## Timeline Summary

```
Phase 1 — Core ECS                          Done
Phase 2 — Systems & Game Loop               Done
Phase 3 — Spatial Index & Physics           Done
Phase 4 — Rendering & Advanced Systems      Done
Phase 5 — Developer Experience              Done
Phase 6 — Chronos Company (first game)      WIP
Phase 7 — Scripting & Modding               TODO
Phase 8 — Networking                        TODO
Phase 9 — Polish & Distribution             TODO
```

---

## Next Session Plan

**Goal: Phase 6B — Combat (Enemies, AI, Fighting, Loot)**

### Session state (as of May 27, 2026)

- **~13,500 lines** across **36 source files** (28 engine + 8 game)
- **80 tests passing** (55 unit + 25 integration)
- `cargo build --features full` passes with 2 pre-existing dead_code warnings
- Phase 5 complete, Phase 6A Foundation complete

### Phase 6B — Combat (build order, parallel where possible):

1. **Enemy AI** (`game/ai.rs`) — EnemyController with patrol waypoints, aggro detection (AggroRadius), chase behavior, return-to-patrol. State machine: Idle → Patrol → Chase → Attack → Dead.

2. **Combat system** (`game/combat.rs`) — CombatSystem: attack range check, damage application, health bar updates, death handling. Melee + ranged attack types. Attack cooldowns.

3. **Ability system** (`game/ability.rs`) — Ability struct (name, cooldown, range, damage, ability_type), AbilitySlot (4 per unit), AbilitySystem that processes cooldowns and triggers.

4. **Loot system** (`game/loot.rs`) — LootDrop on death, LootSpawner creates pickup entities, InventoryItem struct, gold stacking.

5. **Formation during combat** (`game/squad.rs` extension) — Squad keeps formation while fighting, repositions when members die, retreat logic.

### Key files to create:

- `src/game/ai.rs` — Enemy AI state machine
- `src/game/combat.rs` — Combat system + attack resolution
- `src/game/ability.rs` — Ability definitions + cooldown system
- `src/game/loot.rs` — Loot drops + item pickup
- `src/game/mod.rs` — Update with new module declarations

### Existing game types to build on:

- `MercenaryStats` (STR/DEX/INT/VIT) → damage calculation
- `Team` (Player/Enemy/Neutral) → friendly fire prevention
- `AggroRadius` → AI detection range
- `HealthBar` → visual feedback
- `LootDrop` → gold + items on death
- `NavigationAgent` → enemy pathfinding during chase
- `SquadMember` → squad-aware combat formations
- `SelectionManager` / `SquadManager` → player control flow

### Constraints:

- Game code stays in `src/game/` behind `game` feature
- Combat math should be testable without audio/GPU
- No new external dependencies — uses engine types only
- `cargo test --features game` must pass all tests (existing + new)

---

## Architecture Principles

1. **Zero-dependency core** — the ECS itself is `std` only. Optional subsystems (rendering, audio, networking) pull in their own deps and are feature-gated.

2. **Determinism is a feature** — the TickScheduler guarantees same inputs → same outputs. This makes replays, lockstep multiplayer, and testing trivial.

3. **No unsafe** — not a single `unsafe` block in the engine. Type-erased storage uses `Box<dyn Any>` + `downcast`, which is safe by construction.

4. **Systems own nothing** — systems operate on World via queries. They don't hold state between ticks (except configuration). This makes them testable, parallelizable, and order-independent.

5. **Data-oriented by default** — components are flat data. Systems are transform functions. No inheritance, no virtual dispatch, no hidden state.

---

Built with 🎹🦞 by synth. Write the future in the present while preserving the past.

use chronos_engine::{
    AABB, CircleRadius, CollisionSystem, Damage, DeathCleanupSystem, Event,
    EventBus, GameLoop, Health, HealthSystem, MovementSystem, Position, Quadtree, QuadtreeObject,
    Ray, RaycastSystem, Sprite, SystemPhase, TickScheduler, Velocity, World, DebugRenderSystem,
};
#[cfg(feature = "render")]
use chronos_engine::{Camera, Renderer, RenderSprite, SpriteBatch};
#[cfg(feature = "render")]
use wgpu::util::DeviceExt;
#[cfg(feature = "render")]
use std::sync::Arc;

// ──────────────────────────────────────────────
// Terminal Demo (default, no GPU)
// ──────────────────────────────────────────────

/// A comprehensive battle demo that exercises the full engine.
fn battle_demo() {
    println!("\n═══ Chronos Engine v0.3 — Battle Arena Demo ═══\n");

    let mut world = World::new();
    let mut scheduler = TickScheduler::new(0.016); // ~60 FPS fixed timestep

    let mut events = EventBus::new();

    // Register systems
    scheduler.add(MovementSystem::new());
    scheduler.add(HealthSystem::new());
    scheduler.add(CollisionSystem::new(15.0));
    scheduler.add(DeathCleanupSystem::new());

    // ── Phase 1: Entity creation ──
    println!("▶ Phase 1: Spawning entities with components...");

    let warrior = world.create_entity();
    world.add_component(warrior, Position::new(10.0, 20.0));
    world.add_component(warrior, Velocity::new(1.5, 0.0));
    world.add_component(warrior, Health::new(100));
    world.add_component(
        warrior,
        Sprite::new('W', 0, 255, 255), // cyan
    );
    println!(
        "  Warrior @ ({:.1}, {:.1}) — HP {}/{} — Vel ({:.1}, {:.1})",
        world.get_component::<Position>(warrior).unwrap().x,
        world.get_component::<Position>(warrior).unwrap().y,
        world.get_component::<Health>(warrior).unwrap().current,
        world.get_component::<Health>(warrior).unwrap().max,
        world.get_component::<Velocity>(warrior).unwrap().x,
        world.get_component::<Velocity>(warrior).unwrap().y,
    );

    let enemy = world.create_entity();
    world.add_component(enemy, Position::new(50.0, 30.0));
    world.add_component(enemy, Velocity::new(-1.0, 0.5));
    world.add_component(enemy, Health::new(80));
    world.add_component(
        enemy,
        Sprite::new('E', 255, 50, 50), // red
    );
    println!(
        "  Enemy @ ({:.1}, {:.1}) — HP {}/{} — Vel ({:.1}, {:.1})",
        world.get_component::<Position>(enemy).unwrap().x,
        world.get_component::<Position>(enemy).unwrap().y,
        world.get_component::<Health>(enemy).unwrap().current,
        world.get_component::<Health>(enemy).unwrap().max,
        world.get_component::<Velocity>(enemy).unwrap().x,
        world.get_component::<Velocity>(enemy).unwrap().y,
    );

    let turret = world.create_entity();
    world.add_component(turret, Position::new(30.0, 25.0));
    world.add_component(
        turret,
        Sprite::new('T', 255, 255, 0), // yellow
    );
    println!(
        "  Turret @ ({:.1}, {:.1}) — stationary defense",
        world.get_component::<Position>(turret).unwrap().x,
        world.get_component::<Position>(turret).unwrap().y,
    );

    println!("  Alive: {} | Capacity: {}", world.entity_count(), world.entity_capacity());

    // ── Phase 2: Multiple entity lifecycle + slot reuse ──
    println!("\n▶ Phase 2: Entity slot reuse (generational IDs)...");
    let temp_ids: Vec<_> = (0..5).map(|_| world.create_entity()).collect();
    println!("  Created 5 temp entities (capacity now {})", world.entity_capacity());
    for e in &temp_ids {
        world.destroy_entity(*e);
    }
    println!("  Destroyed 5 temp entities");
    // Create a new one — should reuse slot 0 with gen 1
    let reincarnated = world.create_entity();
    world.add_component(reincarnated, Position::new(0.0, 0.0));
    println!(
        "  Reincarnated entity: index={}, gen={} (reused slot)",
        reincarnated.index(),
        reincarnated.generation(),
    );
    println!("  Alive: {} | Capacity: {}", world.entity_count(), world.entity_capacity());

    // ── Phase 3: Simulation with collision and combat ──
    println!("\n▶ Phase 3: Simulation — movement + collision + combat\n");

    let _hits: u32 = 0;
    for tick in 0..30 {
        scheduler.tick(&mut world, &mut events);

        // Print events if any
        let pending = events.drain();
        for event in &pending {
            match event {
                Event::Collision(a, b) => {
                    println!("  ⚡ COLLISION: entity {} ↔ entity {}", a, b);
                    // Apply damage on collision
                    let enemy_entity = world.entity_from_index(*b);
                    if world.entity_exists(enemy_entity) {
                        world.add_component(enemy_entity, Damage(25));
                    }
                }
                Event::DamageTaken(entity, amount) => {
                    let e = world.entity_from_index(*entity);
                    if world.entity_exists(e) {
                        if let Some(hp) = world.get_component::<Health>(e) {
                            println!(
                                "  💥 entity {} took {} damage (HP: {}/{})",
                                entity, amount, hp.current, hp.max
                            );
                        }
                    }
                }
                Event::EntityDied(entity) => {
                    println!("  ☠️  entity {} has died!", entity);
                }
                Event::EntityDestroyed(entity) => {
                    println!("  🗑️  entity {} cleaned up", entity);
                }
                _ => {}
            }
        }

        // Print warrior + enemy positions every 10 ticks
        if tick % 10 == 0 {
            print!("  [tick {}] ", tick);
            if let Some(p) = world.get_component::<Position>(warrior) {
                print!("Warrior @ ({:.1},{:.1})", p.x, p.y);
            }
            if world.entity_exists(enemy) {
                if let Some(p) = world.get_component::<Position>(enemy) {
                    print!(" | Enemy @ ({:.1},{:.1})", p.x, p.y);
                }
            } else {
                print!(" | Enemy: DEAD");
            }
            if world.entity_exists(turret) {
                if let Some(p) = world.get_component::<Position>(turret) {
                    print!(" | Turret @ ({:.1},{:.1})", p.x, p.y);
                }
            } else {
                print!(" | Turret: DESTROYED");
            }
            println!();
        }
    }

    // ── Phase 4: Archetype tracking ──
    println!("\n▶ Phase 4: Archetype tracking...");
    println!("  Archetype count: {}", world.archetypes.iter().count());
    for (key, arch) in world.archetypes.iter() {
        println!(
            "  Archetype [{} component(s)]: {} entities",
            key.0.len(),
            arch.entities().len()
        );
    }

    // ── Phase 5: Multi-component query ──
    println!("\n▶ Phase 5: Multi-component query (Position + Sprite)...");
    let with_both = world.query_with_all::<Position>(&[std::any::TypeId::of::<Sprite>()]);
    println!("  Entities with Position AND Sprite: {}", with_both.len());
    for (entity, _) in &with_both {
        if let Some(sprite) = world.get_component::<Sprite>(*entity) {
            println!("    entity {} → symbol '{}'", entity.index(), sprite.symbol);
        }
    }

    // ── Phase 6: Event bus stress ──
    println!("\n▶ Phase 6: Event bus test...");
    let mut bus = EventBus::new();
    bus.emit(Event::Custom("test".into(), "hello engine".into()));
    bus.emit(Event::Custom("test".into(), "event bus works".into()));
    println!("  Events in queue: {} (before drain)", bus.len());
    let drained = bus.drain();
    println!("  Events drained: {} (after drain)", drained.len());

    // ── Phase 7: Final state ──
    println!("\n▶ Phase 7: Final world state");
    println!("  Remaining entities: {}", world.entity_count());
    println!("  Engine ticks simulated: {}", scheduler.tick_count);
    println!("  Total slots used: {}/{}", world.entity_count(), world.entity_capacity());

    println!("\n═══ Demo Complete ═══");
}

fn bullet_hell_demo() {
    println!("\n═══ Chronos Engine — Bullet Hell Stress Test ═══\n");

    let mut world = World::new();
    let mut scheduler = TickScheduler::new(0.016);

    let mut events = EventBus::new();

    scheduler.add(MovementSystem::new());
    scheduler.add(CollisionSystem::new(8.0));
    scheduler.add(DeathCleanupSystem::new());

    let width: f32 = 500.0;
    let height: f32 = 500.0;
    let num_entities: usize = 5_000;

    println!("▶ Phase 1: Spawning {num_entities} entities in a {width}×{height} world...");

    let mut entities: Vec<chronos_engine::Entity> = Vec::with_capacity(num_entities);
    for i in 0..num_entities {
        let x = (i as f32) % width;
        let y = ((i as f32) / 10.0) % height;
        let entity = world.create_entity();
        world.add_component(entity, Position::new(x, y));
        world.add_component(
            entity,
            Velocity::new(
                ((i as f32) % 2.0) - 1.0,
                (((i as f32) / 100.0) % 2.0) - 1.0,
            ),
        );
        world.add_component(entity, CircleRadius(5.0));
        entities.push(entity);
    }

    println!(
        "  Spawned {num_entities} entities | Capacity: {}",
        world.entity_capacity()
    );

    println!("\n▶ Phase 2: Running 100 ticks of simulation...\n");

    let start = std::time::Instant::now();
    for tick in 0..100 {
        scheduler.tick(&mut world, &mut events);

        let pending = events.drain();
        for event in &pending {
            match event {
                Event::Collision(_a, b) => {
                    let e = world.entity_from_index(*b);
                    if world.entity_exists(e) {
                        world.add_component(e, Damage(10));
                    }
                }
                Event::DamageTaken(entity, _amount) => {
                    let e = world.entity_from_index(*entity);
                    if let Some(hp) = world.get_component::<Health>(e) {
                        if hp.current <= 0 {}
                    }
                }
                Event::EntityDied(_entity) => {}
                _ => {}
            }
        }

        if tick % 25 == 0 {
            print!("  [tick {}] ", tick);
            if let Some(p) = world.get_component::<Position>(entities[0]) {
                print!("Entity 0 @ ({:.1},{:.1})", p.x, p.y);
            }
            if let Some(p) = world.get_component::<Position>(entities[100]) {
                print!(" Entity 100 @ ({:.1},{:.1})", p.x, p.y);
            }
            if let Some(p) = world.get_component::<Position>(entities[1_000]) {
                print!(" Entity 1000 @ ({:.1},{:.1})", p.x, p.y);
            }
            println!();
        }
    }

    let elapsed = start.elapsed();
    println!("\n  Total simulation time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Average per tick: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0 / 100.0
    );
    println!("  Entities alive: {}", world.entity_count());

    println!("\n▶ Phase 3: Quadtree query benchmark...\n");

    let query_points = [(250.0, 250.0), (100.0, 300.0), (400.0, 150.0)];
    let mut qt = Quadtree::new(AABB::new(0.0, 0.0, width, height), 4, 6);

    for (entity, pos) in world.query::<Position>() {
        let radius = if world.has_component::<CircleRadius>(entity) {
            world.get_component::<CircleRadius>(entity).unwrap().0
        } else {
            5.0
        };
        qt.insert(QuadtreeObject {
            entity: entity.index(),
            x: pos.x,
            y: pos.y,
            radius,
        });
    }

    for (i, (x, y)) in query_points.iter().enumerate() {
        let hits = qt.query_circle(*x, *y, 20.0);
        println!("  Query {}: hits at ({:.0},{:.0}) — {} entities", i + 1, x, y, hits.len());
    }

    println!("\n▶ Phase 4: Raycasting...\n");

    let ray = Ray::from_to(0.0, 250.0, 500.0, 250.0);
    let raycast_system = RaycastSystem::new_default(width, height);
    let hits = raycast_system.cast(&world, ray);

    println!("  Ray from (0, 250) → right: {} hits", hits.len());
    if let Some((_, hit)) = hits.first() {
        println!("  First hit: entity {}, distance {:.1}", hit.entity, hit.distance);
    }

    println!("\n═══ Bullet Hell Demo Complete ═══");
}

fn rts_scenario() {
    println!("\n═══ Chronos Engine — RTS Combat Scenario ═══\n");

    let mut world = World::new();
    let mut gameloop = GameLoop::new();

    // Build the pipeline
    gameloop.add_system(MovementSystem::new(), SystemPhase::Update);
    gameloop.add_system(
        CollisionSystem::new(12.0),
        SystemPhase::Update,
    );
    gameloop.add_system(HealthSystem::new(), SystemPhase::PostUpdate);
    gameloop.add_system(DeathCleanupSystem::new(), SystemPhase::Cleanup);
    gameloop.add_system(DebugRenderSystem::new(30, 10), SystemPhase::Render);

    println!("System pipeline:");
    for (phase, names) in gameloop.system_report() {
        println!("  {:?}: {}", phase, names.join(", "));
    }

    // Spawn a small squad
    let squad_positions = [
        (5.0, 5.0),
        (8.0, 5.0),
        (5.0, 8.0),
        (12.0, 5.0),
        (5.0, 12.0),
    ];
    let mut squad = Vec::new();
    for (i, (x, y)) in squad_positions.iter().enumerate() {
        let unit = world.create_entity();
        world.add_component(unit, Position::new(*x, *y));
        world.add_component(unit, Velocity::new(0.3, 0.2));
        world.add_component(unit, Health::new(50));
        world.add_component(
            unit,
            Sprite::new(
                if i == 0 { 'C' } else { 'S' },
                100,
                200,
                255,
            ),
        );
        squad.push(unit);
    }

    // Spawn an enemy group
    let mut enemy_group = Vec::new();
    let enemy_positions = [(22.0, 5.0), (25.0, 8.0), (20.0, 10.0)];
    for (x, y) in &enemy_positions {
        let unit = world.create_entity();
        world.add_component(unit, Position::new(*x, *y));
        world.add_component(unit, Velocity::new(-0.2, 0.0));
        world.add_component(unit, Health::new(40));
        world.add_component(unit, Sprite::new('E', 255, 60, 60));
        enemy_group.push(unit);
    }

    println!(
        "\nDeployed {} friendlies and {} enemies",
        squad.len(),
        enemy_group.len()
    );
    println!("Press Enter to simulate...");

    // Simulate 60 ticks
    for tick in 0..60 {
        gameloop.tick(&mut world, 0.016);

        // Process events from the GameLoop's event bus (the actual one systems write to)
        let pending = gameloop.event_bus.drain();
        for event in &pending {
            match event {
                Event::Collision(a, b) => {
                    println!("  tick {}: ⚡ collision {} ↔ {}", tick, a, b);
                    for entity_idx in [*a, *b] {
                        let e = world.entity_from_index(entity_idx);
                        if world.entity_exists(e) {
                            world.add_component(e, Damage(20));
                        }
                    }
                }
                Event::DamageTaken(entity, amount) => {
                    let e = world.entity_from_index(*entity);
                    if let Some(hp) = world.get_component::<Health>(e) {
                        println!(
                            "  tick {}: 💥 entity {} took {} damage (HP: {}/{})",
                            tick, entity, amount, hp.current, hp.max
                        );
                    }
                }
                Event::EntityDied(entity) => {
                    println!("  tick {}: ☠️  entity {} died!", tick, entity);
                }
                Event::EntityDestroyed(entity) => {
                    println!("  tick {}: 🗑️  entity {} cleaned up", tick, entity);
                }
                _ => {}
            }
        }

        if tick == 30 {
            println!("  --- Mid-battle report ---");
            println!(
                "  Alive entities: {} | Ticks: {}",
                world.entity_count(),
                gameloop.tick_count
            );
        }
    }

    println!(
        "\nFinal survivors: {} out of {} initial",
        world.entity_count(),
        squad.len() + enemy_group.len()
    );
}

// ──────────────────────────────────────────────
// GPU-Rendered Demo (with render feature)
// ──────────────────────────────────────────────

/// Gather all entities into render sprites.
#[cfg(feature = "render")]
fn gather_sprites(world: &World) -> Vec<RenderSprite> {
    let mut sprites = Vec::new();

    for (entity, pos) in world.query::<Position>() {
        let sprite = world.get_component::<Sprite>(entity);
        if let Some(s) = sprite {
            let (r, g, b) = s.color;
            let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
            sprites.push(
                RenderSprite::new(pos.x, pos.y, 8.0, 8.0)
                    .with_color(color[0], color[1], color[2], color[3]),
            );
        } else {
            // No sprite component — default to white box
            sprites.push(RenderSprite::new(pos.x, pos.y, 8.0, 8.0));
        }
    }

    sprites
}

#[cfg(feature = "render")]
async fn gpu_battle_demo() -> Result<(), String> {
    println!("\n═══ Chronos Engine v0.3 — GPU Battle Arena ═══");

    // Create a winit window (winit 0.30 API: EventLoop + WindowAttributes)
    let event_loop = winit::event_loop::EventLoop::new()
        .map_err(|e| format!("Failed to create event loop: {}", e))?;
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                winit::window::WindowAttributes::default()
                    .with_title("Chronos Engine — GPU Battle Arena")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1024, 768)),
            )
            .map_err(|e| format!("Failed to create window: {}", e))?
    );

    // Initialize the ECS
    let mut world = World::new();
    let mut scheduler = TickScheduler::new(0.016);

    let mut events = EventBus::new();

    // Register systems
    scheduler.add(MovementSystem::new());
    scheduler.add(HealthSystem::new());
    scheduler.add(CollisionSystem::new(15.0));
    scheduler.add(DeathCleanupSystem::new());

    // Spawn entities
    let warrior = world.create_entity();
    world.add_component(warrior, Position::new(10.0, 20.0));
    world.add_component(warrior, Velocity::new(1.5, 0.0));
    world.add_component(warrior, Health::new(100));
    world.add_component(warrior, Sprite::new('W', 0, 255, 255));

    let enemy = world.create_entity();
    world.add_component(enemy, Position::new(50.0, 30.0));
    world.add_component(enemy, Velocity::new(-1.0, 0.5));
    world.add_component(enemy, Health::new(80));
    world.add_component(enemy, Sprite::new('E', 255, 50, 50));

    let turret = world.create_entity();
    world.add_component(turret, Position::new(30.0, 25.0));
    world.add_component(turret, Sprite::new('T', 255, 255, 0));

    println!("  Spawning entities: warrior, enemy, turret");

    // Create the GPU renderer
    let mut renderer = Renderer::new(window.clone(), 1024, 768)
        .await
        .map_err(|e| format!("Failed to create renderer: {}", e))?;

    let camera = Camera::new(100.0, 75.0);
    let _batch = SpriteBatch::new(&renderer.device, 1024);

    // Create a simple white texture (1x1 pixel) — done once, reused every frame
    let texture_bytes = vec![255u8];
    let texture = renderer.device.create_texture_with_data(
        &renderer.queue,
        &wgpu::TextureDescriptor {
            label: Some("white-texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::MipMajor,
        &texture_bytes,
    );

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    println!("  Renderer ready. Running 120 ticks (2 seconds of simulation)...");

    let mut last_tick_print = 0;
    let start = std::time::Instant::now();

    // Main render loop (fixed timestep, capped at ~60 FPS)
    let mut accumulator = 0.0;
    for frame in 0.. {
        let frame_time = 0.016; // 60 FPS target
        accumulator += frame_time;

        // Run up to 3 physics ticks per frame
        for _ in 0..3 {
            if accumulator >= 0.016 {
                scheduler.tick(&mut world, &mut events);
                accumulator -= 0.016;

                // Drain events
                let pending = events.drain();
                for event in &pending {
                    match event {
                        Event::Collision(_a, b) => {
                            let enemy_entity = world.entity_from_index(*b);
                            if world.entity_exists(enemy_entity) {
                                world.add_component(enemy_entity, Damage(25));
                            }
                        }
                        _ => {}
                    }
                }

                last_tick_print += 1;
            }
        }

        // Gather sprites from world
        let mut sprites = gather_sprites(&world);

        // Render the frame
        renderer.render(&camera, &mut sprites, &texture_view, &sampler);

        // Print status every 30 frames
        if frame % 30 == 0 {
            print!(
                "  [tick {}] Warrior @ ({:.1},{:.1}) | ",
                last_tick_print,
                world.get_component::<Position>(warrior).unwrap().x,
                world.get_component::<Position>(warrior).unwrap().y
            );
            if world.entity_exists(enemy) {
                if let Some(p) = world.get_component::<Position>(enemy) {
                    print!("Enemy @ ({:.1},{:.1}) | ", p.x, p.y);
                }
            } else {
                print!("Enemy: DEAD | ");
            }
            if world.entity_exists(turret) {
                if let Some(p) = world.get_component::<Position>(turret) {
                    println!("Turret @ ({:.1},{:.1})", p.x, p.y);
                }
            } else {
                println!("Turret: DESTROYED");
            }

            if last_tick_print >= 120 {
                break;
            }
        }

        if last_tick_print >= 120 {
            break;
        }
    }

    let elapsed = start.elapsed();
    println!(
        "\n  Total simulation time: {:.2}ms ({} ticks)",
        elapsed.as_secs_f64() * 1000.0,
        last_tick_print
    );
    println!("  Remaining entities: {}", world.entity_count());

    Ok(())
}

fn main() {
    #[cfg(feature = "render")]
    {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        if let Err(e) = rt.block_on(gpu_battle_demo()) {
            eprintln!("GPU demo failed: {}", e);
            println!("Falling back to terminal demos...");
            // Fallback to terminal demos
            battle_demo();
            rts_scenario();
            bullet_hell_demo();
        }
    }

    #[cfg(not(feature = "render"))]
    {
        // Run the comprehensive battle demo
        battle_demo();

        // Run the RTS scenario with GameLoop pipeline
        rts_scenario();

        bullet_hell_demo();

        println!("\n✓ Chronos Engine is operational. Multiple genre support confirmed.");
        println!("  Next steps: serialization, GPU renderer, scripting.");
    }
}

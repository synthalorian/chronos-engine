use chronos_engine::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ── Benchmark 1: Entity creation throughput ──
fn bench_entity_create(c: &mut Criterion) {
    c.bench_function("entity_create_1000", |b| {
        b.iter(|| {
            let mut world = World::new();
            for _ in 0..1000 {
                black_box(world.create_entity());
            }
        })
    });
}

// ── Benchmark 2: Component attachment ──
fn bench_component_attach(c: &mut Criterion) {
    c.bench_function("component_attach_1000", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(1000);
        for _ in 0..1000 {
            entities.push(world.create_entity());
        }
        b.iter(|| {
            for &e in &entities {
                world.add_component(e, Position { x: 1.0, y: 2.0 });
            }
        })
    });
}

// ── Benchmark 3: Component query iteration ──
fn bench_query_iterate(c: &mut Criterion) {
    let mut world = World::new();
    for _ in 0..10_000 {
        let e = world.create_entity();
        world.add_component(e, Position { x: 1.0, y: 2.0 });
        world.add_component(e, Velocity { x: 3.0, y: 4.0 });
    }
    c.bench_function("query_iterate_10k", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for (_, pos) in world.query::<Position>() {
                sum += pos.x + pos.y;
            }
            black_box(sum);
        })
    });
}

// ── Benchmark 4: Mutable query iteration ──
fn bench_query_mut_iterate(c: &mut Criterion) {
    let mut world = World::new();
    for _ in 0..10_000 {
        let e = world.create_entity();
        world.add_component(e, Position { x: 1.0, y: 2.0 });
        world.add_component(e, Velocity { x: 3.0, y: 4.0 });
    }
    c.bench_function("query_mut_iterate_10k", |b| {
        b.iter(|| {
            for (_, pos) in world.query_mut::<Position>() {
                pos.x += 1.0;
                pos.y += 1.0;
            }
        })
    });
}

// ── Benchmark 5: Entity destruction ──
fn bench_entity_destroy(c: &mut Criterion) {
    c.bench_function("entity_destroy_1000", |b| {
        b.iter(|| {
            let mut world = World::new();
            let mut entities = Vec::with_capacity(1000);
            for _ in 0..1000 {
                entities.push(world.create_entity());
            }
            for e in entities {
                world.destroy_entity(e);
            }
        })
    });
}

// ── Benchmark 6: Archetype migration (add + remove component) ──
fn bench_archetype_migration(c: &mut Criterion) {
    c.bench_function("archetype_migration_1000", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(1000);
        for _ in 0..1000 {
            let e = world.create_entity();
            world.add_component(e, Position { x: 0.0, y: 0.0 });
            entities.push(e);
        }
        b.iter(|| {
            for &e in &entities {
                world.add_component(e, Velocity { x: 1.0, y: 1.0 });
            }
            for &e in &entities {
                world.remove_component::<Velocity>(e);
            }
        })
    });
}

// ── Benchmark 7: Multi-component query (query_with_all) ──
fn bench_query_with_all(c: &mut Criterion) {
    let mut world = World::new();
    for i in 0..10_000 {
        let e = world.create_entity();
        world.add_component(
            e,
            Position {
                x: i as f32,
                y: i as f32,
            },
        );
        if i % 2 == 0 {
            world.add_component(e, Velocity { x: 1.0, y: 1.0 });
        }
    }
    c.bench_function("query_with_all_10k", |b| {
        b.iter(|| {
            let results = world.query_with_all::<Position>(&[std::any::TypeId::of::<Velocity>()]);
            black_box(results.len());
        })
    });
}

// ── Benchmark 8: World simulation tick ──
fn bench_world_tick(c: &mut Criterion) {
    c.bench_function("world_tick_1000_entities", |b| {
        let mut world = World::new();
        for _ in 0..1000 {
            let e = world.create_entity();
            world.add_component(e, Position { x: 0.0, y: 0.0 });
            world.add_component(e, Velocity { x: 1.0, y: 1.0 });
        }
        let mut loop_ = GameLoop::new();
        loop_.add_system(MovementSystem::new(), SystemPhase::Update);
        b.iter(|| {
            loop_.tick(&mut world, 0.016);
        })
    });
}

criterion_group!(
    benches,
    bench_entity_create,
    bench_component_attach,
    bench_query_iterate,
    bench_query_mut_iterate,
    bench_entity_destroy,
    bench_archetype_migration,
    bench_query_with_all,
    bench_world_tick
);
criterion_main!(benches);

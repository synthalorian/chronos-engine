//! Integration tests for Chronos Engine core modules.

use chronos_engine::*;

#[test]
fn entity_creation_and_reuse() {
    let mut world = World::new();
    let e1 = world.create_entity();
    let e2 = world.create_entity();
    assert_ne!(e1.index(), e2.index());

    world.destroy_entity(e1);
    let e3 = world.create_entity();
    assert_eq!(e1.index(), e3.index());
    assert_ne!(e1.generation(), e3.generation());
}

#[test]
fn component_add_get_remove() {
    let mut world = World::new();
    let e = world.create_entity();

    world.add_component(e, Position { x: 10.0, y: 20.0 });
    world.add_component(e, Velocity { x: 1.0, y: -1.0 });

    let pos = world.get_component::<Position>(e).unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);

    world.remove_component::<Position>(e);
    assert!(world.get_component::<Position>(e).is_none());
}

#[test]
fn entity_exists_after_destroy() {
    let mut world = World::new();
    let e = world.create_entity();
    assert!(world.entity_exists(e));
    world.destroy_entity(e);
    assert!(!world.entity_exists(e));
}

#[test]
fn destroy_entity_with_event() {
    let mut world = World::new();
    let mut events = EventBus::new();
    let e = world.create_entity();
    world.add_component(e, Health::new(100));

    world.destroy_entity_with_event(e, &mut events);
    assert!(!world.entity_exists(e));

    let drained = events.drain();
    let destroyed: Vec<_> = drained.iter()
        .filter(|ev| matches!(ev, Event::EntityDestroyed(idx) if *idx == e.index()))
        .collect();
    assert_eq!(destroyed.len(), 1);
}

#[test]
fn movement_system_updates_position() {
    let mut world = World::new();
    let mut events = EventBus::new();
    let e = world.create_entity();
    world.add_component(e, Position { x: 0.0, y: 0.0 });
    world.add_component(e, Velocity { x: 10.0, y: 5.0 });

    let mut movement = MovementSystem::new();
    movement.update(&mut world, &mut events, 1.0);

    let pos = world.get_component::<Position>(e).unwrap();
    assert!((pos.x - 10.0).abs() < 0.001);
    assert!((pos.y - 5.0).abs() < 0.001);
}

#[test]
fn health_system_applies_damage() {
    let mut world = World::new();
    let mut events = EventBus::new();
    let e = world.create_entity();
    world.add_component(e, Health::new(100));
    world.add_component(e, Damage(30));

    let mut health_sys = HealthSystem::new();
    health_sys.update(&mut world, &mut events, 0.016);

    let hp = world.get_component::<Health>(e).unwrap();
    assert_eq!(hp.current, 70);
}

#[test]
fn gravity_system_applies_gravity() {
    let mut world = World::new();
    let mut events = EventBus::new();

    let g = world.create_entity();
    world.add_component(g, Gravity { x: 0.0, y: -9.81 });

    let e = world.create_entity();
    world.add_component(e, Position { x: 0.0, y: 100.0 });
    world.add_component(e, Velocity { x: 0.0, y: 0.0 });
    world.add_component(e, RigidBody::new(1.0, 0.0, 0.5));

    let mut gravity = GravitySystem::new();
    gravity.update(&mut world, &mut events, 1.0);

    let vel = world.get_component::<Velocity>(e).unwrap();
    assert!(vel.y < 0.0);
}

#[test]
fn quadtree_insert_and_query() {
    let bounds = AABB::new(0.0, 0.0, 100.0, 100.0);
    let mut qt = Quadtree::new(bounds, 4, 4);

    qt.insert(QuadtreeObject { entity: 1, x: 10.0, y: 10.0, radius: 2.0 });
    qt.insert(QuadtreeObject { entity: 2, x: 50.0, y: 50.0, radius: 2.0 });
    qt.insert(QuadtreeObject { entity: 3, x: 90.0, y: 90.0, radius: 2.0 });

    let results = qt.query_circle(10.0, 10.0, 5.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity, 1);
}

#[test]
fn quadtree_collision_detection() {
    let bounds = AABB::new(0.0, 0.0, 200.0, 200.0);
    let mut qt = Quadtree::new(bounds, 4, 4);

    qt.insert(QuadtreeObject { entity: 1, x: 10.0, y: 10.0, radius: 5.0 });
    qt.insert(QuadtreeObject { entity: 2, x: 14.0, y: 10.0, radius: 5.0 });
    qt.insert(QuadtreeObject { entity: 3, x: 100.0, y: 100.0, radius: 5.0 });

    let pairs = qt.query_collisions();
    assert_eq!(pairs.len(), 1);
    let (a, b) = pairs[0];
    assert_eq!(a.min(b), 1);
    assert_eq!(a.max(b), 2);
}

#[test]
fn quadtree_point_query() {
    let bounds = AABB::new(0.0, 0.0, 100.0, 100.0);
    let mut qt = Quadtree::new(bounds, 4, 4);
    qt.insert(QuadtreeObject { entity: 1, x: 50.0, y: 50.0, radius: 10.0 });

    let results = qt.query_point(50.0, 50.0);
    assert_eq!(results.len(), 1);

    let empty = qt.query_point(0.0, 0.0);
    assert!(empty.is_empty());
}

#[test]
fn aabb_overlap() {
    let a = AABB::new(0.0, 0.0, 10.0, 10.0);
    let b = AABB::new(5.0, 5.0, 10.0, 10.0);
    let c = AABB::new(20.0, 20.0, 10.0, 10.0);

    assert!(a.overlaps_aabb(&b));
    assert!(!a.overlaps_aabb(&c));
}

#[test]
fn tilemap_set_get() {
    let mut map = TileMap::new(32.0);
    map.set_tile(0, 0, Tile { frame: 1, solid: true });
    map.set_tile(5, 3, Tile { frame: 2, solid: false });

    let t = map.get_tile(0, 0).unwrap();
    assert_eq!(t.frame, 1);
    assert!(t.solid);

    let t2 = map.get_tile(5, 3).unwrap();
    assert_eq!(t2.frame, 2);
    assert!(!t2.solid);

    assert!(map.get_tile(1000, 1000).is_none());
}

#[test]
fn tilemap_visible_chunks() {
    let mut map = TileMap::new(32.0);
    for y in 0..5 {
        for x in 0..5 {
            map.set_tile(x, y, Tile { frame: 1, solid: false });
        }
    }

    let visible = map.visible_chunks(80.0, 80.0, 200.0, 200.0);
    assert!(!visible.is_empty());
}

#[test]
fn fog_of_war_reveal_and_check() {
    let mut fog = FogOfWar::new(100.0, 100.0, 10.0);

    fog.add_revealer(FogRevealer {
        entity: 1,
        x: 50.0,
        y: 50.0,
        radius: 20.0,
        line_of_sight: false,
    });
    fog.compute();

    assert!(fog.grid.is_visible(50.0, 50.0));
    assert!(!fog.grid.is_visible(5.0, 5.0));
    assert!(fog.grid.is_explored(50.0, 50.0));
}

#[test]
fn fog_of_war_demote_to_explored() {
    let mut fog = FogOfWar::new(100.0, 100.0, 10.0);
    fog.add_revealer(FogRevealer {
        entity: 1,
        x: 50.0,
        y: 50.0,
        radius: 20.0,
        line_of_sight: false,
    });

    fog.compute();
    assert!(fog.grid.is_visible(50.0, 50.0));

    fog.revealers.clear();
    fog.compute();
    assert!(!fog.grid.is_visible(50.0, 50.0));
    assert!(fog.grid.is_explored(50.0, 50.0));
}

#[test]
fn obj_parse_cube() {
    let obj_data = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1 4/4/1
";
    let mesh = ObjMesh::parse(obj_data).unwrap();
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.normals.len(), 1);
    assert_eq!(mesh.uvs.len(), 4);
    assert_eq!(mesh.faces.len(), 1);

    let (_positions, _normals, _uvs, indices) = mesh.triangulate();
    assert_eq!(indices.len(), 6);
}

#[test]
fn physics3d_sphere_collision() {
    let mut world = PhysicsWorld3D::new();
    world.gravity = [0.0, 0.0, 0.0];

    world.add_body(RigidBody3D::new(1).with_position(0.0, 0.0, 0.0));
    world.add_body(RigidBody3D::new(2).with_position(1.5, 0.0, 0.0));
    world.add_collider(1, Collider3D::sphere(1.0));
    world.add_collider(2, Collider3D::sphere(1.0));

    let contacts = world.detect_collisions();
    assert_eq!(contacts.len(), 1);
    assert!(contacts[0].depth > 0.0);
}

#[test]
fn physics3d_aabb_no_collision() {
    let mut world = PhysicsWorld3D::new();
    world.add_body(RigidBody3D::new(1).with_position(0.0, 0.0, 0.0));
    world.add_body(RigidBody3D::new(2).with_position(100.0, 100.0, 100.0));
    world.add_collider(1, Collider3D::aabb(1.0, 1.0, 1.0));
    world.add_collider(2, Collider3D::aabb(1.0, 1.0, 1.0));

    let contacts = world.detect_collisions();
    assert!(contacts.is_empty());
}

#[test]
fn physics3d_gravity_integration() {
    let mut world = PhysicsWorld3D::new();
    world.gravity = [0.0, -10.0, 0.0];
    world.add_body(RigidBody3D::new(1).with_position(0.0, 100.0, 0.0));

    world.step(1.0);

    let pos = world.bodies[0].position;
    assert!(pos[1] < 100.0);
}

#[test]
fn skeleton_pose_skin_matrices() {
    let mut skeleton = Skeleton::new();
    skeleton.add_joint("root", None, JointPose::identity());
    skeleton.add_joint("child", Some(0), JointPose::identity().with_translation(1.0, 0.0, 0.0));

    let pose = SkeletonPose::new(2);
    let matrices = pose.compute_skin_matrices(&skeleton);
    assert_eq!(matrices.len(), 2);
}

#[test]
fn animation_player_sample() {
    let mut skeleton = Skeleton::new();
    skeleton.add_joint("root", None, JointPose::identity());

    let mut clip = AnimationClip::new("test", 1.0);
    let mut channel = AnimationChannel::new(0);
    channel.add_translation(0.0, [0.0, 0.0, 0.0]);
    channel.add_translation(1.0, [10.0, 0.0, 0.0]);
    clip.add_channel(channel);

    let mut player = AnimationPlayer::new();
    player.play(clip);

    player.set_time(0.5);
    let pose = player.sample(1);
    let t = pose.get_joint_pose(0).unwrap().translation;
    assert!((t[0] - 5.0).abs() < 0.01);
}

#[test]
fn animation_blender() {
    let mut pose_a = SkeletonPose::new(1);
    pose_a.set_joint_pose(0, JointPose::identity().with_translation(0.0, 0.0, 0.0));

    let mut pose_b = SkeletonPose::new(1);
    pose_b.set_joint_pose(0, JointPose::identity().with_translation(10.0, 0.0, 0.0));

    let blended = AnimationBlender::blend(&pose_a, &pose_b, 0.5);
    let t = blended.get_joint_pose(0).unwrap().translation;
    assert!((t[0] - 5.0).abs() < 0.01);
}

#[test]
fn lighting_system_basic() {
    let mut ls = LightingSystem::new();
    ls.add_light(Light::point(1, 50.0, 50.0, 30.0, 1.0));

    let mut map = LightMap::new(10, 10, 10.0);
    ls.compute_lighting(&mut map);

    let center = map.get_intensity(50.0, 50.0);
    let corner = map.get_intensity(5.0, 5.0);
    assert!(center > corner);
}

#[test]
fn collision_system_cooldown() {
    let mut world = World::new();
    let mut events = EventBus::new();

    let e1 = world.create_entity();
    world.add_component(e1, Position { x: 0.0, y: 0.0 });
    world.add_component(e1, CircleRadius(5.0));

    let e2 = world.create_entity();
    world.add_component(e2, Position { x: 8.0, y: 0.0 });
    world.add_component(e2, CircleRadius(5.0));

    let mut collision = CollisionSystem::new(5.0);
    collision.update(&mut world, &mut events, 0.016);
    let first_count = events.drain().len();

    collision.update(&mut world, &mut events, 0.016);
    let second_count = events.drain().len();

    assert!(first_count > 0);
    assert_eq!(second_count, 0);
}

#[test]
fn game_loop_runs_systems() {
    let mut game_loop = GameLoop::new();
    let mut world = World::new();

    let e = world.create_entity();
    world.add_component(e, Position { x: 0.0, y: 0.0 });
    world.add_component(e, Velocity { x: 10.0, y: 0.0 });

    game_loop.add_system(MovementSystem::new(), SystemPhase::Update);
    game_loop.tick(&mut world, 1.0 / 60.0);

    let pos = world.get_component::<Position>(e).unwrap();
    assert!(pos.x > 0.0);
}

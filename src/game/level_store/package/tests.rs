use super::binio::{decode_chunk, encode_chunk};
use super::chunk::{compile_package, package_to_level};
use super::*;
use std::fs;
use std::path::PathBuf;

use glam::Vec2;

use crate::game::enemy::Enemy;
use crate::game::level::{
    Checkpoint, Door, Hazard, Level, LevelObjectKind, LevelObjectMeta, LevelText, LevelTrigger,
    ObjectMeta, Solid, WorldPortal,
};
use crate::game::level_store::LevelSpec;
use crate::game::portal::{Color, Portal};

#[test]
fn v4_package_roundtrips_current_level_objects() {
    let path = unique_package_path("roundtrip");
    let _ = fs::remove_dir_all(&path);
    let spec = sample_level();

    LevelPackageWriter::write_directory(&path, &spec).expect("write v4 package");
    let loaded = LevelPackageReader::read(&path).expect("read v4 package");
    let _ = fs::remove_dir_all(&path);

    assert_eq!(loaded.name, spec.name);
    assert_vec2_near(loaded.spawn, spec.spawn);
    assert_eq!(loaded.solids.len(), 1);
    assert_eq!(loaded.hazards.len(), 1);
    assert_eq!(loaded.doors.len(), 1);
    assert_eq!(loaded.checkpoints.len(), 1);
    assert_eq!(loaded.enemies.len(), 1);
    assert_eq!(loaded.triggers.len(), 2);
    assert_eq!(loaded.texts.len(), 1);
    assert_eq!(loaded.world_portals.len(), 2);
    assert!(loaded.solids[0].portalable);
    assert_vec2_near(loaded.solids[0].pos(), spec.solids[0].pos());
    assert_vec2_near(loaded.hazards[0].solid.pos(), spec.hazards[0].solid.pos());
    assert_eq!(loaded.texts[0].text, "HELLO \"V4\"");
    assert_eq!(loaded.world_portals[0].id, 20);
    assert_eq!(loaded.world_portals[0].receiver_id, 21);
    assert!(loaded.metadata.contains(&LevelObjectMeta {
        kind: LevelObjectKind::Solid,
        index: 0,
        meta: ObjectMeta {
            group_id: 7,
            editor_layer: 2,
        },
    }));
}

#[test]
fn compiled_chunk_uses_typed_soa_rect_arrays() {
    let package = compile_package(&sample_level()).expect("compile v4 package");
    let chunk = package
        .chunks
        .iter()
        .find(|chunk| !chunk.static_rects.is_empty())
        .expect("static rect chunk");
    let encoded = encode_chunk(chunk);
    let decoded = decode_chunk(&encoded).expect("decode chunk");

    assert_eq!(&encoded[..CHUNK_MAGIC.len()], CHUNK_MAGIC);
    assert_eq!(chunk.static_rects.meta.len(), chunk.static_rects.x.len());
    assert_eq!(chunk.static_rects.x.len(), chunk.static_rects.y.len());
    assert_eq!(chunk.static_rects.y.len(), chunk.static_rects.w.len());
    assert_eq!(chunk.static_rects.w.len(), chunk.static_rects.h.len());
    assert_eq!(
        chunk.static_rects.h.len(),
        chunk.static_rects.rotation.len()
    );
    assert_eq!(decoded.static_rects, chunk.static_rects);
    assert_eq!(
        package.index.chunks[chunk.chunk_id as usize].sha256.len(),
        64
    );
}

#[test]
fn large_world_uses_i32_chunk_origin_and_i16_local_coords() {
    let spec = LevelSpec {
        name: "Large World".to_string(),
        spawn: Vec2::new(100_000.0, 128.0),
        solids: vec![Solid::new(100_000.0, 128.0, 64.0, 32.0, true)],
        doors: Vec::new(),
        hazards: Vec::new(),
        checkpoints: Vec::new(),
        enemies: Vec::new(),
        triggers: Vec::new(),
        texts: Vec::new(),
        world_portals: Vec::new(),
        metadata: Vec::new(),
        path: None,
    };

    let package = compile_package(&spec).expect("compile large world");
    let chunk = package
        .chunks
        .iter()
        .find(|chunk| !chunk.static_rects.is_empty())
        .expect("solid chunk");
    let loaded = package_to_level(&package).expect("load large world package");

    assert!(chunk.origin_q[0].abs() > i16::MAX as i32);
    assert_eq!(chunk.static_rects.x.len(), 1);
    assert_vec2_near(loaded.solids[0].pos(), spec.solids[0].pos());
}

#[test]
fn oversized_object_crossing_chunk_local_range_is_rejected() {
    let spec = LevelSpec {
        name: "Too Large".to_string(),
        spawn: Vec2::ZERO,
        solids: vec![Solid::new(0.0, 0.0, 5000.0, 32.0, true)],
        doors: Vec::new(),
        hazards: Vec::new(),
        checkpoints: Vec::new(),
        enemies: Vec::new(),
        triggers: Vec::new(),
        texts: Vec::new(),
        world_portals: Vec::new(),
        metadata: Vec::new(),
        path: None,
    };

    assert!(compile_package(&spec).is_err());
}

#[test]
fn camera_culling_uses_actual_ultrawide_aspect() {
    let mut level = Level::empty();
    let edge_visible_at_48_9 = Solid::new(2380.0, -10.0, 20.0, 20.0, true);
    let offscreen = Solid::new(2500.0, -10.0, 20.0, 20.0, true);

    level.solids = vec![edge_visible_at_48_9, offscreen];

    let wide = RenderVisibleSet::from_camera(&level, Vec2::ZERO, 1.0, 4800.0, 900.0, 0.0);
    let normal = RenderVisibleSet::from_camera(&level, Vec2::ZERO, 1.0, 1600.0, 900.0, 0.0);

    assert!(wide.contains_solid(edge_visible_at_48_9));
    assert!(!wide.contains_solid(offscreen));
    assert!(!normal.contains_solid(edge_visible_at_48_9));
}

#[test]
fn normal_user_portal_runtime_never_requests_remote_render_or_light() {
    let source = Portal::new(0.0, 0.0, Vec2::Y, 64.0, Color::BLUE);
    let destination = Portal::new(128.0, 0.0, Vec2::Y, 64.0, Color::ORANGE);
    let runtime = UserPortalRuntime {
        source_anchor: source,
        destination_anchor: destination,
        traversal_width: 64.0,
        physics_margin: DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS,
        render_remote_scene: false,
        pass_lighting: false,
    };

    assert!(!runtime.render_remote_scene);
    assert!(!runtime.pass_lighting);
    assert!(runtime.physics_margin > 0.0);
}

#[test]
fn group_index_skips_zero_group_and_keeps_object_handles() {
    let package = compile_package(&sample_level()).expect("compile v4 package");
    let group_index = GroupIndex::from_chunks(&package.chunks);

    assert!(!group_index.contains_group(0));

    let solid_handles = group_index.handles(7);
    assert_eq!(solid_handles.len(), 1);
    assert_eq!(solid_handles[0].kind, LevelObjectKind::Solid);
    assert_eq!(solid_handles[0].index, 0);

    let portal_handles = group_index.handles(8);
    assert_eq!(portal_handles.len(), 1);
    assert_eq!(portal_handles[0].kind, LevelObjectKind::WorldPortal);
}

#[test]
fn trigger_index_uses_world_index_target_groups() {
    let mut package = compile_package(&sample_level()).expect("compile v4 package");
    package.index.triggers[1].target_group = 11;

    let trigger_index = TriggerIndex::from_package(&package).expect("build trigger index");
    let spawn_triggers = trigger_index.for_target_group(11).collect::<Vec<_>>();

    assert_eq!(spawn_triggers.len(), 1);
    assert_eq!(spawn_triggers[0].trigger_id, 1);
    assert_eq!(spawn_triggers[0].kind, 2);
    assert_eq!(spawn_triggers[0].target_group, 11);
    assert_eq!(trigger_index.for_target_group(3).count(), 0);
    assert_eq!(trigger_index.for_target_group(0).count(), 0);
}

#[test]
fn portal_index_tracks_world_portal_capabilities() {
    let package = compile_package(&sample_level()).expect("compile v4 package");
    let portal_index = PortalIndex::from_package(&package).expect("build portal index");

    assert_eq!(portal_index.world_portals.len(), 2);

    let seamless = portal_index.world_portal_by_id(20).expect("portal 20");
    assert_eq!(seamless.receiver_id, 21);
    assert!(seamless.render);
    assert!(seamless.lighting);
    assert!(seamless.physics);
    assert_eq!(seamless.render_depth, 1);
    assert_eq!(seamless.light_depth, 1);
    assert!(seamless.physics_margin > 0.0);

    let traversal_only = portal_index.world_portal_by_id(21).expect("portal 21");
    assert!(!traversal_only.render);
    assert!(!traversal_only.lighting);
    assert!(traversal_only.physics);
}

#[test]
fn portal_index_builds_user_portal_physics_halos_without_remote_render() {
    let source = Portal::new(0.0, 0.0, Vec2::Y, 64.0, Color::BLUE);
    let destination = Portal::new(128.0, 0.0, Vec2::Y, 96.0, Color::ORANGE);
    let portal_index = PortalIndex::from_user_portals(&[Some(source), Some(destination)]);

    assert_eq!(portal_index.world_portals.len(), 0);
    assert_eq!(portal_index.user_portals.len(), 2);
    assert!(portal_index.user_portals[0].source_anchor == source);
    assert!(portal_index.user_portals[0].destination_anchor == destination);
    assert_eq!(portal_index.user_portals[0].traversal_width, 64.0);
    for portal in &portal_index.user_portals {
        assert!(!portal.render_remote_scene);
        assert!(!portal.pass_lighting);
        assert!(portal.physics_margin > 0.0);
    }
}

#[test]
fn portal_index_uses_world_index_source_chunk_for_empty_spawn_chunk() {
    let spec = LevelSpec {
        name: "Portal Chunk Index".to_string(),
        spawn: Vec2::ZERO,
        solids: Vec::new(),
        doors: Vec::new(),
        hazards: Vec::new(),
        checkpoints: Vec::new(),
        enemies: Vec::new(),
        triggers: Vec::new(),
        texts: Vec::new(),
        world_portals: vec![WorldPortal::new(3000.0, 0.0, Vec2::X, 96.0, 40)],
        metadata: Vec::new(),
        path: None,
    };
    let package = compile_package(&spec).expect("compile v4 package");
    let portal_index = PortalIndex::from_package(&package).expect("build portal index");

    assert_eq!(package.index.chunks.len(), 2);
    assert_eq!(package.index.portals[0].source_chunk, 1);
    assert_eq!(portal_index.world_portals[0].source_chunk, 1);
}

#[test]
fn portal_index_honors_world_index_capability_flags() {
    let mut package = compile_package(&sample_level()).expect("compile v4 package");
    package.index.portals[0].render = false;
    package.index.portals[0].lighting = true;
    package.index.portals[0].physics = false;

    let portal_index = PortalIndex::from_package(&package).expect("build portal index");
    let portal = portal_index.world_portal_by_id(20).expect("portal 20");

    assert!(!portal.render);
    assert!(portal.lighting);
    assert!(!portal.physics);
    assert_eq!(portal.render_depth, 0);
    assert_eq!(portal.light_depth, 1);
    assert_eq!(portal.physics_margin, 0.0);
}

fn sample_level() -> LevelSpec {
    let mut door = Door::with_radius(320.0, 160.0, 48.0, 112.0, 144.0);
    door.speed = 4.25;
    door.automatic = false;

    let mut portal_a = WorldPortal::new(512.0, 220.0, Vec2::X, 96.0, 20);
    portal_a.receiver_id = 21;
    portal_a.priority = 2;
    portal_a.seamless = true;
    portal_a.seamless_rely_on_walls = true;
    let mut portal_b = WorldPortal::new(768.0, 220.0, -Vec2::X, 96.0, 21);
    portal_b.receiver_id = 20;

    LevelSpec {
        name: "V4 Test".to_string(),
        spawn: Vec2::new(128.0, 320.0),
        solids: vec![Solid::new(64.0, 400.0, 256.0, 32.0, true)],
        doors: vec![door],
        hazards: vec![Hazard::new(256.0, 440.0, 96.0, 20.0)],
        checkpoints: vec![Checkpoint::new(160.0, 320.0, 48.0, 80.0)],
        enemies: vec![Enemy::filth_spawn(420.0, 360.0, 3, 2)],
        triggers: vec![
            LevelTrigger::level_start(104.0, 280.0, 48.0, 80.0),
            LevelTrigger::enemy_spawn(380.0, 300.0, 96.0, 96.0, 3),
        ],
        texts: vec![LevelText::new(Vec2::new(180.0, 260.0), "HELLO \"V4\"")],
        world_portals: vec![portal_a, portal_b],
        metadata: vec![
            LevelObjectMeta {
                kind: LevelObjectKind::Solid,
                index: 0,
                meta: ObjectMeta {
                    group_id: 7,
                    editor_layer: 2,
                },
            },
            LevelObjectMeta {
                kind: LevelObjectKind::WorldPortal,
                index: 0,
                meta: ObjectMeta {
                    group_id: 8,
                    editor_layer: 1,
                },
            },
        ],
        path: None,
    }
}

fn unique_package_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "portals_v4_{name}_{}_{}.lvl",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

fn assert_vec2_near(actual: Vec2, expected: Vec2) {
    assert!((actual.x - expected.x).abs() <= 1.0 / COORD_SCALE as f32);
    assert!((actual.y - expected.y).abs() <= 1.0 / COORD_SCALE as f32);
}

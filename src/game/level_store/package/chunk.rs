use std::collections::{BTreeMap, HashMap};
use std::io;

use glam::Vec2;

use super::{
    CHUNK_DIR, CHUNK_SIZE_UNITS, COORD_SCALE, CheckpointAnchor, DEFAULT_VIEW_LAYER, LevelManifest,
    LevelPackage, LevelSpec, PACKAGE_SCHEMA, TriggerAnchor, WORLD_INDEX_PATH, WorldAabb,
    WorldChunkEntry, WorldIndex, WorldPortalAnchor, chunk_bounds_units, chunk_key_for_world,
    chunk_origin_for_key, dequant_local_point, dequant_rotation, dequant_u16_units,
    dequant_unit_vec, encode_chunk, invalid_data, quant_local_aabb, quant_local_point,
    quant_rotation, quant_size, quant_u16_fixed, quant_u16_units, quant_unit, sha256_hex,
    stable_level_id,
};
use crate::game::enemy::{Enemy, EnemyKind};
use crate::game::level::{
    Checkpoint, Door, Hazard, LevelObjectKind, LevelObjectMeta, LevelText, LevelTrigger,
    LevelTriggerKind, ObjectMeta, Solid, WorldPortal,
};
use crate::game::portal::{Color, Portal};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChunkObjectCounts {
    pub static_rects: u32,
    pub hazard_rects: u32,
    pub doors: u32,
    pub checkpoints: u32,
    pub triggers: u32,
    pub enemy_spawns: u32,
    pub text_points: u32,
    pub world_portals: u32,
}
#[derive(Clone, Debug, Default)]
pub struct WorldChunk {
    pub chunk_id: u32,
    pub origin_q: [i32; 2],
    pub bounds_local: [i16; 4],
    pub static_rects: RectSoABlock,
    pub hazard_rects: RectSoABlock,
    pub doors: DoorSoABlock,
    pub checkpoints: RectSoABlock,
    pub triggers: TriggerSoABlock,
    pub enemy_spawns: EnemySpawnSoABlock,
    pub text_points: TextPointSoABlock,
    pub world_portals: WorldPortalSoABlock,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RectSoABlock {
    pub meta: Box<[u16]>,
    pub x: Box<[i16]>,
    pub y: Box<[i16]>,
    pub w: Box<[u16]>,
    pub h: Box<[u16]>,
    pub rotation: Box<[i16]>,
    pub editor_layer: Box<[i16]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoorSoABlock {
    pub rects: RectSoABlock,
    pub radius: Box<[u16]>,
    pub speed: Box<[u16]>,
    pub automatic: Box<[u8]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TriggerSoABlock {
    pub rects: RectSoABlock,
    pub kind: Box<[u8]>,
    pub enemy_id: Box<[u16]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnemySpawnSoABlock {
    pub meta: Box<[u16]>,
    pub x: Box<[i16]>,
    pub y: Box<[i16]>,
    pub kind: Box<[u8]>,
    pub spawn_id: Box<[u16]>,
    pub spawn_wave: Box<[u16]>,
    pub editor_layer: Box<[i16]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextPointSoABlock {
    pub meta: Box<[u16]>,
    pub x: Box<[i16]>,
    pub y: Box<[i16]>,
    pub editor_layer: Box<[i16]>,
    pub text: Box<[String]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldPortalSoABlock {
    pub meta: Box<[u16]>,
    pub x: Box<[i16]>,
    pub y: Box<[i16]>,
    pub normal_x: Box<[i16]>,
    pub normal_y: Box<[i16]>,
    pub tangent_x: Box<[i16]>,
    pub tangent_y: Box<[i16]>,
    pub width: Box<[u16]>,
    pub portal_id: Box<[u16]>,
    pub receiver_id: Box<[u16]>,
    pub priority: Box<[i16]>,
    pub scale: Box<[u16]>,
    pub flags: Box<[u8]>,
    pub seamless_depth: Box<[u16]>,
    pub seamless_angle: Box<[u16]>,
    pub editor_layer: Box<[i16]>,
}

#[derive(Clone, Copy)]
pub struct ChunkSoAView<'a> {
    pub static_rects: &'a RectSoABlock,
    pub hazard_rects: &'a RectSoABlock,
    pub doors: &'a DoorSoABlock,
    pub checkpoints: &'a RectSoABlock,
    pub triggers: &'a TriggerSoABlock,
    pub enemy_spawns: &'a EnemySpawnSoABlock,
    pub text_points: &'a TextPointSoABlock,
    pub world_portals: &'a WorldPortalSoABlock,
}

#[derive(Clone, Default)]
pub struct CollisionProxyCache {
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
}

impl WorldChunk {
    pub fn soa_view(&self) -> ChunkSoAView<'_> {
        ChunkSoAView {
            static_rects: &self.static_rects,
            hazard_rects: &self.hazard_rects,
            doors: &self.doors,
            checkpoints: &self.checkpoints,
            triggers: &self.triggers,
            enemy_spawns: &self.enemy_spawns,
            text_points: &self.text_points,
            world_portals: &self.world_portals,
        }
    }

    pub fn collision_proxy_cache(&self) -> CollisionProxyCache {
        let solids = rects_to_solids(self.origin_q, &self.static_rects, |meta| {
            unpack_surface_meta(meta).portalable
        });
        let doors = (0..self.doors.rects.len())
            .map(|index| {
                let solid = rect_to_solid(self.origin_q, &self.doors.rects, index, false);
                let radius = dequant_u16_units(self.doors.radius[index]).max(1.0);
                let mut door = Door::with_radius(
                    solid.pos().x,
                    solid.pos().y,
                    solid.size().x,
                    solid.size().y,
                    radius,
                );

                door.solid.set_rotation(solid.rotation());
                door.speed = self.doors.speed[index] as f32 / 256.0;
                door.automatic = self.doors.automatic[index] != 0;
                door
            })
            .collect();

        CollisionProxyCache { solids, doors }
    }
}

impl RectSoABlock {
    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
struct RectSoABuilder {
    meta: Vec<u16>,
    x: Vec<i16>,
    y: Vec<i16>,
    w: Vec<u16>,
    h: Vec<u16>,
    rotation: Vec<i16>,
    editor_layer: Vec<i16>,
}

#[derive(Clone, Debug, Default)]
struct DoorSoABuilder {
    rects: RectSoABuilder,
    radius: Vec<u16>,
    speed: Vec<u16>,
    automatic: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
struct TriggerSoABuilder {
    rects: RectSoABuilder,
    kind: Vec<u8>,
    enemy_id: Vec<u16>,
}

#[derive(Clone, Debug, Default)]
struct EnemySpawnSoABuilder {
    meta: Vec<u16>,
    x: Vec<i16>,
    y: Vec<i16>,
    kind: Vec<u8>,
    spawn_id: Vec<u16>,
    spawn_wave: Vec<u16>,
    editor_layer: Vec<i16>,
}

#[derive(Clone, Debug, Default)]
struct TextPointSoABuilder {
    meta: Vec<u16>,
    x: Vec<i16>,
    y: Vec<i16>,
    editor_layer: Vec<i16>,
    text: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct WorldPortalSoABuilder {
    meta: Vec<u16>,
    x: Vec<i16>,
    y: Vec<i16>,
    normal_x: Vec<i16>,
    normal_y: Vec<i16>,
    tangent_x: Vec<i16>,
    tangent_y: Vec<i16>,
    width: Vec<u16>,
    portal_id: Vec<u16>,
    receiver_id: Vec<u16>,
    priority: Vec<i16>,
    scale: Vec<u16>,
    flags: Vec<u8>,
    seamless_depth: Vec<u16>,
    seamless_angle: Vec<u16>,
    editor_layer: Vec<i16>,
}

impl RectSoABuilder {
    fn push(&mut self, record: RectRecord) {
        self.meta.push(record.meta);
        self.x.push(record.x);
        self.y.push(record.y);
        self.w.push(record.w);
        self.h.push(record.h);
        self.rotation.push(record.rotation);
        self.editor_layer.push(record.editor_layer);
    }

    fn finish(self) -> RectSoABlock {
        RectSoABlock {
            meta: self.meta.into_boxed_slice(),
            x: self.x.into_boxed_slice(),
            y: self.y.into_boxed_slice(),
            w: self.w.into_boxed_slice(),
            h: self.h.into_boxed_slice(),
            rotation: self.rotation.into_boxed_slice(),
            editor_layer: self.editor_layer.into_boxed_slice(),
        }
    }
}

impl DoorSoABlock {
    pub(super) fn len(&self) -> usize {
        self.rects.len()
    }
}

impl DoorSoABuilder {
    fn push(&mut self, record: RectRecord, radius: u16, speed: u16, automatic: bool) {
        self.rects.push(record);
        self.radius.push(radius);
        self.speed.push(speed);
        self.automatic.push(u8::from(automatic));
    }

    fn finish(self) -> DoorSoABlock {
        DoorSoABlock {
            rects: self.rects.finish(),
            radius: self.radius.into_boxed_slice(),
            speed: self.speed.into_boxed_slice(),
            automatic: self.automatic.into_boxed_slice(),
        }
    }
}

impl TriggerSoABlock {
    pub(super) fn len(&self) -> usize {
        self.rects.len()
    }
}

impl TriggerSoABuilder {
    fn push(&mut self, record: RectRecord, kind: u8, enemy_id: u16) {
        self.rects.push(record);
        self.kind.push(kind);
        self.enemy_id.push(enemy_id);
    }

    fn finish(self) -> TriggerSoABlock {
        TriggerSoABlock {
            rects: self.rects.finish(),
            kind: self.kind.into_boxed_slice(),
            enemy_id: self.enemy_id.into_boxed_slice(),
        }
    }
}

impl EnemySpawnSoABuilder {
    fn finish(self) -> EnemySpawnSoABlock {
        EnemySpawnSoABlock {
            meta: self.meta.into_boxed_slice(),
            x: self.x.into_boxed_slice(),
            y: self.y.into_boxed_slice(),
            kind: self.kind.into_boxed_slice(),
            spawn_id: self.spawn_id.into_boxed_slice(),
            spawn_wave: self.spawn_wave.into_boxed_slice(),
            editor_layer: self.editor_layer.into_boxed_slice(),
        }
    }
}

impl TextPointSoABuilder {
    fn finish(self) -> TextPointSoABlock {
        TextPointSoABlock {
            meta: self.meta.into_boxed_slice(),
            x: self.x.into_boxed_slice(),
            y: self.y.into_boxed_slice(),
            editor_layer: self.editor_layer.into_boxed_slice(),
            text: self.text.into_boxed_slice(),
        }
    }
}

impl WorldPortalSoABuilder {
    fn finish(self) -> WorldPortalSoABlock {
        WorldPortalSoABlock {
            meta: self.meta.into_boxed_slice(),
            x: self.x.into_boxed_slice(),
            y: self.y.into_boxed_slice(),
            normal_x: self.normal_x.into_boxed_slice(),
            normal_y: self.normal_y.into_boxed_slice(),
            tangent_x: self.tangent_x.into_boxed_slice(),
            tangent_y: self.tangent_y.into_boxed_slice(),
            width: self.width.into_boxed_slice(),
            portal_id: self.portal_id.into_boxed_slice(),
            receiver_id: self.receiver_id.into_boxed_slice(),
            priority: self.priority.into_boxed_slice(),
            scale: self.scale.into_boxed_slice(),
            flags: self.flags.into_boxed_slice(),
            seamless_depth: self.seamless_depth.into_boxed_slice(),
            seamless_angle: self.seamless_angle.into_boxed_slice(),
            editor_layer: self.editor_layer.into_boxed_slice(),
        }
    }
}
pub(super) fn compile_package(level: &LevelSpec) -> io::Result<LevelPackage> {
    let mut builders = BTreeMap::<(i32, i32), ChunkBuilder>::new();
    let spawn_key = chunk_key_for_world(level.spawn)?;

    builders.insert(spawn_key, ChunkBuilder::new(spawn_key));
    for (index, solid) in level.solids.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Solid, index)?;
        let key = chunk_key_for_world(solid.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_solid(solid, meta)?;
    }
    for (index, hazard) in level.hazards.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Hazard, index)?;
        let key = chunk_key_for_world(hazard.solid.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_hazard(hazard, meta)?;
    }
    for (index, door) in level.doors.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Door, index)?;
        let key = chunk_key_for_world(door.solid.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_door(door, meta)?;
    }
    for (index, checkpoint) in level.checkpoints.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Checkpoint, index)?;
        let key = chunk_key_for_world(checkpoint.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_checkpoint(checkpoint, meta)?;
    }
    for (index, trigger) in level.triggers.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Trigger, index)?;
        let key = chunk_key_for_world(trigger.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_trigger(trigger, meta)?;
    }
    for (index, enemy) in level.enemies.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Enemy, index)?;
        let key = chunk_key_for_world(enemy.spawn_pos)?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_enemy(enemy, meta)?;
    }
    for (index, text) in level.texts.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Text, index)?;
        let key = chunk_key_for_world(text.pos)?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_text(text, meta)?;
    }
    for (index, portal) in level.world_portals.iter().copied().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::WorldPortal, index)?;
        let key = chunk_key_for_world(portal.center())?;

        builders
            .entry(key)
            .or_insert_with(|| ChunkBuilder::new(key))
            .push_world_portal(portal, meta)?;
    }

    let mut chunks = Vec::with_capacity(builders.len());
    let mut index_chunks = Vec::with_capacity(builders.len());
    let mut chunk_ids_by_key = HashMap::new();

    for (chunk_id, (key, builder)) in builders.into_iter().enumerate() {
        let mut chunk = builder.finish(chunk_id as u32);

        chunk.chunk_id = chunk_id as u32;
        let bytes = encode_chunk(&chunk);
        let path = format!("{CHUNK_DIR}/{chunk_id:04}.wchunk");
        let entry = WorldChunkEntry {
            id: chunk_id as u32,
            origin_q: chunk.origin_q,
            bbox_units: chunk_bounds_units(chunk.bounds_local, chunk.origin_q),
            path,
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            counts: chunk.counts(),
        };

        chunk_ids_by_key.insert(key, chunk_id as u32);
        chunks.push(chunk);
        index_chunks.push(entry);
    }

    let mut triggers = Vec::with_capacity(level.triggers.len());
    for (index, trigger) in level.triggers.iter().enumerate() {
        let trigger_id = u16::try_from(index)
            .map_err(|_| invalid_data("too many triggers for v4 u16 trigger ids"))?;

        triggers.push(TriggerAnchor {
            trigger_id,
            source_chunk: *chunk_ids_by_key
                .get(&chunk_key_for_world(trigger.center())?)
                .unwrap_or(&0),
            target_group: 0,
            kind: trigger_runtime_kind(trigger.kind),
        });
    }

    let mut checkpoints = Vec::with_capacity(level.checkpoints.len());
    for (index, checkpoint) in level.checkpoints.iter().enumerate() {
        checkpoints.push(CheckpointAnchor {
            index: index as u32,
            anchor: checkpoint.center(),
            chunk_id: *chunk_ids_by_key
                .get(&chunk_key_for_world(checkpoint.center())?)
                .unwrap_or(&0),
        });
    }
    let mut portals = Vec::with_capacity(level.world_portals.len());
    for portal in &level.world_portals {
        portals.push(WorldPortalAnchor {
            portal_id: portal.id,
            source_chunk: *chunk_ids_by_key
                .get(&chunk_key_for_world(portal.center())?)
                .unwrap_or(&0),
            receiver_id: portal.receiver_id,
            render: portal.seamless,
            lighting: portal.seamless,
            physics: true,
        });
    }

    Ok(LevelPackage {
        manifest: LevelManifest {
            id: stable_level_id(&level.name),
            title: level.name.clone(),
            author: "Author".to_string(),
            entry: WORLD_INDEX_PATH.to_string(),
            chunk_size_units: CHUNK_SIZE_UNITS,
            coord_scale: COORD_SCALE,
        },
        index: WorldIndex {
            schema: PACKAGE_SCHEMA,
            coord_scale: COORD_SCALE,
            chunk_size_units: CHUNK_SIZE_UNITS,
            level_name: level.name.clone(),
            spawn: level.spawn,
            chunks: index_chunks,
            triggers,
            checkpoints,
            portals,
        },
        chunks,
    })
}

pub(super) fn package_to_level(package: &LevelPackage) -> io::Result<LevelSpec> {
    let mut spec = LevelSpec {
        name: package.index.level_name.clone(),
        spawn: package.index.spawn,
        solids: Vec::new(),
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

    for chunk in &package.chunks {
        for index in 0..chunk.static_rects.len() {
            let surface = unpack_surface_meta(chunk.static_rects.meta[index]);
            let solid = rect_to_solid(
                chunk.origin_q,
                &chunk.static_rects,
                index,
                surface.portalable,
            );
            let object_index = spec.solids.len();

            spec.solids.push(solid);
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Solid,
                object_index,
                surface.group_id,
                chunk.static_rects.editor_layer[index],
            );
        }
        for index in 0..chunk.hazard_rects.len() {
            let hazard = Hazard {
                solid: rect_to_solid(chunk.origin_q, &chunk.hazard_rects, index, false),
            };
            let surface = unpack_surface_meta(chunk.hazard_rects.meta[index]);
            let object_index = spec.hazards.len();

            spec.hazards.push(hazard);
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Hazard,
                object_index,
                surface.group_id,
                chunk.hazard_rects.editor_layer[index],
            );
        }
        for index in 0..chunk.doors.len() {
            let solid = rect_to_solid(chunk.origin_q, &chunk.doors.rects, index, false);
            let mut door = Door::with_radius(
                solid.pos().x,
                solid.pos().y,
                solid.size().x,
                solid.size().y,
                dequant_u16_units(chunk.doors.radius[index]),
            );
            let surface = unpack_surface_meta(chunk.doors.rects.meta[index]);
            let object_index = spec.doors.len();

            door.solid.set_rotation(solid.rotation());
            door.speed = chunk.doors.speed[index] as f32 / 256.0;
            door.automatic = chunk.doors.automatic[index] != 0;
            spec.doors.push(door);
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Door,
                object_index,
                surface.group_id,
                chunk.doors.rects.editor_layer[index],
            );
        }
        for index in 0..chunk.checkpoints.len() {
            let solid = rect_to_solid(chunk.origin_q, &chunk.checkpoints, index, false);
            let surface = unpack_surface_meta(chunk.checkpoints.meta[index]);
            let object_index = spec.checkpoints.len();

            spec.checkpoints.push(Checkpoint::new(
                solid.pos().x,
                solid.pos().y,
                solid.size().x,
                solid.size().y,
            ));
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Checkpoint,
                object_index,
                surface.group_id,
                chunk.checkpoints.editor_layer[index],
            );
        }
        for index in 0..chunk.triggers.len() {
            let solid = rect_to_solid(chunk.origin_q, &chunk.triggers.rects, index, false);
            let kind = match chunk.triggers.kind[index] {
                0 => LevelTriggerKind::LevelStart,
                1 => LevelTriggerKind::LevelEnd,
                2 => LevelTriggerKind::EnemySpawn {
                    enemy_id: chunk.triggers.enemy_id[index].max(1),
                },
                other => {
                    return Err(invalid_data(format!(
                        "unsupported trigger kind {other} in chunk {}",
                        chunk.chunk_id
                    )));
                }
            };
            let surface = unpack_surface_meta(chunk.triggers.rects.meta[index]);
            let object_index = spec.triggers.len();

            spec.triggers.push(LevelTrigger {
                solid,
                kind,
                fired: false,
            });
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Trigger,
                object_index,
                surface.group_id,
                chunk.triggers.rects.editor_layer[index],
            );
        }
        for index in 0..chunk.enemy_spawns.x.len() {
            let pos = dequant_local_point(
                chunk.origin_q,
                chunk.enemy_spawns.x[index],
                chunk.enemy_spawns.y[index],
            );
            let enemy = match chunk.enemy_spawns.kind[index] {
                0 if chunk.enemy_spawns.spawn_wave[index] > 0 => Enemy::filth_spawn(
                    pos.x,
                    pos.y,
                    chunk.enemy_spawns.spawn_id[index].max(1),
                    chunk.enemy_spawns.spawn_wave[index],
                ),
                0 => Enemy::filth(pos.x, pos.y),
                other => {
                    return Err(invalid_data(format!(
                        "unsupported enemy kind {other} in chunk {}",
                        chunk.chunk_id
                    )));
                }
            };
            let surface = unpack_surface_meta(chunk.enemy_spawns.meta[index]);
            let object_index = spec.enemies.len();

            spec.enemies.push(enemy);
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Enemy,
                object_index,
                surface.group_id,
                chunk.enemy_spawns.editor_layer[index],
            );
        }
        for index in 0..chunk.text_points.x.len() {
            let pos = dequant_local_point(
                chunk.origin_q,
                chunk.text_points.x[index],
                chunk.text_points.y[index],
            );
            let surface = unpack_surface_meta(chunk.text_points.meta[index]);
            let object_index = spec.texts.len();

            spec.texts
                .push(LevelText::new(pos, chunk.text_points.text[index].clone()));
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::Text,
                object_index,
                surface.group_id,
                chunk.text_points.editor_layer[index],
            );
        }
        for index in 0..chunk.world_portals.x.len() {
            let pos = dequant_local_point(
                chunk.origin_q,
                chunk.world_portals.x[index],
                chunk.world_portals.y[index],
            );
            let normal = dequant_unit_vec(
                chunk.world_portals.normal_x[index],
                chunk.world_portals.normal_y[index],
            );
            let tangent = dequant_unit_vec(
                chunk.world_portals.tangent_x[index],
                chunk.world_portals.tangent_y[index],
            );
            let mut portal = Portal::with_tangent(
                pos.x,
                pos.y,
                normal,
                tangent,
                dequant_u16_units(chunk.world_portals.width[index]),
                Color::rgb(154, 120, 255),
            );
            let flags = chunk.world_portals.flags[index];
            let surface = unpack_surface_meta(chunk.world_portals.meta[index]);
            let object_index = spec.world_portals.len();

            portal.scale = (chunk.world_portals.scale[index] as f32 / 256.0).max(0.01);
            portal.scale_objects = flags & 0b0000_0001 != 0;
            spec.world_portals.push(WorldPortal {
                portal,
                id: chunk.world_portals.portal_id[index],
                receiver_id: chunk.world_portals.receiver_id[index],
                priority: chunk.world_portals.priority[index],
                seamless: flags & 0b0000_0010 != 0,
                seamless_depth: dequant_u16_units(chunk.world_portals.seamless_depth[index]),
                seamless_angle: chunk.world_portals.seamless_angle[index] as f32 / 10.0,
                seamless_rely_on_walls: flags & 0b0000_0100 != 0,
            });
            push_loaded_meta(
                &mut spec.metadata,
                LevelObjectKind::WorldPortal,
                object_index,
                surface.group_id,
                chunk.world_portals.editor_layer[index],
            );
        }
    }

    Ok(spec)
}

#[derive(Clone, Copy)]
struct SurfaceMeta {
    portalable: bool,
    view_layer: u8,
    group_id: u16,
}

#[derive(Clone, Copy)]
struct RectRecord {
    meta: u16,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    rotation: i16,
    editor_layer: i16,
}

struct ChunkBuilder {
    origin_q: [i32; 2],
    bounds: Option<[i16; 4]>,
    static_rects: RectSoABuilder,
    hazard_rects: RectSoABuilder,
    doors: DoorSoABuilder,
    checkpoints: RectSoABuilder,
    triggers: TriggerSoABuilder,
    enemy_spawns: EnemySpawnSoABuilder,
    text_points: TextPointSoABuilder,
    world_portals: WorldPortalSoABuilder,
}

impl ChunkBuilder {
    fn new(key: (i32, i32)) -> Self {
        Self {
            origin_q: chunk_origin_for_key(key),
            bounds: None,
            static_rects: RectSoABuilder::default(),
            hazard_rects: RectSoABuilder::default(),
            doors: DoorSoABuilder::default(),
            checkpoints: RectSoABuilder::default(),
            triggers: TriggerSoABuilder::default(),
            enemy_spawns: EnemySpawnSoABuilder::default(),
            text_points: TextPointSoABuilder::default(),
            world_portals: WorldPortalSoABuilder::default(),
        }
    }

    fn push_solid(&mut self, solid: Solid, meta: ObjectMeta) -> io::Result<()> {
        let record =
            self.rect_record(solid, meta, solid.portalable, WorldAabb::from_solid(solid))?;

        self.static_rects.push(record);
        Ok(())
    }

    fn push_hazard(&mut self, hazard: Hazard, meta: ObjectMeta) -> io::Result<()> {
        let record = self.rect_record(
            hazard.solid,
            meta,
            false,
            WorldAabb::from_solid(hazard.solid),
        )?;

        self.hazard_rects.push(record);
        Ok(())
    }

    fn push_door(&mut self, door: Door, meta: ObjectMeta) -> io::Result<()> {
        let record =
            self.rect_record(door.solid, meta, false, WorldAabb::from_solid(door.solid))?;
        let radius = quant_u16_units(door.trigger_radius, "door trigger radius")?;
        let speed = quant_u16_fixed(door.speed, 256.0, "door speed")?;

        self.doors.push(record, radius, speed, door.automatic);
        Ok(())
    }

    fn push_checkpoint(&mut self, checkpoint: Checkpoint, meta: ObjectMeta) -> io::Result<()> {
        let record = self.rect_record(
            checkpoint.solid,
            meta,
            false,
            WorldAabb::from_solid(checkpoint.solid),
        )?;

        self.checkpoints.push(record);
        Ok(())
    }

    fn push_trigger(&mut self, trigger: LevelTrigger, meta: ObjectMeta) -> io::Result<()> {
        let record = self.rect_record(
            trigger.solid,
            meta,
            false,
            WorldAabb::from_solid(trigger.solid),
        )?;
        let (kind, enemy_id) = match trigger.kind {
            LevelTriggerKind::LevelStart => (0, 0),
            LevelTriggerKind::LevelEnd => (1, 0),
            LevelTriggerKind::EnemySpawn { enemy_id } => (2, enemy_id.max(1)),
        };

        self.triggers.push(record, kind, enemy_id);
        Ok(())
    }

    fn push_enemy(&mut self, enemy: &Enemy, meta: ObjectMeta) -> io::Result<()> {
        let surface = pack_surface_meta(false, meta)?;
        let local = quant_local_point(self.origin_q, enemy.spawn_pos)?;

        self.include_point(local);
        self.enemy_spawns.meta.push(surface);
        self.enemy_spawns.x.push(local[0]);
        self.enemy_spawns.y.push(local[1]);
        self.enemy_spawns.kind.push(match enemy.kind {
            EnemyKind::Filth => 0,
        });
        self.enemy_spawns.spawn_id.push(enemy.spawn_id);
        self.enemy_spawns.spawn_wave.push(enemy.spawn_wave);
        self.enemy_spawns.editor_layer.push(meta.editor_layer);
        Ok(())
    }

    fn push_text(&mut self, text: &LevelText, meta: ObjectMeta) -> io::Result<()> {
        let surface = pack_surface_meta(false, meta)?;
        let local = quant_local_point(self.origin_q, text.pos)?;

        self.include_point(local);
        self.text_points.meta.push(surface);
        self.text_points.x.push(local[0]);
        self.text_points.y.push(local[1]);
        self.text_points.editor_layer.push(meta.editor_layer);
        self.text_points.text.push(text.text.clone());
        Ok(())
    }

    fn push_world_portal(&mut self, portal: WorldPortal, meta: ObjectMeta) -> io::Result<()> {
        let surface = pack_surface_meta(false, meta)?;
        let local = quant_local_point(self.origin_q, portal.center())?;
        let flags = u8::from(portal.portal.scale_objects)
            | (u8::from(portal.seamless) << 1)
            | (u8::from(portal.seamless_rely_on_walls) << 2);

        self.include_point(local);
        self.world_portals.meta.push(surface);
        self.world_portals.x.push(local[0]);
        self.world_portals.y.push(local[1]);
        self.world_portals
            .normal_x
            .push(quant_unit(portal.portal.normal().x));
        self.world_portals
            .normal_y
            .push(quant_unit(portal.portal.normal().y));
        self.world_portals
            .tangent_x
            .push(quant_unit(portal.portal.tangent().x));
        self.world_portals
            .tangent_y
            .push(quant_unit(portal.portal.tangent().y));
        self.world_portals
            .width
            .push(quant_u16_units(portal.portal.width, "world portal width")?);
        self.world_portals.portal_id.push(portal.id);
        self.world_portals.receiver_id.push(portal.receiver_id);
        self.world_portals.priority.push(portal.priority);
        self.world_portals.scale.push(quant_u16_fixed(
            portal.portal.scale.max(0.01),
            256.0,
            "world portal scale",
        )?);
        self.world_portals.flags.push(flags);
        self.world_portals.seamless_depth.push(quant_u16_units(
            portal.seamless_depth,
            "world portal seamless depth",
        )?);
        self.world_portals.seamless_angle.push(quant_u16_fixed(
            portal.seamless_angle.clamp(1.0, 360.0),
            10.0,
            "world portal seamless angle",
        )?);
        self.world_portals.editor_layer.push(meta.editor_layer);
        Ok(())
    }

    fn rect_record(
        &mut self,
        solid: Solid,
        meta: ObjectMeta,
        portalable: bool,
        bounds: WorldAabb,
    ) -> io::Result<RectRecord> {
        let pos = quant_local_point(self.origin_q, solid.pos())?;
        let size = quant_size(solid.size())?;
        let bounds = quant_local_aabb(self.origin_q, bounds)?;
        let record = RectRecord {
            meta: pack_surface_meta(portalable, meta)?,
            x: pos[0],
            y: pos[1],
            w: size[0],
            h: size[1],
            rotation: quant_rotation(solid.rotation()),
            editor_layer: meta.editor_layer,
        };

        self.include_aabb(bounds);
        Ok(record)
    }

    fn include_point(&mut self, point: [i16; 2]) {
        self.include_aabb([point[0], point[1], point[0], point[1]]);
    }

    fn include_aabb(&mut self, bounds: [i16; 4]) {
        if let Some(existing) = &mut self.bounds {
            existing[0] = existing[0].min(bounds[0]);
            existing[1] = existing[1].min(bounds[1]);
            existing[2] = existing[2].max(bounds[2]);
            existing[3] = existing[3].max(bounds[3]);
        } else {
            self.bounds = Some(bounds);
        }
    }

    fn finish(self, chunk_id: u32) -> WorldChunk {
        let bounds = self.bounds.unwrap_or([0, 0, 0, 0]);

        WorldChunk {
            chunk_id,
            origin_q: self.origin_q,
            bounds_local: bounds,
            static_rects: self.static_rects.finish(),
            hazard_rects: self.hazard_rects.finish(),
            doors: self.doors.finish(),
            checkpoints: self.checkpoints.finish(),
            triggers: self.triggers.finish(),
            enemy_spawns: self.enemy_spawns.finish(),
            text_points: self.text_points.finish(),
            world_portals: self.world_portals.finish(),
        }
    }
}

impl WorldChunk {
    pub(super) fn counts(&self) -> ChunkObjectCounts {
        ChunkObjectCounts {
            static_rects: self.static_rects.len() as u32,
            hazard_rects: self.hazard_rects.len() as u32,
            doors: self.doors.len() as u32,
            checkpoints: self.checkpoints.len() as u32,
            triggers: self.triggers.len() as u32,
            enemy_spawns: self.enemy_spawns.x.len() as u32,
            text_points: self.text_points.x.len() as u32,
            world_portals: self.world_portals.x.len() as u32,
        }
    }
}
fn rects_to_solids(
    origin_q: [i32; 2],
    block: &RectSoABlock,
    portalable: impl Fn(u16) -> bool,
) -> Vec<Solid> {
    (0..block.len())
        .map(|index| rect_to_solid(origin_q, block, index, portalable(block.meta[index])))
        .collect()
}

fn rect_to_solid(
    origin_q: [i32; 2],
    block: &RectSoABlock,
    index: usize,
    portalable: bool,
) -> Solid {
    let pos = dequant_local_point(origin_q, block.x[index], block.y[index]);
    let size = Vec2::new(
        dequant_u16_units(block.w[index]),
        dequant_u16_units(block.h[index]),
    );

    Solid::rotated(
        pos.x,
        pos.y,
        size.x.max(1.0),
        size.y.max(1.0),
        dequant_rotation(block.rotation[index]),
        portalable,
    )
}

fn pack_surface_meta(portalable: bool, meta: ObjectMeta) -> io::Result<u16> {
    if meta.group_id > 1023 {
        return Err(invalid_data(format!(
            "group_id {} exceeds v4 10-bit limit",
            meta.group_id
        )));
    }
    if !(0..=1023).contains(&meta.editor_layer) {
        return Err(invalid_data(format!(
            "editor_layer {} outside v4 range 0..1023",
            meta.editor_layer
        )));
    }

    let view_layer = DEFAULT_VIEW_LAYER;
    Ok(u16::from(portalable)
        | ((0u16 & 0b11) << 1)
        | ((view_layer as u16 & 0b111) << 3)
        | (meta.group_id << 6))
}

fn unpack_surface_meta(meta: u16) -> SurfaceMeta {
    SurfaceMeta {
        portalable: meta & 1 != 0,
        view_layer: ((meta >> 3) & 0b111) as u8,
        group_id: meta >> 6,
    }
}

fn object_meta(
    metadata: &[LevelObjectMeta],
    kind: LevelObjectKind,
    index: usize,
) -> io::Result<ObjectMeta> {
    let meta = object_meta_lossy(metadata, kind, index);

    if meta.group_id > 1023 {
        return Err(invalid_data(format!(
            "{kind:?} {index} group_id {} exceeds 1023",
            meta.group_id
        )));
    }
    if !(0..=1023).contains(&meta.editor_layer) {
        return Err(invalid_data(format!(
            "{kind:?} {index} editor_layer {} outside 0..1023",
            meta.editor_layer
        )));
    }

    Ok(meta)
}

fn object_meta_lossy(
    metadata: &[LevelObjectMeta],
    kind: LevelObjectKind,
    index: usize,
) -> ObjectMeta {
    metadata
        .iter()
        .find(|entry| entry.kind == kind && entry.index == index)
        .map(|entry| entry.meta)
        .unwrap_or_default()
}

fn push_loaded_meta(
    metadata: &mut Vec<LevelObjectMeta>,
    kind: LevelObjectKind,
    index: usize,
    group_id: u16,
    editor_layer: i16,
) {
    let meta = ObjectMeta {
        group_id,
        editor_layer,
    };

    if meta != ObjectMeta::default() {
        metadata.push(LevelObjectMeta { kind, index, meta });
    }
}

fn trigger_runtime_kind(kind: LevelTriggerKind) -> u8 {
    match kind {
        LevelTriggerKind::LevelStart => 0,
        LevelTriggerKind::LevelEnd => 1,
        LevelTriggerKind::EnemySpawn { .. } => 2,
    }
}

use std::collections::{BTreeSet, HashMap};
use std::io;

use glam::Vec2;

use super::{
    ChunkObjectCounts, DEFAULT_VIEW_LAYER, LevelSpec, WORLD_INDEX_PATH, invalid_data,
    parse_key_values, quote,
};
use crate::game::level::{LevelObjectKind, LevelObjectMeta, LevelTriggerKind, ObjectMeta};

#[derive(Clone, Debug)]
pub struct WorldIndex {
    pub schema: u8,
    pub coord_scale: i32,
    pub chunk_size_units: i32,
    pub level_name: String,
    pub spawn: Vec2,
    pub chunks: Vec<WorldChunkEntry>,
    pub triggers: Vec<TriggerAnchor>,
    pub checkpoints: Vec<CheckpointAnchor>,
    pub portals: Vec<WorldPortalAnchor>,
}

#[derive(Clone, Debug)]
pub struct WorldChunkEntry {
    pub id: u32,
    pub origin_q: [i32; 2],
    pub bbox_units: [f32; 4],
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub counts: ChunkObjectCounts,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckpointAnchor {
    pub index: u32,
    pub anchor: Vec2,
    pub chunk_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerAnchor {
    pub trigger_id: u16,
    pub source_chunk: u32,
    pub target_group: u16,
    pub kind: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPortalAnchor {
    pub portal_id: u16,
    pub source_chunk: u32,
    pub receiver_id: u16,
    pub render: bool,
    pub lighting: bool,
    pub physics: bool,
}
pub(super) fn format_world_index(index: &WorldIndex) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "portals_world_index version={} endian=little coord_scale={} chunk_size_units={} encoding=typed_bitpacked_soa\n",
        index.schema, index.coord_scale, index.chunk_size_units
    ));
    output.push_str(&format!(
        "level name={} spawn_x={} spawn_y={}\n",
        quote(&index.level_name),
        index.spawn.x,
        index.spawn.y
    ));
    for chunk in &index.chunks {
        output.push_str(&format!(
            "chunk id={} origin_q={},{} bbox_units={},{},{},{} path={} bytes={} sha256={} static_rects={} hazard_rects={} doors={} checkpoints={} triggers={} enemy_spawns={} text_points={} world_portals={}\n",
            chunk.id,
            chunk.origin_q[0],
            chunk.origin_q[1],
            chunk.bbox_units[0],
            chunk.bbox_units[1],
            chunk.bbox_units[2],
            chunk.bbox_units[3],
            quote(&chunk.path),
            chunk.bytes,
            chunk.sha256,
            chunk.counts.static_rects,
            chunk.counts.hazard_rects,
            chunk.counts.doors,
            chunk.counts.checkpoints,
            chunk.counts.triggers,
            chunk.counts.enemy_spawns,
            chunk.counts.text_points,
            chunk.counts.world_portals
        ));
    }
    for checkpoint in &index.checkpoints {
        output.push_str(&format!(
            "checkpoint index={} anchor={},{} restart_chunks={}\n",
            checkpoint.index, checkpoint.anchor.x, checkpoint.anchor.y, checkpoint.chunk_id
        ));
    }
    for trigger in &index.triggers {
        output.push_str(&format!(
            "trigger trigger_id={} kind={} source_chunk={} target_group={} activation_policy=once payload_offset=0\n",
            trigger.trigger_id,
            format_trigger_kind(trigger.kind),
            trigger.source_chunk,
            trigger.target_group
        ));
    }
    for portal in &index.portals {
        output.push_str(&format!(
            "portal id={} kind=world source_chunk={} receiver={} render={} lighting={} physics={}\n",
            portal.portal_id,
            portal.source_chunk,
            portal.receiver_id,
            portal.render,
            portal.lighting,
            portal.physics
        ));
    }

    output
}

pub(super) fn parse_world_index(source: &str) -> io::Result<WorldIndex> {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let Some(header) = lines.next() else {
        return Err(invalid_data("empty world.index"));
    };
    let Some(rest) = header.strip_prefix("portals_world_index ") else {
        return Err(invalid_data("missing portals_world_index header"));
    };
    let header_fields = parse_key_values(rest);
    let schema = field_u8(&header_fields, "version")
        .ok_or_else(|| invalid_data("world.index missing version"))?;
    let coord_scale = field_i32(&header_fields, "coord_scale")
        .ok_or_else(|| invalid_data("world.index missing coord_scale"))?;
    let chunk_size_units = field_i32(&header_fields, "chunk_size_units")
        .ok_or_else(|| invalid_data("world.index missing chunk_size_units"))?;
    if header_fields.get("encoding").map(String::as_str) != Some("typed_bitpacked_soa") {
        return Err(invalid_data(
            "world.index must use typed_bitpacked_soa encoding",
        ));
    }

    let mut level_name = "Level".to_string();
    let mut spawn = Vec2::ZERO;
    let mut chunks = Vec::new();
    let mut triggers = Vec::new();
    let mut checkpoints = Vec::new();
    let mut portals = Vec::new();

    for line in lines {
        let Some(index) = line.find(char::is_whitespace) else {
            continue;
        };
        let command = &line[..index];
        let rest = line[index..].trim();
        let fields = parse_key_values(rest.trim());
        match command {
            "level" => {
                if let Some(name) = fields.get("name") {
                    level_name = name.clone();
                }
                if let (Some(x), Some(y)) =
                    (field_f32(&fields, "spawn_x"), field_f32(&fields, "spawn_y"))
                {
                    spawn = Vec2::new(x, y);
                }
            }
            "chunk" => {
                let id = field_u32(&fields, "id")
                    .ok_or_else(|| invalid_data("world.index chunk missing id"))?;
                let origin_q = parse_i32_pair(
                    fields
                        .get("origin_q")
                        .ok_or_else(|| invalid_data("world.index chunk missing origin_q"))?,
                )?;
                let bbox_units = parse_f32_quad(
                    fields
                        .get("bbox_units")
                        .ok_or_else(|| invalid_data("world.index chunk missing bbox_units"))?,
                )?;
                let path = fields
                    .get("path")
                    .cloned()
                    .ok_or_else(|| invalid_data("world.index chunk missing path"))?;

                chunks.push(WorldChunkEntry {
                    id,
                    origin_q,
                    bbox_units,
                    path,
                    bytes: fields
                        .get("bytes")
                        .and_then(|value| value.parse::<u64>().ok())
                        .ok_or_else(|| invalid_data("world.index chunk missing bytes"))?,
                    sha256: fields
                        .get("sha256")
                        .cloned()
                        .ok_or_else(|| invalid_data("world.index chunk missing sha256"))?,
                    counts: ChunkObjectCounts {
                        static_rects: field_u32(&fields, "static_rects").unwrap_or(0),
                        hazard_rects: field_u32(&fields, "hazard_rects").unwrap_or(0),
                        doors: field_u32(&fields, "doors").unwrap_or(0),
                        checkpoints: field_u32(&fields, "checkpoints").unwrap_or(0),
                        triggers: field_u32(&fields, "triggers").unwrap_or(0),
                        enemy_spawns: field_u32(&fields, "enemy_spawns").unwrap_or(0),
                        text_points: field_u32(&fields, "text_points").unwrap_or(0),
                        world_portals: field_u32(&fields, "world_portals").unwrap_or(0),
                    },
                });
            }
            "checkpoint" => {
                if let (Some(index), Some(anchor), Some(chunk_id)) = (
                    field_u32(&fields, "index"),
                    fields
                        .get("anchor")
                        .map(|value| parse_f32_pair(value))
                        .transpose()?,
                    fields
                        .get("restart_chunks")
                        .and_then(|value| value.split(',').next())
                        .and_then(|value| value.parse::<u32>().ok()),
                ) {
                    checkpoints.push(CheckpointAnchor {
                        index,
                        anchor: Vec2::new(anchor[0], anchor[1]),
                        chunk_id,
                    });
                }
            }
            "trigger" => {
                let trigger_id = field_u16(&fields, "trigger_id")
                    .or_else(|| field_u16(&fields, "id"))
                    .ok_or_else(|| invalid_data("world.index trigger missing trigger_id"))?;
                let source_chunk = field_u32(&fields, "source_chunk")
                    .ok_or_else(|| invalid_data("world.index trigger missing source_chunk"))?;
                let kind = fields
                    .get("kind")
                    .and_then(|value| parse_trigger_kind(value))
                    .ok_or_else(|| invalid_data("world.index trigger missing kind"))?;
                let target_group = fields
                    .get("target_group")
                    .map(|value| {
                        value
                            .parse::<u16>()
                            .map_err(|_| invalid_data("world.index trigger invalid target_group"))
                    })
                    .transpose()?
                    .unwrap_or(0);

                triggers.push(TriggerAnchor {
                    trigger_id,
                    source_chunk,
                    target_group,
                    kind,
                });
            }
            "portal" => {
                if let (Some(portal_id), Some(source_chunk), Some(receiver_id)) = (
                    field_u16(&fields, "id"),
                    field_u32(&fields, "source_chunk"),
                    field_u16(&fields, "receiver"),
                ) {
                    portals.push(WorldPortalAnchor {
                        portal_id,
                        source_chunk,
                        receiver_id,
                        render: field_bool(&fields, "render").unwrap_or(false),
                        lighting: field_bool(&fields, "lighting").unwrap_or(false),
                        physics: field_bool(&fields, "physics").unwrap_or(true),
                    });
                }
            }
            _ => {}
        }
    }

    if chunks.is_empty() {
        return Err(invalid_data("world.index contains no chunks"));
    }

    Ok(WorldIndex {
        schema,
        coord_scale,
        chunk_size_units,
        level_name,
        spawn,
        chunks,
        triggers,
        checkpoints,
        portals,
    })
}

pub(super) fn format_layers(level: &LevelSpec) -> String {
    let mut output = String::from(
        "view_layer 0 name=\"Back\"\nview_layer 1 name=\"Gameplay\"\nview_layer 2 name=\"Front\"\n",
    );
    let mut layers = BTreeSet::new();

    layers.insert(0);
    for meta in &level.metadata {
        layers.insert(meta.meta.editor_layer);
    }
    for layer in layers {
        output.push_str(&format!(
            "editor_layer {layer} name={}\n",
            quote(&format!("Layer {layer}"))
        ));
    }

    output
}

pub(super) fn format_groups(level: &LevelSpec) -> String {
    let mut groups = BTreeSet::new();
    for meta in &level.metadata {
        if meta.meta.group_id != 0 {
            groups.insert(meta.meta.group_id);
        }
    }

    groups
        .into_iter()
        .map(|group| {
            format!(
                "group {group} name={} reset=checkpoint\n",
                quote(&format!("Group {group}"))
            )
        })
        .collect()
}

pub(super) fn format_triggers(level: &LevelSpec) -> String {
    let mut output = String::new();

    for (index, trigger) in level.triggers.iter().enumerate() {
        let kind = match trigger.kind {
            LevelTriggerKind::LevelStart => "checkpoint",
            LevelTriggerKind::LevelEnd => "level_end",
            LevelTriggerKind::EnemySpawn { .. } => "spawn",
        };
        output.push_str(&format!(
            "trigger trigger_id={} kind={} source_chunk={} target_group={} activation_policy=once payload_offset=0\n",
            index, kind, 0, 0
        ));
    }

    output
}

pub(super) fn format_checkpoints(level: &LevelSpec) -> String {
    let mut output = String::new();

    for (index, checkpoint) in level.checkpoints.iter().enumerate() {
        output.push_str(&format!(
            "checkpoint {} spawn={},{} chunk={} radius={},{} reusable=true invisible=false pinned_chunks={} restart_chunks={}\n",
            index,
            checkpoint.center().x,
            checkpoint.center().y,
            0,
            checkpoint.solid.size().x,
            checkpoint.solid.size().y,
            0,
            0
        ));
    }

    output
}

pub(super) fn format_portals(level: &LevelSpec) -> String {
    let mut output = String::new();

    for portal in &level.world_portals {
        output.push_str(&format!(
            "world_portal id={} receiver={} render={} lighting={} physics=true width={} x={} y={} normal={},{}\n",
            portal.id,
            portal.receiver_id,
            portal.seamless,
            portal.seamless,
            portal.portal.width,
            portal.portal.pos.x,
            portal.portal.pos.y,
            portal.portal.normal().x,
            portal.portal.normal().y
        ));
    }

    output
}

pub(super) fn format_source_level(level: &LevelSpec) -> String {
    let mut output = String::new();

    output.push_str("portals_world version=4\n");
    output.push_str(&format!(
        "level name={} spawn_x={} spawn_y={}\n",
        quote(&level.name),
        level.spawn.x,
        level.spawn.y
    ));
    for (index, solid) in level.solids.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Solid, index);
        output.push_str(&format!(
            "solid x={} y={} w={} h={} portalable={} surface=normal group_id={} view_layer={} editor_layer={} rotation={}\n",
            solid.pos().x,
            solid.pos().y,
            solid.size().x,
            solid.size().y,
            solid.portalable,
            meta.group_id,
            DEFAULT_VIEW_LAYER,
            meta.editor_layer,
            solid.rotation()
        ));
    }
    for (index, hazard) in level.hazards.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Hazard, index);
        output.push_str(&format!(
            "hazard x={} y={} w={} h={} group_id={} editor_layer={} rotation={}\n",
            hazard.solid.pos().x,
            hazard.solid.pos().y,
            hazard.solid.size().x,
            hazard.solid.size().y,
            meta.group_id,
            meta.editor_layer,
            hazard.solid.rotation()
        ));
    }
    for (index, door) in level.doors.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Door, index);
        output.push_str(&format!(
            "door x={} y={} w={} h={} radius={} speed={} automatic={} group_id={} editor_layer={} rotation={}\n",
            door.solid.pos().x,
            door.solid.pos().y,
            door.solid.size().x,
            door.solid.size().y,
            door.trigger_radius,
            door.speed,
            door.automatic,
            meta.group_id,
            meta.editor_layer,
            door.solid.rotation()
        ));
    }
    for (index, checkpoint) in level.checkpoints.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Checkpoint, index);
        output.push_str(&format!(
            "checkpoint x={} y={} w={} h={} group_id={} editor_layer={}\n",
            checkpoint.solid.pos().x,
            checkpoint.solid.pos().y,
            checkpoint.solid.size().x,
            checkpoint.solid.size().y,
            meta.group_id,
            meta.editor_layer
        ));
    }
    for (index, enemy) in level.enemies.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Enemy, index);
        output.push_str(&format!(
            "enemy kind=filth spawn_x={} spawn_y={} spawn_id={} spawn_wave={} group_id={} editor_layer={}\n",
            enemy.spawn_pos.x,
            enemy.spawn_pos.y,
            enemy.spawn_id,
            enemy.spawn_wave,
            meta.group_id,
            meta.editor_layer
        ));
    }
    for (index, trigger) in level.triggers.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Trigger, index);
        let (kind, enemy_id) = match trigger.kind {
            LevelTriggerKind::LevelStart => ("level_start", None),
            LevelTriggerKind::LevelEnd => ("level_end", None),
            LevelTriggerKind::EnemySpawn { enemy_id } => ("enemy_spawn", Some(enemy_id)),
        };
        let enemy_id = enemy_id
            .map(|enemy_id| format!(" enemy_id={enemy_id}"))
            .unwrap_or_default();
        output.push_str(&format!(
            "trigger kind={} x={} y={} w={} h={}{} group_id={} editor_layer={}\n",
            kind,
            trigger.solid.pos().x,
            trigger.solid.pos().y,
            trigger.solid.size().x,
            trigger.solid.size().y,
            enemy_id,
            meta.group_id,
            meta.editor_layer
        ));
    }
    for (index, text) in level.texts.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::Text, index);
        output.push_str(&format!(
            "text x={} y={} group_id={} editor_layer={} value={}\n",
            text.pos.x,
            text.pos.y,
            meta.group_id,
            meta.editor_layer,
            quote(&text.text)
        ));
    }
    for (index, portal) in level.world_portals.iter().enumerate() {
        let meta = object_meta_lossy(&level.metadata, LevelObjectKind::WorldPortal, index);
        output.push_str(&format!(
            "world_portal x={} y={} normal_x={} normal_y={} tangent_x={} tangent_y={} width={} portal_id={} receiver_id={} priority={} scale={} scale_objects={} seamless={} seamless_depth={} seamless_angle={} seamless_rely_on_walls={} group_id={} editor_layer={}\n",
            portal.portal.pos.x,
            portal.portal.pos.y,
            portal.portal.normal().x,
            portal.portal.normal().y,
            portal.portal.tangent().x,
            portal.portal.tangent().y,
            portal.portal.width,
            portal.id,
            portal.receiver_id,
            portal.priority,
            portal.portal.scale,
            portal.portal.scale_objects,
            portal.seamless,
            portal.seamless_depth,
            portal.seamless_angle,
            portal.seamless_rely_on_walls,
            meta.group_id,
            meta.editor_layer
        ));
    }

    output
}

pub(super) fn format_debug_level(level: &LevelSpec) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "# generated debug dump; gameplay uses {WORLD_INDEX_PATH} and .wchunk files\n"
    ));
    output.push_str(&format_source_level(level));
    output
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

fn field_u8(fields: &HashMap<String, String>, key: &str) -> Option<u8> {
    fields.get(key)?.parse::<u8>().ok()
}

fn field_u16(fields: &HashMap<String, String>, key: &str) -> Option<u16> {
    fields.get(key)?.parse::<u16>().ok()
}

fn field_u32(fields: &HashMap<String, String>, key: &str) -> Option<u32> {
    fields.get(key)?.parse::<u32>().ok()
}

fn field_i32(fields: &HashMap<String, String>, key: &str) -> Option<i32> {
    fields.get(key)?.parse::<i32>().ok()
}

fn field_f32(fields: &HashMap<String, String>, key: &str) -> Option<f32> {
    fields
        .get(key)?
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn field_bool(fields: &HashMap<String, String>, key: &str) -> Option<bool> {
    fields
        .get(key)
        .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
}

fn parse_trigger_kind(value: &str) -> Option<u8> {
    match value {
        "0" | "level_start" | "checkpoint" => Some(0),
        "1" | "level_end" => Some(1),
        "2" | "enemy_spawn" | "spawn" => Some(2),
        _ => value.parse::<u8>().ok(),
    }
}

fn format_trigger_kind(kind: u8) -> String {
    match kind {
        0 => "checkpoint".to_string(),
        1 => "level_end".to_string(),
        2 => "spawn".to_string(),
        _ => kind.to_string(),
    }
}

fn parse_i32_pair(value: &str) -> io::Result<[i32; 2]> {
    let values = value
        .split(',')
        .map(|part| {
            part.parse::<i32>()
                .map_err(|_| invalid_data("invalid i32 pair"))
        })
        .collect::<io::Result<Vec<_>>>()?;

    match values.as_slice() {
        [x, y] => Ok([*x, *y]),
        _ => Err(invalid_data("invalid i32 pair length")),
    }
}

fn parse_f32_pair(value: &str) -> io::Result<[f32; 2]> {
    let values = value
        .split(',')
        .map(|part| {
            part.parse::<f32>()
                .map_err(|_| invalid_data("invalid f32 pair"))
        })
        .collect::<io::Result<Vec<_>>>()?;

    match values.as_slice() {
        [x, y] if x.is_finite() && y.is_finite() => Ok([*x, *y]),
        _ => Err(invalid_data("invalid f32 pair")),
    }
}

fn parse_f32_quad(value: &str) -> io::Result<[f32; 4]> {
    let values = value
        .split(',')
        .map(|part| {
            part.parse::<f32>()
                .map_err(|_| invalid_data("invalid f32 quad"))
        })
        .collect::<io::Result<Vec<_>>>()?;

    match values.as_slice() {
        [a, b, c, d] if a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite() => {
            Ok([*a, *b, *c, *d])
        }
        _ => Err(invalid_data("invalid f32 quad")),
    }
}

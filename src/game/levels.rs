use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// Small text level format: name, spawn, then solid rows.
use glam::Vec2;

use crate::game::enemy::{Enemy, EnemyKind};
use crate::game::level::{
    Checkpoint, Door, Hazard, Level, LevelObjectKind, LevelObjectMeta, LevelText, LevelTrigger,
    LevelTriggerKind, ObjectMeta, Solid, WorldPortal,
};
use crate::game::portal::{Color, Portal};

const LEVEL_DIR: &str = "levels";
const LEVEL_FORMAT_VERSION: u8 = 3;

#[derive(Clone)]
pub struct LevelSpec {
    pub name: String,
    pub spawn: Vec2,
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
    pub hazards: Vec<Hazard>,
    pub checkpoints: Vec<Checkpoint>,
    pub enemies: Vec<Enemy>,
    pub triggers: Vec<LevelTrigger>,
    pub texts: Vec<LevelText>,
    pub world_portals: Vec<WorldPortal>,
    pub metadata: Vec<LevelObjectMeta>,
    pub path: Option<PathBuf>,
}

impl LevelSpec {
    pub fn fallback() -> Self {
        Self {
            name: "Test Chamber".to_string(),
            spawn: Vec2::new(110.0, 480.0),
            solids: Level::test_level().solids,
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: Vec::new(),
            triggers: Vec::new(),
            texts: Vec::new(),
            world_portals: Vec::new(),
            metadata: Vec::new(),
            path: None,
        }
    }

    pub fn custom_template(name: String) -> Self {
        let spawn = Vec2::new(128.0, 320.0);

        Self {
            name,
            spawn,
            solids: vec![Solid::new(64.0, 400.0, 960.0, 32.0, false)],
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: Vec::new(),
            triggers: vec![
                LevelTrigger::level_start(spawn.x - 24.0, spawn.y - 40.0, 48.0, 80.0),
                LevelTrigger::level_end(920.0, 320.0, 48.0, 80.0),
            ],
            texts: Vec::new(),
            world_portals: Vec::new(),
            metadata: Vec::new(),
            path: None,
        }
    }

    pub fn replace_world(&mut self, world: &Level) {
        self.solids = world.solids.clone();
        self.doors = world.doors.clone();
        self.hazards = world.hazards.clone();
        self.checkpoints = world.checkpoints.clone();
        self.enemies = world.enemies.clone();
        self.triggers = world.triggers.clone();
        if let Some(spawn) = world.level_start_pos() {
            self.spawn = spawn;
        }
        self.texts = world.texts.clone();
        self.world_portals = world.world_portals.clone();
        self.metadata = world.metadata.clone();
    }

    pub fn level(&self) -> Level {
        Level {
            solids: self.solids.clone(),
            doors: self.doors.clone(),
            hazards: self.hazards.clone(),
            checkpoints: self.checkpoints.clone(),
            enemies: self.enemies.clone(),
            triggers: self.triggers.clone(),
            texts: self.texts.clone(),
            world_portals: self.world_portals.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

pub fn load_levels() -> Vec<LevelSpec> {
    let mut levels = fs::read_dir(LEVEL_DIR)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("lvl") {
                return None;
            }

            match parse_level_file(&path) {
                Ok(level) => Some(level),
                Err(err) => {
                    eprintln!("Skipping unreadable level {}: {err}", path.display());
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    levels.sort_by(|a, b| a.name.cmp(&b.name));
    if levels.is_empty() {
        levels.push(LevelSpec::fallback());
    }

    levels
}

pub fn save_level(level: &mut LevelSpec) -> io::Result<()> {
    fs::create_dir_all(LEVEL_DIR)?;
    let path = level
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(LEVEL_DIR).join(format!("{}.lvl", slug(&level.name))));
    // Versioned key/value rows are still hand-editable, but adding new fields is now safe.
    let mut body = String::new();

    body.push_str(&format!("portals_level version={LEVEL_FORMAT_VERSION}\n"));
    body.push_str(&format!(
        "level name={} spawn_x={} spawn_y={}\n",
        quote(&level.name),
        level.spawn.x,
        level.spawn.y
    ));
    for (index, solid) in level.solids.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Solid, index);
        body.push_str(&format!(
            "solid x={} y={} w={} h={} portalable={} rotation={} obj_id={} layer={}\n",
            solid.pos().x,
            solid.pos().y,
            solid.size().x,
            solid.size().y,
            solid.portalable,
            solid.rotation(),
            meta.id,
            meta.layer
        ));
    }
    for (index, door) in level.doors.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Door, index);
        body.push_str(&format!(
            "door x={} y={} w={} h={} radius={} rotation={} speed={} automatic={} obj_id={} layer={}\n",
            door.solid.pos().x,
            door.solid.pos().y,
            door.solid.size().x,
            door.solid.size().y,
            door.trigger_radius,
            door.solid.rotation(),
            door.speed,
            door.automatic,
            meta.id,
            meta.layer
        ));
    }
    for (index, hazard) in level.hazards.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Hazard, index);
        body.push_str(&format!(
            "hazard x={} y={} w={} h={} rotation={} obj_id={} layer={}\n",
            hazard.solid.pos().x,
            hazard.solid.pos().y,
            hazard.solid.size().x,
            hazard.solid.size().y,
            hazard.solid.rotation(),
            meta.id,
            meta.layer
        ));
    }
    for (index, checkpoint) in level.checkpoints.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Checkpoint, index);
        body.push_str(&format!(
            "checkpoint x={} y={} w={} h={} obj_id={} layer={}\n",
            checkpoint.solid.pos().x,
            checkpoint.solid.pos().y,
            checkpoint.solid.size().x,
            checkpoint.solid.size().y,
            meta.id,
            meta.layer
        ));
    }
    for (index, enemy) in level.enemies.iter().enumerate() {
        let kind = match enemy.kind {
            EnemyKind::Filth => "filth",
        };
        let meta = object_meta(&level.metadata, LevelObjectKind::Enemy, index);

        body.push_str(&format!(
            "enemy kind={} spawn_x={} spawn_y={} spawn_id={} spawn_wave={} obj_id={} layer={}\n",
            kind,
            enemy.spawn_pos.x,
            enemy.spawn_pos.y,
            enemy.spawn_id,
            enemy.spawn_wave,
            meta.id,
            meta.layer
        ));
    }
    for (index, trigger) in level.triggers.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Trigger, index);
        match trigger.kind {
            LevelTriggerKind::LevelStart => {
                body.push_str(&format_trigger("level_start", trigger.solid, None, meta));
            }
            LevelTriggerKind::LevelEnd => {
                body.push_str(&format_trigger("level_end", trigger.solid, None, meta));
            }
            LevelTriggerKind::EnemySpawn { enemy_id } => {
                body.push_str(&format_trigger(
                    "enemy_spawn",
                    trigger.solid,
                    Some(enemy_id),
                    meta,
                ));
            }
        }
    }
    for (index, text) in level.texts.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::Text, index);
        body.push_str(&format!(
            "text x={} y={} obj_id={} layer={} value={}\n",
            text.pos.x,
            text.pos.y,
            meta.id,
            meta.layer,
            quote(&text.text)
        ));
    }
    for (index, portal) in level.world_portals.iter().enumerate() {
        let meta = object_meta(&level.metadata, LevelObjectKind::WorldPortal, index);
        body.push_str(&format!(
            "world_portal x={} y={} normal_x={} normal_y={} tangent_x={} tangent_y={} width={} portal_id={} receiver_id={} priority={} scale={} scale_objects={} seamless={} seamless_depth={} seamless_angle={} seamless_rely_on_walls={} obj_id={} layer={}\n",
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
            meta.id,
            meta.layer
        ));
    }

    fs::write(&path, body)?;
    level.path = Some(path);

    Ok(())
}

fn parse_level_file(path: &Path) -> io::Result<LevelSpec> {
    let source = fs::read_to_string(path)?;
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let Some(header) = lines.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty level file",
        ));
    };
    let Some(("portals_level", header_rest)) = split_command(header) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing portals_level header",
        ));
    };
    let version = parse_key_values(header_rest)
        .get("version")
        .and_then(|value| value.parse::<u8>().ok());
    if version != Some(LEVEL_FORMAT_VERSION) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported level format version {version:?}"),
        ));
    }

    let mut name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Level")
        .replace('_', " ");
    let mut spawn = Vec2::new(110.0, 480.0);
    let mut solids = Vec::new();
    let mut doors = Vec::new();
    let mut hazards = Vec::new();
    let mut checkpoints = Vec::new();
    let mut enemies = Vec::new();
    let mut triggers = Vec::new();
    let mut texts = Vec::new();
    let mut world_portals = Vec::new();
    let mut metadata = Vec::new();

    for line in lines {
        let Some((command, rest)) = split_command(line) else {
            continue;
        };
        let keyed = looks_keyed(rest);
        if !keyed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("non-keyed row in v{LEVEL_FORMAT_VERSION} level: {command}"),
            ));
        }

        match command {
            "level" if keyed => {
                let fields = parse_key_values(rest);
                if let Some(value) = fields.get("name") {
                    name = value.clone();
                }
                if let (Some(x), Some(y)) =
                    (field_f32(&fields, "spawn_x"), field_f32(&fields, "spawn_y"))
                {
                    spawn = Vec2::new(x, y);
                }
            }
            "solid" if keyed => {
                let fields = parse_key_values(rest);
                let Some((pos, size)) = keyed_rect(&fields) else {
                    continue;
                };
                let portalable = field_bool(&fields, "portalable").unwrap_or(true);
                let rotation = field_f32(&fields, "rotation").unwrap_or(0.0);
                let index = solids.len();

                solids.push(Solid::rotated(
                    pos.x, pos.y, size.x, size.y, rotation, portalable,
                ));
                push_inline_meta(&fields, LevelObjectKind::Solid, index, &mut metadata);
            }
            "door" if keyed => {
                let fields = parse_key_values(rest);
                let Some((pos, size)) = keyed_rect(&fields) else {
                    continue;
                };
                let trigger_radius = field_f32(&fields, "radius")
                    .filter(|value| *value > 0.0)
                    .unwrap_or(112.0);
                let rotation = field_f32(&fields, "rotation").unwrap_or(0.0);
                let speed = field_f32(&fields, "speed")
                    .filter(|value| *value > 0.0)
                    .unwrap_or(3.6);
                let automatic = field_bool(&fields, "automatic").unwrap_or(true);
                let index = doors.len();
                let mut door = Door::with_radius(pos.x, pos.y, size.x, size.y, trigger_radius);

                door.solid.set_rotation(rotation);
                door.speed = speed.max(0.1);
                door.automatic = automatic;
                doors.push(door);
                push_inline_meta(&fields, LevelObjectKind::Door, index, &mut metadata);
            }
            "hazard" if keyed => {
                let fields = parse_key_values(rest);
                let Some((pos, size)) = keyed_rect(&fields) else {
                    continue;
                };
                let rotation = field_f32(&fields, "rotation").unwrap_or(0.0);
                let index = hazards.len();
                let mut hazard = Hazard::new(pos.x, pos.y, size.x, size.y);

                hazard.solid.set_rotation(rotation);
                hazards.push(hazard);
                push_inline_meta(&fields, LevelObjectKind::Hazard, index, &mut metadata);
            }
            "checkpoint" if keyed => {
                let fields = parse_key_values(rest);
                let Some((pos, size)) = keyed_rect(&fields) else {
                    continue;
                };
                let index = checkpoints.len();

                checkpoints.push(Checkpoint::new(pos.x, pos.y, size.x, size.y));
                push_inline_meta(&fields, LevelObjectKind::Checkpoint, index, &mut metadata);
            }
            "enemy" if keyed => {
                let fields = parse_key_values(rest);
                let kind = fields.get("kind").map(String::as_str).unwrap_or("filth");
                let Some(pos) = keyed_vec2(&fields, "spawn_x", "spawn_y") else {
                    continue;
                };

                if kind.eq_ignore_ascii_case("filth") {
                    let spawn_id = field_u16(&fields, "spawn_id").unwrap_or(0);
                    let spawn_wave = field_u16(&fields, "spawn_wave").unwrap_or(0);
                    let index = enemies.len();

                    enemies.push(if spawn_wave > 0 {
                        Enemy::filth_spawn(pos.x, pos.y, spawn_id.max(1), spawn_wave)
                    } else {
                        Enemy::filth(pos.x, pos.y)
                    });
                    push_inline_meta(&fields, LevelObjectKind::Enemy, index, &mut metadata);
                }
            }
            "trigger" if keyed => {
                let fields = parse_key_values(rest);
                parse_keyed_trigger(&fields, &mut triggers, &mut metadata);
            }
            "text" if keyed => {
                let fields = parse_key_values(rest);
                let Some(pos) = keyed_pos(&fields) else {
                    continue;
                };
                let text = fields
                    .get("value")
                    .cloned()
                    .or_else(|| fields.get("text").cloned())
                    .unwrap_or_default();

                if !text.is_empty() {
                    let index = texts.len();
                    texts.push(LevelText::new(pos, text));
                    push_inline_meta(&fields, LevelObjectKind::Text, index, &mut metadata);
                }
            }
            "world_portal" if keyed => {
                let fields = parse_key_values(rest);
                let Some(pos) = keyed_pos(&fields) else {
                    continue;
                };
                let Some(normal) = keyed_vec2(&fields, "normal_x", "normal_y") else {
                    continue;
                };
                let tangent = keyed_vec2(&fields, "tangent_x", "tangent_y")
                    .unwrap_or(Vec2::new(-normal.y, normal.x));
                let width = field_f32(&fields, "width")
                    .filter(|value| *value > 0.0)
                    .unwrap_or(crate::constants::PORTAL_WIDTH);
                let id = field_u16(&fields, "portal_id")
                    .or_else(|| field_u16(&fields, "id"))
                    .unwrap_or(0);
                let receiver_id = field_u16(&fields, "receiver_id").unwrap_or(id);
                let priority = field_i16(&fields, "priority").unwrap_or(0);
                let mut portal = Portal::with_tangent(
                    pos.x,
                    pos.y,
                    normal,
                    tangent,
                    width,
                    Color::rgb(154, 120, 255),
                );
                let index = world_portals.len();

                portal.scale = field_f32(&fields, "scale")
                    .filter(|value| *value > 0.0)
                    .unwrap_or(1.0);
                portal.scale_objects = field_bool(&fields, "scale_objects").unwrap_or(false);
                world_portals.push(WorldPortal {
                    portal,
                    id,
                    receiver_id,
                    priority,
                    seamless: field_bool(&fields, "seamless").unwrap_or(false),
                    seamless_depth: field_f32(&fields, "seamless_depth")
                        .filter(|value| *value > 0.0)
                        .unwrap_or(256.0),
                    seamless_angle: field_f32(&fields, "seamless_angle")
                        .filter(|value| *value > 0.0)
                        .unwrap_or(180.0),
                    seamless_rely_on_walls: field_bool(&fields, "seamless_rely_on_walls")
                        .unwrap_or(false),
                });
                push_inline_meta(&fields, LevelObjectKind::WorldPortal, index, &mut metadata);
            }
            "meta" if keyed => {
                let fields = parse_key_values(rest);
                if let (Some(kind), Some(index), Some(meta)) = (
                    fields
                        .get("kind")
                        .and_then(|value| LevelObjectKind::parse(value)),
                    field_usize(&fields, "index"),
                    field_meta(&fields),
                ) {
                    metadata.push(LevelObjectMeta { kind, index, meta });
                }
            }
            _ => {}
        }
    }

    Ok(LevelSpec {
        name,
        spawn,
        solids,
        doors,
        hazards,
        checkpoints,
        enemies,
        triggers,
        texts,
        world_portals,
        metadata: prune_metadata(metadata),
        path: Some(path.to_path_buf()),
    })
}

fn format_trigger(kind: &str, solid: Solid, enemy_id: Option<u16>, meta: ObjectMeta) -> String {
    let enemy_id = enemy_id
        .map(|enemy_id| format!(" enemy_id={enemy_id}"))
        .unwrap_or_default();

    format!(
        "trigger kind={} x={} y={} w={} h={}{} obj_id={} layer={}\n",
        kind,
        solid.pos().x,
        solid.pos().y,
        solid.size().x,
        solid.size().y,
        enemy_id,
        meta.id,
        meta.layer
    )
}

fn parse_keyed_trigger(
    fields: &HashMap<String, String>,
    triggers: &mut Vec<LevelTrigger>,
    metadata: &mut Vec<LevelObjectMeta>,
) {
    let kind = fields.get("kind").map(String::as_str).unwrap_or_default();
    let Some((pos, size)) = keyed_rect(fields) else {
        return;
    };
    let index = triggers.len();

    match kind {
        "level_start" => triggers.push(LevelTrigger::level_start(pos.x, pos.y, size.x, size.y)),
        "level_end" => triggers.push(LevelTrigger::level_end(pos.x, pos.y, size.x, size.y)),
        "enemy_spawn" => {
            let enemy_id = field_u16(fields, "enemy_id").unwrap_or(1).max(1);
            triggers.push(LevelTrigger::enemy_spawn(
                pos.x, pos.y, size.x, size.y, enemy_id,
            ));
        }
        _ => return,
    }

    push_inline_meta(fields, LevelObjectKind::Trigger, index, metadata);
}

fn split_command(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(index) = line.find(char::is_whitespace) {
        Some((&line[..index], line[index..].trim()))
    } else {
        Some((line, ""))
    }
}

fn looks_keyed(rest: &str) -> bool {
    rest.split_whitespace()
        .next()
        .is_some_and(|token| token.contains('='))
}

fn parse_key_values(rest: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut chars = rest.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        let mut key_end = start + ch.len_utf8();
        while let Some(&(index, ch)) = chars.peek() {
            if ch == '=' || ch.is_whitespace() {
                break;
            }
            key_end = index + ch.len_utf8();
            chars.next();
        }

        while let Some(&(_, ch)) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
        if !matches!(chars.peek(), Some((_, '='))) {
            while let Some(&(_, ch)) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                chars.next();
            }
            continue;
        }
        chars.next();

        while let Some(&(_, ch)) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let key = &rest[start..key_end];
        let value = if matches!(chars.peek(), Some((_, '"'))) {
            chars.next();
            let mut value = String::new();
            while let Some((_, ch)) = chars.next() {
                match ch {
                    '"' => break,
                    '\\' => {
                        let Some((_, escaped)) = chars.next() else {
                            break;
                        };
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            other => other,
                        });
                    }
                    other => value.push(other),
                }
            }
            value
        } else {
            let Some(&(value_start, first)) = chars.peek() else {
                fields.insert(key.to_string(), String::new());
                continue;
            };
            let mut value_end = value_start + first.len_utf8();
            chars.next();
            while let Some(&(index, ch)) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                value_end = index + ch.len_utf8();
                chars.next();
            }
            rest[value_start..value_end].to_string()
        };

        fields.insert(key.to_string(), value);
    }

    fields
}

fn quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

fn object_meta(metadata: &[LevelObjectMeta], kind: LevelObjectKind, index: usize) -> ObjectMeta {
    metadata
        .iter()
        .find(|entry| entry.kind == kind && entry.index == index)
        .map(|entry| entry.meta)
        .unwrap_or_default()
}

fn push_inline_meta(
    fields: &HashMap<String, String>,
    kind: LevelObjectKind,
    index: usize,
    metadata: &mut Vec<LevelObjectMeta>,
) {
    if let Some(meta) = field_meta(fields)
        && meta != ObjectMeta::default()
    {
        metadata.push(LevelObjectMeta { kind, index, meta });
    }
}

fn field_meta(fields: &HashMap<String, String>) -> Option<ObjectMeta> {
    let id = field_u16(fields, "obj_id")
        .or_else(|| field_u16(fields, "object_id"))
        .unwrap_or(0);
    let layer = field_i16(fields, "layer").unwrap_or(0);

    (id != 0 || layer != 0).then_some(ObjectMeta { id, layer })
}

fn keyed_rect(fields: &HashMap<String, String>) -> Option<(Vec2, Vec2)> {
    let pos = keyed_pos(fields)?;
    let size = Vec2::new(field_f32(fields, "w")?, field_f32(fields, "h")?);

    valid_size(size.x, size.y).then_some((pos, size))
}

fn keyed_pos(fields: &HashMap<String, String>) -> Option<Vec2> {
    keyed_vec2(fields, "x", "y")
}

fn keyed_vec2(fields: &HashMap<String, String>, x: &str, y: &str) -> Option<Vec2> {
    Some(Vec2::new(field_f32(fields, x)?, field_f32(fields, y)?))
}

fn field_f32(fields: &HashMap<String, String>, key: &str) -> Option<f32> {
    fields
        .get(key)?
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn field_u16(fields: &HashMap<String, String>, key: &str) -> Option<u16> {
    fields.get(key)?.parse::<u16>().ok()
}

fn field_i16(fields: &HashMap<String, String>, key: &str) -> Option<i16> {
    fields.get(key)?.parse::<i16>().ok()
}

fn field_usize(fields: &HashMap<String, String>, key: &str) -> Option<usize> {
    fields.get(key)?.parse::<usize>().ok()
}

fn field_bool(fields: &HashMap<String, String>, key: &str) -> Option<bool> {
    fields.get(key).map(|value| parse_bool(value))
}

fn prune_metadata(metadata: Vec<LevelObjectMeta>) -> Vec<LevelObjectMeta> {
    let mut pruned = Vec::new();

    for entry in metadata {
        if entry.meta == ObjectMeta::default() {
            continue;
        }
        if let Some(index) = pruned.iter().position(|existing: &LevelObjectMeta| {
            existing.kind == entry.kind && existing.index == entry.index
        }) {
            pruned[index].meta = entry.meta;
        } else {
            pruned.push(entry);
        }
    }

    pruned
}

fn parse_bool(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "on")
}

fn slug(name: &str) -> String {
    let slug = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    let slug = slug.trim_matches('_');
    // Non-ASCII-only names still need a stable filename on every platform.
    if slug.is_empty() {
        "untitled_level".to_string()
    } else {
        slug.to_string()
    }
}

fn valid_size(width: f32, height: f32) -> bool {
    width > 0.0 && height > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_level_round_trips_metadata_and_escaped_text() {
        let path = unique_test_path("keyed_round_trip");
        let mut spec = LevelSpec {
            name: "Meta Test Chamber".to_string(),
            spawn: Vec2::new(10.0, 20.0),
            solids: vec![Solid::new(1.0, 2.0, 30.0, 40.0, true)],
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: Vec::new(),
            triggers: Vec::new(),
            texts: vec![LevelText::new(
                Vec2::new(50.0, 60.0),
                "a=b\nquoted \"text\"",
            )],
            world_portals: Vec::new(),
            metadata: vec![
                LevelObjectMeta {
                    kind: LevelObjectKind::Solid,
                    index: 0,
                    meta: ObjectMeta { id: 7, layer: -2 },
                },
                LevelObjectMeta {
                    kind: LevelObjectKind::Text,
                    index: 0,
                    meta: ObjectMeta { id: 12, layer: 3 },
                },
            ],
            path: Some(path.clone()),
        };

        save_level(&mut spec).expect("save keyed level");
        let parsed = parse_level_file(&path).expect("parse keyed level");
        let _ = fs::remove_file(path);

        assert_eq!(parsed.name, "Meta Test Chamber");
        assert_eq!(parsed.spawn, Vec2::new(10.0, 20.0));
        assert_eq!(parsed.texts[0].text, "a=b\nquoted \"text\"");
        assert!(parsed.metadata.contains(&LevelObjectMeta {
            kind: LevelObjectKind::Solid,
            index: 0,
            meta: ObjectMeta { id: 7, layer: -2 },
        }));
        assert!(parsed.metadata.contains(&LevelObjectMeta {
            kind: LevelObjectKind::Text,
            index: 0,
            meta: ObjectMeta { id: 12, layer: 3 },
        }));
    }

    #[test]
    fn enemy_save_uses_spawn_position_not_runtime_position() {
        let path = unique_test_path("enemy_spawn_position");
        let mut enemy = Enemy::filth_spawn(120.0, 80.0, 2, 1);

        enemy.pos = Vec2::new(480.0, 80.0);
        enemy.prev_pos = enemy.pos;

        let mut spec = LevelSpec {
            name: "Enemy Spawn Position".to_string(),
            spawn: Vec2::new(0.0, 0.0),
            solids: Vec::new(),
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: vec![enemy],
            triggers: Vec::new(),
            texts: Vec::new(),
            world_portals: Vec::new(),
            metadata: Vec::new(),
            path: Some(path.clone()),
        };

        save_level(&mut spec).expect("save level");
        let source = fs::read_to_string(&path).expect("read saved level");
        let parsed = parse_level_file(&path).expect("parse saved level");
        let _ = fs::remove_file(path);

        assert!(source.contains("spawn_x=120"));
        assert!(!source.contains("spawn_x=480"));
        assert_eq!(parsed.enemies[0].spawn_pos, Vec2::new(120.0, 80.0));
    }

    #[test]
    fn custom_template_has_start_end_and_floor() {
        let spec = LevelSpec::custom_template("CUSTOM LEVEL 1".to_string());

        assert_eq!(spec.name, "CUSTOM LEVEL 1");
        assert!(!spec.solids.is_empty());
        assert!(
            spec.triggers
                .iter()
                .any(|trigger| trigger.kind == LevelTriggerKind::LevelStart)
        );
        assert!(
            spec.triggers
                .iter()
                .any(|trigger| trigger.kind == LevelTriggerKind::LevelEnd)
        );
    }

    #[test]
    fn legacy_level_without_v3_header_is_rejected() {
        let path = unique_test_path("legacy_equals");
        fs::write(
            &path,
            "name Legacy\nplayer 10 20\ntext 1 2 switch=a value\nsolid 0 0 16 16 true\n",
        )
        .expect("write legacy level");

        let parsed = parse_level_file(&path);
        let _ = fs::remove_file(path);

        assert!(parsed.is_err());
    }

    fn unique_test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "portals_{name}_{}_{}.lvl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ))
    }
}

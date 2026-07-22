use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// Small text level format: name, spawn, then solid rows.
use glam::Vec2;

use crate::game::enemy::{Enemy, EnemyKind};
use crate::game::level::{Checkpoint, Door, Hazard, Level, LevelText, Solid, WorldPortal};
use crate::game::portal::{Color, Portal};

const LEVEL_DIR: &str = "levels";

#[derive(Clone)]
pub struct LevelSpec {
    pub name: String,
    pub spawn: Vec2,
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
    pub hazards: Vec<Hazard>,
    pub checkpoints: Vec<Checkpoint>,
    pub enemies: Vec<Enemy>,
    pub texts: Vec<LevelText>,
    pub world_portals: Vec<WorldPortal>,
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
            texts: Vec::new(),
            world_portals: Vec::new(),
            path: None,
        }
    }

    pub fn replace_world(&mut self, world: &Level) {
        self.solids = world.solids.clone();
        self.doors = world.doors.clone();
        self.hazards = world.hazards.clone();
        self.checkpoints = world.checkpoints.clone();
        self.enemies = world.enemies.clone();
        self.texts = world.texts.clone();
        self.world_portals = world.world_portals.clone();
    }

    pub fn level(&self) -> Level {
        Level {
            solids: self.solids.clone(),
            doors: self.doors.clone(),
            hazards: self.hazards.clone(),
            checkpoints: self.checkpoints.clone(),
            enemies: self.enemies.clone(),
            texts: self.texts.clone(),
            world_portals: self.world_portals.clone(),
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
    // Keep the file format intentionally plain so levels remain editable outside the game.
    let mut body = String::new();

    body.push_str(&format!("name {}\n", level.name));
    body.push_str(&format!("player {} {}\n", level.spawn.x, level.spawn.y));
    for solid in &level.solids {
        body.push_str(&format!(
            "solid {} {} {} {} {} {}\n",
            solid.pos.x,
            solid.pos.y,
            solid.size.x,
            solid.size.y,
            solid.portalable,
            solid.rotation()
        ));
    }
    for door in &level.doors {
        body.push_str(&format!(
            "door {} {} {} {} {} {} {} {}\n",
            door.solid.pos.x,
            door.solid.pos.y,
            door.solid.size.x,
            door.solid.size.y,
            door.trigger_radius,
            door.solid.rotation(),
            door.speed,
            door.automatic
        ));
    }
    for hazard in &level.hazards {
        body.push_str(&format!(
            "hazard {} {} {} {} {}\n",
            hazard.solid.pos.x,
            hazard.solid.pos.y,
            hazard.solid.size.x,
            hazard.solid.size.y,
            hazard.solid.rotation()
        ));
    }
    for checkpoint in &level.checkpoints {
        body.push_str(&format!(
            "checkpoint {} {} {} {}\n",
            checkpoint.solid.pos.x,
            checkpoint.solid.pos.y,
            checkpoint.solid.size.x,
            checkpoint.solid.size.y
        ));
    }
    for enemy in &level.enemies {
        let kind = match enemy.kind {
            EnemyKind::Filth => "filth",
        };

        body.push_str(&format!("enemy {} {} {}\n", kind, enemy.pos.x, enemy.pos.y));
    }
    for text in &level.texts {
        body.push_str(&format!(
            "text {} {} {}\n",
            text.pos.x,
            text.pos.y,
            text.text.replace('\n', " ")
        ));
    }
    for portal in &level.world_portals {
        body.push_str(&format!(
            "world_portal {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
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
            portal.seamless_rely_on_walls
        ));
    }

    fs::write(&path, body)?;
    level.path = Some(path);

    Ok(())
}

fn parse_level_file(path: &Path) -> io::Result<LevelSpec> {
    let source = fs::read_to_string(path)?;
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
    let mut texts = Vec::new();
    let mut world_portals = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("name") => {
                name = parts.collect::<Vec<_>>().join(" ");
            }
            Some("player") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                spawn = Vec2::new(x, y);
            }
            Some("solid") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(w) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(h) = parse_next(&mut parts) else {
                    continue;
                };
                // Skip a bad row instead of letting negative or zero extents poison collision math.
                if !valid_size(w, h) {
                    continue;
                }
                let portalable = parts.next().is_none_or(|value| value != "false");
                let rotation = parse_next(&mut parts).unwrap_or(0.0);

                solids.push(Solid::rotated(x, y, w, h, rotation, portalable));
            }
            Some("door") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(w) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(h) = parse_next(&mut parts) else {
                    continue;
                };
                if !valid_size(w, h) {
                    continue;
                }
                let trigger_radius = parse_next(&mut parts)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(112.0);
                let rotation = parse_next(&mut parts).unwrap_or(0.0);
                let speed = parse_next(&mut parts)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(3.6);
                let automatic = parts.next().is_none_or(|value| value != "false");
                let mut door = Door::with_radius(x, y, w, h, trigger_radius);

                door.solid.set_rotation(rotation);
                door.speed = speed.max(0.1);
                door.automatic = automatic;
                doors.push(door);
            }
            Some("hazard") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(w) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(h) = parse_next(&mut parts) else {
                    continue;
                };
                if !valid_size(w, h) {
                    continue;
                }
                let rotation = parse_next(&mut parts).unwrap_or(0.0);
                let mut hazard = Hazard::new(x, y, w, h);

                hazard.solid.set_rotation(rotation);
                hazards.push(hazard);
            }
            Some("checkpoint") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(w) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(h) = parse_next(&mut parts) else {
                    continue;
                };
                if !valid_size(w, h) {
                    continue;
                }

                checkpoints.push(Checkpoint::new(x, y, w, h));
            }
            Some("enemy") => {
                let Some(kind) = parts.next() else {
                    continue;
                };
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };

                if kind.eq_ignore_ascii_case("filth") {
                    enemies.push(Enemy::filth(x, y));
                }
            }
            Some("text") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let text = parts.collect::<Vec<_>>().join(" ");

                if !text.is_empty() {
                    texts.push(LevelText::new(Vec2::new(x, y), text));
                }
            }
            Some("world_portal") => {
                let Some(x) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(y) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(nx) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(ny) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(tx) = parse_next(&mut parts) else {
                    continue;
                };
                let Some(ty) = parse_next(&mut parts) else {
                    continue;
                };
                let width = parse_next(&mut parts)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(crate::constants::PORTAL_WIDTH);
                let id = parse_next_u16(&mut parts).unwrap_or(0);
                let receiver_id = parse_next_u16(&mut parts).unwrap_or(id);
                let priority = parse_next_i16(&mut parts).unwrap_or(0);
                let extras = parts.collect::<Vec<_>>();
                let (
                    scale,
                    scale_objects,
                    seamless,
                    seamless_depth,
                    seamless_angle,
                    seamless_rely_on_walls,
                ) = parse_world_portal_extras(&extras);
                let mut portal = Portal::with_tangent(
                    x,
                    y,
                    Vec2::new(nx, ny),
                    Vec2::new(tx, ty),
                    width,
                    Color::rgb(154, 120, 255),
                );

                portal.scale = scale;
                portal.scale_objects = scale_objects;
                world_portals.push(WorldPortal {
                    portal,
                    id,
                    receiver_id,
                    priority,
                    seamless,
                    seamless_depth,
                    seamless_angle,
                    seamless_rely_on_walls,
                });
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
        texts,
        world_portals,
        path: Some(path.to_path_buf()),
    })
}

fn parse_next<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<f32> {
    // Reject NaN and infinity at the file boundary rather than guarding every consumer.
    parts
        .next()?
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_next_u16<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<u16> {
    parts.next()?.parse().ok()
}

fn parse_next_i16<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<i16> {
    parts.next()?.parse().ok()
}

fn parse_world_portal_extras(parts: &[&str]) -> (f32, bool, bool, f32, f32, bool) {
    match parts {
        [seamless] => (1.0, false, parse_bool(seamless), 256.0, 180.0, false),
        [scale, seamless] => (
            scale
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(1.0),
            false,
            parse_bool(seamless),
            256.0,
            180.0,
            false,
        ),
        [scale, scale_objects, seamless] => (
            scale
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(1.0),
            parse_bool(scale_objects),
            parse_bool(seamless),
            256.0,
            180.0,
            false,
        ),
        [scale, scale_objects, seamless, seamless_depth] => (
            scale
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(1.0),
            parse_bool(scale_objects),
            parse_bool(seamless),
            seamless_depth
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(256.0),
            180.0,
            false,
        ),
        [
            scale,
            scale_objects,
            seamless,
            seamless_depth,
            seamless_angle,
            seamless_rely_on_walls,
            ..,
        ] => (
            scale
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(1.0),
            parse_bool(scale_objects),
            parse_bool(seamless),
            seamless_depth
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(256.0),
            seamless_angle
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(180.0),
            parse_bool(seamless_rely_on_walls),
        ),
        _ => (1.0, false, false, 256.0, 180.0, false),
    }
}

fn parse_bool(value: &str) -> bool {
    value == "true"
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

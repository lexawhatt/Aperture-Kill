use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// Small text level format: name, spawn, then solid rows.
use glam::Vec2;

use crate::game::level::{Checkpoint, Door, Hazard, Level, LevelText, Solid};

const LEVEL_DIR: &str = "levels";

#[derive(Clone)]
pub struct LevelSpec {
    pub name: String,
    pub spawn: Vec2,
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
    pub hazards: Vec<Hazard>,
    pub checkpoints: Vec<Checkpoint>,
    pub texts: Vec<LevelText>,
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
            texts: Vec::new(),
            path: None,
        }
    }

    pub fn from_world(
        name: String,
        spawn: Vec2,
        solids: Vec<Solid>,
        doors: Vec<Door>,
        hazards: Vec<Hazard>,
        checkpoints: Vec<Checkpoint>,
        texts: Vec<LevelText>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            name,
            spawn,
            solids,
            doors,
            hazards,
            checkpoints,
            texts,
            path,
        }
    }

    pub fn level(&self) -> Level {
        Level {
            solids: self.solids.clone(),
            doors: self.doors.clone(),
            hazards: self.hazards.clone(),
            checkpoints: self.checkpoints.clone(),
            texts: self.texts.clone(),
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

            parse_level_file(&path).ok()
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
    let mut body = String::new();

    body.push_str(&format!("name {}\n", level.name));
    body.push_str(&format!("player {} {}\n", level.spawn.x, level.spawn.y));
    for solid in &level.solids {
        body.push_str(&format!(
            "solid {} {} {} {} {} {}\n",
            solid.pos.x, solid.pos.y, solid.size.x, solid.size.y, solid.portalable, solid.rotation
        ));
    }
    for door in &level.doors {
        body.push_str(&format!(
            "door {} {} {} {} {} {}\n",
            door.solid.pos.x,
            door.solid.pos.y,
            door.solid.size.x,
            door.solid.size.y,
            door.trigger_radius,
            door.solid.rotation
        ));
    }
    for hazard in &level.hazards {
        body.push_str(&format!(
            "hazard {} {} {} {} {}\n",
            hazard.solid.pos.x,
            hazard.solid.pos.y,
            hazard.solid.size.x,
            hazard.solid.size.y,
            hazard.solid.rotation
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
    for text in &level.texts {
        body.push_str(&format!(
            "text {} {} {}\n",
            text.pos.x,
            text.pos.y,
            text.text.replace('\n', " ")
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
    let mut texts = Vec::new();

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
                let trigger_radius = parse_next(&mut parts).unwrap_or(112.0);
                let rotation = parse_next(&mut parts).unwrap_or(0.0);
                let mut door = Door::with_radius(x, y, w, h, trigger_radius);

                door.solid.rotation = rotation;
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
                let rotation = parse_next(&mut parts).unwrap_or(0.0);
                let mut hazard = Hazard::new(x, y, w, h);

                hazard.solid.rotation = rotation;
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

                checkpoints.push(Checkpoint::new(x, y, w, h));
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
        texts,
        path: Some(path.to_path_buf()),
    })
}

fn parse_next<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<f32> {
    parts.next()?.parse().ok()
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

    slug.trim_matches('_').to_string()
}

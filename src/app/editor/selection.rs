use glam::Vec2;

use crate::app::editor_geometry::{EditorSelection, text_bounds};
use crate::game::level::{Level, LevelObjectKind, ObjectMeta, Solid, WorldPortal};

#[derive(Default)]
pub(super) struct SelectionBuckets {
    solids: Vec<usize>,
    doors: Vec<usize>,
    hazards: Vec<usize>,
    checkpoints: Vec<usize>,
    enemies: Vec<usize>,
    triggers: Vec<usize>,
    texts: Vec<usize>,
    world_portals: Vec<usize>,
}

impl SelectionBuckets {
    pub(super) fn from_selected(selected: impl IntoIterator<Item = EditorSelection>) -> Self {
        let mut buckets = Self::default();

        for selection in selected {
            buckets.push(selection);
        }

        buckets
    }

    fn push(&mut self, selection: EditorSelection) {
        match selection {
            EditorSelection::Solid(index) => self.solids.push(index),
            EditorSelection::Door(index) => self.doors.push(index),
            EditorSelection::Hazard(index) => self.hazards.push(index),
            EditorSelection::Checkpoint(index) => self.checkpoints.push(index),
            EditorSelection::Enemy(index) => self.enemies.push(index),
            EditorSelection::Trigger(index) => self.triggers.push(index),
            EditorSelection::Text(index) => self.texts.push(index),
            EditorSelection::WorldPortal(index) => self.world_portals.push(index),
        }
    }

    pub(super) fn remove_metadata_from(&self, level: &mut Level) {
        level.remove_object_metadata(LevelObjectKind::Solid, &self.solids);
        level.remove_object_metadata(LevelObjectKind::Door, &self.doors);
        level.remove_object_metadata(LevelObjectKind::Hazard, &self.hazards);
        level.remove_object_metadata(LevelObjectKind::Checkpoint, &self.checkpoints);
        level.remove_object_metadata(LevelObjectKind::Enemy, &self.enemies);
        level.remove_object_metadata(LevelObjectKind::Trigger, &self.triggers);
        level.remove_object_metadata(LevelObjectKind::Text, &self.texts);
        level.remove_object_metadata(LevelObjectKind::WorldPortal, &self.world_portals);
    }

    pub(super) fn remove_from(self, level: &mut Level) {
        remove_indices(&mut level.solids, self.solids);
        remove_indices(&mut level.doors, self.doors);
        remove_indices(&mut level.hazards, self.hazards);
        remove_indices(&mut level.checkpoints, self.checkpoints);
        remove_indices(&mut level.enemies, self.enemies);
        remove_indices(&mut level.triggers, self.triggers);
        remove_indices(&mut level.texts, self.texts);
        remove_indices(&mut level.world_portals, self.world_portals);
    }
}

pub(super) fn selection_sort_key(selection: &EditorSelection) -> (usize, usize) {
    match selection {
        EditorSelection::Solid(index) => (0, *index),
        EditorSelection::Door(index) => (1, *index),
        EditorSelection::Hazard(index) => (2, *index),
        EditorSelection::Checkpoint(index) => (3, *index),
        EditorSelection::Enemy(index) => (4, *index),
        EditorSelection::Trigger(index) => (5, *index),
        EditorSelection::Text(index) => (6, *index),
        EditorSelection::WorldPortal(index) => (7, *index),
    }
}

pub(super) fn selection_object_key(selection: EditorSelection) -> (LevelObjectKind, usize) {
    match selection {
        EditorSelection::Solid(index) => (LevelObjectKind::Solid, index),
        EditorSelection::Door(index) => (LevelObjectKind::Door, index),
        EditorSelection::Hazard(index) => (LevelObjectKind::Hazard, index),
        EditorSelection::Checkpoint(index) => (LevelObjectKind::Checkpoint, index),
        EditorSelection::Enemy(index) => (LevelObjectKind::Enemy, index),
        EditorSelection::Trigger(index) => (LevelObjectKind::Trigger, index),
        EditorSelection::Text(index) => (LevelObjectKind::Text, index),
        EditorSelection::WorldPortal(index) => (LevelObjectKind::WorldPortal, index),
    }
}

pub(super) fn apply_clipboard_meta(
    level: &mut Level,
    selection: EditorSelection,
    meta: ObjectMeta,
) {
    if meta == ObjectMeta::default() {
        return;
    }
    let (kind, index) = selection_object_key(selection);
    level.set_object_meta(kind, index, meta);
}

pub(super) fn selection_bounds(level: &Level, selected: &[EditorSelection]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);

    for selection in selected {
        match *selection {
            EditorSelection::Solid(index) => {
                if let Some(solid) = level.solids.get(index) {
                    include_solid_bounds(*solid, &mut min, &mut max);
                }
            }
            EditorSelection::Door(index) => {
                if let Some(door) = level.doors.get(index) {
                    include_solid_bounds(door.solid, &mut min, &mut max);
                }
            }
            EditorSelection::Hazard(index) => {
                if let Some(hazard) = level.hazards.get(index) {
                    include_solid_bounds(hazard.solid, &mut min, &mut max);
                }
            }
            EditorSelection::Checkpoint(index) => {
                if let Some(checkpoint) = level.checkpoints.get(index) {
                    include_solid_bounds(checkpoint.solid, &mut min, &mut max);
                }
            }
            EditorSelection::Enemy(index) => {
                if let Some(enemy) = level.enemies.get(index) {
                    include_solid_bounds(enemy.spawn_solid(), &mut min, &mut max);
                }
            }
            EditorSelection::Trigger(index) => {
                if let Some(trigger) = level.triggers.get(index) {
                    include_solid_bounds(trigger.solid, &mut min, &mut max);
                }
            }
            EditorSelection::Text(index) => {
                if let Some(text) = level.texts.get(index) {
                    let (pos, size) = text_bounds(text);
                    min = min.min(pos);
                    max = max.max(pos + size);
                }
            }
            EditorSelection::WorldPortal(index) => {
                if let Some(portal) = level.world_portals.get(index) {
                    let (portal_min, portal_max) = world_portal_bounds(*portal);
                    min = min.min(portal_min);
                    max = max.max(portal_max);
                }
            }
        }
    }

    (min, max)
}

pub(in crate::app::editor) fn include_solid_bounds(solid: Solid, min: &mut Vec2, max: &mut Vec2) {
    for corner in solid.corners() {
        *min = min.min(corner);
        *max = max.max(corner);
    }
}

pub(in crate::app::editor) fn world_portal_bounds(portal: WorldPortal) -> (Vec2, Vec2) {
    let corners = portal.edit_solid().corners();
    let min = corners
        .iter()
        .copied()
        .reduce(Vec2::min)
        .unwrap_or(portal.center());
    let max = corners
        .iter()
        .copied()
        .reduce(Vec2::max)
        .unwrap_or(portal.center());

    (min, max)
}

pub(super) fn offset_u16(value: u16, delta: i16) -> u16 {
    if delta.is_negative() {
        value.saturating_sub(delta.unsigned_abs())
    } else {
        value.saturating_add(delta as u16)
    }
}

pub(super) fn offset_u16_min(value: u16, delta: i16, min: u16) -> u16 {
    offset_u16(value, delta).max(min)
}

fn remove_indices<T>(items: &mut Vec<T>, mut indices: Vec<usize>) {
    indices.sort_unstable_by(|a, b| b.cmp(a));
    indices.dedup();
    for index in indices {
        items.remove(index);
    }
}

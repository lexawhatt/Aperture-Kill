use glam::Vec2;

use crate::app::editor::selection::{include_solid_bounds, world_portal_bounds};
use crate::app::editor_geometry::text_bounds;
use crate::game::enemy::Enemy;
use crate::game::level::{
    Checkpoint, Door, Hazard, LevelText, LevelTrigger, ObjectMeta, Solid, WorldPortal,
};

#[derive(Clone)]
pub(super) struct EditorClipboardItem {
    pub(super) object: EditorClipboard,
    pub(super) meta: ObjectMeta,
}

#[derive(Clone)]
pub(super) enum EditorClipboard {
    Solid(Solid),
    Door(Door),
    Hazard(Hazard),
    Checkpoint(Checkpoint),
    Enemy(Enemy),
    Trigger(LevelTrigger),
    Text(LevelText),
    WorldPortal(WorldPortal),
}

pub(super) fn clipboard_bounds(clipboard: &[EditorClipboardItem]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);

    for item in clipboard {
        match &item.object {
            EditorClipboard::Solid(solid) => include_solid_bounds(*solid, &mut min, &mut max),
            EditorClipboard::Door(door) => include_solid_bounds(door.solid, &mut min, &mut max),
            EditorClipboard::Hazard(hazard) => {
                include_solid_bounds(hazard.solid, &mut min, &mut max);
            }
            EditorClipboard::Checkpoint(checkpoint) => {
                include_solid_bounds(checkpoint.solid, &mut min, &mut max);
            }
            EditorClipboard::Enemy(enemy) => {
                include_solid_bounds(enemy.spawn_solid(), &mut min, &mut max);
            }
            EditorClipboard::Trigger(trigger) => {
                include_solid_bounds(trigger.solid, &mut min, &mut max);
            }
            EditorClipboard::Text(text) => {
                let (pos, size) = text_bounds(text);
                min = min.min(pos);
                max = max.max(pos + size);
            }
            EditorClipboard::WorldPortal(portal) => {
                let (portal_min, portal_max) = world_portal_bounds(*portal);
                min = min.min(portal_min);
                max = max.max(portal_max);
            }
        }
    }

    (min, max)
}

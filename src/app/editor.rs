use std::collections::VecDeque;

use glam::Vec2;
use winit::keyboard::KeyCode;

mod clipboard;
mod selection;
mod types;

// Editor stores mode state and applies commands to level objects.
use crate::constants::PORTAL_WIDTH;
use crate::game::enemy::Enemy;
use crate::game::level::{
    Checkpoint, Door, Hazard, Level, LevelText, LevelTrigger, LevelTriggerKind, ObjectMeta, Solid,
    WorldPortal,
};

use self::clipboard::{EditorClipboard, EditorClipboardItem, clipboard_bounds};
use self::selection::{
    SelectionBuckets, apply_clipboard_meta, offset_u16, offset_u16_min, selection_bounds,
    selection_object_key, selection_sort_key, world_portal_bounds,
};
pub(super) use self::types::{EditorCategory, EditorMode, EditorSelectionKind, EditorTool};
use self::types::{EditorPan, LevelSnapshot};
use super::editor_geometry::{
    EditorDrag, EditorMoveStart, EditorSelection, SolidHit, drag_from_hit, rect_intersects_rect,
    resized_local_bounds, selection_rect, snap, snap_angle, snap_delta, solid_at,
    solid_intersects_rect, text_at, text_bounds,
};

pub(super) struct Editor {
    selected: Vec<EditorSelection>,
    clipboard: Vec<EditorClipboardItem>,
    drag: EditorDrag,
    pub(super) tool: EditorTool,
    category: EditorCategory,
    mode: EditorMode,
    pan: EditorPan,
    pub(super) rotate_ui: bool,
    pub(super) dirty: bool,
    pub(super) status_timer: f32,
    grid_snap: bool,
    current_layer: i16,
    text_editing: bool,
    undo: VecDeque<LevelSnapshot>,
}

impl Editor {
    pub(super) fn new() -> Self {
        Self {
            selected: Vec::new(),
            clipboard: Vec::new(),
            drag: EditorDrag::None,
            tool: EditorTool::Portalable,
            category: EditorCategory::Building,
            mode: EditorMode::Build,
            pan: EditorPan::default(),
            rotate_ui: false,
            dirty: false,
            status_timer: 0.0,
            grid_snap: true,
            current_layer: 0,
            text_editing: false,
            undo: VecDeque::new(),
        }
    }

    pub(super) fn update(&mut self, dt: f32) {
        self.status_timer = (self.status_timer - dt).max(0.0);
    }

    pub(super) fn begin_left_drag(
        &mut self,
        pos: Vec2,
        additive: bool,
        force_move_selected: bool,
        level: &mut Level,
    ) {
        if force_move_selected {
            // Ctrl-drag moves the current selection without hit-testing anything under the cursor.
            self.begin_forced_move(pos, level);
            return;
        }

        let hit = self.object_at(pos, level);

        if let Some((selection, hit)) = hit {
            if additive {
                self.toggle_selection(selection);
                self.drag = EditorDrag::None;
                self.text_editing = false;
                return;
            }

            let was_selected = self.is_selected(selection);
            let selected_count = self.selected.len();
            self.save_undo(level);
            self.drag = match hit {
                SolidHit::Body if was_selected && selected_count > 1 => EditorDrag::MoveSelection {
                    start_cursor: pos,
                    starts: self.selected_starts(level),
                },
                _ => {
                    self.set_single_selection(selection);
                    self.drag_from_selection(selection, hit, pos, level)
                }
            };
            self.text_editing = false;
            return;
        }

        if !additive {
            self.clear_selection();
        }
        self.text_editing = false;
        self.drag = EditorDrag::Marquee {
            start: pos,
            current: pos,
            additive,
        };
    }

    fn begin_forced_move(&mut self, pos: Vec2, level: &mut Level) {
        let starts = self.selected_starts(level);
        if starts.is_empty() {
            self.drag = EditorDrag::None;
            self.text_editing = false;
            return;
        }

        self.save_undo(level);
        // Each object keeps its own start position; snapping is applied to the shared cursor delta.
        self.drag = EditorDrag::MoveSelection {
            start_cursor: pos,
            starts,
        };
        self.text_editing = false;
    }

    pub(super) fn create_block(&mut self, pos: Vec2, tool: EditorTool, level: &mut Level) {
        self.tool = tool;
        self.category = tool.category();
        self.save_undo(level);
        self.push_block(maybe_snap(pos, self.grid_snap), level);
        self.tag_new_object(level, EditorSelection::Solid(level.solids.len() - 1));
        self.set_single_selection(EditorSelection::Solid(level.solids.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_door(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::Door;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(48.0, 112.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level.doors.push(Door::new(pos.x, pos.y, size.x, size.y));
        self.tag_new_object(level, EditorSelection::Door(level.doors.len() - 1));
        self.set_single_selection(EditorSelection::Door(level.doors.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_text(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::Text;
        self.category = self.tool.category();
        self.save_undo(level);
        level
            .texts
            .push(LevelText::new(maybe_snap(pos, self.grid_snap), "TEXT"));
        self.tag_new_object(level, EditorSelection::Text(level.texts.len() - 1));
        self.set_single_selection(EditorSelection::Text(level.texts.len() - 1));
        self.text_editing = true;
        self.dirty = true;
    }

    pub(super) fn create_hazard(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::Hazard;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(128.0, 24.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .hazards
            .push(Hazard::new(pos.x, pos.y, size.x, size.y));
        self.tag_new_object(level, EditorSelection::Hazard(level.hazards.len() - 1));
        self.set_single_selection(EditorSelection::Hazard(level.hazards.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_checkpoint(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::Checkpoint;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(48.0, 80.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .checkpoints
            .push(Checkpoint::new(pos.x, pos.y, size.x, size.y));
        self.tag_new_object(
            level,
            EditorSelection::Checkpoint(level.checkpoints.len() - 1),
        );
        self.set_single_selection(EditorSelection::Checkpoint(level.checkpoints.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_filth(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::Filth;
        self.category = self.tool.category();
        self.save_undo(level);
        let pos = maybe_snap(pos, self.grid_snap);

        level.enemies.push(Enemy::filth_spawn(pos.x, pos.y, 1, 1));
        self.tag_new_object(level, EditorSelection::Enemy(level.enemies.len() - 1));
        self.set_single_selection(EditorSelection::Enemy(level.enemies.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_level_start(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::LevelStart;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(48.0, 80.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .triggers
            .push(LevelTrigger::level_start(pos.x, pos.y, size.x, size.y));
        self.tag_new_object(level, EditorSelection::Trigger(level.triggers.len() - 1));
        self.set_single_selection(EditorSelection::Trigger(level.triggers.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_level_end(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::LevelEnd;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(48.0, 80.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .triggers
            .push(LevelTrigger::level_end(pos.x, pos.y, size.x, size.y));
        self.tag_new_object(level, EditorSelection::Trigger(level.triggers.len() - 1));
        self.set_single_selection(EditorSelection::Trigger(level.triggers.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_enemy_spawn_trigger(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::EnemySpawnTrigger;
        self.category = self.tool.category();
        self.save_undo(level);
        let size = Vec2::new(128.0, 96.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .triggers
            .push(LevelTrigger::enemy_spawn(pos.x, pos.y, size.x, size.y, 1));
        self.tag_new_object(level, EditorSelection::Trigger(level.triggers.len() - 1));
        self.set_single_selection(EditorSelection::Trigger(level.triggers.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_world_portal(&mut self, pos: Vec2, level: &mut Level) {
        self.tool = EditorTool::WorldPortal;
        self.category = self.tool.category();
        self.save_undo(level);
        let center = maybe_snap(pos, self.grid_snap);

        level.world_portals.push(WorldPortal::new(
            center.x,
            center.y,
            Vec2::new(0.0, -1.0),
            PORTAL_WIDTH,
            0,
        ));
        self.tag_new_object(
            level,
            EditorSelection::WorldPortal(level.world_portals.len() - 1),
        );
        self.set_single_selection(EditorSelection::WorldPortal(level.world_portals.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_active_tool(&mut self, pos: Vec2, level: &mut Level) {
        if !self.category.contains_tool(self.tool) {
            return;
        }

        match self.tool {
            EditorTool::Solid | EditorTool::Portalable => self.create_block(pos, self.tool, level),
            EditorTool::Hazard => self.create_hazard(pos, level),
            EditorTool::Door => self.create_door(pos, level),
            EditorTool::Checkpoint => self.create_checkpoint(pos, level),
            EditorTool::WorldPortal => self.create_world_portal(pos, level),
            EditorTool::Text => self.create_text(pos, level),
            EditorTool::Filth => self.create_filth(pos, level),
            EditorTool::LevelStart => self.create_level_start(pos, level),
            EditorTool::LevelEnd => self.create_level_end(pos, level),
            EditorTool::EnemySpawnTrigger => self.create_enemy_spawn_trigger(pos, level),
        }
    }

    pub(super) fn set_tool(&mut self, tool: EditorTool) {
        self.tool = tool;
        self.category = tool.category();
        self.mode = EditorMode::Build;
        self.text_editing = false;
    }

    pub(super) fn category(&self) -> EditorCategory {
        self.category
    }

    pub(super) fn set_category(&mut self, category: EditorCategory) {
        self.category = category;
        self.mode = EditorMode::Build;
        if let Some(tool) = category.tools().first().copied() {
            self.tool = tool;
        }
        self.text_editing = false;
    }

    pub(super) fn mode(&self) -> EditorMode {
        self.mode
    }

    pub(super) fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
        self.text_editing = false;
    }

    pub(super) fn has_object_at(&self, pos: Vec2, level: &Level) -> bool {
        self.object_at(pos, level).is_some()
    }

    pub(super) fn drag_to(&mut self, pos: Vec2, level: &mut Level) {
        let primary = self.primary_selected();
        let grid_snap = self.grid_snap;

        match &mut self.drag {
            EditorDrag::None => {}
            EditorDrag::Move { grab } => {
                let Some(selection) = primary else {
                    return;
                };
                let center = pos - *grab;

                Self::set_object_center(level, selection, center, grid_snap);
                self.dirty = true;
            }
            EditorDrag::MoveSelection {
                start_cursor,
                starts,
            } => {
                let delta = maybe_snap_delta(pos - *start_cursor, grid_snap);

                for start in starts.iter() {
                    Self::set_object_pos(level, start.selection, start.pos + delta, grid_snap);
                }
                self.dirty = true;
            }
            EditorDrag::Resize {
                edge_x,
                edge_y,
                start,
                start_cursor,
            } => {
                let Some(selection) = primary else {
                    return;
                };
                if let EditorSelection::WorldPortal(index) = selection {
                    if let Some(portal) = level.world_portals.get_mut(index) {
                        let mut solid = portal.edit_solid();

                        Self::resize_solid(
                            &mut solid,
                            pos,
                            *edge_x,
                            *edge_y,
                            *start,
                            *start_cursor,
                            grid_snap,
                        );
                        portal.apply_edit_solid(solid);
                        self.dirty = true;
                    }
                } else if let Some(solid) = Self::solid_like_mut(level, selection) {
                    Self::resize_solid(
                        solid,
                        pos,
                        *edge_x,
                        *edge_y,
                        *start,
                        *start_cursor,
                        grid_snap,
                    );
                    self.dirty = true;
                }
            }
            EditorDrag::Rotate {
                start_rotation,
                center,
                start_angle,
            } => {
                let Some(selection) = primary else {
                    return;
                };
                if let EditorSelection::WorldPortal(index) = selection {
                    if let Some(portal) = level.world_portals.get_mut(index) {
                        let mut solid = portal.edit_solid();

                        Self::rotate_solid(
                            &mut solid,
                            pos,
                            *start_rotation,
                            *center,
                            *start_angle,
                            grid_snap,
                        );
                        portal.apply_edit_solid(solid);
                        self.dirty = true;
                    }
                } else if let EditorSelection::Solid(index) = selection
                    && let Some(solid) = level.solids.get_mut(index)
                {
                    Self::rotate_solid(
                        solid,
                        pos,
                        *start_rotation,
                        *center,
                        *start_angle,
                        grid_snap,
                    );
                    self.dirty = true;
                }
            }
            EditorDrag::Marquee { current, .. } => {
                *current = pos;
            }
        }
    }

    pub(super) fn end_drag(&mut self, level: &Level) {
        if let EditorDrag::Marquee {
            start,
            current,
            additive,
        } = self.drag.clone()
        {
            let (rect_pos, rect_size) = selection_rect(start, current);
            let hits = self.objects_intersecting(level, rect_pos, rect_size);

            if additive {
                for selection in hits {
                    self.add_selection(selection);
                }
            } else {
                self.selected = hits;
            }
            self.normalize_selection(level);
        }

        self.drag = EditorDrag::None;
    }

    pub(super) fn cancel_drag(&mut self) {
        self.drag = EditorDrag::None;
    }

    pub(super) fn delete_selected(&mut self, level: &mut Level) {
        if self.selected.is_empty() {
            return;
        }

        self.save_undo(level);
        let buckets = SelectionBuckets::from_selected(self.valid_selected(level));
        buckets.remove_metadata_from(level);
        buckets.remove_from(level);
        self.clear_selection();
        self.dirty = true;
    }

    pub(super) fn delete_at(&mut self, pos: Vec2, level: &mut Level) -> bool {
        let Some((selection, _)) = self.object_at(pos, level) else {
            return false;
        };

        self.set_single_selection(selection);
        self.delete_selected(level);
        true
    }

    pub(super) fn copy_selected(&mut self, level: &Level) {
        self.clipboard = self
            .valid_selected(level)
            .into_iter()
            .filter_map(|selection| {
                let object = match selection {
                    EditorSelection::Solid(index) => {
                        level.solids.get(index).copied().map(EditorClipboard::Solid)
                    }
                    EditorSelection::Door(index) => {
                        level.doors.get(index).copied().map(EditorClipboard::Door)
                    }
                    EditorSelection::Hazard(index) => level
                        .hazards
                        .get(index)
                        .copied()
                        .map(EditorClipboard::Hazard),
                    EditorSelection::Checkpoint(index) => level
                        .checkpoints
                        .get(index)
                        .copied()
                        .map(EditorClipboard::Checkpoint),
                    EditorSelection::Enemy(index) => level
                        .enemies
                        .get(index)
                        .cloned()
                        .map(EditorClipboard::Enemy),
                    EditorSelection::Trigger(index) => level
                        .triggers
                        .get(index)
                        .copied()
                        .map(EditorClipboard::Trigger),
                    EditorSelection::Text(index) => {
                        level.texts.get(index).cloned().map(EditorClipboard::Text)
                    }
                    EditorSelection::WorldPortal(index) => level
                        .world_portals
                        .get(index)
                        .copied()
                        .map(EditorClipboard::WorldPortal),
                }?;
                let (kind, index) = selection_object_key(selection);

                Some(EditorClipboardItem {
                    object,
                    meta: level.object_meta(kind, index),
                })
            })
            .collect();
    }

    pub(super) fn cut_selected(&mut self, level: &mut Level) {
        self.copy_selected(level);
        self.delete_selected(level);
    }

    pub(super) fn paste_clipboard(&mut self, pos: Vec2, level: &mut Level) {
        if self.clipboard.is_empty() {
            return;
        }

        self.save_undo(level);
        let (min, max) = clipboard_bounds(&self.clipboard);
        let offset = maybe_snap(pos, self.grid_snap) - (min + max) / 2.0;
        let mut pasted = Vec::new();

        for item in self.clipboard.iter().cloned() {
            match item.object {
                EditorClipboard::Solid(mut solid) => {
                    solid.translate(offset);
                    level.solids.push(solid);
                    let selection = EditorSelection::Solid(level.solids.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Door(mut door) => {
                    door.solid.translate(offset);
                    door.open = 0.0;
                    level.doors.push(door);
                    let selection = EditorSelection::Door(level.doors.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Hazard(mut hazard) => {
                    hazard.solid.translate(offset);
                    level.hazards.push(hazard);
                    let selection = EditorSelection::Hazard(level.hazards.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Checkpoint(mut checkpoint) => {
                    checkpoint.solid.translate(offset);
                    level.checkpoints.push(checkpoint);
                    let selection = EditorSelection::Checkpoint(level.checkpoints.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Enemy(mut enemy) => {
                    enemy.spawn_pos += offset;
                    enemy.pos = enemy.spawn_pos;
                    enemy.prev_pos = enemy.spawn_pos;
                    enemy.vel = Vec2::ZERO;
                    level.enemies.push(enemy);
                    let selection = EditorSelection::Enemy(level.enemies.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Trigger(mut trigger) => {
                    trigger.solid.translate(offset);
                    trigger.fired = false;
                    level.triggers.push(trigger);
                    let selection = EditorSelection::Trigger(level.triggers.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::Text(mut text) => {
                    text.pos += offset;
                    level.texts.push(text);
                    let selection = EditorSelection::Text(level.texts.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
                EditorClipboard::WorldPortal(mut portal) => {
                    portal.portal.pos += offset;
                    level.world_portals.push(portal);
                    let selection = EditorSelection::WorldPortal(level.world_portals.len() - 1);
                    apply_clipboard_meta(level, selection, item.meta);
                    pasted.push(selection);
                }
            }
        }

        self.selected = pasted;
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn duplicate_selected(&mut self, level: &mut Level) {
        self.copy_selected(level);
        self.paste_clipboard(self.selection_center(level) + Vec2::splat(16.0), level);
    }

    pub(super) fn undo(&mut self, level: &mut Level) {
        // Snapshots contain only level data; camera, tool, and transient drag state stay intact.
        let Some(previous) = self.undo.pop_back() else {
            return;
        };

        level.solids = previous.solids;
        level.doors = previous.doors;
        level.hazards = previous.hazards;
        level.checkpoints = previous.checkpoints;
        level.enemies = previous.enemies;
        level.triggers = previous.triggers;
        level.texts = previous.texts;
        level.world_portals = previous.world_portals;
        level.metadata = previous.metadata;
        self.normalize_selection(level);
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn toggle_portalable(&mut self, level: &mut Level) {
        let solid_indices = self
            .valid_selected(level)
            .into_iter()
            .filter_map(EditorSelection::solid_index)
            .collect::<Vec<_>>();
        if solid_indices.is_empty() {
            return;
        }

        self.save_undo(level);
        let make_portalable = !solid_indices.iter().all(|index| {
            level
                .solids
                .get(*index)
                .is_some_and(|solid| solid.portalable)
        });
        for index in solid_indices {
            if let Some(solid) = level.solids.get_mut(index) {
                solid.portalable = make_portalable;
            }
        }
        self.dirty = true;
    }

    pub(super) fn toggle_grid_snap(&mut self, level: &mut Level) {
        self.grid_snap = !self.grid_snap;

        if !self.grid_snap {
            return;
        }

        let selected = self.selected_starts(level);
        if selected.is_empty() {
            return;
        }

        self.save_undo(level);
        for start in selected {
            Self::set_object_pos(level, start.selection, start.pos, true);
        }
        self.dirty = true;
    }

    pub(super) fn grid_snap(&self) -> bool {
        self.grid_snap
    }

    pub(super) fn active_layer(&self, level: &Level) -> i16 {
        self.primary_object_meta(level)
            .map(|meta| meta.editor_layer)
            .unwrap_or(self.current_layer)
    }

    pub(super) fn primary_object_meta(&self, level: &Level) -> Option<ObjectMeta> {
        let (kind, index) = selection_object_key(self.primary_selected()?);

        Some(level.object_meta(kind, index))
    }

    pub(super) fn adjust_selected_object_meta(
        &mut self,
        level: &mut Level,
        group_delta: i16,
        layer_delta: i16,
    ) -> bool {
        let selected = self.valid_selected(level);
        if selected.is_empty() {
            if group_delta == 0 && layer_delta != 0 {
                self.current_layer = self.current_layer.saturating_add(layer_delta);
                return true;
            }
            return false;
        }

        self.save_undo(level);
        for selection in selected {
            let (kind, index) = selection_object_key(selection);
            let mut meta = level.object_meta(kind, index);

            meta.group_id = offset_u16(meta.group_id, group_delta);
            meta.editor_layer = meta.editor_layer.saturating_add(layer_delta);
            level.set_object_meta(kind, index, meta);
        }
        self.current_layer = self.active_layer(level);
        self.dirty = true;
        true
    }

    pub(super) fn snap_point(&self, pos: Vec2) -> Vec2 {
        maybe_snap(pos, self.grid_snap)
    }

    pub(super) fn select_all(&mut self, level: &Level) {
        self.selected = (0..level.solids.len())
            .map(EditorSelection::Solid)
            .chain((0..level.doors.len()).map(EditorSelection::Door))
            .chain((0..level.hazards.len()).map(EditorSelection::Hazard))
            .chain((0..level.checkpoints.len()).map(EditorSelection::Checkpoint))
            .chain((0..level.enemies.len()).map(EditorSelection::Enemy))
            .chain((0..level.triggers.len()).map(EditorSelection::Trigger))
            .chain((0..level.texts.len()).map(EditorSelection::Text))
            .chain((0..level.world_portals.len()).map(EditorSelection::WorldPortal))
            .collect();
        self.text_editing = false;
    }

    pub(super) fn toggle_text_editing(&mut self) -> bool {
        if self.selected_text().is_none() {
            return false;
        }

        self.text_editing = !self.text_editing;
        true
    }

    pub(super) fn adjust_selected_world_portal(
        &mut self,
        level: &mut Level,
        id_delta: i16,
        receiver_delta: i16,
        priority_delta: i16,
    ) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.id = offset_u16(portal.id, id_delta);
        portal.receiver_id = offset_u16(portal.receiver_id, receiver_delta);
        portal.priority = portal.priority.saturating_add(priority_delta);
        self.dirty = true;
        true
    }

    pub(super) fn toggle_selected_door_automatic(&mut self, level: &mut Level) -> bool {
        let Some(EditorSelection::Door(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(door) = level.doors.get_mut(index) else {
            return false;
        };

        door.automatic = !door.automatic;
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_door_radius(&mut self, level: &mut Level, delta: f32) -> bool {
        let Some(EditorSelection::Door(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(door) = level.doors.get_mut(index) else {
            return false;
        };

        door.trigger_radius = (door.trigger_radius + delta).clamp(16.0, 2048.0);
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_door_speed(&mut self, level: &mut Level, delta: f32) -> bool {
        let Some(EditorSelection::Door(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(door) = level.doors.get_mut(index) else {
            return false;
        };

        door.speed = (door.speed + delta).clamp(0.2, 12.0);
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_enemy_spawn(
        &mut self,
        level: &mut Level,
        id_delta: i16,
        wave_delta: i16,
    ) -> bool {
        let Some(EditorSelection::Enemy(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(enemy) = level.enemies.get_mut(index) else {
            return false;
        };

        enemy.spawn_id = offset_u16_min(enemy.spawn_id, id_delta, 1);
        enemy.spawn_wave = offset_u16_min(enemy.spawn_wave.max(1), wave_delta, 1);
        enemy.pos = enemy.spawn_pos;
        enemy.prev_pos = enemy.spawn_pos;
        enemy.vel = Vec2::ZERO;
        enemy.active = false;
        enemy.spawned = false;
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_enemy_spawn_trigger(
        &mut self,
        level: &mut Level,
        id_delta: i16,
    ) -> bool {
        let Some(EditorSelection::Trigger(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(trigger) = level.triggers.get_mut(index) else {
            return false;
        };
        let LevelTriggerKind::EnemySpawn { enemy_id } = &mut trigger.kind else {
            return false;
        };

        *enemy_id = offset_u16_min(*enemy_id, id_delta, 1);
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_world_portal_scale(
        &mut self,
        level: &mut Level,
        delta: f32,
    ) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.portal.scale = (portal.portal.scale + delta).clamp(0.25, 4.0);
        self.dirty = true;
        true
    }

    pub(super) fn toggle_selected_world_portal_seamless(&mut self, level: &mut Level) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.seamless = !portal.seamless;
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_world_portal_seamless_depth(
        &mut self,
        level: &mut Level,
        delta: f32,
    ) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.seamless_depth = (portal.seamless_depth + delta).clamp(16.0, 4096.0);
        self.dirty = true;
        true
    }

    pub(super) fn adjust_selected_world_portal_seamless_angle(
        &mut self,
        level: &mut Level,
        delta: f32,
    ) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.seamless_angle = (portal.seamless_angle + delta).clamp(5.0, 360.0);
        self.dirty = true;
        true
    }

    pub(super) fn toggle_selected_world_portal_rely_on_walls(&mut self, level: &mut Level) -> bool {
        let Some(EditorSelection::WorldPortal(index)) = self.primary_selected() else {
            return false;
        };

        self.save_undo(level);
        let Some(portal) = level.world_portals.get_mut(index) else {
            return false;
        };

        portal.seamless_rely_on_walls = !portal.seamless_rely_on_walls;
        self.dirty = true;
        true
    }

    pub(super) fn handle_text_key(
        &mut self,
        code: KeyCode,
        shift: bool,
        level: &mut Level,
    ) -> bool {
        if !self.text_editing {
            return false;
        }

        let Some(index) = self.selected_text() else {
            self.text_editing = false;
            return false;
        };

        match code {
            KeyCode::Enter => {
                self.text_editing = false;
            }
            KeyCode::Backspace => {
                self.save_undo(level);
                if let Some(text) = level.texts.get_mut(index) {
                    text.text.pop();
                    if text.text.is_empty() {
                        text.text.push(' ');
                    }
                }
                self.dirty = true;
            }
            _ => {
                let Some(ch) = key_char(code, shift) else {
                    return true;
                };

                self.save_undo(level);
                if let Some(text) = level.texts.get_mut(index) {
                    if text.text == "TEXT" {
                        text.text.clear();
                    }
                    text.text.push(ch);
                }
                self.dirty = true;
            }
        }

        true
    }

    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(super) fn selected_solids(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::solid_index)
    }

    pub(super) fn selected_doors(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::door_index)
    }

    pub(super) fn selected_hazards(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::hazard_index)
    }

    pub(super) fn selected_checkpoints(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::checkpoint_index)
    }

    pub(super) fn selected_enemies(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::enemy_index)
    }

    pub(super) fn selected_triggers(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::trigger_index)
    }

    pub(super) fn selected_texts(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::text_index)
    }

    pub(super) fn selected_world_portals(&self) -> Vec<usize> {
        self.selected_indices(EditorSelection::world_portal_index)
    }

    fn selected_indices(&self, index_of: fn(EditorSelection) -> Option<usize>) -> Vec<usize> {
        let mut selected = self
            .selected
            .iter()
            .copied()
            .filter_map(index_of)
            .collect::<Vec<_>>();

        selected.sort_unstable();
        selected.dedup();
        selected
    }

    pub(super) fn selection_count(&self) -> usize {
        self.selected.len()
    }

    pub(super) fn primary_selection_kind(&self) -> EditorSelectionKind {
        match self.primary_selected() {
            Some(EditorSelection::Solid(_)) => EditorSelectionKind::Solid,
            Some(EditorSelection::Door(_)) => EditorSelectionKind::Door,
            Some(EditorSelection::Hazard(_)) => EditorSelectionKind::Hazard,
            Some(EditorSelection::Checkpoint(_)) => EditorSelectionKind::Checkpoint,
            Some(EditorSelection::Enemy(_)) => EditorSelectionKind::Enemy,
            Some(EditorSelection::Trigger(_)) => EditorSelectionKind::Trigger,
            Some(EditorSelection::Text(_)) => EditorSelectionKind::Text,
            Some(EditorSelection::WorldPortal(_)) => EditorSelectionKind::WorldPortal,
            None => EditorSelectionKind::None,
        }
    }

    pub(super) fn primary_door_index(&self) -> Option<usize> {
        match self.primary_selected()? {
            EditorSelection::Door(index) => Some(index),
            _ => None,
        }
    }

    pub(super) fn primary_world_portal_index(&self) -> Option<usize> {
        match self.primary_selected()? {
            EditorSelection::WorldPortal(index) => Some(index),
            _ => None,
        }
    }

    pub(super) fn primary_enemy_index(&self) -> Option<usize> {
        match self.primary_selected()? {
            EditorSelection::Enemy(index) => Some(index),
            _ => None,
        }
    }

    pub(super) fn primary_trigger_index(&self) -> Option<usize> {
        match self.primary_selected()? {
            EditorSelection::Trigger(index) => Some(index),
            _ => None,
        }
    }

    pub(super) fn text_editing(&self) -> bool {
        self.text_editing
    }

    pub(super) fn marquee_rect(&self) -> Option<(Vec2, Vec2)> {
        match &self.drag {
            EditorDrag::Marquee { start, current, .. } => Some(selection_rect(*start, *current)),
            _ => None,
        }
    }

    pub(super) fn mark_saved(&mut self) {
        self.dirty = false;
        self.status_timer = 1.5;
    }

    pub(super) fn set_pan_key(&mut self, code: KeyCode, down: bool) -> bool {
        if self.text_editing {
            return false;
        }

        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => self.pan.up = down,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.pan.left = down,
            KeyCode::KeyS | KeyCode::ArrowDown => self.pan.down = down,
            KeyCode::KeyD | KeyCode::ArrowRight => self.pan.right = down,
            _ => return false,
        }

        true
    }

    pub(super) fn pan_direction(&self) -> Vec2 {
        Vec2::new(
            self.pan.right as i32 as f32 - self.pan.left as i32 as f32,
            self.pan.down as i32 as f32 - self.pan.up as i32 as f32,
        )
    }

    fn push_block(&self, center: Vec2, level: &mut Level) {
        let size = Vec2::new(96.0, 32.0);
        level.solids.push(Solid::new(
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            size.x,
            size.y,
            self.tool.portalable(),
        ));
    }

    fn tag_new_object(&self, level: &mut Level, selection: EditorSelection) {
        if self.current_layer == 0 {
            return;
        }
        let (kind, index) = selection_object_key(selection);
        level.set_object_meta(
            kind,
            index,
            ObjectMeta {
                group_id: 0,
                editor_layer: self.current_layer,
            },
        );
    }

    fn object_at(&self, pos: Vec2, level: &Level) -> Option<(EditorSelection, SolidHit)> {
        if let Some((index, _)) = level
            .texts
            .iter()
            .enumerate()
            .rev()
            .find(|(_, text)| text_at(pos, text))
        {
            return Some((EditorSelection::Text(index), SolidHit::Body));
        }

        if let Some((index, hit)) =
            level
                .world_portals
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, portal)| {
                    solid_at(pos, portal.edit_solid(), self.rotate_ui).map(|hit| (index, hit))
                })
        {
            return Some((EditorSelection::WorldPortal(index), hit));
        }

        if let Some((index, hit)) = level
            .doors
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, door)| solid_at(pos, door.solid, false).map(|hit| (index, hit)))
        {
            return Some((EditorSelection::Door(index), hit));
        }

        if let Some((index, hit)) =
            level
                .checkpoints
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, checkpoint)| {
                    solid_at(pos, checkpoint.solid(), false).map(|hit| (index, hit))
                })
        {
            return Some((EditorSelection::Checkpoint(index), hit));
        }

        if let Some((index, hit)) =
            level
                .triggers
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, trigger)| {
                    solid_at(pos, trigger.solid, false).map(|hit| (index, hit))
                })
        {
            return Some((EditorSelection::Trigger(index), hit));
        }

        if let Some((index, hit)) =
            level
                .enemies
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, enemy)| {
                    solid_at(pos, enemy.spawn_solid(), false).map(|hit| (index, hit))
                })
        {
            return Some((EditorSelection::Enemy(index), hit));
        }

        if let Some((index, hit)) = level
            .hazards
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, hazard)| solid_at(pos, hazard.solid, false).map(|hit| (index, hit)))
        {
            return Some((EditorSelection::Hazard(index), hit));
        }

        level
            .solids
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, solid)| {
                solid_at(pos, *solid, self.rotate_ui)
                    .map(|hit| (EditorSelection::Solid(index), hit))
            })
    }

    fn drag_from_selection(
        &self,
        selection: EditorSelection,
        hit: SolidHit,
        pos: Vec2,
        level: &Level,
    ) -> EditorDrag {
        match selection {
            EditorSelection::Solid(index) => level
                .solids
                .get(index)
                .copied()
                .map(|solid| drag_from_hit(hit, pos, solid))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Door(index) => level
                .doors
                .get(index)
                .copied()
                .map(|door| drag_from_hit(hit, pos, door.solid))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Hazard(index) => level
                .hazards
                .get(index)
                .copied()
                .map(|hazard| drag_from_hit(hit, pos, hazard.solid))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Checkpoint(index) => level
                .checkpoints
                .get(index)
                .copied()
                .map(|checkpoint| drag_from_hit(hit, pos, checkpoint.solid()))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Enemy(index) => level
                .enemies
                .get(index)
                .map(|enemy| drag_from_hit(hit, pos, enemy.spawn_solid()))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Trigger(index) => level
                .triggers
                .get(index)
                .copied()
                .map(|trigger| drag_from_hit(hit, pos, trigger.solid))
                .unwrap_or(EditorDrag::None),
            EditorSelection::Text(index) => level
                .texts
                .get(index)
                .map(|text| {
                    let (_, size) = text_bounds(text);
                    EditorDrag::Move {
                        grab: pos - (text.pos + size / 2.0),
                    }
                })
                .unwrap_or(EditorDrag::None),
            EditorSelection::WorldPortal(index) => level
                .world_portals
                .get(index)
                .copied()
                .map(|portal| drag_from_hit(hit, pos, portal.edit_solid()))
                .unwrap_or(EditorDrag::None),
        }
    }

    fn objects_intersecting(
        &self,
        level: &Level,
        rect_pos: Vec2,
        rect_size: Vec2,
    ) -> Vec<EditorSelection> {
        let solids = level
            .solids
            .iter()
            .enumerate()
            .filter_map(|(index, solid)| {
                solid_intersects_rect(*solid, rect_pos, rect_size)
                    .then_some(EditorSelection::Solid(index))
            });
        let doors = level.doors.iter().enumerate().filter_map(|(index, door)| {
            solid_intersects_rect(door.solid, rect_pos, rect_size)
                .then_some(EditorSelection::Door(index))
        });
        let hazards = level
            .hazards
            .iter()
            .enumerate()
            .filter_map(|(index, hazard)| {
                solid_intersects_rect(hazard.solid, rect_pos, rect_size)
                    .then_some(EditorSelection::Hazard(index))
            });
        let checkpoints = level
            .checkpoints
            .iter()
            .enumerate()
            .filter_map(|(index, checkpoint)| {
                solid_intersects_rect(checkpoint.solid(), rect_pos, rect_size)
                    .then_some(EditorSelection::Checkpoint(index))
            });
        let enemies = level
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| {
                solid_intersects_rect(enemy.spawn_solid(), rect_pos, rect_size)
                    .then_some(EditorSelection::Enemy(index))
            });
        let triggers = level
            .triggers
            .iter()
            .enumerate()
            .filter_map(|(index, trigger)| {
                solid_intersects_rect(trigger.solid, rect_pos, rect_size)
                    .then_some(EditorSelection::Trigger(index))
            });
        let texts = level.texts.iter().enumerate().filter_map(|(index, text)| {
            let (text_pos, text_size) = text_bounds(text);

            rect_intersects_rect(rect_pos, rect_size, text_pos, text_size)
                .then_some(EditorSelection::Text(index))
        });
        let world_portals = level
            .world_portals
            .iter()
            .enumerate()
            .filter_map(|(index, portal)| {
                let (min, max) = world_portal_bounds(*portal);

                rect_intersects_rect(rect_pos, rect_size, min, max - min)
                    .then_some(EditorSelection::WorldPortal(index))
            });

        solids
            .chain(doors)
            .chain(hazards)
            .chain(checkpoints)
            .chain(enemies)
            .chain(triggers)
            .chain(texts)
            .chain(world_portals)
            .collect()
    }

    fn selected_starts(&self, level: &Level) -> Vec<EditorMoveStart> {
        self.valid_selected(level)
            .into_iter()
            .filter_map(|selection| {
                Self::object_pos(level, selection).map(|pos| EditorMoveStart { selection, pos })
            })
            .collect()
    }

    fn selection_center(&self, level: &Level) -> Vec2 {
        let selected = self.valid_selected(level);
        if selected.is_empty() {
            return Vec2::ZERO;
        }

        let (min, max) = selection_bounds(level, &selected);

        (min + max) / 2.0
    }

    fn selected_text(&self) -> Option<usize> {
        match self.primary_selected()? {
            EditorSelection::Text(index) => Some(index),
            _ => None,
        }
    }

    fn primary_selected(&self) -> Option<EditorSelection> {
        self.selected.last().copied()
    }

    fn is_selected(&self, selection: EditorSelection) -> bool {
        self.selected.contains(&selection)
    }

    fn set_single_selection(&mut self, selection: EditorSelection) {
        self.selected.clear();
        self.selected.push(selection);
    }

    fn add_selection(&mut self, selection: EditorSelection) {
        if !self.is_selected(selection) {
            self.selected.push(selection);
        }
    }

    fn toggle_selection(&mut self, selection: EditorSelection) {
        if let Some(pos) = self
            .selected
            .iter()
            .position(|selected| *selected == selection)
        {
            self.selected.remove(pos);
        } else {
            self.selected.push(selection);
        }
    }

    fn clear_selection(&mut self) {
        self.selected.clear();
    }

    fn valid_selected(&self, level: &Level) -> Vec<EditorSelection> {
        let mut selected = self
            .selected
            .iter()
            .copied()
            .filter(|selection| match selection {
                EditorSelection::Solid(index) => *index < level.solids.len(),
                EditorSelection::Door(index) => *index < level.doors.len(),
                EditorSelection::Hazard(index) => *index < level.hazards.len(),
                EditorSelection::Checkpoint(index) => *index < level.checkpoints.len(),
                EditorSelection::Enemy(index) => *index < level.enemies.len(),
                EditorSelection::Trigger(index) => *index < level.triggers.len(),
                EditorSelection::Text(index) => *index < level.texts.len(),
                EditorSelection::WorldPortal(index) => *index < level.world_portals.len(),
            })
            .collect::<Vec<_>>();

        selected.sort_by_key(selection_sort_key);
        selected.dedup();
        selected
    }

    fn normalize_selection(&mut self, level: &Level) {
        self.selected = self.valid_selected(level);
    }

    fn save_undo(&mut self, level: &Level) {
        const UNDO_LIMIT: usize = 64;
        let snapshot = LevelSnapshot {
            solids: level.solids.clone(),
            doors: level.doors.clone(),
            hazards: level.hazards.clone(),
            checkpoints: level.checkpoints.clone(),
            enemies: level.enemies.clone(),
            triggers: level.triggers.clone(),
            texts: level.texts.clone(),
            world_portals: level.world_portals.clone(),
            metadata: level.metadata.clone(),
        };

        if self.undo.back().is_some_and(|last| *last == snapshot) {
            return;
        }
        // Keep history bounded by content, not by editing session length.
        if self.undo.len() == UNDO_LIMIT {
            self.undo.pop_front();
        }

        self.undo.push_back(snapshot);
    }

    fn object_pos(level: &Level, selection: EditorSelection) -> Option<Vec2> {
        match selection {
            EditorSelection::Solid(index) => level.solids.get(index).map(|solid| solid.pos()),
            EditorSelection::Door(index) => level.doors.get(index).map(|door| door.solid.pos()),
            EditorSelection::Hazard(index) => {
                level.hazards.get(index).map(|hazard| hazard.solid.pos())
            }
            EditorSelection::Checkpoint(index) => level
                .checkpoints
                .get(index)
                .map(|checkpoint| checkpoint.solid.pos()),
            EditorSelection::Enemy(index) => level
                .enemies
                .get(index)
                .map(|enemy| enemy.spawn_pos - enemy.half_size()),
            EditorSelection::Trigger(index) => {
                level.triggers.get(index).map(|trigger| trigger.solid.pos())
            }
            EditorSelection::Text(index) => level.texts.get(index).map(|text| text.pos),
            EditorSelection::WorldPortal(index) => {
                level.world_portals.get(index).map(|portal| portal.center())
            }
        }
    }

    fn set_object_pos(level: &mut Level, selection: EditorSelection, pos: Vec2, grid_snap: bool) {
        let pos = maybe_snap(pos, grid_snap);

        match selection {
            EditorSelection::Solid(index) => {
                if let Some(solid) = level.solids.get_mut(index) {
                    solid.set_pos(pos);
                }
            }
            EditorSelection::Door(index) => {
                if let Some(door) = level.doors.get_mut(index) {
                    door.solid.set_pos(pos);
                }
            }
            EditorSelection::Hazard(index) => {
                if let Some(hazard) = level.hazards.get_mut(index) {
                    hazard.solid.set_pos(pos);
                }
            }
            EditorSelection::Checkpoint(index) => {
                if let Some(checkpoint) = level.checkpoints.get_mut(index) {
                    checkpoint.solid.set_pos(pos);
                }
            }
            EditorSelection::Enemy(index) => {
                if let Some(enemy) = level.enemies.get_mut(index) {
                    enemy.spawn_pos = pos + enemy.half_size();
                    enemy.pos = enemy.spawn_pos;
                    enemy.prev_pos = enemy.spawn_pos;
                    enemy.vel = Vec2::ZERO;
                }
            }
            EditorSelection::Trigger(index) => {
                if let Some(trigger) = level.triggers.get_mut(index) {
                    trigger.solid.set_pos(pos);
                }
            }
            EditorSelection::Text(index) => {
                if let Some(text) = level.texts.get_mut(index) {
                    text.pos = pos;
                }
            }
            EditorSelection::WorldPortal(index) => {
                if let Some(portal) = level.world_portals.get_mut(index) {
                    portal.set_center(pos);
                }
            }
        }
    }

    fn set_object_center(
        level: &mut Level,
        selection: EditorSelection,
        center: Vec2,
        grid_snap: bool,
    ) {
        match selection {
            EditorSelection::Solid(index) => {
                if let Some(solid) = level.solids.get_mut(index) {
                    solid.set_pos(maybe_snap(center - solid.size() / 2.0, grid_snap));
                }
            }
            EditorSelection::Door(index) => {
                if let Some(door) = level.doors.get_mut(index) {
                    door.solid
                        .set_pos(maybe_snap(center - door.solid.size() / 2.0, grid_snap));
                }
            }
            EditorSelection::Hazard(index) => {
                if let Some(hazard) = level.hazards.get_mut(index) {
                    hazard
                        .solid
                        .set_pos(maybe_snap(center - hazard.solid.size() / 2.0, grid_snap));
                }
            }
            EditorSelection::Checkpoint(index) => {
                if let Some(checkpoint) = level.checkpoints.get_mut(index) {
                    checkpoint.solid.set_pos(maybe_snap(
                        center - checkpoint.solid.size() / 2.0,
                        grid_snap,
                    ));
                }
            }
            EditorSelection::Enemy(index) => {
                if let Some(enemy) = level.enemies.get_mut(index) {
                    enemy.spawn_pos = maybe_snap(center, grid_snap);
                    enemy.pos = enemy.spawn_pos;
                    enemy.prev_pos = enemy.spawn_pos;
                    enemy.vel = Vec2::ZERO;
                }
            }
            EditorSelection::Trigger(index) => {
                if let Some(trigger) = level.triggers.get_mut(index) {
                    trigger
                        .solid
                        .set_pos(maybe_snap(center - trigger.solid.size() / 2.0, grid_snap));
                }
            }
            EditorSelection::Text(index) => {
                if let Some(text) = level.texts.get_mut(index) {
                    let (_, size) = text_bounds(text);
                    text.pos = maybe_snap(center - size / 2.0, grid_snap);
                }
            }
            EditorSelection::WorldPortal(index) => {
                if let Some(portal) = level.world_portals.get_mut(index) {
                    portal.set_center(maybe_snap(center, grid_snap));
                }
            }
        }
    }

    fn solid_like_mut(level: &mut Level, selection: EditorSelection) -> Option<&mut Solid> {
        match selection {
            EditorSelection::Solid(index) => level.solids.get_mut(index),
            EditorSelection::Door(index) => level.doors.get_mut(index).map(|door| &mut door.solid),
            EditorSelection::Hazard(index) => {
                level.hazards.get_mut(index).map(|hazard| &mut hazard.solid)
            }
            EditorSelection::Checkpoint(index) => level
                .checkpoints
                .get_mut(index)
                .map(|checkpoint| &mut checkpoint.solid),
            EditorSelection::Trigger(index) => level
                .triggers
                .get_mut(index)
                .map(|trigger| &mut trigger.solid),
            EditorSelection::Text(_)
            | EditorSelection::Enemy(_)
            | EditorSelection::WorldPortal(_) => None,
        }
    }

    fn rotate_solid(
        solid: &mut Solid,
        pos: Vec2,
        start_rotation: f32,
        center: Vec2,
        start_angle: f32,
        grid_snap: bool,
    ) {
        let angle = (pos - center).to_angle();
        solid.set_rotation(maybe_snap_angle(
            start_rotation + angle - start_angle,
            grid_snap,
        ));
    }

    fn resize_solid(
        solid: &mut Solid,
        pos: Vec2,
        edge_x: i8,
        edge_y: i8,
        start: Solid,
        start_cursor: Vec2,
        grid_snap: bool,
    ) {
        let local_start_cursor = start.local_from_world(start_cursor);
        let local_cursor = start.local_from_world(pos);
        let delta = maybe_snap_delta(local_cursor - local_start_cursor, grid_snap);
        let start_size = start.size();
        let (min, max) = resized_local_bounds(edge_x, edge_y, delta, start_size);

        let center = start.center()
            + start.axis_x() * ((min.x + max.x - start_size.x) / 2.0)
            + start.axis_y() * ((min.y + max.y - start_size.y) / 2.0);

        solid.set_centered_rect(center, max - min);
        solid.set_rotation(start.rotation());
    }
}

fn maybe_snap(value: Vec2, grid_snap: bool) -> Vec2 {
    if grid_snap { snap(value) } else { value }
}

fn maybe_snap_delta(value: Vec2, grid_snap: bool) -> Vec2 {
    if grid_snap { snap_delta(value) } else { value }
}

fn maybe_snap_angle(value: f32, grid_snap: bool) -> f32 {
    if grid_snap { snap_angle(value) } else { value }
}

fn place_rect(center: Vec2, size: Vec2, grid_snap: bool) -> Vec2 {
    maybe_snap(center - size / 2.0, grid_snap)
}

fn key_char(code: KeyCode, shift: bool) -> Option<char> {
    match code {
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some(if shift { '_' } else { '-' }),
        KeyCode::Equal => Some(if shift { '+' } else { '=' }),
        KeyCode::Slash => Some(if shift { '?' } else { '/' }),
        KeyCode::Backslash => Some(if shift { '|' } else { '\\' }),
        KeyCode::Period => Some(if shift { '>' } else { '.' }),
        KeyCode::Comma => Some(if shift { '<' } else { ',' }),
        KeyCode::Semicolon => Some(if shift { ':' } else { ';' }),
        KeyCode::Quote => Some(if shift { '"' } else { '\'' }),
        KeyCode::BracketLeft => Some(if shift { '{' } else { '[' }),
        KeyCode::BracketRight => Some(if shift { '}' } else { ']' }),
        KeyCode::Backquote => Some(if shift { '~' } else { '`' }),
        KeyCode::Digit0 => Some(if shift { ')' } else { '0' }),
        KeyCode::Digit1 => Some(if shift { '!' } else { '1' }),
        KeyCode::Digit2 => Some(if shift { '@' } else { '2' }),
        KeyCode::Digit3 => Some(if shift { '#' } else { '3' }),
        KeyCode::Digit4 => Some(if shift { '$' } else { '4' }),
        KeyCode::Digit5 => Some(if shift { '%' } else { '5' }),
        KeyCode::Digit6 => Some(if shift { '^' } else { '6' }),
        KeyCode::Digit7 => Some(if shift { '&' } else { '7' }),
        KeyCode::Digit8 => Some(if shift { '*' } else { '8' }),
        KeyCode::Digit9 => Some(if shift { '(' } else { '9' }),
        KeyCode::KeyA => Some('A'),
        KeyCode::KeyB => Some('B'),
        KeyCode::KeyC => Some('C'),
        KeyCode::KeyD => Some('D'),
        KeyCode::KeyE => Some('E'),
        KeyCode::KeyF => Some('F'),
        KeyCode::KeyG => Some('G'),
        KeyCode::KeyH => Some('H'),
        KeyCode::KeyI => Some('I'),
        KeyCode::KeyJ => Some('J'),
        KeyCode::KeyK => Some('K'),
        KeyCode::KeyL => Some('L'),
        KeyCode::KeyM => Some('M'),
        KeyCode::KeyN => Some('N'),
        KeyCode::KeyO => Some('O'),
        KeyCode::KeyP => Some('P'),
        KeyCode::KeyQ => Some('Q'),
        KeyCode::KeyR => Some('R'),
        KeyCode::KeyS => Some('S'),
        KeyCode::KeyT => Some('T'),
        KeyCode::KeyU => Some('U'),
        KeyCode::KeyV => Some('V'),
        KeyCode::KeyW => Some('W'),
        KeyCode::KeyX => Some('X'),
        KeyCode::KeyY => Some('Y'),
        KeyCode::KeyZ => Some('Z'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::level::LevelObjectKind;

    #[test]
    fn duplicate_selected_preserves_object_metadata() {
        let mut editor = Editor::new();
        let mut level = Level::empty();

        editor.create_block(Vec2::new(128.0, 128.0), EditorTool::Solid, &mut level);
        assert!(editor.adjust_selected_object_meta(&mut level, 3, 2));
        editor.duplicate_selected(&mut level);

        assert_eq!(level.solids.len(), 2);
        assert_eq!(
            level.object_meta(LevelObjectKind::Solid, 1),
            ObjectMeta {
                group_id: 3,
                editor_layer: 2,
            }
        );
    }

    #[test]
    fn delete_selected_shifts_surviving_metadata() {
        let mut editor = Editor::new();
        let mut level = Level {
            solids: vec![
                Solid::new(0.0, 0.0, 32.0, 32.0, false),
                Solid::new(64.0, 0.0, 32.0, 32.0, false),
            ],
            ..Level::empty()
        };
        level.set_object_meta(
            LevelObjectKind::Solid,
            0,
            ObjectMeta {
                group_id: 1,
                editor_layer: 0,
            },
        );
        level.set_object_meta(
            LevelObjectKind::Solid,
            1,
            ObjectMeta {
                group_id: 2,
                editor_layer: 4,
            },
        );
        editor.set_single_selection(EditorSelection::Solid(0));

        editor.delete_selected(&mut level);

        assert_eq!(level.solids.len(), 1);
        assert_eq!(level.object_meta(LevelObjectKind::Solid, 0).group_id, 2);
        assert_eq!(level.object_meta(LevelObjectKind::Solid, 0).editor_layer, 4);
    }
}

use std::collections::VecDeque;

use glam::Vec2;
use winit::keyboard::KeyCode;

// Editor stores mode state and applies commands to level objects.
use crate::constants::PORTAL_WIDTH;
use crate::game::level::{Checkpoint, Door, Hazard, Level, LevelText, Solid, WorldPortal};
use crate::game::portal::Portal;

use super::editor_geometry::{
    EditorDrag, EditorMoveStart, EditorSelection, SolidHit, drag_from_hit, rect_intersects_rect,
    resized_local_bounds, selection_rect, snap, snap_angle, snap_delta, solid_at,
    solid_intersects_rect, text_at, text_bounds,
};

const WORLD_PORTAL_EDIT_THICKNESS: f32 = 24.0;

pub(super) struct Editor {
    selected: Vec<EditorSelection>,
    clipboard: Vec<EditorClipboard>,
    drag: EditorDrag,
    pub(super) tool: EditorTool,
    pan: EditorPan,
    pub(super) rotate_ui: bool,
    pub(super) dirty: bool,
    pub(super) status_timer: f32,
    grid_snap: bool,
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
            pan: EditorPan::default(),
            rotate_ui: false,
            dirty: false,
            status_timer: 0.0,
            grid_snap: true,
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
        self.save_undo(level);
        self.push_block(maybe_snap(pos, self.grid_snap), level);
        self.set_single_selection(EditorSelection::Solid(level.solids.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_door(&mut self, pos: Vec2, level: &mut Level) {
        self.save_undo(level);
        let size = Vec2::new(48.0, 112.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level.doors.push(Door::new(pos.x, pos.y, size.x, size.y));
        self.set_single_selection(EditorSelection::Door(level.doors.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_text(&mut self, pos: Vec2, level: &mut Level) {
        self.save_undo(level);
        level
            .texts
            .push(LevelText::new(maybe_snap(pos, self.grid_snap), "TEXT"));
        self.set_single_selection(EditorSelection::Text(level.texts.len() - 1));
        self.text_editing = true;
        self.dirty = true;
    }

    pub(super) fn create_hazard(&mut self, pos: Vec2, level: &mut Level) {
        self.save_undo(level);
        let size = Vec2::new(128.0, 24.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .hazards
            .push(Hazard::new(pos.x, pos.y, size.x, size.y));
        self.set_single_selection(EditorSelection::Hazard(level.hazards.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_checkpoint(&mut self, pos: Vec2, level: &mut Level) {
        self.save_undo(level);
        let size = Vec2::new(48.0, 80.0);
        let pos = place_rect(pos, size, self.grid_snap);

        level
            .checkpoints
            .push(Checkpoint::new(pos.x, pos.y, size.x, size.y));
        self.set_single_selection(EditorSelection::Checkpoint(level.checkpoints.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_world_portal(&mut self, pos: Vec2, level: &mut Level) {
        self.save_undo(level);
        let center = maybe_snap(pos, self.grid_snap);

        level.world_portals.push(WorldPortal::new(
            center.x,
            center.y,
            Vec2::new(0.0, -1.0),
            PORTAL_WIDTH,
            0,
        ));
        self.set_single_selection(EditorSelection::WorldPortal(level.world_portals.len() - 1));
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn create_active_tool(&mut self, pos: Vec2, level: &mut Level) {
        match self.tool {
            EditorTool::Solid | EditorTool::Portalable => self.create_block(pos, self.tool, level),
            EditorTool::Door => self.create_door(pos, level),
            EditorTool::Text => self.create_text(pos, level),
            EditorTool::Hazard => self.create_hazard(pos, level),
            EditorTool::Checkpoint => self.create_checkpoint(pos, level),
            EditorTool::WorldPortal => self.create_world_portal(pos, level),
        }
    }

    pub(super) fn set_tool(&mut self, tool: EditorTool) {
        self.tool = tool;
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
                        let mut solid = world_portal_edit_solid(*portal);

                        Self::resize_solid(
                            &mut solid,
                            pos,
                            *edge_x,
                            *edge_y,
                            *start,
                            *start_cursor,
                            grid_snap,
                        );
                        apply_edit_solid_to_world_portal(portal, solid);
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
                        let mut solid = world_portal_edit_solid(*portal);

                        Self::rotate_solid(
                            &mut solid,
                            pos,
                            *start_rotation,
                            *center,
                            *start_angle,
                            grid_snap,
                        );
                        apply_edit_solid_to_world_portal(portal, solid);
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
        let mut solids = Vec::new();
        let mut doors = Vec::new();
        let mut hazards = Vec::new();
        let mut checkpoints = Vec::new();
        let mut texts = Vec::new();
        let mut world_portals = Vec::new();

        for selection in self.valid_selected(level) {
            match selection {
                EditorSelection::Solid(index) => solids.push(index),
                EditorSelection::Door(index) => doors.push(index),
                EditorSelection::Hazard(index) => hazards.push(index),
                EditorSelection::Checkpoint(index) => checkpoints.push(index),
                EditorSelection::Text(index) => texts.push(index),
                EditorSelection::WorldPortal(index) => world_portals.push(index),
            }
        }

        remove_indices(&mut level.solids, solids);
        remove_indices(&mut level.doors, doors);
        remove_indices(&mut level.hazards, hazards);
        remove_indices(&mut level.checkpoints, checkpoints);
        remove_indices(&mut level.texts, texts);
        remove_indices(&mut level.world_portals, world_portals);
        self.clear_selection();
        self.dirty = true;
    }

    pub(super) fn copy_selected(&mut self, level: &Level) {
        self.clipboard = self
            .valid_selected(level)
            .into_iter()
            .filter_map(|selection| match selection {
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
                EditorSelection::Text(index) => {
                    level.texts.get(index).cloned().map(EditorClipboard::Text)
                }
                EditorSelection::WorldPortal(index) => level
                    .world_portals
                    .get(index)
                    .copied()
                    .map(EditorClipboard::WorldPortal),
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
            match item {
                EditorClipboard::Solid(mut solid) => {
                    solid.pos += offset;
                    level.solids.push(solid);
                    pasted.push(EditorSelection::Solid(level.solids.len() - 1));
                }
                EditorClipboard::Door(mut door) => {
                    door.solid.pos += offset;
                    door.open = 0.0;
                    level.doors.push(door);
                    pasted.push(EditorSelection::Door(level.doors.len() - 1));
                }
                EditorClipboard::Hazard(mut hazard) => {
                    hazard.solid.pos += offset;
                    level.hazards.push(hazard);
                    pasted.push(EditorSelection::Hazard(level.hazards.len() - 1));
                }
                EditorClipboard::Checkpoint(mut checkpoint) => {
                    checkpoint.solid.pos += offset;
                    level.checkpoints.push(checkpoint);
                    pasted.push(EditorSelection::Checkpoint(level.checkpoints.len() - 1));
                }
                EditorClipboard::Text(mut text) => {
                    text.pos += offset;
                    level.texts.push(text);
                    pasted.push(EditorSelection::Text(level.texts.len() - 1));
                }
                EditorClipboard::WorldPortal(mut portal) => {
                    portal.portal.pos += offset;
                    level.world_portals.push(portal);
                    pasted.push(EditorSelection::WorldPortal(level.world_portals.len() - 1));
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
        level.texts = previous.texts;
        level.world_portals = previous.world_portals;
        self.normalize_selection(level);
        self.text_editing = false;
        self.dirty = true;
    }

    pub(super) fn toggle_portalable(&mut self, level: &mut Level) {
        let selected = self.valid_selected(level);
        let solid_indices = selected
            .into_iter()
            .filter_map(|selection| match selection {
                EditorSelection::Solid(index) => Some(index),
                _ => None,
            })
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

    pub(super) fn snap_point(&self, pos: Vec2) -> Vec2 {
        maybe_snap(pos, self.grid_snap)
    }

    pub(super) fn select_all(&mut self, level: &Level) {
        self.selected = (0..level.solids.len())
            .map(EditorSelection::Solid)
            .chain((0..level.doors.len()).map(EditorSelection::Door))
            .chain((0..level.hazards.len()).map(EditorSelection::Hazard))
            .chain((0..level.checkpoints.len()).map(EditorSelection::Checkpoint))
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
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::Solid(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    pub(super) fn selected_doors(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::Door(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    pub(super) fn selected_hazards(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::Hazard(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    pub(super) fn selected_checkpoints(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::Checkpoint(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    pub(super) fn selected_texts(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::Text(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    pub(super) fn selected_world_portals(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|selection| match selection {
                EditorSelection::WorldPortal(index) => Some(*index),
                _ => None,
            })
            .collect()
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
                    solid_at(pos, world_portal_edit_solid(*portal), self.rotate_ui)
                        .map(|hit| (index, hit))
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
                .map(|portal| drag_from_hit(hit, pos, world_portal_edit_solid(portal)))
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
            texts: level.texts.clone(),
            world_portals: level.world_portals.clone(),
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
            EditorSelection::Solid(index) => level.solids.get(index).map(|solid| solid.pos),
            EditorSelection::Door(index) => level.doors.get(index).map(|door| door.solid.pos),
            EditorSelection::Hazard(index) => {
                level.hazards.get(index).map(|hazard| hazard.solid.pos)
            }
            EditorSelection::Checkpoint(index) => level
                .checkpoints
                .get(index)
                .map(|checkpoint| checkpoint.solid.pos),
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
                    solid.pos = pos;
                }
            }
            EditorSelection::Door(index) => {
                if let Some(door) = level.doors.get_mut(index) {
                    door.solid.pos = pos;
                }
            }
            EditorSelection::Hazard(index) => {
                if let Some(hazard) = level.hazards.get_mut(index) {
                    hazard.solid.pos = pos;
                }
            }
            EditorSelection::Checkpoint(index) => {
                if let Some(checkpoint) = level.checkpoints.get_mut(index) {
                    checkpoint.solid.pos = pos;
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
                    solid.pos = maybe_snap(center - solid.size / 2.0, grid_snap);
                }
            }
            EditorSelection::Door(index) => {
                if let Some(door) = level.doors.get_mut(index) {
                    door.solid.pos = maybe_snap(center - door.solid.size / 2.0, grid_snap);
                }
            }
            EditorSelection::Hazard(index) => {
                if let Some(hazard) = level.hazards.get_mut(index) {
                    hazard.solid.pos = maybe_snap(center - hazard.solid.size / 2.0, grid_snap);
                }
            }
            EditorSelection::Checkpoint(index) => {
                if let Some(checkpoint) = level.checkpoints.get_mut(index) {
                    checkpoint.solid.pos =
                        maybe_snap(center - checkpoint.solid.size / 2.0, grid_snap);
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
            EditorSelection::Text(_) | EditorSelection::WorldPortal(_) => None,
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
        let (min, max) = resized_local_bounds(edge_x, edge_y, delta, start.size);

        let center = start.center()
            + start.axis_x() * ((min.x + max.x - start.size.x) / 2.0)
            + start.axis_y() * ((min.y + max.y - start.size.y) / 2.0);

        solid.size = max - min;
        solid.pos = center - solid.size / 2.0;
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

#[derive(Clone, PartialEq)]
struct LevelSnapshot {
    solids: Vec<Solid>,
    doors: Vec<Door>,
    hazards: Vec<Hazard>,
    checkpoints: Vec<Checkpoint>,
    texts: Vec<LevelText>,
    world_portals: Vec<WorldPortal>,
}

#[derive(Clone)]
enum EditorClipboard {
    Solid(Solid),
    Door(Door),
    Hazard(Hazard),
    Checkpoint(Checkpoint),
    Text(LevelText),
    WorldPortal(WorldPortal),
}

#[derive(Default)]
struct EditorPan {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

#[derive(Clone, Copy)]
pub(super) enum EditorTool {
    Solid,
    Portalable,
    Door,
    Text,
    Hazard,
    Checkpoint,
    WorldPortal,
}

impl EditorTool {
    pub(super) fn portalable(self) -> bool {
        matches!(self, Self::Portalable)
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::Solid => 1,
            Self::Portalable => 2,
            Self::Door => 3,
            Self::Text => 4,
            Self::Hazard => 5,
            Self::Checkpoint => 6,
            Self::WorldPortal => 7,
        }
    }

    pub(super) fn from_index(index: usize) -> Option<Self> {
        match index {
            1 => Some(Self::Solid),
            2 => Some(Self::Portalable),
            3 => Some(Self::Door),
            4 => Some(Self::Text),
            5 => Some(Self::Hazard),
            6 => Some(Self::Checkpoint),
            7 => Some(Self::WorldPortal),
            _ => None,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Solid => "SOLID",
            Self::Portalable => "PORTAL",
            Self::Door => "DOOR",
            Self::Text => "TEXT",
            Self::Hazard => "ACID",
            Self::Checkpoint => "CHECK",
            Self::WorldPortal => "W PORT",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum EditorSelectionKind {
    None,
    Solid,
    Door,
    Hazard,
    Checkpoint,
    Text,
    WorldPortal,
}

impl EditorSelectionKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Solid => "SOLID",
            Self::Door => "DOOR",
            Self::Hazard => "ACID",
            Self::Checkpoint => "CHECKPOINT",
            Self::Text => "TEXT",
            Self::WorldPortal => "WORLD PORTAL",
        }
    }
}

fn remove_indices<T>(items: &mut Vec<T>, mut indices: Vec<usize>) {
    indices.sort_unstable_by(|a, b| b.cmp(a));
    indices.dedup();
    for index in indices {
        items.remove(index);
    }
}

fn selection_sort_key(selection: &EditorSelection) -> (usize, usize) {
    match selection {
        EditorSelection::Solid(index) => (0, *index),
        EditorSelection::Door(index) => (1, *index),
        EditorSelection::Hazard(index) => (2, *index),
        EditorSelection::Checkpoint(index) => (3, *index),
        EditorSelection::Text(index) => (4, *index),
        EditorSelection::WorldPortal(index) => (5, *index),
    }
}

fn selection_bounds(level: &Level, selected: &[EditorSelection]) -> (Vec2, Vec2) {
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

fn include_solid_bounds(solid: Solid, min: &mut Vec2, max: &mut Vec2) {
    for corner in solid.corners() {
        *min = min.min(corner);
        *max = max.max(corner);
    }
}

fn world_portal_bounds(portal: WorldPortal) -> (Vec2, Vec2) {
    let corners = world_portal_edit_solid(portal).corners();
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

fn world_portal_edit_solid(portal: WorldPortal) -> Solid {
    let width = portal.portal.active_width().max(24.0);
    let size = Vec2::new(width, WORLD_PORTAL_EDIT_THICKNESS);
    let center = portal.center();

    Solid::rotated(
        center.x - size.x / 2.0,
        center.y - size.y / 2.0,
        size.x,
        size.y,
        portal.portal.tangent().to_angle(),
        false,
    )
}

fn apply_edit_solid_to_world_portal(portal: &mut WorldPortal, solid: Solid) {
    let mut edited = Portal::with_tangent(
        solid.center().x,
        solid.center().y,
        -solid.axis_y(),
        solid.axis_x(),
        solid.size.x.max(24.0),
        portal.portal.color,
    );

    edited.scale = portal.portal.scale;
    edited.scale_objects = portal.portal.scale_objects;
    portal.portal = edited;
}

fn offset_u16(value: u16, delta: i16) -> u16 {
    if delta.is_negative() {
        value.saturating_sub(delta.unsigned_abs())
    } else {
        value.saturating_add(delta as u16)
    }
}

fn clipboard_bounds(clipboard: &[EditorClipboard]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);

    for item in clipboard {
        match item {
            EditorClipboard::Solid(solid) => include_solid_bounds(*solid, &mut min, &mut max),
            EditorClipboard::Door(door) => include_solid_bounds(door.solid, &mut min, &mut max),
            EditorClipboard::Hazard(hazard) => {
                include_solid_bounds(hazard.solid, &mut min, &mut max)
            }
            EditorClipboard::Checkpoint(checkpoint) => {
                include_solid_bounds(checkpoint.solid, &mut min, &mut max)
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

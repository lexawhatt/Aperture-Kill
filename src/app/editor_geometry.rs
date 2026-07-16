use glam::Vec2;

// Hit tests and snapping live here so editor state stays small.
use crate::game::level::{LevelText, Solid};

const EDITOR_GRID: f32 = 16.0;
const BODY_HIT_PADDING: f32 = 8.0;
const HANDLE_HIT_SIZE: f32 = 18.0;
const ROTATE_HIT_SIZE: f32 = 24.0;
const ROTATE_RING_OFFSET: f32 = 18.0;

#[derive(Clone)]
pub(super) enum EditorDrag {
    None,
    Move {
        grab: Vec2,
    },
    MoveSelection {
        start_cursor: Vec2,
        starts: Vec<EditorMoveStart>,
    },
    Resize {
        edge_x: i8,
        edge_y: i8,
        start: Solid,
        start_cursor: Vec2,
    },
    Rotate {
        start_rotation: f32,
        center: Vec2,
        start_angle: f32,
    },
    Marquee {
        start: Vec2,
        current: Vec2,
        additive: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorSelection {
    Solid(usize),
    Door(usize),
    Hazard(usize),
    Checkpoint(usize),
    Text(usize),
    WorldPortal(usize),
}

#[derive(Clone)]
pub(super) struct EditorMoveStart {
    pub(super) selection: EditorSelection,
    pub(super) pos: Vec2,
}

pub(super) enum SolidHit {
    Body,
    Resize { edge_x: i8, edge_y: i8 },
    Rotate,
}

pub(super) fn drag_from_hit(hit: SolidHit, pos: Vec2, solid: Solid) -> EditorDrag {
    match hit {
        SolidHit::Body => EditorDrag::Move {
            grab: pos - solid.center(),
        },
        SolidHit::Rotate => {
            let center = solid.center();

            EditorDrag::Rotate {
                start_rotation: solid.rotation(),
                center,
                start_angle: (pos - center).to_angle(),
            }
        }
        SolidHit::Resize { edge_x, edge_y } => EditorDrag::Resize {
            edge_x,
            edge_y,
            start: solid,
            start_cursor: pos,
        },
    }
}

pub(super) fn solid_at(pos: Vec2, solid: Solid, rotate_ui: bool) -> Option<SolidHit> {
    if rotate_ui && rotate_hit(pos, solid) {
        return Some(SolidHit::Rotate);
    }

    let local = solid.local_from_world(pos);
    if outside_padded_body(local, solid.size) {
        return None;
    }

    resize_hit(local, solid.size).or(Some(SolidHit::Body))
}

pub(super) fn resized_local_bounds(
    edge_x: i8,
    edge_y: i8,
    delta: Vec2,
    size: Vec2,
) -> (Vec2, Vec2) {
    let mut min = Vec2::ZERO;
    let mut max = size;

    if edge_x < 0 {
        min.x = (min.x + delta.x).min(max.x - EDITOR_GRID);
    } else if edge_x > 0 {
        max.x = (max.x + delta.x).max(min.x + EDITOR_GRID);
    }

    if edge_y < 0 {
        min.y = (min.y + delta.y).min(max.y - EDITOR_GRID);
    } else if edge_y > 0 {
        max.y = (max.y + delta.y).max(min.y + EDITOR_GRID);
    }

    (min, max)
}

pub(super) fn selection_rect(start: Vec2, current: Vec2) -> (Vec2, Vec2) {
    let min = start.min(current);
    let max = start.max(current);

    (min, max - min)
}

pub(super) fn solid_intersects_rect(solid: Solid, rect_pos: Vec2, rect_size: Vec2) -> bool {
    let rect_max = rect_pos + rect_size;
    let corners = solid.corners();
    let solid_min = corners
        .iter()
        .copied()
        .reduce(Vec2::min)
        .unwrap_or(solid.pos);
    let solid_max = corners
        .iter()
        .copied()
        .reduce(Vec2::max)
        .unwrap_or(solid.pos + solid.size);

    solid_min.x <= rect_max.x
        && solid_max.x >= rect_pos.x
        && solid_min.y <= rect_max.y
        && solid_max.y >= rect_pos.y
}

pub(super) fn text_bounds(text: &LevelText) -> (Vec2, Vec2) {
    let width = (text.text.chars().count().max(1) as f32 * 12.0).max(24.0);

    (text.pos, Vec2::new(width, 14.0))
}

pub(super) fn text_at(pos: Vec2, text: &LevelText) -> bool {
    let (text_pos, text_size) = text_bounds(text);

    pos.x >= text_pos.x
        && pos.y >= text_pos.y
        && pos.x <= text_pos.x + text_size.x
        && pos.y <= text_pos.y + text_size.y
}

pub(super) fn rect_intersects_rect(a_pos: Vec2, a_size: Vec2, b_pos: Vec2, b_size: Vec2) -> bool {
    let a_max = a_pos + a_size;
    let b_max = b_pos + b_size;

    a_pos.x <= b_max.x && a_max.x >= b_pos.x && a_pos.y <= b_max.y && a_max.y >= b_pos.y
}

pub(super) fn snap(value: Vec2) -> Vec2 {
    Vec2::new(
        (value.x / EDITOR_GRID).round() * EDITOR_GRID,
        (value.y / EDITOR_GRID).round() * EDITOR_GRID,
    )
}

pub(super) fn snap_delta(value: Vec2) -> Vec2 {
    Vec2::new(
        (value.x / EDITOR_GRID).round() * EDITOR_GRID,
        (value.y / EDITOR_GRID).round() * EDITOR_GRID,
    )
}

pub(super) fn snap_angle(value: f32) -> f32 {
    let step = std::f32::consts::FRAC_PI_4 / 4.0;

    (value / step).round() * step
}

fn outside_padded_body(local: Vec2, size: Vec2) -> bool {
    local.x < -BODY_HIT_PADDING
        || local.y < -BODY_HIT_PADDING
        || local.x > size.x + BODY_HIT_PADDING
        || local.y > size.y + BODY_HIT_PADDING
}

fn resize_hit(local: Vec2, size: Vec2) -> Option<SolidHit> {
    let near_left = local.x.abs() <= HANDLE_HIT_SIZE;
    let near_right = (local.x - size.x).abs() <= HANDLE_HIT_SIZE;
    let near_top = local.y.abs() <= HANDLE_HIT_SIZE;
    let near_bottom = (local.y - size.y).abs() <= HANDLE_HIT_SIZE;
    let edge_x = if near_left {
        -1
    } else if near_right {
        1
    } else {
        0
    };
    let edge_y = if near_top {
        -1
    } else if near_bottom {
        1
    } else {
        0
    };

    (edge_x != 0 || edge_y != 0).then_some(SolidHit::Resize { edge_x, edge_y })
}

fn rotate_hit(pos: Vec2, solid: Solid) -> bool {
    let center = solid.center();
    let radius = solid.size.x.max(solid.size.y) / 2.0 + ROTATE_RING_OFFSET;
    let distance = pos.distance(center);
    let rotation = solid.rotation();
    let handle_dir = Vec2::new(rotation.sin(), -rotation.cos());
    let handle = center + handle_dir * radius;

    (distance - radius).abs() <= ROTATE_HIT_SIZE || pos.distance(handle) <= ROTATE_HIT_SIZE
}

mod collision;
mod raycast;

// Level geometry is stored in world space.
use glam::Vec2;

use crate::game::enemy::Enemy;
use crate::game::portal::{Color, Portal};

const RAY_EPSILON: f32 = 0.001;
const PORTAL_CLEARANCE: f32 = 1.0;
const DEFAULT_DOOR_RADIUS: f32 = 112.0;
const DEFAULT_DOOR_SPEED: f32 = 3.6;

#[derive(Clone, Copy, PartialEq)]
pub struct Solid {
    pos: Vec2,
    size: Vec2,
    pub portalable: bool,
    rotation: f32,
    axis_x: Vec2,
    axis_y: Vec2,
    aabb_min: Vec2,
    aabb_max: Vec2,
}

impl Solid {
    pub const fn new(x: f32, y: f32, w: f32, h: f32, portalable: bool) -> Self {
        Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
            rotation: 0.0,
            portalable,
            axis_x: Vec2::X,
            axis_y: Vec2::Y,
            aabb_min: Vec2::new(x, y),
            aabb_max: Vec2::new(x + w, y + h),
        }
    }

    pub fn rotated(x: f32, y: f32, w: f32, h: f32, rotation: f32, portalable: bool) -> Self {
        let mut solid = Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
            rotation: 0.0,
            portalable,
            axis_x: Vec2::X,
            axis_y: Vec2::Y,
            aabb_min: Vec2::new(x, y),
            aabb_max: Vec2::new(x + w, y + h),
        };
        solid.set_rotation(rotation);
        solid
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        let rotation = if rotation.is_finite() { rotation } else { 0.0 };

        self.rotation = rotation;
        let (sin, cos) = rotation.sin_cos();
        self.axis_x = Vec2::new(cos, sin);
        self.axis_y = Vec2::new(-sin, cos);
        self.refresh_bounds();
    }

    pub fn rotation(self) -> f32 {
        self.rotation
    }

    pub fn pos(self) -> Vec2 {
        self.pos
    }

    pub fn size(self) -> Vec2 {
        self.size
    }

    pub fn set_pos(&mut self, pos: Vec2) {
        let offset = pos - self.pos;
        self.pos = pos;
        self.aabb_min += offset;
        self.aabb_max += offset;
    }

    pub fn translate(&mut self, offset: Vec2) {
        self.pos += offset;
        self.aabb_min += offset;
        self.aabb_max += offset;
    }

    pub fn set_centered_rect(&mut self, center: Vec2, size: Vec2) {
        self.size = Vec2::new(size.x.max(1.0), size.y.max(1.0));
        self.pos = center - self.size / 2.0;
        self.refresh_bounds();
    }

    pub fn basis(self) -> (Vec2, Vec2) {
        (self.axis_x, self.axis_y)
    }

    pub fn center(self) -> Vec2 {
        self.pos + self.size / 2.0
    }

    pub fn axis_x(self) -> Vec2 {
        self.axis_x
    }

    pub fn axis_y(self) -> Vec2 {
        self.axis_y
    }

    pub fn local_from_world(self, point: Vec2) -> Vec2 {
        let (axis_x, axis_y) = self.basis();
        let offset = point - self.center();

        Vec2::new(
            offset.dot(axis_x) + self.size.x / 2.0,
            offset.dot(axis_y) + self.size.y / 2.0,
        )
    }

    pub fn world_from_local(self, point: Vec2) -> Vec2 {
        let (axis_x, axis_y) = self.basis();
        let offset = point - self.size / 2.0;

        self.center() + axis_x * offset.x + axis_y * offset.y
    }

    pub fn contains_point(self, point: Vec2) -> bool {
        let local = self.local_from_world(point);

        local.x >= 0.0 && local.y >= 0.0 && local.x <= self.size.x && local.y <= self.size.y
    }

    pub fn corners(self) -> [Vec2; 4] {
        [
            self.world_from_local(Vec2::ZERO),
            self.world_from_local(Vec2::new(self.size.x, 0.0)),
            self.world_from_local(self.size),
            self.world_from_local(Vec2::new(0.0, self.size.y)),
        ]
    }

    pub fn bounds(self) -> (Vec2, Vec2) {
        (self.aabb_min, self.aabb_max)
    }

    pub fn overlaps_body_bounds(self, center: Vec2, half_size: Vec2) -> bool {
        let (solid_min, solid_max) = self.bounds();
        let body_min = center - half_size;
        let body_max = center + half_size;

        solid_min.x < body_max.x
            && solid_max.x > body_min.x
            && solid_min.y < body_max.y
            && solid_max.y > body_min.y
    }

    pub fn overlaps_aabb(self, center: Vec2, half_size: Vec2) -> bool {
        let solid_center = self.center();
        let (axis_x, axis_y) = self.basis();
        let axes = [Vec2::X, Vec2::Y, axis_x, axis_y];

        axes.into_iter().all(|axis| {
            let player_extent = half_size.x * axis.x.abs() + half_size.y * axis.y.abs();
            let solid_axis = Vec2::new(axis.dot(axis_x).abs(), axis.dot(axis_y).abs());
            let solid_extent = self.size.x / 2.0 * solid_axis.x + self.size.y / 2.0 * solid_axis.y;
            let delta = center.dot(axis) - solid_center.dot(axis);

            player_extent + solid_extent - delta.abs() > 0.0
        })
    }

    fn refresh_bounds(&mut self) {
        let half_size = self.size / 2.0;
        let center = self.center();
        let extent = Vec2::new(
            self.axis_x.x.abs() * half_size.x + self.axis_y.x.abs() * half_size.y,
            self.axis_x.y.abs() * half_size.x + self.axis_y.y.abs() * half_size.y,
        );

        self.aabb_min = center - extent;
        self.aabb_max = center + extent;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Door {
    pub solid: Solid,
    pub trigger_radius: f32,
    pub speed: f32,
    pub automatic: bool,
    pub open: f32,
    triggered: bool,
}

impl Door {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
            trigger_radius: DEFAULT_DOOR_RADIUS,
            speed: DEFAULT_DOOR_SPEED,
            automatic: true,
            open: 0.0,
            triggered: false,
        }
    }

    pub fn with_radius(x: f32, y: f32, w: f32, h: f32, trigger_radius: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
            trigger_radius: positive_or(trigger_radius, DEFAULT_DOOR_RADIUS),
            speed: DEFAULT_DOOR_SPEED,
            automatic: true,
            open: 0.0,
            triggered: false,
        }
    }

    pub fn update(&mut self, player_pos: Vec2, dt: f32, mut emit: impl FnMut(DoorEvent)) {
        // Editor data can be malformed; repair it before it reaches movement or rendering.
        self.trigger_radius = positive_or(self.trigger_radius, DEFAULT_DOOR_RADIUS);
        self.speed = positive_or(self.speed, DEFAULT_DOOR_SPEED);
        if !self.open.is_finite() {
            self.open = 0.0;
        }
        self.open = self.open.clamp(0.0, 1.0);

        let triggered = self.automatic
            && player_pos.is_finite()
            && player_pos.distance(self.solid.center()) <= self.trigger_radius;

        if triggered != self.triggered {
            emit(if triggered {
                DoorEvent::Opening
            } else {
                DoorEvent::Closing
            });
        }

        self.triggered = triggered;
        let target = if triggered { 1.0 } else { 0.0 };
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let speed = self.speed * dt;
        let was_open = self.open;

        if self.open < target {
            self.open = (self.open + speed).min(target);
        } else {
            self.open = (self.open - speed).max(target);
        }

        if was_open != self.open {
            if triggered && self.open == 1.0 {
                emit(DoorEvent::Opened);
            } else if !triggered && self.open == 0.0 {
                emit(DoorEvent::Closed);
            }
        }
    }

    pub fn moving_solid(self) -> Solid {
        let mut solid = self.solid;
        let open = if self.open.is_finite() {
            self.open
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let slide = solid.axis_y() * -(solid.size().y + 8.0) * open;

        solid.translate(slide);
        solid
    }

    pub fn blocks_player(self) -> bool {
        !self.open.is_finite() || self.open < 0.92
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DoorEvent {
    Opening,
    Closing,
    Opened,
    Closed,
}

#[derive(Clone, PartialEq)]
pub struct LevelText {
    pub pos: Vec2,
    pub text: String,
}

impl LevelText {
    pub fn new(pos: Vec2, text: impl Into<String>) -> Self {
        Self {
            pos,
            text: text.into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct WorldPortal {
    pub portal: Portal,
    pub id: u16,
    pub receiver_id: u16,
    pub priority: i16,
    pub seamless: bool,
    pub seamless_depth: f32,
    pub seamless_angle: f32,
    pub seamless_rely_on_walls: bool,
}

impl WorldPortal {
    pub fn new(x: f32, y: f32, normal: Vec2, width: f32, id: u16) -> Self {
        Self {
            portal: Portal::new(x, y, normal, width, Color::rgb(154, 120, 255)),
            id,
            receiver_id: id,
            priority: 0,
            seamless: false,
            seamless_depth: 256.0,
            seamless_angle: 180.0,
            seamless_rely_on_walls: false,
        }
    }

    pub fn center(self) -> Vec2 {
        self.portal.pos
    }

    pub fn set_center(&mut self, center: Vec2) {
        self.portal.pos = center;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Hazard {
    pub solid: Solid,
}

impl Hazard {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Checkpoint {
    pub solid: Solid,
}

impl Checkpoint {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
        }
    }

    pub fn solid(self) -> Solid {
        self.solid
    }

    pub fn center(self) -> Vec2 {
        self.solid.center()
    }
}

#[derive(Clone)]
pub struct Level {
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
    pub hazards: Vec<Hazard>,
    pub checkpoints: Vec<Checkpoint>,
    pub enemies: Vec<Enemy>,
    pub texts: Vec<LevelText>,
    pub world_portals: Vec<WorldPortal>,
}

#[derive(Clone, Copy)]
pub struct CollisionGeometry<'a> {
    pub(super) solids: &'a [Solid],
    pub(super) doors: &'a [Door],
}

impl<'a> CollisionGeometry<'a> {
    pub fn new(solids: &'a [Solid], doors: &'a [Door]) -> Self {
        Self { solids, doors }
    }
}

#[derive(Clone, Copy)]
pub struct RayHit {
    pub point: Vec2,
    pub normal: Vec2,
    pub tangent: Vec2,
    pub solid_index: usize,
    pub surface_coord: f32,
    pub surface_max: f32,
    pub surface_min: f32,
    // Span of the hit surface along the portal line.
    pub surface_span: f32,
}

impl RayHit {
    pub fn portal_center(self, portal_width: f32) -> Option<Vec2> {
        // Invalid spans used to reach f32::clamp with inverted bounds and panic.
        if !portal_width.is_finite()
            || portal_width <= 0.0
            || !self.surface_span.is_finite()
            || !self.surface_min.is_finite()
            || !self.surface_max.is_finite()
            || self.surface_max < self.surface_min
            || self.surface_span < portal_width
        {
            return None;
        }

        let mut center = self.point;
        let half_width = portal_width / 2.0;
        let min_center = self.surface_min + half_width;
        let max_center = self.surface_max - half_width;
        if min_center > max_center {
            return None;
        }
        // Clamp along the surface, not away from it.
        let clamped = self.surface_coord.clamp(min_center, max_center);
        center += self.tangent * (clamped - self.surface_coord);

        Some(center)
    }
}

impl Level {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            solids: Vec::new(),
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: Vec::new(),
            texts: Vec::new(),
            world_portals: Vec::new(),
        }
    }

    pub fn test_level() -> Self {
        Self {
            solids: vec![
                Solid::new(0.0, 560.0, 900.0, 40.0, true),
                Solid::new(0.0, 0.0, 30.0, 600.0, true),
                Solid::new(870.0, 0.0, 30.0, 600.0, true),
                Solid::new(0.0, 0.0, 900.0, 30.0, true),
                Solid::new(170.0, 430.0, 210.0, 28.0, true),
                Solid::new(520.0, 390.0, 220.0, 28.0, true),
                Solid::new(430.0, 250.0, 32.0, 170.0, true),
                Solid::new(250.0, 150.0, 180.0, 26.0, true),
                Solid::new(610.0, 160.0, 32.0, 160.0, true),
            ],
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            enemies: Vec::new(),
            texts: Vec::new(),
            world_portals: Vec::new(),
        }
    }

    pub fn update_doors(
        &mut self,
        player_pos: Vec2,
        dt: f32,
        events: &mut Vec<(usize, DoorEvent)>,
    ) {
        for (index, door) in self.doors.iter_mut().enumerate() {
            door.update(player_pos, dt, |event| events.push((index, event)));
        }
    }
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hit_rejects_invalid_portal_width_without_clamping() {
        let hit = RayHit {
            point: Vec2::ZERO,
            normal: Vec2::Y,
            tangent: Vec2::X,
            solid_index: 0,
            surface_coord: 16.0,
            surface_min: 0.0,
            surface_max: 32.0,
            surface_span: 32.0,
        };

        assert_eq!(hit.portal_center(0.0), None);
        assert_eq!(hit.portal_center(f32::NAN), None);
        assert_eq!(hit.portal_center(64.0), None);
    }

    #[test]
    fn solid_bounds_track_rotation_translation_and_resize() {
        let mut solid = Solid::rotated(10.0, 20.0, 80.0, 24.0, std::f32::consts::FRAC_PI_4, true);

        assert_bounds_contain_corners(solid);

        solid.translate(Vec2::new(30.0, -12.0));
        assert_bounds_contain_corners(solid);

        solid.set_centered_rect(Vec2::new(100.0, 160.0), Vec2::new(120.0, 36.0));
        solid.set_rotation(std::f32::consts::FRAC_PI_6);
        assert_bounds_contain_corners(solid);
    }

    fn assert_bounds_contain_corners(solid: Solid) {
        let (min, max) = solid.bounds();

        for corner in solid.corners() {
            assert!(corner.x >= min.x - RAY_EPSILON);
            assert!(corner.x <= max.x + RAY_EPSILON);
            assert!(corner.y >= min.y - RAY_EPSILON);
            assert!(corner.y <= max.y + RAY_EPSILON);
        }
    }
}

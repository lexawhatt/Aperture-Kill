mod collision;
mod raycast;

// Level geometry is stored in world space.
use glam::Vec2;

const RAY_EPSILON: f32 = 0.001;
const PORTAL_CLEARANCE: f32 = 1.0;

#[derive(Clone, Copy, PartialEq)]
pub struct Solid {
    pub pos: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    pub portalable: bool,
}

impl Solid {
    pub const fn new(x: f32, y: f32, w: f32, h: f32, portalable: bool) -> Self {
        Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
            rotation: 0.0,
            portalable,
        }
    }

    pub const fn rotated(x: f32, y: f32, w: f32, h: f32, rotation: f32, portalable: bool) -> Self {
        Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
            rotation,
            portalable,
        }
    }

    pub fn center(self) -> Vec2 {
        self.pos + self.size / 2.0
    }

    pub fn axis_x(self) -> Vec2 {
        Vec2::new(self.rotation.cos(), self.rotation.sin())
    }

    pub fn axis_y(self) -> Vec2 {
        Vec2::new(-self.rotation.sin(), self.rotation.cos())
    }

    pub fn local_from_world(self, point: Vec2) -> Vec2 {
        let offset = point - self.center();

        Vec2::new(
            offset.dot(self.axis_x()) + self.size.x / 2.0,
            offset.dot(self.axis_y()) + self.size.y / 2.0,
        )
    }

    pub fn world_from_local(self, point: Vec2) -> Vec2 {
        let offset = point - self.size / 2.0;

        self.center() + self.axis_x() * offset.x + self.axis_y() * offset.y
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

    pub fn overlaps_aabb(self, center: Vec2, half_size: Vec2) -> bool {
        let axes = [Vec2::X, Vec2::Y, self.axis_x(), self.axis_y()];

        axes.into_iter().all(|axis| {
            let player_extent = half_size.x * axis.x.abs() + half_size.y * axis.y.abs();
            let solid_axis =
                Vec2::new(axis.dot(self.axis_x()).abs(), axis.dot(self.axis_y()).abs());
            let solid_extent = self.size.x / 2.0 * solid_axis.x + self.size.y / 2.0 * solid_axis.y;
            let delta = center.dot(axis) - self.center().dot(axis);

            player_extent + solid_extent - delta.abs() > 0.0
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Door {
    pub solid: Solid,
    pub trigger_radius: f32,
    pub open: f32,
    triggered: bool,
}

impl Door {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
            trigger_radius: 112.0,
            open: 0.0,
            triggered: false,
        }
    }

    pub fn with_radius(x: f32, y: f32, w: f32, h: f32, trigger_radius: f32) -> Self {
        Self {
            solid: Solid::new(x, y, w, h, false),
            trigger_radius,
            open: 0.0,
            triggered: false,
        }
    }

    pub fn update(&mut self, player_pos: Vec2, dt: f32) -> Vec<DoorEvent> {
        let triggered = player_pos.distance(self.solid.center()) <= self.trigger_radius;
        let mut events = Vec::new();

        if triggered != self.triggered {
            events.push(if triggered {
                DoorEvent::Opening
            } else {
                DoorEvent::Closing
            });
        }

        self.triggered = triggered;
        let target = if triggered { 1.0 } else { 0.0 };
        let speed = 3.6 * dt;
        let was_open = self.open;

        if self.open < target {
            self.open = (self.open + speed).min(target);
        } else {
            self.open = (self.open - speed).max(target);
        }

        if was_open != self.open {
            if triggered && self.open == 1.0 {
                events.push(DoorEvent::Opened);
            } else if !triggered && self.open == 0.0 {
                events.push(DoorEvent::Closed);
            }
        }

        events
    }

    pub fn moving_solid(self) -> Solid {
        let mut solid = self.solid;
        let slide = solid.axis_y() * -(solid.size.y + 8.0) * self.open;

        solid.pos += slide;
        solid
    }

    pub fn blocks_player(self) -> bool {
        self.open < 0.92
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
    pub texts: Vec<LevelText>,
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
        if self.surface_span < portal_width {
            return None;
        }

        let mut center = self.point;
        let half_width = portal_width / 2.0;
        // Clamp along the surface, not away from it.
        let clamped = self
            .surface_coord
            .clamp(self.surface_min + half_width, self.surface_max - half_width);
        center += self.tangent * (clamped - self.surface_coord);

        Some(center)
    }
}

impl Level {
    pub fn empty() -> Self {
        Self {
            solids: Vec::new(),
            doors: Vec::new(),
            hazards: Vec::new(),
            checkpoints: Vec::new(),
            texts: Vec::new(),
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
            texts: Vec::new(),
        }
    }

    pub fn update_doors(&mut self, player_pos: Vec2, dt: f32) -> Vec<(usize, DoorEvent)> {
        let mut events = Vec::new();

        for (index, door) in self.doors.iter_mut().enumerate() {
            events.extend(
                door.update(player_pos, dt)
                    .into_iter()
                    .map(|event| (index, event)),
            );
        }

        events
    }
}

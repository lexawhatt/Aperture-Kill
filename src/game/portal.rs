use glam::Vec2;

// Portal maps positions and velocity through local tangent/normal axes.
use crate::game::player::Player;

const CROSSING_EPSILON: f32 = 0.001;
const EXIT_MARGIN: f32 = 1.0;

#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLUE: Self = Self {
        r: 54,
        g: 139,
        b: 255,
    };
    pub const ORANGE: Self = Self {
        r: 255,
        g: 151,
        b: 42,
    };
}

#[derive(Clone, Copy, PartialEq)]
pub struct Portal {
    pub pos: Vec2,
    pub normal: Vec2,
    pub tangent: Vec2,
    pub width: f32,
    pub scale: f32,
    pub scale_objects: bool,
    pub color: Color,
}

impl Portal {
    pub fn new(x: f32, y: f32, normal: Vec2, width: f32, color: Color) -> Self {
        let normal = normal.normalize();

        Self::with_tangent(x, y, normal, Vec2::new(-normal.y, normal.x), width, color)
    }

    pub fn with_tangent(
        x: f32,
        y: f32,
        normal: Vec2,
        tangent: Vec2,
        width: f32,
        color: Color,
    ) -> Self {
        let normal = normal.normalize();
        let tangent = (tangent - normal * tangent.dot(normal)).normalize();

        Self {
            pos: Vec2::new(x, y),
            normal,
            tangent,
            width,
            scale: 1.0,
            scale_objects: false,
            color,
        }
    }

    pub fn endpoints(&self) -> (Vec2, Vec2) {
        let half_width = self.active_width() / 2.0;

        (
            self.pos + self.tangent() * half_width,
            self.pos - self.tangent() * half_width,
        )
    }

    pub fn crossing_time(
        &self,
        previous_center: Vec2,
        current_center: Vec2,
        half_size: Vec2,
    ) -> Option<f32> {
        let previous = previous_center - self.pos;
        let current = current_center - self.pos;
        let tangent_extent = projected_extent(half_size, self.tangent());
        let previous_distance = previous.dot(self.normal);
        let current_distance = current.dot(self.normal);
        let distance_delta = current_distance - previous_distance;

        if distance_delta.abs() < CROSSING_EPSILON
            || previous_distance.abs() < CROSSING_EPSILON
            || previous_distance.signum() == current_distance.signum()
        {
            return None;
        }

        let time = -previous_distance / distance_delta;
        if !(0.0..=1.0).contains(&time) {
            return None;
        }

        let crossing_center = previous_center.lerp(current_center, time) - self.pos;
        let crossing_tangent = crossing_center.dot(self.tangent());
        let half_width = self.active_width() / 2.0;
        if crossing_tangent.abs() > half_width + tangent_extent {
            return None;
        }

        Some(time)
    }

    pub fn teleport_player_to(&self, destination: &Portal, player: &mut Player) {
        let scale = self.object_scale_to(destination);
        let destination_tangent = destination.aligned_tangent(self);
        let mut previous =
            self.map_point_to(destination, player.prev_pos, scale, destination_tangent);
        let mut current = self.map_point_to(destination, player.pos, scale, destination_tangent);
        let mapped_size = player.size * scale;
        let min_exit_distance =
            projected_extent(mapped_size / 2.0, destination.normal()) + EXIT_MARGIN;
        let current_distance = (current - destination.pos).dot(destination.normal());

        // Keep the full hitbox in front of the exit so collision does not push it behind the wall.
        if current_distance < min_exit_distance {
            let push = destination.normal() * (min_exit_distance - current_distance);

            previous += push;
            current += push;
        }

        player.size *= scale;
        player.prev_pos = previous;
        player.pos = current;
        player.vel = transform_velocity(player.vel, self, destination, destination_tangent, scale);
    }

    pub fn map_body_to(&self, destination: &Portal, center: Vec2, size: Vec2) -> (Vec2, Vec2) {
        let scale = self.object_scale_to(destination);
        let destination_tangent = destination.aligned_tangent(self);
        let half_size = size / 2.0;
        let tangent_extent = projected_extent(half_size, self.tangent()) * scale;
        let normal_extent = projected_extent(half_size, self.normal) * scale;
        let mapped_half_size =
            destination_tangent.abs() * tangent_extent + destination.normal.abs() * normal_extent;

        (
            self.map_point_to(destination, center, scale, destination_tangent),
            mapped_half_size * 2.0,
        )
    }

    pub fn opens_for_body(&self, center: Vec2, half_size: Vec2) -> bool {
        let offset = center - self.pos;
        let tangent_extent = projected_extent(half_size, self.tangent());
        let normal_extent = projected_extent(half_size, self.normal);
        let half_width = self.active_width() / 2.0;

        offset.dot(self.tangent()).abs() <= half_width + tangent_extent
            && offset.dot(self.normal()).abs() <= normal_extent
    }

    pub fn normal(&self) -> Vec2 {
        self.normal
    }

    pub fn tangent(&self) -> Vec2 {
        self.tangent
    }

    pub fn active_width(&self) -> f32 {
        self.width * self.scale
    }

    fn map_point_to(
        &self,
        destination: &Portal,
        point: Vec2,
        scale: f32,
        destination_tangent: Vec2,
    ) -> Vec2 {
        let source_offset = point - self.pos;
        let tangent_offset = source_offset.dot(self.tangent()) * scale;
        let normal_offset = source_offset.dot(self.normal) * scale;

        destination.pos + destination_tangent * tangent_offset - destination.normal * normal_offset
    }

    fn aligned_tangent(&self, source: &Portal) -> Vec2 {
        let tangent = self.tangent();
        if source.tangent().dot(tangent) < 0.0 {
            -tangent
        } else {
            tangent
        }
    }

    fn object_scale_to(&self, destination: &Portal) -> f32 {
        if self.scale_objects && destination.scale_objects {
            destination.scale / self.scale
        } else {
            1.0
        }
    }
}

fn projected_extent(half_size: Vec2, axis: Vec2) -> f32 {
    half_size.x * axis.x.abs() + half_size.y * axis.y.abs()
}

fn transform_velocity(
    velocity: Vec2,
    source: &Portal,
    destination: &Portal,
    destination_tangent: Vec2,
    scale: f32,
) -> Vec2 {
    // Convert velocity into portal-local axes, then rebuild it at the exit.
    let tangent_speed = velocity.dot(source.tangent());
    let normal_speed = velocity.dot(source.normal);

    destination_tangent * (tangent_speed * scale) + destination.normal * (-normal_speed * scale)
}

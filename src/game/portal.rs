use glam::Vec2;

// Portal maps positions and velocity through local tangent/normal axes.
use crate::game::geometry::projected_extent;
use crate::game::player::Player;

const CROSSING_EPSILON: f32 = 0.001;
const EXIT_MARGIN: f32 = 1.0;
const MIN_PORTAL_DIMENSION: f32 = 1.0;

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

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Portal {
    pub pos: Vec2,
    pub width: f32,
    pub scale: f32,
    pub scale_objects: bool,
    pub color: Color,
    normal: Vec2,
    tangent: Vec2,
}

impl Portal {
    pub fn new(x: f32, y: f32, normal: Vec2, width: f32, color: Color) -> Self {
        let normal = normalized_or(normal, Vec2::new(0.0, -1.0));

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
        // Saved levels may contain damaged portal vectors; keep a usable orthonormal basis.
        let normal = normalized_or(normal, Vec2::new(0.0, -1.0));
        let tangent = normalized_or(
            tangent - normal * tangent.dot(normal),
            Vec2::new(-normal.y, normal.x),
        );

        Self {
            pos: Vec2::new(finite_or(x, 0.0), finite_or(y, 0.0)),
            normal,
            tangent,
            width: positive_or(width, MIN_PORTAL_DIMENSION),
            scale: 1.0,
            scale_objects: false,
            color,
        }
    }

    pub fn endpoints(&self) -> (Vec2, Vec2) {
        let half_width = self.active_width() / 2.0;

        (
            self.pos + self.tangent * half_width,
            self.pos - self.tangent * half_width,
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
        let tangent_extent = projected_extent(half_size, self.tangent);
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
        let crossing_tangent = crossing_center.dot(self.tangent);
        let half_width = self.active_width() / 2.0;
        if crossing_tangent.abs() > half_width + tangent_extent {
            return None;
        }

        Some(time)
    }

    pub fn teleport_player_to(&self, destination: &Portal, player: &mut Player) {
        let scale = self.object_scale_to(destination);
        let destination_pos = destination.pos;
        let destination_normal = destination.normal;
        let destination_tangent = destination.oriented_tangent(self);
        let mut previous =
            self.map_point_to(destination, player.prev_pos, scale, destination_tangent);
        let mut current = self.map_point_to(destination, player.pos, scale, destination_tangent);
        let mapped_size = player.size * scale;
        let min_exit_distance =
            projected_extent(mapped_size / 2.0, destination_normal) + EXIT_MARGIN;
        let current_distance = (current - destination_pos).dot(destination_normal);

        // Keep the full hitbox in front of the exit so collision does not push it behind the wall.
        if current_distance < min_exit_distance {
            let push = destination_normal * (min_exit_distance - current_distance);

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
        let destination_tangent = destination.oriented_tangent(self);
        let half_size = size / 2.0;
        let tangent_extent = projected_extent(half_size, self.tangent) * scale;
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
        let tangent_extent = projected_extent(half_size, self.tangent);
        let normal_extent = projected_extent(half_size, self.normal);
        let half_width = self.active_width() / 2.0;

        offset.dot(self.tangent).abs() <= half_width + tangent_extent
            && offset.dot(self.normal).abs() <= normal_extent
    }

    pub fn normal(&self) -> Vec2 {
        self.normal
    }

    pub fn tangent(&self) -> Vec2 {
        self.tangent
    }

    pub fn active_width(&self) -> f32 {
        positive_or(self.width, MIN_PORTAL_DIMENSION) * positive_or(self.scale, 1.0)
    }

    fn uses_scale(&self) -> bool {
        self.scale_objects
    }

    fn map_point_to(
        &self,
        destination: &Portal,
        point: Vec2,
        scale: f32,
        destination_tangent: Vec2,
    ) -> Vec2 {
        let source_offset = point - self.pos;
        let tangent_offset = source_offset.dot(self.tangent) * scale;
        let normal_offset = source_offset.dot(self.normal) * scale;

        destination.pos + destination_tangent * tangent_offset - destination.normal * normal_offset
    }

    fn oriented_tangent(&self, source: &Portal) -> Vec2 {
        let tangent = self.tangent;
        if source.tangent.dot(tangent) < 0.0 {
            -tangent
        } else {
            tangent
        }
    }

    fn object_scale_to(&self, destination: &Portal) -> f32 {
        if self.uses_scale() && destination.uses_scale() {
            positive_or(destination.scale, 1.0) / positive_or(self.scale, 1.0)
        } else {
            1.0
        }
    }
}

fn normalized_or(value: Vec2, fallback: Vec2) -> Vec2 {
    // normalize() propagates zero vectors and NaN through every portal transform.
    if value.is_finite() && value.length_squared() > CROSSING_EPSILON * CROSSING_EPSILON {
        value.normalize()
    } else {
        fallback
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn transform_velocity(
    velocity: Vec2,
    source: &Portal,
    destination: &Portal,
    destination_tangent: Vec2,
    scale: f32,
) -> Vec2 {
    // Convert velocity into portal-local axes, then rebuild it at the exit.
    let tangent_speed = velocity.dot(source.tangent);
    let normal_speed = velocity.dot(source.normal);

    destination_tangent * (tangent_speed * scale) + destination.normal * (-normal_speed * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_sanitizes_invalid_geometry() {
        let portal = Portal::with_tangent(
            f32::NAN,
            f32::INFINITY,
            Vec2::ZERO,
            Vec2::ZERO,
            f32::NAN,
            Color::BLUE,
        );

        assert!(portal.pos.is_finite());
        assert!(portal.normal().is_finite());
        assert!(portal.tangent().is_finite());
        assert!(portal.active_width().is_finite());
        assert!(portal.active_width() > 0.0);
    }
}

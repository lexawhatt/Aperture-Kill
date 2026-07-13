use glam::Vec2;

use crate::game::player::Player;

const EXIT_PADDING: f32 = 2.0;

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
    pub width: f32,
    pub scale: f32,
    pub scale_objects: bool,
    pub color: Color,
}

impl Portal {
    pub fn new(x: f32, y: f32, normal: Vec2, width: f32, color: Color) -> Self {
        Self {
            pos: Vec2::new(x, y),
            normal: normal.normalize(),
            width,
            scale: 1.0,
            scale_objects: true,
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

    pub fn intersects_sweep(
        &self,
        previous_center: Vec2,
        current_center: Vec2,
        half_size: Vec2,
    ) -> bool {
        let previous = previous_center - self.pos;
        let current = current_center - self.pos;
        let tangent_extent = projected_extent(half_size, self.tangent());
        let normal_extent = projected_extent(half_size, self.normal);

        // First reject objects that missed the portal span.
        let current_tangent = current.dot(self.tangent());
        let half_width = self.active_width() / 2.0;
        if current_tangent.abs() > half_width + tangent_extent {
            return false;
        }

        let previous_distance = previous.dot(self.normal);
        let current_distance = current.dot(self.normal);

        // Trigger only when the object enters through the front face.
        previous_distance > normal_extent && current_distance <= normal_extent
    }

    pub fn teleport_player_to(&self, destination: &Portal, player: &mut Player) {
        let scale = self.object_scale_to(destination);
        let source_tangent = self.tangent();
        let destination_tangent = destination.aligned_tangent(self);

        let source_offset = player.pos - self.pos;
        let tangent_offset = source_offset.dot(source_tangent) * scale;
        let exit_distance =
            projected_extent(player.half_size() * scale, destination.normal) + EXIT_PADDING;

        player.size *= scale;
        player.pos = destination.pos
            + destination_tangent * tangent_offset
            + destination.normal * exit_distance;
        player.prev_pos = player.pos;
        player.vel = transform_velocity(player.vel, self, destination, destination_tangent, scale);
    }

    fn active_width(&self) -> f32 {
        self.width * self.scale
    }

    fn aligned_tangent(&self, source: &Portal) -> Vec2 {
        let tangent = self.tangent();
        if source.normal.dot(self.normal) < -0.9 {
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

    fn tangent(&self) -> Vec2 {
        Vec2::new(-self.normal.y, self.normal.x)
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
    let tangent_speed = velocity.dot(source.tangent());
    let normal_speed = velocity.dot(source.normal);

    destination_tangent * (tangent_speed * scale) + destination.normal * (-normal_speed * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn portal() -> Portal {
        Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE)
    }

    #[test]
    fn sweep_hits_when_player_enters_front_face() {
        let portal = portal();
        let half_size = Vec2::new(10.0, 20.0);

        assert!(portal.intersects_sweep(Vec2::new(79.9, 50.0), Vec2::new(90.1, 50.0), half_size));
    }

    #[test]
    fn sweep_ignores_objects_outside_portal_width() {
        let portal = portal();
        let half_size = Vec2::new(10.0, 20.0);

        assert!(!portal.intersects_sweep(
            Vec2::new(79.9, 120.0),
            Vec2::new(90.1, 120.0),
            half_size
        ));
    }

    #[test]
    fn teleport_preserves_velocity_in_destination_space() {
        let source = Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE);
        let destination = Portal::new(20.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
        let mut player = Player::new(90.0, 50.0);

        player.vel = Vec2::new(100.0, 25.0);
        source.teleport_player_to(&destination, &mut player);

        assert!(player.pos.x > destination.pos.x);
        assert_eq!(player.vel, Vec2::new(100.0, 25.0));
    }
}

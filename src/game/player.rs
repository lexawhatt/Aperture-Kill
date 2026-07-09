use glam::Vec2;

use crate::constants::{GRAVITY, PLAYER_SPEED};

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub aim_pos: Vec2,
    pub on_ground: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        let pos = Vec2::new(x, y);

        Self {
            pos,
            vel: Vec2::ZERO,
            size: Vec2::new(34.0, 72.0),
            aim_pos: pos,
            on_ground: false,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        move_x: f32,
        aim_pos: Vec2,
        screen_width: f32,
        screen_height: f32,
    ) {
        // Only Horizontal movement for now
        self.vel.x = move_x * PLAYER_SPEED;
        self.vel.y += GRAVITY * dt;

        self.pos += self.vel * dt;
        self.aim_pos = aim_pos;

        let half = self.half_size();
        self.pos.x = self.pos.x.clamp(half.x, screen_width - half.x);

        let floor_y = screen_height - half.y;
        if self.pos.y > floor_y {
            self.pos.y = floor_y;
            self.vel.y = 0.0;
            self.on_ground = true;
        } else {
            self.on_ground = false;
        }
    }

    pub fn half_size(&self) -> Vec2 {
        self.size / 2.0
    }

    pub fn aim_from(&self) -> Vec2 {
        // ray => center
        self.pos + Vec2::new(0.0, -self.size.y * 0.18)
    }
}

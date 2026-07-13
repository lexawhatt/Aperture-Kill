use glam::Vec2;

use crate::constants::{
    AIR_ACCEL, AIR_DRAG, DASH_COOLDOWN, DASH_DURATION, DASH_SPEED, GRAVITY, GROUND_ACCEL,
    GROUND_FRICTION, JUMP_BUFFER_TIME, JUMP_COYOTE_TIME, JUMP_VELOCITY, PLAYER_SIZE, PLAYER_SPEED,
    SLIDE_BOOST, SLIDE_FRICTION,
};
use crate::platform::input::Input;

pub struct Player {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub aim_pos: Vec2,
    pub on_ground: bool,
    pub sliding: bool,
    pub dashing: bool,
    coyote_timer: f32,
    dash_cooldown: f32,
    dash_dir: Vec2,
    dash_timer: f32,
    facing: f32,
    jump_buffer: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        let pos = Vec2::new(x, y);

        Self {
            pos,
            prev_pos: pos,
            vel: Vec2::ZERO,
            size: Vec2::new(PLAYER_SIZE.0, PLAYER_SIZE.1),
            aim_pos: pos,
            on_ground: false,
            sliding: false,
            dashing: false,
            coyote_timer: 0.0,
            dash_cooldown: 0.0,
            dash_dir: Vec2::X,
            dash_timer: 0.0,
            facing: 1.0,
            jump_buffer: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.prev_pos = self.pos;
        self.aim_pos = input.aim_pos;

        self.tick_timers(dt);
        self.track_jump_window(input);
        self.start_dash(input);
        self.apply_movement(dt, input);
        self.pos += self.vel * dt;

        self.constrain_to_screen(screen_width, screen_height);
    }

    pub fn constrain_to_screen(&mut self, screen_width: f32, screen_height: f32) {
        let half = self.half_size();
        self.pos.x = self.pos.x.clamp(half.x, screen_width - half.x);

        // The floor is the only solid surface until map geometry exists.
        let floor_y = screen_height - half.y;
        if self.pos.y > floor_y {
            self.pos.y = floor_y;
            self.vel.y = 0.0;
            self.on_ground = true;
            self.coyote_timer = JUMP_COYOTE_TIME;
        } else {
            self.on_ground = false;
        }
    }

    pub fn half_size(&self) -> Vec2 {
        self.size / 2.0
    }

    pub fn aim_from(&self) -> Vec2 {
        self.pos + Vec2::new(0.0, -self.size.y * 0.18)
    }

    fn tick_timers(&mut self, dt: f32) {
        self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        self.coyote_timer = (self.coyote_timer - dt).max(0.0);
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.0);
        self.dash_timer = (self.dash_timer - dt).max(0.0);
        self.dashing = self.dash_timer > 0.0;
    }

    fn track_jump_window(&mut self, input: &Input) {
        if input.move_x != 0.0 {
            self.facing = input.move_x.signum();
        }
        if input.jump_pressed {
            self.jump_buffer = JUMP_BUFFER_TIME;
        }
    }

    fn start_dash(&mut self, input: &Input) {
        if !input.dash_pressed || self.dash_cooldown > 0.0 {
            return;
        }

        self.dash_dir = self.input_direction(input);
        self.dash_timer = DASH_DURATION;
        self.dash_cooldown = DASH_COOLDOWN;
        self.dashing = true;
        self.vel = self.dash_dir * DASH_SPEED;
    }

    fn apply_movement(&mut self, dt: f32, input: &Input) {
        if self.dashing {
            self.vel = self.dash_dir * DASH_SPEED;
            return;
        }

        self.try_jump();
        self.sliding = input.slide_down && self.on_ground && self.vel.x.abs() > PLAYER_SPEED * 0.35;
        if input.slide_pressed && self.on_ground {
            let slide_dir = if self.vel.x.abs() > 1.0 {
                self.vel.x.signum()
            } else {
                self.facing
            };
            self.vel.x = slide_dir * SLIDE_BOOST;
            self.sliding = true;
        }

        let accel = if self.on_ground {
            GROUND_ACCEL
        } else {
            AIR_ACCEL
        };
        self.vel.x = approach(self.vel.x, input.move_x * PLAYER_SPEED, accel * dt);

        if self.sliding {
            self.vel.x = approach(self.vel.x, 0.0, SLIDE_FRICTION * dt);
        } else if self.on_ground && input.move_x == 0.0 {
            self.vel.x = approach(self.vel.x, 0.0, GROUND_FRICTION * dt);
        } else if !self.on_ground {
            self.vel.x *= 1.0 - AIR_DRAG * dt;
        }

        self.vel.y += GRAVITY * dt;
    }

    fn try_jump(&mut self) {
        if self.jump_buffer == 0.0 || self.coyote_timer == 0.0 {
            return;
        }

        self.vel.y = -JUMP_VELOCITY;
        self.on_ground = false;
        self.sliding = false;
        self.jump_buffer = 0.0;
        self.coyote_timer = 0.0;
    }

    fn input_direction(&self, input: &Input) -> Vec2 {
        let aim = input.aim_pos - self.aim_from();
        if aim.length_squared() > 1.0 {
            return aim.normalize();
        }

        Vec2::new(self.facing, 0.0)
    }
}

fn approach(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        (current + delta).min(target)
    } else {
        (current - delta).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::input::GameKey;

    #[test]
    fn jump_uses_buffered_press_on_ground() {
        let mut player = Player::new(100.0, 100.0);
        let mut input = Input::new();

        player.on_ground = true;
        player.coyote_timer = JUMP_COYOTE_TIME;
        input.set_key(GameKey::Jump, true);
        input.update();

        player.update(1.0 / 60.0, &input, 900.0, 600.0);

        assert!(player.vel.y < 0.0);
        assert!(!player.on_ground);
    }

    #[test]
    fn dash_follows_aim_direction() {
        let mut player = Player::new(100.0, 100.0);
        let mut input = Input::new();

        input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
        input.set_key(GameKey::Dash, true);
        input.update();

        player.update(1.0 / 60.0, &input, 900.0, 600.0);

        assert!(player.dashing);
        assert!(player.vel.x > DASH_SPEED * 0.9);
        assert!(player.vel.y.abs() < 1.0);
    }

    #[test]
    fn slide_applies_ground_boost() {
        let mut player = Player::new(100.0, 100.0);
        let mut input = Input::new();

        player.on_ground = true;
        player.vel.x = PLAYER_SPEED;
        input.set_key(GameKey::Slide, true);
        input.update();

        player.update(1.0 / 60.0, &input, 900.0, 600.0);

        assert!(player.sliding);
        assert!(player.vel.x > PLAYER_SPEED);
    }
}

use glam::Vec2;

use crate::constants::{
    AIR_ACCEL, AIR_DRAG, DASH_CHARGE_RECOVERY, DASH_DURATION, DASH_SPEED, GRAVITY, GROUND_ACCEL,
    GROUND_FRICTION, JUMP_BUFFER_TIME, JUMP_COYOTE_TIME, JUMP_VELOCITY, MAX_DASH_CHARGES,
    MAX_WALL_JUMPS, PLAYER_SIZE, PLAYER_SPEED, SLIDE_BOOST, SLIDE_FRICTION, SLIDE_HEIGHT_SCALE,
    WALL_GRAVITY_RAMP_TIME, WALL_JUMP_X, WALL_JUMP_Y, WALL_MIN_GRAVITY_SCALE, WALL_STICK_TIME,
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
    pub dash_charges: f32,
    pub wall_sliding: bool,
    coyote_timer: f32,
    dash_dir: Vec2,
    dash_timer: f32,
    facing: f32,
    jump_buffer: f32,
    slide_dir: f32,
    wall_dir: f32,
    wall_gravity_timer: f32,
    wall_jumps_left: u8,
    wall_timer: f32,
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
            dash_charges: MAX_DASH_CHARGES,
            wall_sliding: false,
            coyote_timer: 0.0,
            dash_dir: Vec2::X,
            dash_timer: 0.0,
            facing: 1.0,
            jump_buffer: 0.0,
            slide_dir: 1.0,
            wall_dir: 0.0,
            wall_gravity_timer: 0.0,
            wall_jumps_left: MAX_WALL_JUMPS,
            wall_timer: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.prev_pos = self.pos;
        self.aim_pos = input.aim_pos;

        // Update intent-driven state before integrating position.
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
            self.land();
        } else {
            self.on_ground = false;
        }
    }

    pub fn clear_contacts(&mut self) {
        self.on_ground = false;
        self.wall_sliding = false;
        self.wall_dir = 0.0;
    }

    pub fn land(&mut self) {
        self.vel.y = 0.0;
        self.on_ground = true;
        self.coyote_timer = JUMP_COYOTE_TIME;
        self.wall_jumps_left = MAX_WALL_JUMPS;
        self.wall_sliding = false;
    }

    pub fn set_wall_contact(&mut self, wall_dir: f32) {
        if self.wall_timer == 0.0 {
            self.wall_gravity_timer = WALL_GRAVITY_RAMP_TIME;
        }
        self.wall_dir = wall_dir;
        self.wall_timer = WALL_STICK_TIME;
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
        self.dash_timer = (self.dash_timer - dt).max(0.0);
        self.wall_gravity_timer = (self.wall_gravity_timer - dt).max(0.0);
        self.wall_timer = (self.wall_timer - dt).max(0.0);
        self.dashing = self.dash_timer > 0.0;
    }

    fn track_jump_window(&mut self, input: &Input) {
        // Facing is used when dash/slide has no stronger direction.
        if input.move_x != 0.0 {
            self.facing = input.move_x.signum();
        }

        // Jump buffer accepts slightly early jump inputs.
        if input.jump_pressed {
            self.jump_buffer = JUMP_BUFFER_TIME;
        }
    }

    fn start_dash(&mut self, input: &Input) {
        if !input.dash_pressed || self.dash_charges < 1.0 {
            return;
        }

        // Dash uses aim direction, falling back to facing direction.
        self.dash_charges -= 1.0;
        self.dash_dir = self.input_direction(input);
        self.dash_timer = DASH_DURATION;
        self.dashing = true;
        self.vel = self.dash_dir * DASH_SPEED;
    }

    fn apply_movement(&mut self, dt: f32, input: &Input) {
        if self.dashing {
            self.recover_dash(dt);
            self.vel = self.dash_dir * DASH_SPEED;
            return;
        }

        self.try_jump();
        self.update_slide(input);
        self.recover_dash(dt);

        if self.sliding {
            self.vel.x = approach(self.vel.x, self.slide_dir * SLIDE_BOOST, GROUND_ACCEL * dt);
            self.vel.x = approach(self.vel.x, 0.0, SLIDE_FRICTION * dt);
            self.vel.y += GRAVITY * dt;
            return;
        }

        let accel = if self.on_ground {
            GROUND_ACCEL
        } else {
            AIR_ACCEL
        };
        self.vel.x = approach(self.vel.x, input.move_x * PLAYER_SPEED, accel * dt);

        if self.on_ground && input.move_x == 0.0 {
            self.vel.x = approach(self.vel.x, 0.0, GROUND_FRICTION * dt);
        } else if !self.on_ground {
            self.vel.x *= 1.0 - AIR_DRAG * dt;
        }

        self.vel.y += GRAVITY * self.wall_gravity_scale() * dt;
        self.apply_wall_slide(input);
    }

    fn try_jump(&mut self) {
        // Coyote time allows jumping shortly after leaving the ground.
        if self.jump_buffer == 0.0 {
            return;
        }

        if self.wall_timer > 0.0 && !self.on_ground && self.wall_jumps_left > 0 {
            self.vel.x = -self.wall_dir * WALL_JUMP_X;
            self.vel.y = -WALL_JUMP_Y;
            self.wall_jumps_left -= 1;
            self.wall_gravity_timer = 0.0;
            self.wall_timer = 0.0;
        } else if self.coyote_timer > 0.0 {
            self.vel.y = -JUMP_VELOCITY;
            self.coyote_timer = 0.0;
        } else {
            return;
        }

        self.on_ground = false;
        if self.sliding {
            self.exit_slide();
        }
        self.jump_buffer = 0.0;
    }

    fn apply_wall_slide(&mut self, input: &Input) {
        self.wall_sliding =
            !self.on_ground && self.wall_timer > 0.0 && input.move_x == self.wall_dir;
    }

    fn input_direction(&self, input: &Input) -> Vec2 {
        let aim = input.aim_pos - self.aim_from();
        if aim.length_squared() > 1.0 {
            return aim.normalize();
        }

        Vec2::new(self.facing, 0.0)
    }

    fn slide_direction(&self) -> f32 {
        let aim_x = self.aim_pos.x - self.pos.x;
        if aim_x.abs() > 1.0 {
            aim_x.signum()
        } else {
            self.facing
        }
    }

    fn update_slide(&mut self, input: &Input) {
        let wants_slide = input.slide_down && (self.on_ground || self.sliding);
        if wants_slide && !self.sliding {
            // Slide locks direction at start; movement keys cannot steer it.
            self.slide_dir = self.slide_direction();
            self.enter_slide();
            self.vel.x = self.slide_dir * SLIDE_BOOST;
        } else if !wants_slide && self.sliding {
            self.exit_slide();
        }
    }

    fn enter_slide(&mut self) {
        let bottom = self.pos.y + self.half_size().y;
        self.size.y = PLAYER_SIZE.1 * SLIDE_HEIGHT_SCALE;
        self.pos.y = bottom - self.half_size().y;
        self.sliding = true;
    }

    fn exit_slide(&mut self) {
        let bottom = self.pos.y + self.half_size().y;
        self.size.y = PLAYER_SIZE.1;
        self.pos.y = bottom - self.half_size().y;
        self.sliding = false;
    }

    fn recover_dash(&mut self, dt: f32) {
        if self.sliding {
            return;
        }

        // Dash charges refill over time unless the player is sliding.
        self.dash_charges = (self.dash_charges + DASH_CHARGE_RECOVERY * dt).min(MAX_DASH_CHARGES);
    }

    fn wall_gravity_scale(&self) -> f32 {
        if self.wall_gravity_timer == 0.0 {
            return 1.0;
        }

        // Wall contact starts floaty and ramps back to full gravity.
        let progress = 1.0 - (self.wall_gravity_timer / WALL_GRAVITY_RAMP_TIME).clamp(0.0, 1.0);
        WALL_MIN_GRAVITY_SCALE + (1.0 - WALL_MIN_GRAVITY_SCALE) * progress
    }
}

fn approach(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        (current + delta).min(target)
    } else {
        (current - delta).max(target)
    }
}

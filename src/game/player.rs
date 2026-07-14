use glam::Vec2;

// Player owns input-driven movement; level collision owns contact correction.
use crate::constants::{
    AIR_ACCEL, AIR_DRAG, DASH_CHARGE_RECOVERY, DASH_DURATION, DASH_SPEED, GRAVITY, GROUND_ACCEL,
    GROUND_FRICTION, GROUND_SLAM_ACCEL, GROUND_SLAM_SPEED, JUMP_BUFFER_TIME, JUMP_COYOTE_TIME,
    JUMP_VELOCITY, MAX_DASH_CHARGES, MAX_WALL_JUMPS, PLAYER_SIZE, PLAYER_SPEED,
    SLAM_BOUNCE_MAX_SPEED, SLAM_BOUNCE_SCALE, SLAM_DIVE_MAX_X, SLAM_DIVE_SCALE_X,
    SLAM_DIVE_SCALE_Y, SLAM_JUMP_WINDOW, SLAM_NORMAL_BOUNCE_MAX_SPEED, SLAM_NORMAL_HEIGHT_GAIN,
    SLAM_NORMAL_MIN_BONUS, SLAM_SLIDE_SCALE_X, SLAM_STORAGE_MIN_SPEED, SLIDE_BOOST, SLIDE_FRICTION,
    SLIDE_HEIGHT_SCALE, WALL_JUMP_X, WALL_JUMP_Y, WALL_SLIDE_SPEED, WALL_STICK_TIME,
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
    pub ground_slamming: bool,
    pub wall_sliding: bool,
    coyote_timer: f32,
    dash_dir: Vec2,
    dash_timer: f32,
    facing: f32,
    jump_buffer: f32,
    slide_dir: f32,
    slam_entry_speed: f32,
    slam_start_y: f32,
    slam_jump_timer: f32,
    slam_storage_ready: bool,
    slam_stored_speed: f32,
    wall_dir: f32,
    wall_jumps_left: u8,
    wall_timer: f32,
    events: Vec<PlayerEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    Jump,
    DashStart,
    DashEnd,
    SlideStart,
    SlideEnd,
    GroundSlamStart,
    GroundSlamEnd,
    Land,
    HeavyLand,
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
            ground_slamming: false,
            wall_sliding: false,
            coyote_timer: 0.0,
            dash_dir: Vec2::X,
            dash_timer: 0.0,
            facing: 1.0,
            jump_buffer: 0.0,
            slide_dir: 1.0,
            slam_entry_speed: 0.0,
            slam_start_y: pos.y,
            slam_jump_timer: 0.0,
            slam_storage_ready: false,
            slam_stored_speed: 0.0,
            wall_dir: 0.0,
            wall_jumps_left: MAX_WALL_JUMPS,
            wall_timer: 0.0,
            events: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, _screen_width: f32, _screen_height: f32) {
        self.events.clear();
        self.prev_pos = self.pos;
        self.aim_pos = input.aim_pos;

        // Update intent-driven state before integrating position.
        self.tick_timers(dt);
        self.track_jump_window(input);
        self.start_dash(input);
        self.start_ground_slam(input);
        self.apply_movement(dt, input);
        self.pos += self.vel * dt;
    }

    pub fn clear_contacts(&mut self) {
        self.on_ground = false;
        self.wall_sliding = false;
    }

    pub fn land(&mut self) {
        self.vel.y = 0.0;
        self.touch_ground();
    }

    pub fn touch_ground(&mut self) {
        self.finish_ground_contact(0.0, false);
    }

    pub fn touch_ground_with_impact(&mut self, down_speed: f32) {
        self.finish_ground_contact(down_speed, true);
    }

    pub fn touch_ground_contact(&mut self, down_speed: f32, fresh_landing: bool) {
        self.finish_ground_contact(down_speed, fresh_landing);
    }

    fn finish_ground_contact(&mut self, down_speed: f32, fresh_landing: bool) {
        if fresh_landing {
            let event = if down_speed > 850.0 {
                PlayerEvent::HeavyLand
            } else {
                PlayerEvent::Land
            };

            self.events.push(event);
        }

        if fresh_landing && self.ground_slamming {
            self.slam_stored_speed = self.natural_slam_speed();
            self.slam_jump_timer = SLAM_JUMP_WINDOW;
        } else if fresh_landing && self.slam_stored_speed > 0.0 {
            self.slam_stored_speed = self.slam_stored_speed.max(down_speed);
            self.slam_jump_timer = SLAM_JUMP_WINDOW;
        }

        self.stop_ground_slam();
        self.on_ground = true;
        self.coyote_timer = JUMP_COYOTE_TIME;
        self.wall_jumps_left = MAX_WALL_JUMPS;
        self.wall_sliding = false;
    }

    pub fn set_wall_contact(&mut self, wall_dir: f32) {
        self.wall_dir = wall_dir;
        self.wall_timer = WALL_STICK_TIME;
    }

    pub fn half_size(&self) -> Vec2 {
        self.size / 2.0
    }

    pub fn aim_from(&self) -> Vec2 {
        self.pos + Vec2::new(0.0, -self.size.y * 0.18)
    }

    pub fn slam_storage_ready(&self) -> bool {
        self.slam_storage_ready
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = PlayerEvent> + '_ {
        self.events.drain(..)
    }

    fn tick_timers(&mut self, dt: f32) {
        let was_dashing = self.dashing;

        self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        self.coyote_timer = (self.coyote_timer - dt).max(0.0);
        self.dash_timer = (self.dash_timer - dt).max(0.0);
        self.slam_jump_timer = (self.slam_jump_timer - dt).max(0.0);
        if self.on_ground && self.slam_jump_timer == 0.0 {
            self.slam_storage_ready = false;
            self.slam_stored_speed = 0.0;
        }
        self.wall_timer = (self.wall_timer - dt).max(0.0);
        if self.wall_timer == 0.0 {
            self.wall_dir = 0.0;
        }
        self.dashing = self.dash_timer > 0.0;
        if was_dashing && !self.dashing {
            self.events.push(PlayerEvent::DashEnd);
        }
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
        self.events.push(PlayerEvent::DashStart);
    }

    fn start_ground_slam(&mut self, input: &Input) {
        if self.on_ground || self.dashing || !input.slide_pressed {
            return;
        }

        self.ground_slamming = true;
        self.slam_storage_ready = false;
        self.slam_entry_speed = self.vel.y.max(0.0);
        self.slam_start_y = self.pos.y;
        self.slam_stored_speed = self.slam_entry_speed;
        self.events.push(PlayerEvent::GroundSlamStart);
    }

    fn apply_movement(&mut self, dt: f32, input: &Input) {
        if self.dashing {
            self.recover_dash(dt);
            self.vel = self.dash_dir * DASH_SPEED;
            return;
        }

        self.try_jump(input);
        self.update_slide(input);
        self.recover_dash(dt);

        if self.ground_slamming {
            self.vel.y = (self.vel.y + GROUND_SLAM_ACCEL * dt).min(GROUND_SLAM_SPEED);
            return;
        }

        if self.sliding {
            // Slide ignores movement input after start, but gravity still applies.
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

        self.vel.y += GRAVITY * dt;
        self.apply_wall_slide();
    }

    fn try_jump(&mut self, input: &Input) {
        // Coyote time allows jumping shortly after leaving the ground.
        if self.jump_buffer == 0.0 {
            return;
        }

        if self.try_slam_jump(input) {
            return;
        }

        if self.wall_timer > 0.0 && !self.on_ground && self.wall_jumps_left > 0 {
            self.store_wall_slam();
            // Wall jumps spend limited air charges until the next landing.
            self.vel.x = -self.wall_dir * WALL_JUMP_X;
            self.vel.y = -WALL_JUMP_Y;
            self.wall_jumps_left -= 1;
            self.wall_timer = 0.0;
            self.events.push(PlayerEvent::Jump);
        } else if self.coyote_timer > 0.0 {
            self.vel.y = -JUMP_VELOCITY;
            self.coyote_timer = 0.0;
            self.events.push(PlayerEvent::Jump);
        } else {
            return;
        }

        self.on_ground = false;
        if self.sliding {
            self.exit_slide();
        }
        self.jump_buffer = 0.0;
    }

    fn try_slam_jump(&mut self, input: &Input) -> bool {
        if self.slam_jump_timer == 0.0 || self.slam_stored_speed == 0.0 {
            return false;
        }

        let speed = self.slam_bounce_speed();
        if self.sliding {
            self.exit_slide();
        }

        if self.slide_dir == 0.0 {
            self.slide_dir = self.facing;
        }

        if input.slide_down {
            self.slide_dir = self.slide_direction();
            self.vel.x =
                self.slide_dir * (SLIDE_BOOST + speed * SLAM_DIVE_SCALE_X).min(SLAM_DIVE_MAX_X);
            self.vel.y = -(JUMP_VELOCITY * 0.55).max(speed * SLAM_DIVE_SCALE_Y);
        } else {
            self.vel.y = -speed;
        }

        self.consume_slam_energy();
        self.on_ground = false;
        self.coyote_timer = 0.0;
        self.jump_buffer = 0.0;
        self.events.push(PlayerEvent::Jump);
        true
    }

    fn apply_wall_slide(&mut self) {
        self.wall_sliding = !self.on_ground && self.wall_timer > 0.0;
        if self.wall_sliding && self.vel.y > WALL_SLIDE_SPEED {
            self.vel.y = WALL_SLIDE_SPEED;
        }
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
            self.vel.x = self.slide_start_speed();
            self.events.push(PlayerEvent::SlideStart);
        } else if !wants_slide && self.sliding {
            self.exit_slide();
        }
    }

    fn slide_start_speed(&mut self) -> f32 {
        if self.slam_jump_timer == 0.0 || self.slam_stored_speed == 0.0 {
            return self.slide_dir * SLIDE_BOOST;
        }

        let speed = self.slam_bounce_speed();
        self.consume_slam_energy();
        self.slide_dir * (SLIDE_BOOST + speed * SLAM_SLIDE_SCALE_X).min(SLAM_DIVE_MAX_X)
    }

    fn enter_slide(&mut self) {
        let bottom = self.pos.y + self.half_size().y;
        // Preserve feet position so crouching does not lift or drop the player.
        self.size.y = PLAYER_SIZE.1 * SLIDE_HEIGHT_SCALE;
        self.pos.y = bottom - self.half_size().y;
        self.sliding = true;
    }

    fn exit_slide(&mut self) {
        let bottom = self.pos.y + self.half_size().y;
        // Restore standing size from the same floor contact point.
        self.size.y = PLAYER_SIZE.1;
        self.pos.y = bottom - self.half_size().y;
        self.sliding = false;
        self.events.push(PlayerEvent::SlideEnd);
    }

    fn recover_dash(&mut self, dt: f32) {
        if self.sliding {
            return;
        }

        // Dash charges refill over time unless the player is sliding.
        self.dash_charges = (self.dash_charges + DASH_CHARGE_RECOVERY * dt).min(MAX_DASH_CHARGES);
    }

    fn store_wall_slam(&mut self) {
        if !self.ground_slamming {
            return;
        }

        self.slam_stored_speed = self
            .slam_stored_speed
            .max(self.vel.y.max(SLAM_STORAGE_MIN_SPEED));
        self.slam_storage_ready = true;
        self.stop_ground_slam();
    }

    fn natural_slam_speed(&self) -> f32 {
        let fall_distance = (self.pos.y - self.slam_start_y).max(0.0);
        // Normal slam bounce ignores artificial slam acceleration.
        (self.slam_entry_speed.powi(2) + 2.0 * GRAVITY * fall_distance).sqrt()
    }

    fn slam_bounce_speed(&self) -> f32 {
        if self.slam_storage_ready {
            return (JUMP_VELOCITY + self.slam_stored_speed * SLAM_BOUNCE_SCALE)
                .min(SLAM_BOUNCE_MAX_SPEED);
        }

        // Height scales with velocity squared, so +10% height is sqrt(1.10) speed.
        let height_return = self.slam_stored_speed * SLAM_NORMAL_HEIGHT_GAIN.sqrt();
        let low_fall_boost = JUMP_VELOCITY + SLAM_NORMAL_MIN_BONUS;

        low_fall_boost
            .max(height_return)
            .min(SLAM_NORMAL_BOUNCE_MAX_SPEED)
    }

    fn consume_slam_energy(&mut self) {
        self.stop_ground_slam();
        self.slam_jump_timer = 0.0;
        self.slam_storage_ready = false;
        self.slam_entry_speed = 0.0;
        self.slam_stored_speed = 0.0;
    }

    fn stop_ground_slam(&mut self) {
        if self.ground_slamming {
            self.events.push(PlayerEvent::GroundSlamEnd);
        }
        self.ground_slamming = false;
    }
}

fn approach(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        (current + delta).min(target)
    } else {
        (current - delta).max(target)
    }
}

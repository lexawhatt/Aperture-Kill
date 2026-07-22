use glam::Vec2;

// Player owns input-driven movement; level collision owns contact correction.
use crate::constants::{
    AIR_ACCEL, AIR_DRAG, AIR_SLIDE_DRAG, AIR_SLIDE_GRAVITY_SCALE, AIR_SLIDE_WALL_FALL_SPEED,
    AIR_SLIDE_WALL_KICK_Y, BUNNY_HOP_GRACE, CROUCH_SPEED_SCALE, DASH_CHARGE_RECOVERY,
    DASH_DENY_FLASH_TIME, DASH_DURATION, DASH_SPEED, DASH_STORAGE_TIME, DIVE_BOOST, DIVE_WINDOW,
    GRAVITY, GROUND_ACCEL, GROUND_FRICTION, GROUND_SLAM_ACCEL, GROUND_SLAM_SPEED, JUMP_BUFFER_TIME,
    JUMP_COYOTE_TIME, JUMP_VELOCITY, MAX_DASH_CHARGES, MAX_WALL_JUMPS, PLAYER_DAMAGE_GRACE,
    PLAYER_MAX_HEALTH, PLAYER_SIZE, PLAYER_SPEED, PORTAL_EXIT_GRACE, PORTAL_KICK_SPEED,
    PORTAL_SLIDE_BOOST, SLAM_BOUNCE_MAX_SPEED, SLAM_BOUNCE_SCALE, SLAM_DIVE_MAX_X,
    SLAM_DIVE_SCALE_X, SLAM_DIVE_SCALE_Y, SLAM_JUMP_WINDOW, SLAM_NORMAL_BOUNCE_MAX_SPEED,
    SLAM_NORMAL_HEIGHT_GAIN, SLAM_NORMAL_MIN_BONUS, SLAM_SLIDE_SCALE_X, SLAM_STORAGE_MIN_SPEED,
    SLIDE_BOOST, SLIDE_CHAIN_BOOST, SLIDE_CHAIN_MAX_SPEED, SLIDE_FRICTION, SLIDE_HEIGHT_SCALE,
    SLIDE_JUMP_HEIGHT_SCALE, VERTICAL_WALL_JUMP_SPEED, VERTICAL_WALL_JUMP_THRESHOLD,
    VERTICAL_WALL_JUMP_WINDOW, WALL_JUMP_X, WALL_JUMP_Y, WALL_SLAM_MAX_SPEED, WALL_SLAM_SCALE,
    WALL_SLIDE_SPEED, WALL_STICK_TIME,
};
use crate::platform::input::{GameKey, Input};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementState {
    Normal,
    Crouching,
    Sliding,
    AirSlide,
    Dashing,
    GroundSlam,
    WallSlide,
}

#[derive(Clone, Copy, Debug)]
pub struct MovementInput {
    pub move_x: f32,
    pub aim_pos: Vec2,
    pub dash_pressed: bool,
    pub jump_pressed: bool,
    pub slide_down: bool,
    pub slide_pressed: bool,
}

impl From<&Input> for MovementInput {
    fn from(input: &Input) -> Self {
        Self {
            move_x: input.move_x,
            aim_pos: input.aim_pos,
            dash_pressed: input.key_pressed(GameKey::Dash),
            jump_pressed: input.key_pressed(GameKey::Jump),
            slide_down: input.key_down(GameKey::Slide),
            slide_pressed: input.key_pressed(GameKey::Slide),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MovementTimers {
    bunny_hop: f32,
    coyote: f32,
    dash: f32,
    dash_deny_flash: f32,
    dash_storage: f32,
    damage_grace: f32,
    dive: f32,
    jump_buffer: f32,
    portal_exit: f32,
    slam_jump: f32,
    vertical_wall_jump: f32,
    wall: f32,
}

impl MovementTimers {
    fn new() -> Self {
        Self {
            bunny_hop: 0.0,
            coyote: 0.0,
            dash: 0.0,
            dash_deny_flash: 0.0,
            dash_storage: 0.0,
            damage_grace: 0.0,
            dive: 0.0,
            jump_buffer: 0.0,
            portal_exit: 0.0,
            slam_jump: 0.0,
            vertical_wall_jump: 0.0,
            wall: 0.0,
        }
    }

    fn tick(&mut self, dt: f32) {
        self.bunny_hop = (self.bunny_hop - dt).max(0.0);
        self.coyote = (self.coyote - dt).max(0.0);
        self.dash = (self.dash - dt).max(0.0);
        self.dash_deny_flash = (self.dash_deny_flash - dt).max(0.0);
        self.dash_storage = (self.dash_storage - dt).max(0.0);
        self.damage_grace = (self.damage_grace - dt).max(0.0);
        self.dive = (self.dive - dt).max(0.0);
        self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        self.portal_exit = (self.portal_exit - dt).max(0.0);
        self.slam_jump = (self.slam_jump - dt).max(0.0);
        self.vertical_wall_jump = (self.vertical_wall_jump - dt).max(0.0);
        self.wall = (self.wall - dt).max(0.0);
    }
}

#[derive(Clone, Copy, Debug)]
struct SlamState {
    entry_speed: f32,
    start_y: f32,
    storage_ready: bool,
    stored_speed: f32,
}

impl SlamState {
    fn new(start_y: f32) -> Self {
        Self {
            entry_speed: 0.0,
            start_y,
            storage_ready: false,
            stored_speed: 0.0,
        }
    }

    fn start(&mut self, pos_y: f32, velocity_y: f32) {
        self.storage_ready = false;
        self.entry_speed = velocity_y.max(0.0);
        self.start_y = pos_y;
        self.stored_speed = self.entry_speed;
    }

    fn clear(&mut self) {
        self.storage_ready = false;
        self.entry_speed = 0.0;
        self.stored_speed = 0.0;
    }
}

pub struct Player {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub aim_pos: Vec2,
    pub on_ground: bool,
    pub dash_charges: f32,
    pub health: f32,
    movement_state: MovementState,
    dash_dir: Vec2,
    facing: f32,
    air_slide_wall_speed: f32,
    slide_dir: f32,
    slam: SlamState,
    wall_dir: f32,
    wall_jumps_left: u8,
    timers: MovementTimers,
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
            movement_state: MovementState::Normal,
            dash_charges: MAX_DASH_CHARGES,
            health: PLAYER_MAX_HEALTH,
            dash_dir: Vec2::X,
            facing: 1.0,
            air_slide_wall_speed: 0.0,
            slide_dir: 1.0,
            slam: SlamState::new(pos.y),
            wall_dir: 0.0,
            wall_jumps_left: MAX_WALL_JUMPS,
            timers: MovementTimers::new(),
            events: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn update(&mut self, dt: f32, input: &Input, _screen_width: f32, _screen_height: f32) {
        self.update_movement(dt, MovementInput::from(input));
    }

    pub fn update_movement(&mut self, dt: f32, input: MovementInput) {
        self.events.clear();
        self.prev_pos = self.pos;
        self.aim_pos = input.aim_pos;

        // Update intent-driven state before integrating position.
        self.tick_timers(dt);
        self.track_jump_window(&input);
        self.start_dash(&input);
        self.start_ground_slam(&input);
        self.apply_movement(dt, &input);
        self.pos += self.vel * dt;
    }

    pub fn clear_contacts(&mut self) {
        self.on_ground = false;
        if self.is_wall_sliding() {
            self.movement_state = MovementState::Normal;
        }
    }

    #[cfg(test)]
    pub fn land(&mut self) {
        self.vel.y = 0.0;
        self.touch_ground();
    }

    pub fn touch_ground(&mut self) {
        self.finish_ground_contact(0.0, false);
    }

    #[cfg(test)]
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
            self.timers.bunny_hop = BUNNY_HOP_GRACE;
        }

        if fresh_landing && self.is_ground_slamming() {
            self.slam.stored_speed = self.natural_slam_speed();
            self.timers.slam_jump = SLAM_JUMP_WINDOW;
        } else if fresh_landing && self.slam.stored_speed > 0.0 {
            self.slam.stored_speed = self.slam.stored_speed.max(down_speed);
            self.timers.slam_jump = SLAM_JUMP_WINDOW;
        }

        self.stop_ground_slam();
        self.on_ground = true;
        self.timers.coyote = JUMP_COYOTE_TIME;
        self.wall_jumps_left = MAX_WALL_JUMPS;
        if self.is_wall_sliding() {
            self.movement_state = MovementState::Normal;
        }
        if self.is_air_sliding() {
            self.exit_slide();
        }
    }

    pub fn set_wall_contact(&mut self, wall_dir: f32) {
        self.wall_dir = wall_dir;
        self.timers.wall = WALL_STICK_TIME;
    }

    pub fn set_wall_contact_with_speed(&mut self, wall_dir: f32, incoming_speed: f32) {
        self.set_wall_contact(wall_dir);
        if self.is_air_sliding() {
            self.air_slide_wall_speed = self.air_slide_wall_speed.max(incoming_speed.abs());
            self.vel.y = self.vel.y.min(AIR_SLIDE_WALL_FALL_SPEED);
        }
        if incoming_speed.abs() >= VERTICAL_WALL_JUMP_THRESHOLD {
            self.timers.vertical_wall_jump = VERTICAL_WALL_JUMP_WINDOW;
        }
    }

    pub fn try_wall_slam(&mut self, wall_normal: Vec2) -> bool {
        if !self.is_ground_slamming() {
            return false;
        }

        let speed = self
            .vel
            .y
            .max(self.slam.stored_speed)
            .max(SLAM_STORAGE_MIN_SPEED)
            * WALL_SLAM_SCALE;
        self.vel.x = wall_normal.x.signum() * speed.min(WALL_SLAM_MAX_SPEED);
        self.vel.y = -(JUMP_VELOCITY * 0.22);
        self.slam.stored_speed = speed;
        self.slam.storage_ready = true;
        self.timers.slam_jump = SLAM_JUMP_WINDOW;
        self.stop_ground_slam();

        true
    }

    pub fn on_player_portal_exit(&mut self, exit_normal: Vec2, input: MovementInput) {
        self.timers.portal_exit = PORTAL_EXIT_GRACE;

        if input.jump_pressed {
            self.vel += exit_normal * PORTAL_KICK_SPEED;
            self.events.push(PlayerEvent::Jump);
        }

        if input.slide_down {
            self.slide_dir = self.portal_slide_direction(exit_normal);
            self.enter_air_slide();
            self.vel += Vec2::new(self.slide_dir * PORTAL_SLIDE_BOOST, 0.0);
            self.events.push(PlayerEvent::SlideStart);
        }
    }

    pub fn half_size(&self) -> Vec2 {
        self.size / 2.0
    }

    pub fn aim_from(&self) -> Vec2 {
        self.pos + Vec2::new(0.0, -self.size.y * 0.18)
    }

    pub fn slam_storage_ready(&self) -> bool {
        self.slam.storage_ready
    }

    pub fn dash_deny_flash(&self) -> f32 {
        (self.timers.dash_deny_flash / DASH_DENY_FLASH_TIME).clamp(0.0, 1.0)
    }

    pub fn health_percent(&self) -> f32 {
        (self.health / PLAYER_MAX_HEALTH).clamp(0.0, 1.0)
    }

    pub fn damage(&mut self, amount: f32) -> bool {
        if amount <= 0.0 || self.timers.damage_grace > 0.0 {
            return false;
        }

        self.health = (self.health - amount).max(0.0);
        self.timers.damage_grace = PLAYER_DAMAGE_GRACE;

        self.health == 0.0
    }

    pub fn is_sliding(&self) -> bool {
        self.movement_state == MovementState::Sliding
    }

    pub fn is_crouching(&self) -> bool {
        self.movement_state == MovementState::Crouching
    }

    pub fn is_air_sliding(&self) -> bool {
        self.movement_state == MovementState::AirSlide
    }

    pub fn is_dashing(&self) -> bool {
        self.movement_state == MovementState::Dashing
    }

    pub fn is_ground_slamming(&self) -> bool {
        self.movement_state == MovementState::GroundSlam
    }

    pub fn is_wall_sliding(&self) -> bool {
        self.movement_state == MovementState::WallSlide
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = PlayerEvent> + '_ {
        self.events.drain(..)
    }

    pub fn standing_body(&self) -> Option<(Vec2, Vec2)> {
        if !self.is_crouching() {
            return None;
        }

        let standing_size = Vec2::new(PLAYER_SIZE.0, PLAYER_SIZE.1);
        let bottom = self.pos.y + self.half_size().y;
        let center = Vec2::new(self.pos.x, bottom - standing_size.y / 2.0);

        Some((center, standing_size / 2.0))
    }

    pub fn stand_up(&mut self) {
        if !self.is_crouching() {
            return;
        }

        let bottom = self.pos.y + self.half_size().y;
        self.size.y = PLAYER_SIZE.1;
        self.pos.y = bottom - self.half_size().y;
        self.movement_state = MovementState::Normal;
    }

    fn tick_timers(&mut self, dt: f32) {
        let was_dashing = self.is_dashing();

        self.timers.tick(dt);
        if self.on_ground && self.timers.slam_jump == 0.0 {
            self.slam.storage_ready = false;
            self.slam.stored_speed = 0.0;
        }
        if self.timers.wall == 0.0 {
            self.wall_dir = 0.0;
        }
        if self.is_dashing() && self.timers.dash == 0.0 {
            self.movement_state = MovementState::Normal;
        }
        if was_dashing && !self.is_dashing() {
            self.events.push(PlayerEvent::DashEnd);
        }
    }

    fn track_jump_window(&mut self, input: &MovementInput) {
        // Facing is used when dash/slide has no stronger direction.
        if input.move_x != 0.0 {
            self.facing = input.move_x.signum();
        }

        // Jump buffer accepts slightly early jump inputs.
        if input.jump_pressed {
            self.timers.jump_buffer = JUMP_BUFFER_TIME;
        }
    }

    fn start_dash(&mut self, input: &MovementInput) {
        if !input.dash_pressed {
            return;
        }

        if self.dash_charges < 1.0 {
            self.timers.dash_deny_flash = DASH_DENY_FLASH_TIME;
            return;
        }

        if self.is_crouching() {
            return;
        }

        // Dash uses aim direction, falling back to facing direction.
        self.dash_charges -= 1.0;
        self.dash_dir = self.input_direction(input);
        if self.is_ground_slamming() {
            self.cancel_ground_slam();
        }
        if self.is_sliding() {
            self.exit_slide();
        }
        self.timers.dash = DASH_DURATION;
        self.movement_state = MovementState::Dashing;
        self.vel = self.dash_dir * DASH_SPEED;
        self.events.push(PlayerEvent::DashStart);
    }

    fn start_ground_slam(&mut self, input: &MovementInput) {
        if self.on_ground || self.is_dashing() || !input.slide_pressed {
            return;
        }
        if self.try_start_dive(input) {
            return;
        }
        if self.is_sliding() {
            self.exit_slide();
        }
        self.movement_state = MovementState::GroundSlam;
        self.slam.start(self.pos.y, self.vel.y);
        self.events.push(PlayerEvent::GroundSlamStart);
    }

    fn try_start_dive(&mut self, input: &MovementInput) -> bool {
        if !input.slide_pressed
            || self.on_ground
            || self.timers.dive == 0.0
            || self.is_air_sliding()
            || self.is_ground_slamming()
        {
            return false;
        }

        self.slide_dir = self.slide_direction();
        self.enter_air_slide();
        if self.vel.x.abs() < DIVE_BOOST {
            self.vel.x = self.slide_dir * DIVE_BOOST;
        }
        self.timers.dive = 0.0;
        self.events.push(PlayerEvent::SlideStart);
        true
    }

    fn apply_movement(&mut self, dt: f32, input: &MovementInput) {
        if self.is_dashing() {
            if self.try_dash_jump(input) || self.try_dash_slide(input) {
                self.recover_dash(dt);
                return;
            }
            self.recover_dash(dt);
            self.vel = self.dash_dir * DASH_SPEED;
            return;
        }

        self.try_jump(input);
        self.try_start_dive(input);
        self.update_slide(input);
        self.recover_dash(dt);

        if self.is_ground_slamming() {
            self.vel.y = (self.vel.y + GROUND_SLAM_ACCEL * dt).min(GROUND_SLAM_SPEED);
            return;
        }

        if self.is_sliding() {
            // Slide keeps fast entry momentum; it only accelerates if the slide started slowly.
            if self.vel.x.abs() < SLIDE_BOOST {
                self.vel.x = approach(self.vel.x, self.slide_dir * SLIDE_BOOST, GROUND_ACCEL * dt);
            }
            self.vel.x = approach(self.vel.x, 0.0, SLIDE_FRICTION * dt);
            self.vel.y += GRAVITY * dt;
            return;
        }

        if self.is_air_sliding() {
            self.vel.x *= 1.0 - AIR_SLIDE_DRAG * dt;
            self.vel.y += GRAVITY * AIR_SLIDE_GRAVITY_SCALE * dt;
            if !input.slide_down {
                self.exit_slide();
            }
            return;
        }

        let accel = if self.on_ground {
            GROUND_ACCEL
        } else {
            AIR_ACCEL
        };
        let speed = if self.is_crouching() {
            PLAYER_SPEED * CROUCH_SPEED_SCALE
        } else {
            PLAYER_SPEED
        };
        self.vel.x = approach(self.vel.x, input.move_x * speed, accel * dt);

        if self.on_ground && input.move_x == 0.0 && self.timers.bunny_hop == 0.0 {
            self.vel.x = approach(self.vel.x, 0.0, GROUND_FRICTION * dt);
        } else if !self.on_ground {
            self.vel.x *= 1.0 - AIR_DRAG * dt;
        }

        self.vel.y += GRAVITY * dt;
        self.apply_wall_slide();
    }

    fn try_dash_jump(&mut self, input: &MovementInput) -> bool {
        if !input.jump_pressed || self.timers.coyote == 0.0 {
            return false;
        }

        self.vel = self.dash_dir * DASH_SPEED;
        self.vel.y = -JUMP_VELOCITY;
        self.on_ground = false;
        self.timers.coyote = 0.0;
        self.timers.dive = DIVE_WINDOW;
        self.timers.jump_buffer = 0.0;
        self.finish_dash();
        self.events.push(PlayerEvent::Jump);
        true
    }

    fn try_dash_slide(&mut self, input: &MovementInput) -> bool {
        if !input.slide_down || !self.on_ground {
            return false;
        }

        let dash_speed = self.vel.x.abs().max(DASH_SPEED);
        self.slide_dir = self.dash_dir.x.signum();
        if self.slide_dir == 0.0 {
            self.slide_dir = self.slide_direction();
        }
        self.finish_dash();
        self.enter_slide();
        self.vel.x = self.slide_dir * dash_speed.max(SLIDE_BOOST);
        self.timers.dash_storage = DASH_STORAGE_TIME;
        self.events.push(PlayerEvent::SlideStart);
        true
    }

    fn try_jump(&mut self, input: &MovementInput) {
        // Coyote time allows jumping shortly after leaving the ground.
        if self.timers.jump_buffer == 0.0 {
            return;
        }

        if self.try_slam_jump(input) {
            return;
        }

        if self.try_air_slide_wall_kick() {
            // Air-slide wall kicks reuse the wall-jump counter but keep slide momentum.
        } else if self.try_vertical_wall_jump() {
            // Vertical wall jumps are precision jumps and do not spend the regular wall counter.
        } else if self.timers.wall > 0.0 && !self.on_ground && self.wall_jumps_left > 0 {
            self.store_wall_slam();
            // Wall jumps spend limited air charges until the next landing.
            self.vel.x = -self.wall_dir * WALL_JUMP_X;
            self.vel.y = -WALL_JUMP_Y;
            self.wall_jumps_left -= 1;
            self.timers.wall = 0.0;
            self.events.push(PlayerEvent::Jump);
        } else if self.timers.coyote > 0.0 {
            let slide_jump = self.is_sliding();
            if self.is_sliding() && self.timers.dash_storage > 0.0 {
                let dir = if self.slide_dir != 0.0 {
                    self.slide_dir
                } else {
                    self.facing
                };
                self.vel.x = dir * self.vel.x.abs().max(DASH_SPEED);
                self.timers.dash_storage = 0.0;
            }
            self.vel.y = if slide_jump {
                -JUMP_VELOCITY * SLIDE_JUMP_HEIGHT_SCALE
            } else {
                -JUMP_VELOCITY
            };
            self.timers.coyote = 0.0;
            self.events.push(PlayerEvent::Jump);
        } else {
            return;
        }

        self.on_ground = false;
        if self.is_sliding() {
            self.exit_slide();
        }
        self.timers.dive = DIVE_WINDOW;
        self.timers.jump_buffer = 0.0;
    }

    fn try_vertical_wall_jump(&mut self) -> bool {
        if self.timers.vertical_wall_jump == 0.0 || self.timers.wall == 0.0 || self.on_ground {
            return false;
        }

        self.store_wall_slam();
        self.vel.x = 0.0;
        self.vel.y = -VERTICAL_WALL_JUMP_SPEED;
        self.timers.wall = 0.0;
        self.timers.vertical_wall_jump = 0.0;
        self.events.push(PlayerEvent::Jump);
        true
    }

    fn try_air_slide_wall_kick(&mut self) -> bool {
        if !self.is_air_sliding()
            || self.timers.wall == 0.0
            || self.on_ground
            || self.wall_jumps_left == 0
        {
            return false;
        }

        let speed = self
            .air_slide_wall_speed
            .max(self.vel.x.abs())
            .max(SLIDE_BOOST);
        self.vel.x = -self.wall_dir * speed;
        self.vel.y = -AIR_SLIDE_WALL_KICK_Y;
        self.wall_jumps_left -= 1;
        self.timers.wall = 0.0;
        self.air_slide_wall_speed = 0.0;
        self.events.push(PlayerEvent::Jump);
        true
    }

    fn try_slam_jump(&mut self, input: &MovementInput) -> bool {
        if self.timers.slam_jump == 0.0 || self.slam.stored_speed == 0.0 {
            return false;
        }

        let speed = self.slam_bounce_speed();
        if self.is_sliding() {
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
        self.timers.coyote = 0.0;
        self.timers.dive = DIVE_WINDOW;
        self.timers.jump_buffer = 0.0;
        self.events.push(PlayerEvent::Jump);
        true
    }

    fn apply_wall_slide(&mut self) {
        let wall_sliding = !self.on_ground && self.timers.wall > 0.0;

        if wall_sliding
            && matches!(
                self.movement_state,
                MovementState::Normal | MovementState::WallSlide
            )
        {
            self.movement_state = MovementState::WallSlide;
        } else if !wall_sliding && self.is_wall_sliding() {
            self.movement_state = MovementState::Normal;
        }

        if self.is_wall_sliding() && self.vel.y > WALL_SLIDE_SPEED {
            self.vel.y = WALL_SLIDE_SPEED;
        }
    }

    fn input_direction(&self, input: &MovementInput) -> Vec2 {
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

    fn update_slide(&mut self, input: &MovementInput) {
        let wants_ground_slide = input.slide_down && (self.on_ground || self.is_sliding());
        if wants_ground_slide && !self.is_sliding() {
            // Slide locks direction at start; movement keys cannot steer it.
            self.slide_dir = self.slide_direction();
            self.enter_slide();
            self.vel.x = self.slide_start_speed();
            self.events.push(PlayerEvent::SlideStart);
        } else if !input.slide_down && (self.is_sliding() || self.is_air_sliding()) {
            self.exit_slide();
        }
    }

    fn slide_start_speed(&mut self) -> f32 {
        let incoming_speed = if self.vel.x.signum() == self.slide_dir {
            self.vel.x.abs()
        } else {
            0.0
        };
        let mut base_speed = incoming_speed.max(SLIDE_BOOST);
        if self.timers.bunny_hop > 0.0 && incoming_speed > SLIDE_BOOST * 0.85 {
            base_speed = (base_speed + SLIDE_CHAIN_BOOST).min(SLIDE_CHAIN_MAX_SPEED);
        }
        if self.timers.slam_jump == 0.0 || self.slam.stored_speed == 0.0 {
            return self.slide_dir * base_speed;
        }

        let speed = self.slam_bounce_speed();
        self.consume_slam_energy();
        self.slide_dir * (base_speed + speed * SLAM_SLIDE_SCALE_X).min(SLAM_DIVE_MAX_X)
    }

    fn enter_slide(&mut self) {
        let bottom = self.pos.y + self.half_size().y;
        // Preserve feet position so crouching does not lift or drop the player.
        self.size.y = PLAYER_SIZE.1 * SLIDE_HEIGHT_SCALE;
        self.pos.y = bottom - self.half_size().y;
        self.movement_state = MovementState::Sliding;
    }

    fn enter_air_slide(&mut self) {
        self.air_slide_wall_speed = self.air_slide_wall_speed.max(self.vel.x.abs());
        if self.size.y < PLAYER_SIZE.1 {
            self.movement_state = MovementState::AirSlide;
            return;
        }

        let bottom = self.pos.y + self.half_size().y;
        self.size.y = PLAYER_SIZE.1 * SLIDE_HEIGHT_SCALE;
        self.pos.y = bottom - self.half_size().y;
        self.movement_state = MovementState::AirSlide;
    }

    fn exit_slide(&mut self) {
        self.air_slide_wall_speed = 0.0;
        self.movement_state = MovementState::Crouching;
        self.events.push(PlayerEvent::SlideEnd);
    }

    fn finish_dash(&mut self) {
        if self.is_dashing() {
            self.timers.dash = 0.0;
            self.movement_state = MovementState::Normal;
            self.events.push(PlayerEvent::DashEnd);
        }
    }

    fn portal_slide_direction(&self, exit_normal: Vec2) -> f32 {
        if exit_normal.x.abs() > 0.25 {
            exit_normal.x.signum()
        } else if self.vel.x.abs() > 1.0 {
            self.vel.x.signum()
        } else {
            self.facing
        }
    }

    fn recover_dash(&mut self, dt: f32) {
        if self.is_sliding() || self.is_air_sliding() {
            return;
        }

        // Dash charges refill over time unless the player is sliding.
        self.dash_charges = (self.dash_charges + DASH_CHARGE_RECOVERY * dt).min(MAX_DASH_CHARGES);
    }

    fn store_wall_slam(&mut self) {
        if !self.is_ground_slamming() {
            return;
        }

        self.slam.stored_speed = self
            .slam
            .stored_speed
            .max(self.vel.y.max(SLAM_STORAGE_MIN_SPEED));
        self.slam.storage_ready = true;
        self.stop_ground_slam();
    }

    fn natural_slam_speed(&self) -> f32 {
        let fall_distance = (self.pos.y - self.slam.start_y).max(0.0);
        // Normal slam bounce ignores artificial slam acceleration.
        (self.slam.entry_speed.powi(2) + 2.0 * GRAVITY * fall_distance).sqrt()
    }

    fn slam_bounce_speed(&self) -> f32 {
        if self.slam.storage_ready {
            return (JUMP_VELOCITY + self.slam.stored_speed * SLAM_BOUNCE_SCALE)
                .min(SLAM_BOUNCE_MAX_SPEED);
        }

        // Height scales with velocity squared, so +10% height is sqrt(1.10) speed.
        let height_return = self.slam.stored_speed * SLAM_NORMAL_HEIGHT_GAIN.sqrt();
        let low_fall_boost = JUMP_VELOCITY + SLAM_NORMAL_MIN_BONUS;

        low_fall_boost
            .max(height_return)
            .min(SLAM_NORMAL_BOUNCE_MAX_SPEED)
    }

    fn consume_slam_energy(&mut self) {
        self.stop_ground_slam();
        self.timers.slam_jump = 0.0;
        self.slam.clear();
    }

    fn cancel_ground_slam(&mut self) {
        self.stop_ground_slam();
        self.timers.slam_jump = 0.0;
        self.slam.clear();
    }

    fn stop_ground_slam(&mut self) {
        if self.is_ground_slamming() {
            self.events.push(PlayerEvent::GroundSlamEnd);
            self.movement_state = MovementState::Normal;
        }
    }
}

fn approach(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        (current + delta).min(target)
    } else {
        (current - delta).max(target)
    }
}

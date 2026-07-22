use crate::constants::{
    BUNNY_HOP_GRACE, DASH_DENY_FLASH_TIME, DASH_DURATION, DASH_STORAGE_TIME, DIVE_WINDOW,
    JUMP_BUFFER_TIME, JUMP_COYOTE_TIME, PLAYER_DAMAGE_GRACE, PORTAL_EXIT_GRACE, SLAM_JUMP_WINDOW,
    VERTICAL_WALL_JUMP_WINDOW, WALL_STICK_TIME,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct MovementTimers {
    pub(super) bunny_hop: f32,
    pub(super) coyote: f32,
    pub(super) dash: f32,
    pub(super) dash_deny_flash: f32,
    pub(super) dash_storage: f32,
    pub(super) damage_grace: f32,
    pub(super) dive: f32,
    pub(super) jump_buffer: f32,
    pub(super) portal_exit: f32,
    pub(super) slam_jump: f32,
    pub(super) vertical_wall_jump: f32,
    pub(super) wall: f32,
}

impl MovementTimers {
    pub(super) fn new() -> Self {
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

    pub(super) fn tick(&mut self, dt: f32) {
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

    pub(super) fn start_bunny_hop(&mut self) {
        self.bunny_hop = BUNNY_HOP_GRACE;
    }

    pub(super) fn start_coyote(&mut self) {
        self.coyote = JUMP_COYOTE_TIME;
    }

    pub(super) fn start_dash(&mut self) {
        self.dash = DASH_DURATION;
    }

    pub(super) fn flash_dash_deny(&mut self) {
        self.dash_deny_flash = DASH_DENY_FLASH_TIME;
    }

    pub(super) fn start_dash_storage(&mut self) {
        self.dash_storage = DASH_STORAGE_TIME;
    }

    pub(super) fn start_damage_grace(&mut self) {
        self.damage_grace = PLAYER_DAMAGE_GRACE;
    }

    pub(super) fn start_dive_window(&mut self) {
        self.dive = DIVE_WINDOW;
    }

    pub(super) fn start_jump_buffer(&mut self) {
        self.jump_buffer = JUMP_BUFFER_TIME;
    }

    pub(super) fn start_portal_exit(&mut self) {
        self.portal_exit = PORTAL_EXIT_GRACE;
    }

    pub(super) fn start_slam_jump(&mut self) {
        self.slam_jump = SLAM_JUMP_WINDOW;
    }

    pub(super) fn start_vertical_wall_jump(&mut self) {
        self.vertical_wall_jump = VERTICAL_WALL_JUMP_WINDOW;
    }

    pub(super) fn start_wall_stick(&mut self) {
        self.wall = WALL_STICK_TIME;
    }
}

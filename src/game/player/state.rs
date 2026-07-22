use glam::Vec2;

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

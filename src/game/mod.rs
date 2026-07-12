pub mod player;
pub mod portal;

use glam::Vec2;

use crate::game::player::Player;
use crate::game::portal::{Color, Portal};
use crate::platform::input::Input;

pub struct World {
    pub player: Player,
    pub portals: [Portal; 2],
}

impl World {
    pub fn new() -> Self {
        let mut portal_a = Portal::new(1, 750.0, 300.0, Vec2::new(-1.0, 0.0), 100.0, Color::BLUE);
        let mut portal_b = Portal::new(2, 50.0, 300.0, Vec2::new(1.0, 0.0), 100.0, Color::ORANGE);

        portal_a.scale = 4.0;
        portal_b.scale = 2.0;

        Self {
            player: Player::new(160.0, 430.0),
            portals: [portal_a, portal_b],
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.player
            .update(dt, input.move_x, input.aim_pos, screen_width, screen_height);

        let [portal_a, portal_b] = self.portals;
        if !portal_a.check_coll(&portal_b, &mut self.player) {
            portal_b.check_coll(&portal_a, &mut self.player);
        }
    }
}

pub mod player;
pub mod portal;

use glam::Vec2;

use crate::constants::TELEPORT_COOLDOWN;
use crate::game::player::Player;
use crate::game::portal::{Color, Portal};
use crate::platform::input::Input;

pub struct World {
    pub player: Player,
    pub portals: [Portal; 2],
    teleport_cooldown: f32,
}

impl World {
    pub fn new() -> Self {
        let mut portal_a = Portal::new(750.0, 300.0, Vec2::new(-1.0, 0.0), 100.0, Color::BLUE);
        let mut portal_b = Portal::new(50.0, 300.0, Vec2::new(1.0, 0.0), 100.0, Color::ORANGE);

        portal_a.scale = 4.0;
        portal_b.scale = 2.0;

        Self {
            player: Player::new(160.0, 430.0),
            portals: [portal_a, portal_b],
            teleport_cooldown: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.teleport_cooldown = (self.teleport_cooldown - dt).max(0.0);

        self.player.update(dt, input, screen_width, screen_height);

        if self.teleport_cooldown == 0.0 && self.try_teleport() {
            self.teleport_cooldown = TELEPORT_COOLDOWN;
            self.player.constrain_to_screen(screen_width, screen_height);
        }
    }

    fn try_teleport(&mut self) -> bool {
        let previous = self.player.prev_pos;
        let current = self.player.pos;
        let half_size = self.player.half_size();

        let [source, destination] = self.portals;
        if source.intersects_sweep(previous, current, half_size) {
            source.teleport_player_to(&destination, &mut self.player);
            return true;
        }

        let [destination, source] = self.portals;
        if source.intersects_sweep(previous, current, half_size) {
            source.teleport_player_to(&destination, &mut self.player);
            return true;
        }

        false
    }
}

pub mod level;
pub mod player;
pub mod portal;

#[cfg(test)]
mod tests;

use crate::constants::{PORTAL_WIDTH, TELEPORT_COOLDOWN};
use crate::game::level::Level;
use crate::game::player::Player;
use crate::game::portal::{Color, Portal};
use crate::platform::input::Input;

pub struct World {
    pub level: Level,
    pub player: Player,
    pub portals: [Option<Portal>; 2],
    teleport_cooldown: f32,
}

impl World {
    pub fn new() -> Self {
        Self {
            level: Level::test_level(),
            player: Player::new(110.0, 480.0),
            portals: [None, None],
            teleport_cooldown: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.teleport_cooldown = (self.teleport_cooldown - dt).max(0.0);

        self.player.update(dt, input, screen_width, screen_height);
        self.level.resolve_player(&mut self.player);
        self.shoot_portals(input);

        // Teleport is checked after movement so sweep uses this frame's path.
        if self.teleport_cooldown == 0.0 && self.try_teleport() {
            self.teleport_cooldown = TELEPORT_COOLDOWN;
            self.level.resolve_player(&mut self.player);
        }
    }

    fn shoot_portals(&mut self, input: &Input) {
        if input.blue_portal_pressed {
            self.place_portal(0, Color::BLUE);
        }
        if input.orange_portal_pressed {
            self.place_portal(1, Color::ORANGE);
        }
    }

    fn place_portal(&mut self, index: usize, color: Color) {
        let origin = self.player.aim_from();
        let aim = self.player.aim_pos - origin;
        if aim.length_squared() <= 1.0 {
            return;
        }

        let target = origin + aim.normalize() * 2_000.0;
        let Some(hit) = self.level.raycast_portalable(origin, target) else {
            return;
        };
        // Tiny surfaces cannot hold the whole portal line.
        if hit.surface_span < PORTAL_WIDTH {
            return;
        }

        self.portals[index] = Some(Portal::new(
            hit.point.x,
            hit.point.y,
            hit.normal,
            PORTAL_WIDTH,
            color,
        ));
    }

    fn try_teleport(&mut self) -> bool {
        let previous = self.player.prev_pos;
        let current = self.player.pos;
        let half_size = self.player.half_size();

        let [Some(source), Some(destination)] = self.portals else {
            return false;
        };

        if source.intersects_sweep(previous, current, half_size) {
            source.teleport_player_to(&destination, &mut self.player);
            return true;
        }

        if destination.intersects_sweep(previous, current, half_size) {
            destination.teleport_player_to(&source, &mut self.player);
            return true;
        }

        false
    }
}

pub mod level;
pub mod player;
pub mod portal;

#[cfg(test)]
mod tests;

use crate::constants::{PORTAL_MIN_DISTANCE, PORTAL_WIDTH, TELEPORT_COOLDOWN};
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

        // Movement owns velocity; level collision corrects the final position.
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

        // The portal gun shoots past the cursor until the first valid wall.
        let target = origin + aim.normalize() * 2_000.0;
        let Some(hit) = self.level.raycast_portalable(origin, target) else {
            return;
        };
        // Near edges, the hit point is shifted along the wall so the portal fits.
        let Some(center) = hit.portal_center(PORTAL_WIDTH) else {
            return;
        };
        // Prevent overlapping portal pairs; they become unreadable and unstable.
        if self.portal_too_close(index, center) {
            return;
        }

        self.portals[index] = Some(Portal::new(
            center.x,
            center.y,
            hit.normal,
            PORTAL_WIDTH,
            color,
        ));
    }

    fn portal_too_close(&self, index: usize, center: glam::Vec2) -> bool {
        self.portals
            .iter()
            .enumerate()
            .any(|(other_index, portal)| {
                other_index != index
                    && portal
                        .is_some_and(|portal| portal.pos.distance(center) < PORTAL_MIN_DISTANCE)
            })
    }

    fn try_teleport(&mut self) -> bool {
        let previous = self.player.prev_pos;
        let current = self.player.pos;
        let half_size = self.player.half_size();

        // A pair is required; a single portal is only a visual marker.
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

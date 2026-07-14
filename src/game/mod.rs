pub mod level;
pub mod levels;
pub mod player;
pub mod portal;

// World owns gameplay simulation and keeps rendering out of physics.
#[cfg(test)]
mod tests;

use crate::constants::{PORTAL_MIN_DISTANCE, PORTAL_WIDTH};
use crate::game::level::{DoorEvent, Level};
use crate::game::levels::LevelSpec;
use crate::game::player::{Player, PlayerEvent};
use crate::game::portal::{Color, Portal};
use crate::platform::input::Input;

const MAX_STEP_DISTANCE: f32 = 8.0;
const MAX_PHYSICS_STEPS: usize = 96;
const FOOTSTEP_MIN_SPEED: f32 = 70.0;

pub struct World {
    pub level: Level,
    pub player: Player,
    pub portals: [Option<Portal>; 2],
    respawn_pos: glam::Vec2,
    sound_events: Vec<SoundEvent>,
    footstep_timer: f32,
    footstep_index: usize,
}

#[derive(Clone, Copy)]
pub enum SoundEvent {
    DoorOpen { index: usize, pos: glam::Vec2 },
    DoorClose { index: usize, pos: glam::Vec2 },
    DoorStop { index: usize },
    Footstep(usize, glam::Vec2),
    Jump(glam::Vec2),
    DashStart(glam::Vec2),
    DashEnd,
    SlideStart(glam::Vec2),
    SlideEnd,
    GroundSlamStart(glam::Vec2),
    GroundSlamEnd,
    Land,
    HeavyLand(glam::Vec2),
    PortalFire(glam::Vec2),
    PortalPlace(glam::Vec2),
}

impl World {
    pub fn new() -> Self {
        Self::from_level(&LevelSpec::fallback())
    }

    pub fn from_level(level: &LevelSpec) -> Self {
        Self {
            level: level.level(),
            player: Player::new(level.spawn.x, level.spawn.y),
            portals: [None, None],
            respawn_pos: level.spawn,
            sound_events: Vec::new(),
            footstep_timer: 0.0,
            footstep_index: 0,
        }
    }

    pub fn load_level(&mut self, level: &LevelSpec) {
        self.level = level.level();
        self.player = Player::new(level.spawn.x, level.spawn.y);
        self.portals = [None, None];
        self.respawn_pos = level.spawn;
        self.sound_events.clear();
        self.footstep_timer = 0.0;
        self.footstep_index = 0;
    }

    pub fn update(&mut self, dt: f32, input: &Input, screen_width: f32, screen_height: f32) {
        self.sound_events.clear();
        let distance = (self.player.vel.length() * dt).max(1.0);
        let steps = ((distance / MAX_STEP_DISTANCE).ceil() as usize).clamp(1, MAX_PHYSICS_STEPS);
        let step_dt = dt / steps as f32;
        let mut step_input = input.clone();

        for step in 0..steps {
            // Movement owns velocity; portals and level collision correct the final position.
            self.player
                .update(step_dt, &step_input, screen_width, screen_height);
            let events = self.level.update_doors(self.player.pos, step_dt);
            let door_sounds = events.into_iter().map(|(index, event)| {
                let pos = self.level.doors[index].solid.center();
                match event {
                    DoorEvent::Opening => SoundEvent::DoorOpen { index, pos },
                    DoorEvent::Closing => SoundEvent::DoorClose { index, pos },
                    DoorEvent::Opened | DoorEvent::Closed => SoundEvent::DoorStop { index },
                }
            });
            self.sound_events.extend(door_sounds);
            self.try_teleport();
            self.resolve_player();
            self.check_level_triggers();
            self.collect_player_events();
            self.tick_footsteps(step_dt);

            if step == 0 {
                self.shoot_portals(&step_input);
                step_input.consume_presses();
            }
        }
    }

    pub fn drain_sound_events(&mut self) -> impl Iterator<Item = SoundEvent> + '_ {
        self.sound_events.drain(..)
    }

    fn shoot_portals(&mut self, input: &Input) {
        if input.blue_portal_pressed {
            self.sound_events
                .push(SoundEvent::PortalFire(self.player.pos));
            self.place_portal(0, Color::BLUE);
        }
        if input.orange_portal_pressed {
            self.sound_events
                .push(SoundEvent::PortalFire(self.player.pos));
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
        let Some(center) = self.level.portal_center(hit, PORTAL_WIDTH) else {
            return;
        };
        // Prevent overlapping portal pairs; they become unreadable and unstable.
        if self.portal_too_close(index, center) {
            return;
        }

        self.portals[index] = Some(Portal::with_tangent(
            center.x,
            center.y,
            hit.normal,
            hit.tangent,
            PORTAL_WIDTH,
            color,
        ));
        self.sound_events.push(SoundEvent::PortalPlace(center));
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

        let source_time = source.crossing_time(previous, current, half_size);
        let destination_time = destination.crossing_time(previous, current, half_size);
        match (source_time, destination_time) {
            (Some(source_time), Some(destination_time)) if destination_time < source_time => {
                destination.teleport_player_to(&source, &mut self.player);
                true
            }
            (Some(_), _) => {
                source.teleport_player_to(&destination, &mut self.player);
                true
            }
            (_, Some(_)) => {
                destination.teleport_player_to(&source, &mut self.player);
                true
            }
            _ => false,
        }
    }

    fn resolve_player(&mut self) {
        if let [Some(source), Some(destination)] = self.portals {
            self.level
                .resolve_player_with_portals(&mut self.player, &[source, destination]);
        } else {
            self.level.resolve_player(&mut self.player);
        }
    }

    fn collect_player_events(&mut self) {
        let player_pos = self.player.pos;
        let events = self.player.drain_events().map(|event| match event {
            PlayerEvent::Jump => SoundEvent::Jump(player_pos),
            PlayerEvent::DashStart => SoundEvent::DashStart(player_pos),
            PlayerEvent::DashEnd => SoundEvent::DashEnd,
            PlayerEvent::SlideStart => SoundEvent::SlideStart(player_pos),
            PlayerEvent::SlideEnd => SoundEvent::SlideEnd,
            PlayerEvent::GroundSlamStart => SoundEvent::GroundSlamStart(player_pos),
            PlayerEvent::GroundSlamEnd => SoundEvent::GroundSlamEnd,
            PlayerEvent::Land => SoundEvent::Land,
            PlayerEvent::HeavyLand => SoundEvent::HeavyLand(player_pos),
        });

        self.sound_events.extend(events);
    }

    fn tick_footsteps(&mut self, dt: f32) {
        let speed = self.player.vel.x.abs();
        let walking = self.player.on_ground
            && !self.player.sliding
            && !self.player.dashing
            && speed > FOOTSTEP_MIN_SPEED;

        if !walking {
            self.footstep_timer = 0.0;
            return;
        }

        self.footstep_timer -= dt * (speed / 220.0).clamp(0.75, 1.45);
        if self.footstep_timer > 0.0 {
            return;
        }

        self.sound_events
            .push(SoundEvent::Footstep(self.footstep_index, self.player.pos));
        self.footstep_index = (self.footstep_index + 1) % 3;
        self.footstep_timer = 0.22;
    }

    fn check_level_triggers(&mut self) {
        let center = self.player.pos;
        let half_size = self.player.half_size();

        // Checkpoints are volumes: touching one moves the respawn target to its center.
        for checkpoint in &self.level.checkpoints {
            if checkpoint.solid().overlaps_aabb(center, half_size) {
                self.respawn_pos = checkpoint.center();
            }
        }

        // Hazards reset the player to the last checkpoint and clear transient portal state.
        let hit_hazard = self
            .level
            .hazards
            .iter()
            .any(|hazard| hazard.solid.overlaps_aabb(center, half_size));
        if hit_hazard {
            self.respawn_player();
        }
    }

    fn respawn_player(&mut self) {
        self.player = Player::new(self.respawn_pos.x, self.respawn_pos.y);
        self.portals = [None, None];
        self.sound_events
            .push(SoundEvent::HeavyLand(self.respawn_pos));
    }
}

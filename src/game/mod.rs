pub mod enemy;
pub mod geometry;
pub mod level;
pub mod levels;
pub mod player;
pub mod portal;

// World owns gameplay simulation and keeps rendering out of physics.
#[cfg(test)]
mod tests;

use crate::constants::{
    PIERCE_ALT_CHARGE_TIME, PIERCE_ALT_COOLDOWN, PIERCE_ALT_DAMAGE, PIERCE_BEAM_TIME,
    PIERCE_PRIMARY_COOLDOWN, PIERCE_PRIMARY_DAMAGE, PIERCE_RANGE, PLAYER_DAMAGE_PULSE_TIME,
    PLAYER_DEATH_LAUGH_LOOP_TIME, PLAYER_DEATH_PROMPT_TIME, PLAYER_DEATH_TEXT_RATE,
    PLAYER_MAX_HEALTH, PORTAL_MIN_DISTANCE, PORTAL_WIDTH,
};
use crate::game::level::{CollisionGeometry, DoorEvent, Level, WorldPortal};
use crate::game::levels::LevelSpec;
use crate::game::player::{MovementInput, Player, PlayerEvent};
use crate::game::portal::{Color, Portal};
use crate::platform::input::{GameKey, Input};

const MAX_STEP_DISTANCE: f32 = 8.0;
const MAX_PHYSICS_STEPS: usize = 96;
const FOOTSTEP_MIN_SPEED: f32 = 70.0;
const WORLD_PORTAL_COOLDOWN_STEPS: u8 = 2;

pub struct World {
    pub level: Level,
    pub player: Player,
    pub portals: [Option<Portal>; 2],
    pub piercer: PiercerState,
    pub death: Option<DeathSequence>,
    pub damage_pulse: DamagePulse,
    respawn_pos: glam::Vec2,
    sound_events: Vec<SoundEvent>,
    door_events: Vec<(usize, DoorEvent)>,
    footstep_timer: f32,
    footstep_index: usize,
    camera_shift: glam::Vec2,
    world_portal_cooldown: u8,
    collision_portals: Vec<Portal>,
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
    PlayerHurt(glam::Vec2),
    DeathSequence,
    DeathSkull,
    DeathStop,
    FilthBite(glam::Vec2),
    PiercerChargeStart(glam::Vec2),
    PiercerChargeStop,
    PiercerFire(glam::Vec2),
    PiercerCharged(glam::Vec2),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponSlot {
    Piercer,
}

#[derive(Clone, Copy, Debug)]
pub struct PiercerState {
    pub selected: WeaponSlot,
    pub primary_cooldown: f32,
    pub alt_cooldown: f32,
    pub alt_charge: f32,
    pub alt_charging: bool,
    pub beam: Option<PiercerBeam>,
}

#[derive(Clone, Copy, Debug)]
pub struct PiercerBeam {
    pub start: glam::Vec2,
    pub end: glam::Vec2,
    pub charged: bool,
    pub time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct DeathSequence {
    pub timer: f32,
    pub text_chars: usize,
    pub skull_frame: usize,
    laugh_loop_timer: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct DamagePulse {
    pub timer: f32,
    pub strength: f32,
}

impl PiercerState {
    fn new() -> Self {
        Self {
            selected: WeaponSlot::Piercer,
            primary_cooldown: 0.0,
            alt_cooldown: 0.0,
            alt_charge: 0.0,
            alt_charging: false,
            beam: None,
        }
    }

    fn tick(&mut self, dt: f32) {
        self.primary_cooldown = (self.primary_cooldown - dt).max(0.0);
        self.alt_cooldown = (self.alt_cooldown - dt).max(0.0);
        if self.alt_charging {
            self.alt_charge += dt;
        }
        if let Some(beam) = &mut self.beam {
            beam.time -= dt;
            if beam.time <= 0.0 {
                self.beam = None;
            }
        }
    }
}

impl DeathSequence {
    fn new() -> Self {
        Self {
            timer: 0.0,
            text_chars: 0,
            skull_frame: 0,
            laugh_loop_timer: 0.0,
        }
    }

    pub fn prompt_ready(self) -> bool {
        self.timer >= PLAYER_DEATH_PROMPT_TIME
    }

    pub fn camera_offset(self) -> glam::Vec2 {
        if self.prompt_ready() {
            return glam::Vec2::ZERO;
        }

        let t = (self.timer / PLAYER_DEATH_PROMPT_TIME).clamp(0.0, 1.0);
        let collapse = t.powf(2.15);
        let late = ((t - 0.62) / 0.38).clamp(0.0, 1.0);
        let x = (self.timer * 4.2).sin() * 5.0 * (0.35 + late * 0.65);
        let y = collapse * 210.0 + (self.timer * 6.0).sin().max(0.0) * late * 8.0;

        glam::Vec2::new(x, y)
    }

    pub fn camera_zoom(self) -> f32 {
        if self.prompt_ready() {
            return 1.0;
        }

        let t = (self.timer / PLAYER_DEATH_PROMPT_TIME).clamp(0.0, 1.0);
        1.0 + t * 0.025
    }
}

impl DamagePulse {
    fn new() -> Self {
        Self {
            timer: 0.0,
            strength: 0.0,
        }
    }

    fn trigger(&mut self, damage: f32) {
        self.timer = PLAYER_DAMAGE_PULSE_TIME;
        self.strength = (0.28 + damage / PLAYER_MAX_HEALTH * 1.15).clamp(0.32, 0.95);
    }

    fn tick(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
        if self.timer == 0.0 {
            self.strength = 0.0;
        }
    }

    pub fn amount(self) -> f32 {
        if self.timer <= 0.0 {
            return 0.0;
        }

        let t = 1.0 - (self.timer / PLAYER_DAMAGE_PULSE_TIME).clamp(0.0, 1.0);
        let attack = (t / 0.18).clamp(0.0, 1.0);
        let release = (1.0 - t).powf(1.85);
        self.strength * attack * release
    }
}

impl World {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::from_level(&LevelSpec::fallback())
    }

    pub fn from_level(level: &LevelSpec) -> Self {
        Self {
            level: level.level(),
            player: Player::new(level.spawn.x, level.spawn.y),
            portals: [None, None],
            piercer: PiercerState::new(),
            death: None,
            damage_pulse: DamagePulse::new(),
            respawn_pos: level.spawn,
            sound_events: Vec::new(),
            door_events: Vec::new(),
            footstep_timer: 0.0,
            footstep_index: 0,
            camera_shift: glam::Vec2::ZERO,
            world_portal_cooldown: 0,
            collision_portals: Vec::new(),
        }
    }

    pub fn load_level(&mut self, level: &LevelSpec) {
        self.level = level.level();
        self.player = Player::new(level.spawn.x, level.spawn.y);
        self.portals = [None, None];
        self.piercer = PiercerState::new();
        self.death = None;
        self.damage_pulse = DamagePulse::new();
        self.respawn_pos = level.spawn;
        self.sound_events.clear();
        self.door_events.clear();
        self.footstep_timer = 0.0;
        self.footstep_index = 0;
        self.camera_shift = glam::Vec2::ZERO;
        self.world_portal_cooldown = 0;
        self.collision_portals.clear();
    }

    pub fn update(&mut self, dt: f32, input: &Input, _screen_width: f32, _screen_height: f32) {
        self.sound_events.clear();
        self.damage_pulse.tick(dt);
        if self.update_death(dt, input) {
            return;
        }

        self.piercer.tick(dt);
        let distance = (self.player.vel.length() * dt).max(1.0);
        // Long frames are subdivided so a fast player cannot tunnel through thin geometry.
        let steps = ((distance / MAX_STEP_DISTANCE).ceil() as usize).clamp(1, MAX_PHYSICS_STEPS);
        let step_dt = dt / steps as f32;
        let mut step_input = input.clone();

        for step in 0..steps {
            // Movement owns velocity; portals and level collision correct the final position.
            self.player
                .update_movement(step_dt, MovementInput::from(&step_input));
            self.tick_doors(step_dt);
            self.try_teleport(MovementInput::from(&step_input));
            self.resolve_player();
            self.tick_enemies(step_dt);
            self.check_level_triggers();
            self.collect_player_events();
            self.tick_footsteps(step_dt);

            if step == 0 {
                self.update_weapons(&step_input);
                self.shoot_portals(&step_input);
                step_input.consume_presses();
            }
        }
    }

    pub fn drain_sound_events(&mut self) -> impl Iterator<Item = SoundEvent> + '_ {
        self.sound_events.drain(..)
    }

    pub fn take_camera_shift(&mut self) -> glam::Vec2 {
        std::mem::take(&mut self.camera_shift)
    }

    fn tick_doors(&mut self, dt: f32) {
        self.door_events.clear();
        self.level
            .update_doors(self.player.pos, dt, &mut self.door_events);
        for (index, event) in self.door_events.iter().copied() {
            let pos = self.level.doors[index].solid.center();
            let sound = match event {
                DoorEvent::Opening => SoundEvent::DoorOpen { index, pos },
                DoorEvent::Closing => SoundEvent::DoorClose { index, pos },
                DoorEvent::Opened | DoorEvent::Closed => SoundEvent::DoorStop { index },
            };
            self.sound_events.push(sound);
        }
    }

    fn shoot_portals(&mut self, input: &Input) {
        if input.key_pressed(GameKey::BluePortal) {
            self.sound_events
                .push(SoundEvent::PortalFire(self.player.pos));
            self.place_portal(0, Color::BLUE);
        }
        if input.key_pressed(GameKey::OrangePortal) {
            self.sound_events
                .push(SoundEvent::PortalFire(self.player.pos));
            self.place_portal(1, Color::ORANGE);
        }
    }

    fn update_weapons(&mut self, input: &Input) {
        if input.weapon_1_pressed() {
            self.piercer.selected = WeaponSlot::Piercer;
        }
        if self.piercer.selected != WeaponSlot::Piercer {
            return;
        }

        if self.piercer.alt_charging {
            if self.piercer.alt_charge >= PIERCE_ALT_CHARGE_TIME && input.primary_fire_pressed() {
                self.finish_piercer_charge(true);
            } else if input.alt_fire_released() {
                let charged = self.piercer.alt_charge >= PIERCE_ALT_CHARGE_TIME;
                self.finish_piercer_charge(charged);
            }
            return;
        }

        if input.primary_fire_down() && self.piercer.primary_cooldown <= 0.0 {
            self.fire_piercer(false);
            self.piercer.primary_cooldown = PIERCE_PRIMARY_COOLDOWN;
        }

        if input.alt_fire_down() && self.piercer.alt_cooldown <= 0.0 && !self.piercer.alt_charging {
            self.piercer.alt_charging = true;
            self.sound_events
                .push(SoundEvent::PiercerChargeStart(self.player.aim_from()));
        }
    }

    fn finish_piercer_charge(&mut self, charged: bool) {
        self.piercer.alt_charging = false;
        self.piercer.alt_charge = 0.0;
        self.sound_events.push(SoundEvent::PiercerChargeStop);

        if charged {
            self.fire_piercer(true);
            self.piercer.alt_cooldown = PIERCE_ALT_COOLDOWN;
        }
    }

    fn fire_piercer(&mut self, charged: bool) {
        let origin = self.player.aim_from();
        let aim = self.player.aim_pos - origin;
        if aim.length_squared() <= 1.0 {
            return;
        }

        let dir = aim.normalize();
        let max_end = origin + dir * PIERCE_RANGE;
        let wall_hit = self
            .level
            .raycast_any_solid(origin, max_end)
            .map(|hit| hit.point);
        let wall_distance = wall_hit
            .map(|point| point.distance(origin))
            .unwrap_or(PIERCE_RANGE);
        let damage = if charged {
            PIERCE_ALT_DAMAGE
        } else {
            PIERCE_PRIMARY_DAMAGE
        };

        for enemy in &mut self.level.enemies {
            let Some(distance) =
                ray_hits_enemy(origin, dir, wall_distance, enemy.pos, enemy.half_size())
            else {
                continue;
            };
            if distance <= wall_distance && enemy.damage(damage) && !charged {
                break;
            }
        }
        self.level.enemies.retain(|enemy| enemy.health > 0.0);

        let end = wall_hit.unwrap_or(max_end);
        self.piercer.beam = Some(PiercerBeam {
            start: origin,
            end,
            charged,
            time: PIERCE_BEAM_TIME,
        });
        self.sound_events.push(if charged {
            SoundEvent::PiercerCharged(origin)
        } else {
            SoundEvent::PiercerFire(origin)
        });
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

    fn try_teleport(&mut self, input: MovementInput) -> bool {
        let previous = self.player.prev_pos;
        let current = self.player.pos;
        let half_size = self.player.half_size();

        let used_player_portal = if let [Some(source), Some(destination)] = self.portals {
            let source_time = source.crossing_time(previous, current, half_size);
            let destination_time = destination.crossing_time(previous, current, half_size);

            match (source_time, destination_time) {
                // A sweep can cross both planes in one step; teleport through the first one hit.
                (Some(source_time), Some(destination_time)) if destination_time < source_time => {
                    destination.teleport_player_to(&source, &mut self.player);
                    self.player.on_player_portal_exit(source.normal(), input);
                    true
                }
                (Some(_), _) => {
                    source.teleport_player_to(&destination, &mut self.player);
                    self.player
                        .on_player_portal_exit(destination.normal(), input);
                    true
                }
                (_, Some(_)) => {
                    destination.teleport_player_to(&source, &mut self.player);
                    self.player.on_player_portal_exit(source.normal(), input);
                    true
                }
                _ => false,
            }
        } else {
            false
        };

        if used_player_portal {
            return true;
        }

        if self.world_portal_cooldown > 0 {
            self.world_portal_cooldown -= 1;
            return false;
        }

        self.try_world_portal_teleport(input)
    }

    fn try_world_portal_teleport(&mut self, input: MovementInput) -> bool {
        let previous = self.player.prev_pos;
        let current = self.player.pos;
        let half_size = self.player.half_size();
        let portals = &self.level.world_portals;
        let mut best = None;

        for (source_index, source) in portals.iter().enumerate() {
            let Some(time) = source.portal.crossing_time(previous, current, half_size) else {
                continue;
            };
            let Some(destination_index) = WorldPortal::receiver_index(portals, source_index) else {
                continue;
            };

            // Multiple world portals may overlap, so retain the earliest crossing only.
            if best.is_none_or(|(best_time, _, _)| time < best_time) {
                best = Some((time, source_index, destination_index));
            }
        }

        let Some((_, source_index, destination_index)) = best else {
            return false;
        };
        let source = portals[source_index].portal;
        let destination = portals[destination_index].portal;
        let camera_before = self.player.pos;

        source.teleport_player_to(&destination, &mut self.player);
        self.camera_shift += self.player.pos - camera_before;
        self.world_portal_cooldown = WORLD_PORTAL_COOLDOWN_STEPS;
        self.player
            .on_player_portal_exit(destination.normal(), input);
        true
    }

    fn resolve_player(&mut self) {
        self.collision_portals.clear();
        if let [Some(source), Some(destination)] = self.portals {
            self.collision_portals.extend([source, destination]);
        }

        self.collision_portals
            .extend(
                self.level
                    .world_portals
                    .iter()
                    .enumerate()
                    .filter_map(|(index, portal)| {
                        WorldPortal::receiver_index(&self.level.world_portals, index)
                            .map(|_| portal.portal)
                    }),
            );

        self.level
            .resolve_player_with_portals(&mut self.player, &self.collision_portals);
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
            && !self.player.is_sliding()
            && !self.player.is_dashing()
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

    fn tick_enemies(&mut self, dt: f32) {
        let player_pos = self.player.pos;
        let mut damage = 0.0;
        let collision = CollisionGeometry::new(&self.level.solids, &self.level.doors);

        for enemy in &mut self.level.enemies {
            if let Some(amount) = enemy.update(dt, player_pos, collision) {
                damage += amount;
                self.sound_events.push(SoundEvent::FilthBite(enemy.pos));
            }
        }

        if damage > 0.0 {
            let health_before = self.player.health;
            let killed = self.player.damage(damage);

            if self.player.health < health_before {
                let applied_damage = health_before - self.player.health;
                self.sound_events
                    .push(SoundEvent::PlayerHurt(self.player.pos));
                self.damage_pulse.trigger(applied_damage);
            }

            if killed {
                self.start_death_sequence();
            }
        }
    }

    fn update_death(&mut self, dt: f32, input: &Input) -> bool {
        let Some(death) = &mut self.death else {
            return false;
        };

        death.timer += dt;
        death.text_chars = (death.timer / PLAYER_DEATH_TEXT_RATE) as usize;
        if death.prompt_ready() {
            death.laugh_loop_timer -= dt;
        }
        if death.prompt_ready() && death.laugh_loop_timer <= 0.0 {
            death.laugh_loop_timer = PLAYER_DEATH_LAUGH_LOOP_TIME;
            death.skull_frame = 1;
            self.sound_events.push(SoundEvent::DeathSkull);
        } else if death.prompt_ready()
            && death.laugh_loop_timer <= PLAYER_DEATH_LAUGH_LOOP_TIME * 0.35
        {
            death.skull_frame = 0;
        }

        let should_respawn = input.respawn_pressed();

        if should_respawn {
            self.respawn_player();
        }

        true
    }

    fn start_death_sequence(&mut self) {
        if self.death.is_some() {
            return;
        }

        self.death = Some(DeathSequence::new());
        self.piercer.alt_charging = false;
        self.piercer.beam = None;
        self.sound_events.push(SoundEvent::DeathSequence);
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
        self.piercer = PiercerState::new();
        self.death = None;
        self.damage_pulse = DamagePulse::new();
        self.sound_events.push(SoundEvent::DeathStop);
        self.sound_events
            .push(SoundEvent::HeavyLand(self.respawn_pos));
    }
}

fn ray_hits_enemy(
    origin: glam::Vec2,
    dir: glam::Vec2,
    max_distance: f32,
    center: glam::Vec2,
    half_size: glam::Vec2,
) -> Option<f32> {
    let min = center - half_size;
    let max = center + half_size;
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = max_distance;

    for axis in 0..2 {
        let origin_axis = origin[axis];
        let dir_axis = dir[axis];
        let min_axis = min[axis];
        let max_axis = max[axis];

        if dir_axis.abs() < 0.001 {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }

        let inv_dir = 1.0 / dir_axis;
        let t1 = (min_axis - origin_axis) * inv_dir;
        let t2 = (max_axis - origin_axis) * inv_dir;
        t_min = t_min.max(t1.min(t2));
        t_max = t_max.min(t1.max(t2));
        if t_min > t_max {
            return None;
        }
    }

    (t_min >= 0.0 && t_min <= max_distance).then_some(t_min)
}

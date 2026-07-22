use std::collections::HashMap;
use std::io::Cursor;

use glam::Vec2;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::game::SoundEvent;

const DOOR_OPEN: &[u8] = include_bytes!("../../assets/sounds/door_open.wav");
const DOOR_CLOSE: &[u8] = include_bytes!("../../assets/sounds/door_close.wav");
const FOOTSTEP_1: &[u8] = include_bytes!("../../assets/sounds/footstep_stone1.wav");
const FOOTSTEP_2: &[u8] = include_bytes!("../../assets/sounds/footstep_stone2.wav");
const FOOTSTEP_3: &[u8] = include_bytes!("../../assets/sounds/footstep_stone3.wav");
const JUMP: &[u8] = include_bytes!("../../assets/sounds/jump.wav");
const DASH: &[u8] = include_bytes!("../../assets/sounds/dash.wav");
const SLIDE: &[u8] = include_bytes!("../../assets/sounds/slide.wav");
const GROUND_SLAM: &[u8] = include_bytes!("../../assets/sounds/ground_slam.wav");
const LAND_HEAVY: &[u8] = include_bytes!("../../assets/sounds/land_heavy.wav");
const MENU_MUSIC: &[u8] = include_bytes!("../../assets/sounds/menu_the_fire_is_gone.wav");
const PORTAL_FIRE: &[u8] = include_bytes!("../../assets/sounds/portal_fire.wav");
const PORTAL_PLACE: &[u8] = include_bytes!("../../assets/sounds/portal_place.wav");
const V1_HURT: &[u8] = include_bytes!("../../assets/sounds/death/V1_hurt.wav");
const DEATH_SEQUENCE: &[u8] = include_bytes!("../../assets/sounds/death/DeathSequence.wav");
const DEATH_SKULL: &[u8] = include_bytes!("../../assets/sounds/death/8bitAhh.wav");
const DEATH_CAMERA_CUT: &[u8] =
    include_bytes!("../../assets/sounds/death/camera_cutting_out_in_death_seq.wav");
const PIERCER_CHARGE: &[u8] = include_bytes!("../../assets/sounds/weapons/PierceCharge.wav");
const PIERCER_SHOT_1: &[u8] = include_bytes!("../../assets/sounds/weapons/Shoot1.wav");
const PIERCER_SHOT_2: &[u8] = include_bytes!("../../assets/sounds/weapons/Shoot1c3.wav");
const PIERCER_SHOT_3: &[u8] = include_bytes!("../../assets/sounds/weapons/Shoot1c4.wav");
const FILTH_BITE: &[u8] =
    include_bytes!("../../assets/sounds/enemies/Zombie_Weak_Death_Reverse.wav");

struct OneShotSound {
    bytes: &'static [u8],
    volume: f32,
}

pub(super) struct Audio {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    dash: Option<Sink>,
    slide: Option<Sink>,
    ground_slam: Option<Sink>,
    piercer_charge: Option<Sink>,
    death_sequence: Option<Sink>,
    death_skull: Option<Sink>,
    death_camera_cut: Option<Sink>,
    menu_music: Option<Sink>,
    menu_falling: Option<Sink>,
    doors: HashMap<usize, Sink>,
    piercer_shot_index: usize,
    master_volume: f32,
    sfx_volume: f32,
    music_volume: f32,
}

impl Audio {
    pub(super) fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Some(stream),
                handle: Some(handle),
                dash: None,
                slide: None,
                ground_slam: None,
                piercer_charge: None,
                death_sequence: None,
                death_skull: None,
                death_camera_cut: None,
                menu_music: None,
                menu_falling: None,
                doors: HashMap::new(),
                piercer_shot_index: 0,
                master_volume: 1.0,
                sfx_volume: 1.0,
                music_volume: 1.0,
            },
            Err(err) => {
                eprintln!(
                    "Audio disabled: failed to initialize the system audio backend: {err}\n\
                     On Linux, install ALSA runtime packages and restart the game.\n\
                     Ubuntu/Debian: sudo apt install libasound2\n\
                     Fedora: sudo dnf install alsa-lib\n\
                     Arch: sudo pacman -S alsa-lib"
                );

                Self::disabled()
            }
        }
    }

    #[cfg(test)]
    pub(super) fn silent() -> Self {
        Self::disabled()
    }

    fn disabled() -> Self {
        Self {
            _stream: None,
            handle: None,
            dash: None,
            slide: None,
            ground_slam: None,
            piercer_charge: None,
            death_sequence: None,
            death_skull: None,
            death_camera_cut: None,
            menu_music: None,
            menu_falling: None,
            doors: HashMap::new(),
            piercer_shot_index: 0,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }

    pub(super) fn set_volumes(&mut self, master: u8, sfx: u8, music: u8) {
        self.master_volume = volume_factor(master);
        self.sfx_volume = volume_factor(sfx);
        self.music_volume = volume_factor(music);

        let sfx_volume = self.sfx_sink_volume();
        for sink in [
            self.dash.as_ref(),
            self.slide.as_ref(),
            self.ground_slam.as_ref(),
            self.piercer_charge.as_ref(),
            self.death_sequence.as_ref(),
            self.death_skull.as_ref(),
            self.death_camera_cut.as_ref(),
            self.menu_falling.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            sink.set_volume(sfx_volume);
        }
        for sink in self.doors.values() {
            sink.set_volume(sfx_volume);
        }
        if let Some(sink) = self.menu_music.as_ref() {
            sink.set_volume(self.music_sink_volume());
        }
    }

    pub(super) fn play(&mut self, event: SoundEvent, listener: Vec2) {
        if self.handle_control_event(event, listener) {
            return;
        }

        let Some(sound) = self.one_shot_sound(event, listener) else {
            return;
        };

        self.play_one_shot(sound.bytes, sound.volume);
    }

    fn handle_control_event(&mut self, event: SoundEvent, listener: Vec2) -> bool {
        match event {
            SoundEvent::DoorOpen { index, pos } => {
                self.start_door(index, DOOR_OPEN, 1.0 * attenuation(pos, listener));
                true
            }
            SoundEvent::DoorClose { index, pos } => {
                self.start_door(index, DOOR_CLOSE, 1.0 * attenuation(pos, listener));
                true
            }
            SoundEvent::DoorStop { index } => {
                // Door sinks are keyed by door index so completion events can stop the right sound.
                self.stop_door(index);
                true
            }
            SoundEvent::DashStart(pos) => {
                self.play_one_shot(DASH, 0.92 * attenuation(pos, listener));
                true
            }
            SoundEvent::DashEnd => true,
            SoundEvent::SlideStart(pos) => {
                self.start_action(
                    ActionSound::Slide,
                    SLIDE,
                    0.42 * attenuation(pos, listener),
                    true,
                );
                true
            }
            SoundEvent::SlideEnd => {
                self.stop_action(ActionSound::Slide);
                true
            }
            SoundEvent::GroundSlamStart(pos) => {
                self.start_action(
                    ActionSound::GroundSlam,
                    GROUND_SLAM,
                    0.42 * attenuation(pos, listener),
                    true,
                );
                true
            }
            SoundEvent::GroundSlamEnd => {
                self.stop_action(ActionSound::GroundSlam);
                true
            }
            SoundEvent::PiercerChargeStart(pos) => {
                self.start_action(
                    ActionSound::PiercerCharge,
                    PIERCER_CHARGE,
                    0.64 * attenuation(pos, listener),
                    true,
                );
                true
            }
            SoundEvent::PiercerChargeStop => {
                self.stop_action(ActionSound::PiercerCharge);
                true
            }
            SoundEvent::DeathSequence => {
                self.stop_actions();
                self.stop_death();
                self.start_death_sound(DeathSound::CameraCut, DEATH_CAMERA_CUT, 0.72);
                self.start_death_sound(DeathSound::Sequence, DEATH_SEQUENCE, 0.94);
                true
            }
            SoundEvent::DeathSkull => {
                self.start_death_sound(DeathSound::Skull, DEATH_SKULL, 0.86);
                true
            }
            SoundEvent::DeathStop => {
                self.stop_death();
                true
            }
            _ => false,
        }
    }

    fn one_shot_sound(&mut self, event: SoundEvent, listener: Vec2) -> Option<OneShotSound> {
        let (bytes, volume) = match event {
            SoundEvent::Footstep(index, pos) => {
                (footstep_sound(index), 0.48 * attenuation(pos, listener))
            }
            SoundEvent::Jump(pos) => (JUMP, 0.78 * attenuation(pos, listener)),
            SoundEvent::HeavyLand(pos) => (LAND_HEAVY, 0.72 * attenuation(pos, listener)),
            SoundEvent::PortalFire(pos) => (PORTAL_FIRE, 0.72 * attenuation(pos, listener)),
            SoundEvent::PortalPlace(pos) => (PORTAL_PLACE, 0.82 * attenuation(pos, listener)),
            SoundEvent::PlayerHurt(pos) => (V1_HURT, 0.84 * attenuation(pos, listener)),
            SoundEvent::FilthBite(pos) => (FILTH_BITE, 0.76 * attenuation(pos, listener)),
            SoundEvent::PiercerFire(pos) | SoundEvent::PiercerCharged(pos) => {
                (self.next_piercer_shot(), 0.72 * attenuation(pos, listener))
            }
            SoundEvent::Land
            | SoundEvent::DoorOpen { .. }
            | SoundEvent::DoorClose { .. }
            | SoundEvent::DoorStop { .. }
            | SoundEvent::DashStart(_)
            | SoundEvent::DashEnd
            | SoundEvent::SlideStart(_)
            | SoundEvent::SlideEnd
            | SoundEvent::GroundSlamStart(_)
            | SoundEvent::GroundSlamEnd
            | SoundEvent::PiercerChargeStart(_)
            | SoundEvent::PiercerChargeStop
            | SoundEvent::DeathSequence
            | SoundEvent::DeathSkull
            | SoundEvent::DeathStop => return None,
        };

        Some(OneShotSound { bytes, volume })
    }

    fn play_one_shot(&self, bytes: &'static [u8], volume: f32) {
        if volume <= 0.01 {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.set_volume(self.sfx_sink_volume());
        sink.append(decoder.amplify(volume));
        // One-shots own themselves until playback ends; looping sounds stay in Audio for stopping.
        sink.detach();
    }

    pub(super) fn stop_actions(&mut self) {
        self.stop_action(ActionSound::Dash);
        self.stop_action(ActionSound::Slide);
        self.stop_action(ActionSound::GroundSlam);
        self.stop_action(ActionSound::PiercerCharge);
        for (_, sink) in self.doors.drain() {
            sink.stop();
        }
    }

    fn start_death_sound(&mut self, sound: DeathSound, bytes: &'static [u8], volume: f32) {
        self.stop_death_sound(sound);

        if volume <= 0.01 {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.set_volume(self.sfx_sink_volume());
        sink.append(decoder.amplify(volume));
        *self.death_sink(sound) = Some(sink);
    }

    fn stop_death(&mut self) {
        self.stop_death_sound(DeathSound::Sequence);
        self.stop_death_sound(DeathSound::Skull);
        self.stop_death_sound(DeathSound::CameraCut);
    }

    pub(super) fn start_menu_ambience(&mut self) {
        self.start_loop_if_missing(MenuLoop::Music, MENU_MUSIC, 0.34);
        self.start_loop_if_missing(MenuLoop::Falling, GROUND_SLAM, 0.12);
    }

    pub(super) fn stop_menu_ambience(&mut self) {
        if let Some(sink) = self.menu_music.take() {
            sink.stop();
        }
        if let Some(sink) = self.menu_falling.take() {
            sink.stop();
        }
    }

    fn start_action(
        &mut self,
        action: ActionSound,
        bytes: &'static [u8],
        volume: f32,
        repeat: bool,
    ) {
        self.stop_action(action);

        if volume <= 0.01 {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.set_volume(self.sfx_sink_volume());
        if repeat {
            sink.append(decoder.amplify(volume).repeat_infinite());
        } else {
            sink.append(decoder.amplify(volume));
        }
        *self.action_sink(action) = Some(sink);
    }

    fn start_loop_if_missing(&mut self, sound: MenuLoop, bytes: &'static [u8], volume: f32) {
        if self.menu_sink(sound).is_some() || volume <= 0.01 {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        let sink_volume = match sound {
            MenuLoop::Music => self.music_sink_volume(),
            MenuLoop::Falling => self.sfx_sink_volume(),
        };
        sink.set_volume(sink_volume);
        sink.append(decoder.amplify(volume).repeat_infinite());
        *self.menu_sink(sound) = Some(sink);
    }

    fn stop_action(&mut self, action: ActionSound) {
        if let Some(sink) = self.action_sink(action).take() {
            sink.stop();
        }
    }

    fn stop_death_sound(&mut self, sound: DeathSound) {
        if let Some(sink) = self.death_sink(sound).take() {
            sink.stop();
        }
    }

    fn start_door(&mut self, index: usize, bytes: &'static [u8], volume: f32) {
        self.stop_door(index);

        if volume <= 0.01 {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(decoder) = Decoder::new(Cursor::new(bytes)) else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };

        sink.set_volume(self.sfx_sink_volume());
        sink.append(decoder.amplify(volume));
        self.doors.insert(index, sink);
    }

    fn stop_door(&mut self, index: usize) {
        if let Some(sink) = self.doors.remove(&index) {
            sink.stop();
        }
    }

    fn action_sink(&mut self, action: ActionSound) -> &mut Option<Sink> {
        match action {
            ActionSound::Dash => &mut self.dash,
            ActionSound::Slide => &mut self.slide,
            ActionSound::GroundSlam => &mut self.ground_slam,
            ActionSound::PiercerCharge => &mut self.piercer_charge,
        }
    }

    fn death_sink(&mut self, sound: DeathSound) -> &mut Option<Sink> {
        match sound {
            DeathSound::Sequence => &mut self.death_sequence,
            DeathSound::Skull => &mut self.death_skull,
            DeathSound::CameraCut => &mut self.death_camera_cut,
        }
    }

    fn next_piercer_shot(&mut self) -> &'static [u8] {
        let shot = match self.piercer_shot_index % 3 {
            0 => PIERCER_SHOT_1,
            1 => PIERCER_SHOT_2,
            _ => PIERCER_SHOT_3,
        };

        self.piercer_shot_index = (self.piercer_shot_index + 1) % 3;
        shot
    }

    fn menu_sink(&mut self, sound: MenuLoop) -> &mut Option<Sink> {
        match sound {
            MenuLoop::Music => &mut self.menu_music,
            MenuLoop::Falling => &mut self.menu_falling,
        }
    }

    fn sfx_sink_volume(&self) -> f32 {
        self.master_volume * self.sfx_volume
    }

    fn music_sink_volume(&self) -> f32 {
        self.master_volume * self.music_volume
    }
}

#[derive(Clone, Copy)]
enum ActionSound {
    Dash,
    Slide,
    GroundSlam,
    PiercerCharge,
}

#[derive(Clone, Copy)]
enum DeathSound {
    Sequence,
    Skull,
    CameraCut,
}

#[derive(Clone, Copy)]
enum MenuLoop {
    Music,
    Falling,
}

fn attenuation(source: Vec2, listener: Vec2) -> f32 {
    let distance = source.distance(listener);

    // Full volume nearby, then a simple linear falloff until the sound is inaudible.
    if distance <= 180.0 {
        1.0
    } else if distance >= 780.0 {
        0.0
    } else {
        1.0 - (distance - 180.0) / 600.0
    }
}

fn volume_factor(value: u8) -> f32 {
    f32::from(value.min(100)) / 100.0
}

fn footstep_sound(index: usize) -> &'static [u8] {
    match index % 3 {
        0 => FOOTSTEP_1,
        1 => FOOTSTEP_2,
        _ => FOOTSTEP_3,
    }
}

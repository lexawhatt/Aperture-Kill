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
const PORTAL_FIRE: &[u8] = include_bytes!("../../assets/sounds/portal_fire.wav");
const PORTAL_PLACE: &[u8] = include_bytes!("../../assets/sounds/portal_place.wav");

pub(super) struct Audio {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    dash: Option<Sink>,
    slide: Option<Sink>,
    ground_slam: Option<Sink>,
    doors: HashMap<usize, Sink>,
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
                doors: HashMap::new(),
            },
            Err(_) => Self {
                _stream: None,
                handle: None,
                dash: None,
                slide: None,
                ground_slam: None,
                doors: HashMap::new(),
            },
        }
    }

    pub(super) fn play(&mut self, event: SoundEvent, listener: Vec2) {
        match event {
            SoundEvent::DoorOpen { index, pos } => {
                self.start_door(index, DOOR_OPEN, 1.0 * attenuation(pos, listener));
                return;
            }
            SoundEvent::DoorClose { index, pos } => {
                self.start_door(index, DOOR_CLOSE, 1.0 * attenuation(pos, listener));
                return;
            }
            SoundEvent::DoorStop { index } => {
                // Door sinks are keyed by door index so completion events can stop the right sound.
                self.stop_door(index);
                return;
            }
            SoundEvent::DashStart(pos) => {
                self.play_one_shot(DASH, 0.92 * attenuation(pos, listener));
                return;
            }
            SoundEvent::DashEnd => {
                return;
            }
            SoundEvent::SlideStart(pos) => {
                self.start_action(
                    ActionSound::Slide,
                    SLIDE,
                    0.42 * attenuation(pos, listener),
                    true,
                );
                return;
            }
            SoundEvent::SlideEnd => {
                self.stop_action(ActionSound::Slide);
                return;
            }
            SoundEvent::GroundSlamStart(pos) => {
                self.start_action(
                    ActionSound::GroundSlam,
                    GROUND_SLAM,
                    0.42 * attenuation(pos, listener),
                    true,
                );
                return;
            }
            SoundEvent::GroundSlamEnd => {
                self.stop_action(ActionSound::GroundSlam);
                return;
            }
            _ => {}
        }

        let (bytes, volume) = match event {
            SoundEvent::Footstep(index, pos) => match index % 3 {
                0 => (FOOTSTEP_1, 0.48 * attenuation(pos, listener)),
                1 => (FOOTSTEP_2, 0.48 * attenuation(pos, listener)),
                _ => (FOOTSTEP_3, 0.48 * attenuation(pos, listener)),
            },
            SoundEvent::Jump(pos) => (JUMP, 0.78 * attenuation(pos, listener)),
            SoundEvent::Land => return,
            SoundEvent::HeavyLand(pos) => (LAND_HEAVY, 0.72 * attenuation(pos, listener)),
            SoundEvent::PortalFire(pos) => (PORTAL_FIRE, 0.72 * attenuation(pos, listener)),
            SoundEvent::PortalPlace(pos) => (PORTAL_PLACE, 0.82 * attenuation(pos, listener)),
            SoundEvent::DoorOpen { .. }
            | SoundEvent::DoorClose { .. }
            | SoundEvent::DoorStop { .. }
            | SoundEvent::DashStart(_)
            | SoundEvent::DashEnd
            | SoundEvent::SlideStart(_)
            | SoundEvent::SlideEnd
            | SoundEvent::GroundSlamStart(_)
            | SoundEvent::GroundSlamEnd => return,
        };
        self.play_one_shot(bytes, volume);
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

        sink.append(decoder.amplify(volume));
        sink.detach();
    }

    pub(super) fn stop_actions(&mut self) {
        self.stop_action(ActionSound::Dash);
        self.stop_action(ActionSound::Slide);
        self.stop_action(ActionSound::GroundSlam);
        for (_, sink) in self.doors.drain() {
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

        if repeat {
            sink.append(decoder.amplify(volume).repeat_infinite());
        } else {
            sink.append(decoder.amplify(volume));
        }
        *self.action_sink(action) = Some(sink);
    }

    fn stop_action(&mut self, action: ActionSound) {
        if let Some(sink) = self.action_sink(action).take() {
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
        }
    }
}

#[derive(Clone, Copy)]
enum ActionSound {
    Dash,
    Slide,
    GroundSlam,
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

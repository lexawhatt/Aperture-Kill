use winit::keyboard::KeyCode;

use crate::platform::input::GameKey;

#[derive(Clone)]
pub struct Settings {
    pub show_fps: bool,
    pub display_mode: DisplayMode,
    pub resolution: Resolution,
    pub resolutions: Vec<Resolution>,
    pub master_volume: u8,
    pub sfx_volume: u8,
    pub music_volume: u8,
    bindings: [(GameKey, KeyCode); GAME_ACTIONS.len()],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    General,
    Controls,
    Graphics,
    Audio,
    Assist,
    Saves,
    Hud,
    Colors,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Fullscreen,
    Borderless,
    Windowed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    Master,
    Sfx,
    Music,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OptionsClick {
    Tab(OptionsTab),
    ToggleFps,
    Bind(GameKey),
    DisplayMode,
    ToggleResolutionDropdown,
    ResolutionChoice(usize),
    Volume(VolumeKind, u8),
    Back,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

pub const RESOLUTIONS: [Resolution; 7] = [
    Resolution {
        width: 1280,
        height: 720,
    },
    Resolution {
        width: 1600,
        height: 900,
    },
    Resolution {
        width: 1920,
        height: 1080,
    },
    Resolution {
        width: 2560,
        height: 1080,
    },
    Resolution {
        width: 2560,
        height: 1440,
    },
    Resolution {
        width: 3440,
        height: 1440,
    },
    Resolution {
        width: 3840,
        height: 2160,
    },
];

pub const GAME_ACTIONS: [GameKey; 7] = [
    GameKey::Left,
    GameKey::Right,
    GameKey::Jump,
    GameKey::Dash,
    GameKey::Slide,
    GameKey::BluePortal,
    GameKey::OrangePortal,
];

impl Settings {
    pub fn new() -> Self {
        Self {
            show_fps: true,
            display_mode: DisplayMode::Borderless,
            resolution: Resolution {
                width: 0,
                height: 0,
            },
            resolutions: fallback_resolutions(),
            master_volume: 100,
            sfx_volume: 100,
            music_volume: 100,
            bindings: [
                (GameKey::Left, KeyCode::KeyA),
                (GameKey::Right, KeyCode::KeyD),
                (GameKey::Jump, KeyCode::Space),
                (GameKey::Dash, KeyCode::ShiftLeft),
                (GameKey::Slide, KeyCode::ControlLeft),
                (GameKey::BluePortal, KeyCode::KeyQ),
                (GameKey::OrangePortal, KeyCode::KeyE),
            ],
        }
    }

    pub fn bind(&mut self, key: GameKey, code: KeyCode) {
        let Some(target) = self
            .bindings
            .iter_mut()
            .position(|(candidate, _)| *candidate == key)
        else {
            return;
        };
        if let Some(other) = self
            .bindings
            .iter()
            .position(|(_, binding)| *binding == code)
            .filter(|other| *other != target)
        {
            // Swapping keeps every gameplay action reachable and avoids duplicate bindings.
            let previous = self.bindings[target].1;
            self.bindings[target].1 = code;
            self.bindings[other].1 = previous;
        } else {
            self.bindings[target].1 = code;
        }
    }

    pub fn is_rebindable(&self, code: KeyCode) -> bool {
        !matches!(
            code,
            KeyCode::Escape | KeyCode::F1 | KeyCode::F3 | KeyCode::F7
        )
    }

    pub fn key_for_code(&self, code: KeyCode) -> Option<GameKey> {
        self.bindings
            .iter()
            .find_map(|(key, binding)| (*binding == code).then_some(*key))
    }

    pub fn action_bindings(&self) -> &[(GameKey, KeyCode); GAME_ACTIONS.len()] {
        &self.bindings
    }

    pub fn set_resolutions(&mut self, mut resolutions: Vec<Resolution>) {
        if resolutions.is_empty() {
            resolutions = fallback_resolutions();
        }
        // Present the native and common modes from largest to smallest for predictable selection.
        resolutions.sort_by_key(|resolution| {
            std::cmp::Reverse((
                resolution.width.saturating_mul(resolution.height),
                resolution.width,
                resolution.height,
            ))
        });
        resolutions.dedup();

        if !resolutions.contains(&self.resolution) {
            self.resolution = resolutions
                .first()
                .copied()
                .unwrap_or_else(Resolution::default);
        }
        self.resolutions = resolutions;
    }
}

impl DisplayMode {
    pub fn next(self) -> Self {
        match self {
            Self::Fullscreen => Self::Windowed,
            Self::Windowed => Self::Borderless,
            Self::Borderless => Self::Fullscreen,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fullscreen => "FULLSCREEN",
            Self::Windowed => "WINDOWED",
            Self::Borderless => "BORDERLESS",
        }
    }
}

impl Resolution {
    pub fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }

    pub fn label(self) -> String {
        format!("{} x {}", self.width, self.height)
    }
}

impl OptionsTab {
    pub fn enabled(self) -> bool {
        matches!(
            self,
            Self::General | Self::Controls | Self::Graphics | Self::Audio
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "GENERAL",
            Self::Controls => "CONTROLS",
            Self::Graphics => "GRAPHICS",
            Self::Audio => "AUDIO",
            Self::Assist => "ASSIST",
            Self::Saves => "SAVES",
            Self::Hud => "HUD",
            Self::Colors => "COLORS",
        }
    }
}

pub fn game_key_label(key: GameKey) -> &'static str {
    match key {
        GameKey::Left => "MOVE LEFT",
        GameKey::Right => "MOVE RIGHT",
        GameKey::Jump => "JUMP",
        GameKey::Dash => "DASH",
        GameKey::Slide => "SLIDE",
        GameKey::BluePortal => "BLUE PORTAL",
        GameKey::OrangePortal => "ORANGE PORTAL",
    }
}

pub fn key_code_label(code: KeyCode) -> &'static str {
    match code {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Space => "SPACE",
        KeyCode::ShiftLeft => "L SHIFT",
        KeyCode::ShiftRight => "R SHIFT",
        KeyCode::ControlLeft => "L CTRL",
        KeyCode::ControlRight => "R CTRL",
        KeyCode::AltLeft => "L ALT",
        KeyCode::AltRight => "R ALT",
        KeyCode::ArrowLeft => "LEFT",
        KeyCode::ArrowRight => "RIGHT",
        KeyCode::ArrowUp => "UP",
        KeyCode::ArrowDown => "DOWN",
        KeyCode::Enter => "ENTER",
        KeyCode::Tab => "TAB",
        KeyCode::Backspace => "BACKSPACE",
        KeyCode::Escape => "ESC",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        _ => "KEY",
    }
}

fn fallback_resolutions() -> Vec<Resolution> {
    RESOLUTIONS.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebinding_an_in_use_key_swaps_actions() {
        let mut settings = Settings::new();

        settings.bind(GameKey::Left, KeyCode::KeyD);

        assert_eq!(settings.key_for_code(KeyCode::KeyD), Some(GameKey::Left));
        assert_eq!(settings.key_for_code(KeyCode::KeyA), Some(GameKey::Right));
    }

    #[test]
    fn global_shortcuts_cannot_be_bound_as_gameplay_actions() {
        let settings = Settings::new();

        assert!(!settings.is_rebindable(KeyCode::Escape));
        assert!(!settings.is_rebindable(KeyCode::F1));
        assert!(settings.is_rebindable(KeyCode::KeyQ));
    }
}

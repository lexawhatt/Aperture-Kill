use winit::event::MouseButton;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

// Level menu input is screen-space UI, not world-space editing.
use crate::app::menu::{menu_hit, options_drag_volume, options_hit, social_hit};
use crate::app::{App, AppMode};
use crate::settings::{OptionsClick, VolumeKind};

impl App {
    pub(super) fn handle_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.current_level = self.current_level.saturating_sub(1);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.current_level = (self.current_level + 1).min(self.levels.len() - 1);
            }
            KeyCode::Enter | KeyCode::Space => self.open_selected_level(),
            _ => {}
        }
    }

    pub(super) fn handle_options_key(&mut self, code: KeyCode) {
        if let Some(key) = self.binding_capture {
            if code == KeyCode::Escape {
                self.binding_capture = None;
            } else if self.settings.is_rebindable(code) {
                self.settings.bind(key, code);
                self.input.release_gameplay();
                self.binding_capture = None;
            }
            return;
        }

        match code {
            KeyCode::Escape | KeyCode::Backspace => self.mode = AppMode::LevelMenu,
            _ => {}
        }
    }

    pub(super) fn handle_changelog_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Escape | KeyCode::Backspace | KeyCode::Enter | KeyCode::Space => {
                self.mode = AppMode::LevelMenu;
            }
            _ => {}
        }
    }

    pub(super) fn handle_menu_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        event_loop: &ActiveEventLoop,
    ) {
        if !down || button != MouseButton::Left {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        if let Some(url) = social_hit(self.cursor_screen, size.width as f32, size.height as f32) {
            open_social_url(url);
            return;
        }

        if let Some(index) = menu_hit(self.cursor_screen, size.width as f32, size.height as f32) {
            match index {
                0 => self.open_selected_level(),
                1 => self.mode = AppMode::Options,
                2 => self.mode = AppMode::Changelog,
                3 => event_loop.exit(),
                _ => {}
            }
        }
    }

    pub(super) fn handle_changelog_mouse(&mut self, button: MouseButton, down: bool) {
        if down && button == MouseButton::Left {
            self.mode = AppMode::LevelMenu;
        }
    }

    pub(super) fn handle_options_mouse(&mut self, button: MouseButton, down: bool) {
        if button != MouseButton::Left {
            return;
        }
        if !down {
            self.volume_drag = None;
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        match options_hit(
            self.cursor_screen,
            size.width as f32,
            size.height as f32,
            self.options_tab,
            &self.settings,
            self.resolution_dropdown,
        ) {
            OptionsClick::Tab(tab) => {
                self.options_tab = tab;
                self.binding_capture = None;
                self.resolution_dropdown = false;
                self.volume_drag = None;
            }
            OptionsClick::ToggleFps => {
                self.settings.show_fps = !self.settings.show_fps;
                self.resolution_dropdown = false;
            }
            OptionsClick::Bind(key) => {
                self.binding_capture = Some(key);
                self.resolution_dropdown = false;
            }
            OptionsClick::DisplayMode => {
                self.settings.display_mode = self.settings.display_mode.next();
                self.resolution_dropdown = false;
                self.apply_display_settings();
            }
            OptionsClick::ToggleResolutionDropdown => {
                self.resolution_dropdown = !self.resolution_dropdown;
            }
            OptionsClick::ResolutionChoice(index) => {
                if let Some(resolution) = self.settings.resolutions.get(index).copied() {
                    self.settings.resolution = resolution;
                }
                self.resolution_dropdown = false;
                self.apply_display_settings();
            }
            OptionsClick::Volume(kind, value) => {
                self.resolution_dropdown = false;
                self.volume_drag = Some(kind);
                self.set_options_volume(kind, value);
            }
            OptionsClick::Back => {
                self.binding_capture = None;
                self.resolution_dropdown = false;
                self.volume_drag = None;
                self.mode = AppMode::LevelMenu;
            }
            OptionsClick::None => self.resolution_dropdown = false,
        }
    }

    pub(crate) fn drag_options_volume(&mut self) {
        let Some(kind) = self.volume_drag else {
            return;
        };
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        if let Some(value) = options_drag_volume(
            self.cursor_screen,
            size.width as f32,
            size.height as f32,
            kind,
        ) {
            self.set_options_volume(kind, value);
        }
    }

    fn set_options_volume(&mut self, kind: VolumeKind, value: u8) {
        match kind {
            VolumeKind::Master => self.settings.master_volume = value,
            VolumeKind::Sfx => self.settings.sfx_volume = value,
            VolumeKind::Music => self.settings.music_volume = value,
        }
        self.audio.set_volumes(
            self.settings.master_volume,
            self.settings.sfx_volume,
            self.settings.music_volume,
        );
    }

    fn open_selected_level(&mut self) {
        self.load_current_level();
        self.mode = AppMode::Playing;
    }
}

fn open_social_url(url: &str) {
    let _ = std::process::Command::new(url_open_command())
        .args(url_open_args(url))
        .spawn();
}

#[cfg(target_os = "windows")]
fn url_open_command() -> &'static str {
    "cmd"
}

#[cfg(target_os = "windows")]
fn url_open_args(url: &str) -> [&str; 4] {
    ["/C", "start", "", url]
}

#[cfg(target_os = "macos")]
fn url_open_command() -> &'static str {
    "open"
}

#[cfg(target_os = "macos")]
fn url_open_args(url: &str) -> [&str; 1] {
    [url]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn url_open_command() -> &'static str {
    "xdg-open"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn url_open_args(url: &str) -> [&str; 1] {
    [url]
}

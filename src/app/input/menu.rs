use winit::event::MouseButton;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

// Level menu input is screen-space UI, not world-space editing.
use crate::app::menu::{
    PAUSE_ACTION_COUNT, chapter_hit, custom_level_hit, difficulty_hit, layer_level_hit, menu_hit,
    options_drag_volume, options_hit, pause_hit, social_hit,
};
use crate::app::{App, AppMode, MenuScreen};
use crate::game::Difficulty;
use crate::game::progression::{CHAPTERS, is_custom_chapter};
use crate::settings::{OptionsClick, VolumeKind};

impl App {
    pub(in crate::app) fn open_pause_menu(&mut self) {
        self.pause_cursor = 0;
        self.pause_keyboard_focus = false;
        self.mode = AppMode::Pause;
        self.input.release_gameplay();
        self.audio.stop_actions();
    }

    pub(in crate::app) fn resume_from_pause(&mut self) {
        self.mode = AppMode::Playing;
        self.input.release_gameplay();
        self.audio.stop_actions();
    }

    pub(in crate::app) fn close_options(&mut self) {
        self.binding_capture = None;
        self.resolution_dropdown = false;
        self.volume_drag = None;
        self.mode = if self.options_return_to_pause {
            AppMode::Pause
        } else {
            AppMode::LevelMenu
        };
        self.options_return_to_pause = false;
    }

    pub(in crate::app) fn handle_pause_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.pause_cursor = if self.pause_keyboard_focus {
                    self.pause_cursor.saturating_sub(1)
                } else {
                    PAUSE_ACTION_COUNT - 1
                };
                self.pause_keyboard_focus = true;
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.pause_cursor = if self.pause_keyboard_focus {
                    (self.pause_cursor + 1).min(PAUSE_ACTION_COUNT - 1)
                } else {
                    0
                };
                self.pause_keyboard_focus = true;
            }
            KeyCode::Enter | KeyCode::Space => {
                if let Some(index) = self.pause_active_action() {
                    self.activate_pause_action(index);
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_pause_mouse(&mut self, button: MouseButton, down: bool) {
        if !down || button != MouseButton::Left {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        if let Some(index) = pause_hit(self.cursor_screen, size.width as f32, size.height as f32) {
            self.pause_cursor = index;
            self.pause_keyboard_focus = false;
            self.activate_pause_action(index);
        }
    }

    fn pause_active_action(&self) -> Option<usize> {
        let Some(window) = self.window.as_ref() else {
            return self.pause_keyboard_focus.then_some(self.pause_cursor);
        };
        let size = window.inner_size();

        pause_hit(self.cursor_screen, size.width as f32, size.height as f32)
            .or_else(|| self.pause_keyboard_focus.then_some(self.pause_cursor))
    }

    fn activate_pause_action(&mut self, index: usize) {
        match index {
            0 => self.resume_from_pause(),
            1 => {
                self.world.restart_from_checkpoint();
                let listener = self.world.player.pos;
                for event in self.world.drain_sound_events() {
                    self.audio.play(event, listener);
                }
                self.resume_from_pause();
            }
            2 => {
                self.load_current_level();
                self.mode = AppMode::Playing;
            }
            3 => {
                self.options_return_to_pause = true;
                self.mode = AppMode::Options;
            }
            4 => {
                self.options_return_to_pause = false;
                self.mode = AppMode::LevelMenu;
                self.input.release_gameplay();
                self.audio.stop_actions();
            }
            _ => {}
        }
    }

    pub(in crate::app) fn handle_menu_key(&mut self, code: KeyCode) {
        self.clamp_menu_cursors();

        match self.menu_screen {
            MenuScreen::Main => self.handle_main_menu_key(code),
            MenuScreen::Difficulty => self.handle_difficulty_menu_key(code),
            MenuScreen::Chapter => self.handle_chapter_menu_key(code),
            MenuScreen::Layer => self.handle_layer_menu_key(code),
        }
    }

    pub(in crate::app) fn menu_back(&mut self) {
        self.menu_screen = match self.menu_screen {
            MenuScreen::Main => MenuScreen::Main,
            MenuScreen::Difficulty => MenuScreen::Main,
            MenuScreen::Chapter => MenuScreen::Difficulty,
            MenuScreen::Layer => MenuScreen::Chapter,
        };
    }

    fn handle_main_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.main_menu_cursor = self.main_menu_cursor.saturating_sub(1);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.main_menu_cursor = (self.main_menu_cursor + 1).min(3);
            }
            KeyCode::Enter | KeyCode::Space => match self.main_menu_cursor {
                0 => self.menu_screen = MenuScreen::Difficulty,
                1 => {
                    self.options_return_to_pause = false;
                    self.mode = AppMode::Options;
                }
                2 => self.mode = AppMode::Changelog,
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_difficulty_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.difficulty_cursor = self.difficulty_cursor.saturating_sub(1);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.difficulty_cursor = (self.difficulty_cursor + 1).min(Difficulty::COUNT - 1);
            }
            KeyCode::Escape | KeyCode::Backspace => self.menu_back(),
            KeyCode::Enter | KeyCode::Space => self.select_difficulty(self.difficulty_cursor),
            _ => {}
        }
    }

    fn handle_chapter_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.chapter_cursor = self.chapter_cursor.saturating_sub(1);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.chapter_cursor = (self.chapter_cursor + 1).min(CHAPTERS.len() - 1);
            }
            KeyCode::Escape | KeyCode::Backspace => self.menu_back(),
            KeyCode::Enter | KeyCode::Space => self.select_chapter(self.chapter_cursor),
            _ => {}
        }
    }

    fn handle_layer_menu_key(&mut self, code: KeyCode) {
        let count = self.menu_level_count();
        if count == 0 {
            if matches!(code, KeyCode::Escape | KeyCode::Backspace) {
                self.menu_back();
            }
            return;
        }

        match code {
            KeyCode::ArrowLeft | KeyCode::KeyA => {
                self.level_cursor = self.level_cursor.saturating_sub(1);
            }
            KeyCode::ArrowRight | KeyCode::KeyD => {
                self.level_cursor = (self.level_cursor + 1).min(count - 1);
            }
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.level_cursor = self.level_cursor.saturating_sub(4);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.level_cursor = (self.level_cursor + 4).min(count - 1);
            }
            KeyCode::Escape | KeyCode::Backspace => self.menu_back(),
            KeyCode::Enter | KeyCode::Space => self.open_selected_layer_item(),
            _ => {}
        }
    }

    pub(in crate::app) fn handle_options_key(&mut self, code: KeyCode) {
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
            KeyCode::Escape | KeyCode::Backspace => self.close_options(),
            _ => {}
        }
    }

    pub(in crate::app) fn handle_changelog_key(&mut self, code: KeyCode) {
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
        if self.menu_screen == MenuScreen::Main {
            if let Some(url) = social_hit(self.cursor_screen, size.width as f32, size.height as f32)
            {
                open_social_url(url);
                return;
            }
        }

        match self.menu_screen {
            MenuScreen::Main => {
                if let Some(index) =
                    menu_hit(self.cursor_screen, size.width as f32, size.height as f32)
                {
                    self.main_menu_cursor = index;
                    match index {
                        0 => self.menu_screen = MenuScreen::Difficulty,
                        1 => {
                            self.options_return_to_pause = false;
                            self.mode = AppMode::Options;
                        }
                        2 => self.mode = AppMode::Changelog,
                        3 => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            MenuScreen::Difficulty => {
                if let Some(index) =
                    difficulty_hit(self.cursor_screen, size.width as f32, size.height as f32)
                {
                    self.difficulty_cursor = index;
                    self.select_difficulty(index);
                }
            }
            MenuScreen::Chapter => {
                if let Some(index) =
                    chapter_hit(self.cursor_screen, size.width as f32, size.height as f32)
                {
                    self.chapter_cursor = index;
                    self.select_chapter(index);
                }
            }
            MenuScreen::Layer => {
                let hit = if is_custom_chapter(self.chapter_cursor) {
                    custom_level_hit(
                        self.cursor_screen,
                        size.width as f32,
                        size.height as f32,
                        self.menu_level_count(),
                    )
                } else {
                    layer_level_hit(
                        self.cursor_screen,
                        size.width as f32,
                        size.height as f32,
                        self.chapter_cursor,
                    )
                };
                if let Some(index) = hit {
                    self.level_cursor = index;
                    self.open_selected_layer_item();
                }
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
                self.close_options();
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

    fn open_selected_catalog_level(&mut self) {
        let Some(level_index) = self.selected_catalog_level_index() else {
            return;
        };

        self.current_level = level_index;
        self.open_selected_level();
    }

    fn open_selected_layer_item(&mut self) {
        if is_custom_chapter(self.chapter_cursor) && self.level_cursor == 0 {
            self.create_custom_level();
        } else {
            self.open_selected_catalog_level();
        }
    }

    fn select_difficulty(&mut self, index: usize) {
        let Some(difficulty) = Difficulty::from_index(index) else {
            return;
        };
        if !difficulty.available() {
            return;
        }

        self.selected_difficulty = difficulty;
        self.difficulty_cursor = index;
        self.chapter_cursor = 0;
        self.level_cursor = 0;
        self.menu_screen = MenuScreen::Chapter;
    }

    fn select_chapter(&mut self, index: usize) {
        let Some(chapter) = CHAPTERS.get(index) else {
            return;
        };

        self.chapter_cursor = index;
        self.level_cursor = 0;
        if chapter.layers.is_empty() && !is_custom_chapter(index) {
            return;
        }

        self.menu_screen = MenuScreen::Layer;
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

use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

// Keys that should work no matter which mode is active.
use crate::app::{App, AppMode};

impl App {
    pub(super) fn handle_global_key(
        &mut self,
        code: KeyCode,
        down: bool,
        _event_loop: &ActiveEventLoop,
    ) -> bool {
        if !down {
            return false;
        }

        match code {
            KeyCode::F1 => self.toggle_level_menu(),
            KeyCode::F3 => self.toggle_editor(),
            KeyCode::F7 => {
                self.debug_gui = !self.debug_gui;
                true
            }
            KeyCode::Escape if self.mode == AppMode::Options && self.binding_capture.is_some() => {
                self.binding_capture = None;
                true
            }
            KeyCode::Escape if self.mode == AppMode::Options && self.resolution_dropdown => {
                self.resolution_dropdown = false;
                true
            }
            KeyCode::Escape if self.mode == AppMode::Options => {
                self.close_options();
                true
            }
            KeyCode::Escape if self.mode == AppMode::Changelog => {
                self.mode = AppMode::LevelMenu;
                true
            }
            KeyCode::Escape if self.mode == AppMode::LevelMenu => {
                self.menu_back();
                true
            }
            KeyCode::Escape if self.mode == AppMode::Pause => {
                self.resume_from_pause();
                true
            }
            KeyCode::Escape if self.mode != AppMode::Playing => {
                self.mode = AppMode::Playing;
                self.camera.reset_zoom();
                self.input.release_gameplay();
                self.audio.stop_actions();
                true
            }
            KeyCode::Escape => {
                self.open_pause_menu();
                true
            }
            _ => false,
        }
    }

    fn toggle_level_menu(&mut self) -> bool {
        if self.mode == AppMode::Editor {
            self.camera.reset_zoom();
        }
        self.mode = if matches!(
            self.mode,
            AppMode::LevelMenu | AppMode::Changelog | AppMode::Options
        ) {
            self.options_return_to_pause = false;
            AppMode::Playing
        } else {
            self.input.release_gameplay();
            self.audio.stop_actions();
            self.options_return_to_pause = false;
            AppMode::LevelMenu
        };
        true
    }

    fn toggle_editor(&mut self) -> bool {
        self.mode = if self.mode == AppMode::Editor {
            self.camera.reset_zoom();
            AppMode::Playing
        } else {
            self.input.release_gameplay();
            self.audio.stop_actions();
            AppMode::Editor
        };
        true
    }
}

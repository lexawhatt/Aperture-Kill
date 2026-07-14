use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

// Keys that should work no matter which mode is active.
use crate::app::{App, AppMode};

impl App {
    pub(super) fn handle_global_key(
        &mut self,
        code: KeyCode,
        down: bool,
        event_loop: &ActiveEventLoop,
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
            KeyCode::Escape if self.mode != AppMode::Playing => {
                self.mode = AppMode::Playing;
                self.camera.reset_zoom();
                self.input.release_gameplay();
                self.audio.stop_actions();
                true
            }
            KeyCode::Escape => {
                event_loop.exit();
                true
            }
            _ => false,
        }
    }

    fn toggle_level_menu(&mut self) -> bool {
        if self.mode == AppMode::Editor {
            self.camera.reset_zoom();
        }
        self.mode = if self.mode == AppMode::LevelMenu {
            AppMode::Playing
        } else {
            self.input.release_gameplay();
            self.audio.stop_actions();
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

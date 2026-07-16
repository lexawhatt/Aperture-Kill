mod editor;
mod gameplay;
mod global;
mod menu;

// Dispatches raw key/mouse input to the active app mode.
use winit::event::{MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;

use crate::app::{App, AppMode};

impl App {
    pub(super) fn handle_key(
        &mut self,
        key: PhysicalKey,
        down: bool,
        event_loop: &ActiveEventLoop,
    ) {
        if let PhysicalKey::Code(code) = key {
            // Binding capture owns the next key press so global shortcuts cannot hijack it.
            if self.mode == AppMode::Options && self.binding_capture.is_some() && down {
                self.handle_options_key(code);
                return;
            }
            if self.handle_global_key(code, down, event_loop) {
                return;
            }

            match self.mode {
                AppMode::Playing => self.handle_game_key(code, down),
                AppMode::LevelMenu if down => self.handle_menu_key(code),
                AppMode::Changelog if down => self.handle_changelog_key(code),
                AppMode::Options if down => self.handle_options_key(code),
                AppMode::Editor => self.handle_editor_key(code, down),
                _ => {}
            }
        }
    }

    pub(super) fn handle_mouse(
        &mut self,
        button: MouseButton,
        down: bool,
        event_loop: &ActiveEventLoop,
    ) {
        match self.mode {
            AppMode::Playing => {}
            AppMode::LevelMenu => self.handle_menu_mouse(button, down, event_loop),
            AppMode::Changelog => self.handle_changelog_mouse(button, down),
            AppMode::Options => self.handle_options_mouse(button, down),
            AppMode::Editor => self.handle_editor_mouse(button, down),
        }
    }

    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.mode != AppMode::Editor {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let steps = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 120.0,
        };

        self.camera
            .zoom_editor_at(self.cursor_screen, width as f32, height as f32, steps);
        self.refresh_cursor_world_for(width, height);
    }
}

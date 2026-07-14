use winit::event::MouseButton;
use winit::keyboard::KeyCode;

// Level menu input is screen-space UI, not world-space editing.
use crate::app::menu::menu_hit;
use crate::app::{App, AppMode};

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

    pub(super) fn handle_menu_mouse(&mut self, button: MouseButton, down: bool) {
        if !down || button != MouseButton::Left {
            return;
        }

        if let Some(index) = menu_hit(self.cursor_screen, self.levels.len()) {
            self.current_level = index;
            self.open_selected_level();
        }
    }

    fn open_selected_level(&mut self) {
        self.load_current_level();
        self.mode = AppMode::Playing;
    }
}

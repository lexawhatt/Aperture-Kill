use winit::keyboard::KeyCode;

// Maps physical gameplay keys into the frame-stable Input state.
use crate::app::App;

impl App {
    pub(super) fn handle_game_key(&mut self, code: KeyCode, down: bool) {
        if code == KeyCode::Digit1 {
            self.input.set_weapon_1(down);
            return;
        }
        if code == KeyCode::KeyR {
            self.input.set_respawn(down);
            return;
        }

        if let Some(key) = self.settings.key_for_code(code) {
            self.input.set_key(key, down);
        }
    }
}

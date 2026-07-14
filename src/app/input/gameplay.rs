use winit::keyboard::KeyCode;

// Maps physical gameplay keys into the frame-stable Input state.
use crate::app::App;
use crate::platform::input::GameKey;

impl App {
    pub(super) fn handle_game_key(&mut self, code: KeyCode, down: bool) {
        match code {
            // Physical keys keep movement stable across keyboard layouts.
            KeyCode::KeyA | KeyCode::ArrowLeft => self.input.set_key(GameKey::Left, down),
            KeyCode::KeyD | KeyCode::ArrowRight => self.input.set_key(GameKey::Right, down),
            KeyCode::Space => self.input.set_key(GameKey::Jump, down),
            KeyCode::ShiftLeft => self.input.set_key(GameKey::Dash, down),
            KeyCode::ControlLeft | KeyCode::ControlRight => {
                self.input.set_key(GameKey::Slide, down)
            }
            KeyCode::KeyQ => self.input.set_key(GameKey::BluePortal, down),
            KeyCode::KeyE => self.input.set_key(GameKey::OrangePortal, down),
            _ => {}
        }
    }
}

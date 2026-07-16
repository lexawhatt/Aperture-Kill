use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameKey {
    BluePortal,
    Dash,
    OrangePortal,
    Jump,
    Left,
    Right,
    Slide,
}

#[derive(Clone, Copy, Default)]
struct ButtonState {
    down: bool,
    pressed: bool,
    was_down: bool,
}

impl ButtonState {
    fn set_down(&mut self, down: bool) {
        self.down = down;
    }

    fn release(&mut self) {
        self.down = false;
    }

    fn consume_press(&mut self) {
        self.pressed = false;
    }

    fn update(&mut self) {
        self.pressed = self.down && !self.was_down;
        self.was_down = self.down;
    }
}

#[derive(Clone, Default)]
pub struct Input {
    pub move_x: f32,
    pub aim_pos: Vec2,
    blue_portal: ButtonState,
    dash: ButtonState,
    jump: ButtonState,
    left_down: bool,
    orange_portal: ButtonState,
    right_down: bool,
    slide: ButtonState,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_key(&mut self, key: GameKey, down: bool) {
        match key {
            GameKey::BluePortal => self.blue_portal.set_down(down),
            GameKey::Dash => self.dash.set_down(down),
            GameKey::OrangePortal => self.orange_portal.set_down(down),
            GameKey::Jump => self.jump.set_down(down),
            GameKey::Left => self.left_down = down,
            GameKey::Right => self.right_down = down,
            GameKey::Slide => self.slide.set_down(down),
        }
    }

    pub fn set_aim_pos(&mut self, aim_pos: Vec2) {
        self.aim_pos = aim_pos;
    }

    pub fn release_gameplay(&mut self) {
        self.blue_portal.release();
        self.dash.release();
        self.jump.release();
        self.left_down = false;
        self.orange_portal.release();
        self.right_down = false;
        self.slide.release();
    }

    pub fn consume_presses(&mut self) {
        self.blue_portal.consume_press();
        self.dash.consume_press();
        self.jump.consume_press();
        self.orange_portal.consume_press();
        self.slide.consume_press();
    }

    pub fn key_down(&self, key: GameKey) -> bool {
        match key {
            GameKey::BluePortal => self.blue_portal.down,
            GameKey::Dash => self.dash.down,
            GameKey::OrangePortal => self.orange_portal.down,
            GameKey::Jump => self.jump.down,
            GameKey::Left => self.left_down,
            GameKey::Right => self.right_down,
            GameKey::Slide => self.slide.down,
        }
    }

    pub fn key_pressed(&self, key: GameKey) -> bool {
        match key {
            GameKey::BluePortal => self.blue_portal.pressed,
            GameKey::Dash => self.dash.pressed,
            GameKey::OrangePortal => self.orange_portal.pressed,
            GameKey::Jump => self.jump.pressed,
            GameKey::Slide => self.slide.pressed,
            GameKey::Left | GameKey::Right => false,
        }
    }

    pub fn update(&mut self) {
        // Edge flags let actions fire once, while *_down keeps hold state.
        self.blue_portal.update();
        self.dash.update();
        self.jump.update();
        self.orange_portal.update();
        self.slide.update();

        // Left and right cancel out naturally through a single intent axis.
        self.move_x = 0.0;
        if self.left_down {
            self.move_x -= 1.0;
        }
        if self.right_down {
            self.move_x += 1.0;
        }
    }
}

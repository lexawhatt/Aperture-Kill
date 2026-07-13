use glam::Vec2;

#[derive(Clone, Copy)]
pub enum GameKey {
    BluePortal,
    Dash,
    OrangePortal,
    Jump,
    Left,
    Right,
    Slide,
}

#[derive(Default)]
pub struct Input {
    pub move_x: f32,
    pub aim_pos: Vec2,
    pub blue_portal_pressed: bool,
    pub dash_down: bool,
    pub dash_pressed: bool,
    pub jump_down: bool,
    pub jump_pressed: bool,
    pub orange_portal_pressed: bool,
    pub slide_down: bool,
    pub slide_pressed: bool,
    blue_portal_down: bool,
    blue_portal_was_down: bool,
    dash_was_down: bool,
    jump_was_down: bool,
    left_down: bool,
    orange_portal_down: bool,
    orange_portal_was_down: bool,
    right_down: bool,
    slide_was_down: bool,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_key(&mut self, key: GameKey, down: bool) {
        match key {
            GameKey::BluePortal => self.blue_portal_down = down,
            GameKey::Dash => self.dash_down = down,
            GameKey::OrangePortal => self.orange_portal_down = down,
            GameKey::Jump => self.jump_down = down,
            GameKey::Left => self.left_down = down,
            GameKey::Right => self.right_down = down,
            GameKey::Slide => self.slide_down = down,
        }
    }

    pub fn set_aim_pos(&mut self, aim_pos: Vec2) {
        self.aim_pos = aim_pos;
    }

    pub fn update(&mut self) {
        // Edge flags let actions fire once, while *_down keeps hold state.
        self.blue_portal_pressed = self.blue_portal_down && !self.blue_portal_was_down;
        self.dash_pressed = self.dash_down && !self.dash_was_down;
        self.jump_pressed = self.jump_down && !self.jump_was_down;
        self.orange_portal_pressed = self.orange_portal_down && !self.orange_portal_was_down;
        self.slide_pressed = self.slide_down && !self.slide_was_down;

        // Left and right cancel out naturally through a single intent axis.
        self.move_x = 0.0;
        if self.left_down {
            self.move_x -= 1.0;
        }
        if self.right_down {
            self.move_x += 1.0;
        }

        self.blue_portal_was_down = self.blue_portal_down;
        self.dash_was_down = self.dash_down;
        self.jump_was_down = self.jump_down;
        self.orange_portal_was_down = self.orange_portal_down;
        self.slide_was_down = self.slide_down;
    }
}

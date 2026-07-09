use glam::Vec2;

#[derive(Clone, Copy)]
pub enum GameKey {
    Left,
    Right,
}

#[derive(Default)]
pub struct Input {
    pub move_x: f32,
    pub aim_pos: Vec2,
    pub shoot_primary: bool,
    pub shoot_secondary: bool,
    left_down: bool,
    right_down: bool,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_key(&mut self, key: GameKey, down: bool) {
        match key {
            GameKey::Left => self.left_down = down,
            GameKey::Right => self.right_down = down,
        }
    }

    pub fn set_aim_pos(&mut self, aim_pos: Vec2) {
        self.aim_pos = aim_pos;
    }

    pub fn set_primary_fire(&mut self, down: bool) {
        self.shoot_primary = down;
    }

    pub fn set_secondary_fire(&mut self, down: bool) {
        self.shoot_secondary = down;
    }

    pub fn update(&mut self) {
        // Input stores intent
        self.move_x = 0.0;
        if self.left_down {
            self.move_x -= 1.0;
        }
        if self.right_down {
            self.move_x += 1.0;
        }
    }
}

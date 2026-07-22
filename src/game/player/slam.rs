#[derive(Clone, Copy, Debug)]
pub(super) struct SlamState {
    pub(super) entry_speed: f32,
    pub(super) start_y: f32,
    pub(super) storage_ready: bool,
    pub(super) stored_speed: f32,
}

impl SlamState {
    pub(super) fn new(start_y: f32) -> Self {
        Self {
            entry_speed: 0.0,
            start_y,
            storage_ready: false,
            stored_speed: 0.0,
        }
    }

    pub(super) fn start(&mut self, pos_y: f32, velocity_y: f32) {
        self.storage_ready = false;
        self.entry_speed = velocity_y.max(0.0);
        self.start_y = pos_y;
        self.stored_speed = self.entry_speed;
    }

    pub(super) fn clear(&mut self) {
        self.storage_ready = false;
        self.entry_speed = 0.0;
        self.stored_speed = 0.0;
    }
}

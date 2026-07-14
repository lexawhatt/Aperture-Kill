use glam::Vec2;

// Camera works in world space; rendering converts through it.
const CAMERA_LEASH: f32 = 90.0;
const CAMERA_PULL: f32 = 7.5;
const EDITOR_PAN_SPEED: f32 = 650.0;
const EDITOR_ZOOM_MAX: f32 = 3.0;
const EDITOR_ZOOM_MIN: f32 = 0.35;
const EDITOR_ZOOM_STEP: f32 = 1.12;

#[derive(Clone, Copy)]
pub(super) struct Camera {
    pub(super) center: Vec2,
    pub(super) zoom: f32,
}

impl Camera {
    pub(super) fn new(center: Vec2) -> Self {
        Self { center, zoom: 1.0 }
    }

    pub(super) fn screen_to_world(self, point: Vec2, width: f32, height: f32) -> Vec2 {
        (point - Vec2::new(width, height) / 2.0) / self.zoom + self.center
    }

    pub(super) fn follow(&mut self, target: Vec2, dt: f32) {
        let offset = target - self.center;
        let distance = offset.length();
        if distance <= CAMERA_LEASH {
            return;
        }

        // A small dead zone makes the camera feel pulled by a loose line.
        let leash_target = target - offset / distance * CAMERA_LEASH;
        let blend = 1.0 - (-CAMERA_PULL * dt).exp();
        self.center = self.center.lerp(leash_target, blend);
    }

    pub(super) fn pan(&mut self, direction: Vec2, dt: f32) {
        if direction.length_squared() == 0.0 {
            return;
        }

        self.center += direction.normalize() * EDITOR_PAN_SPEED * dt / self.zoom;
    }

    pub(super) fn zoom_editor_at(&mut self, cursor: Vec2, width: f32, height: f32, steps: f32) {
        if steps == 0.0 {
            return;
        }

        let before = self.screen_to_world(cursor, width, height);
        self.zoom =
            (self.zoom * EDITOR_ZOOM_STEP.powf(steps)).clamp(EDITOR_ZOOM_MIN, EDITOR_ZOOM_MAX);
        let after = self.screen_to_world(cursor, width, height);

        self.center += before - after;
    }

    pub(super) fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }
}

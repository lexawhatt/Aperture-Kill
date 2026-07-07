use macroquad::input::{is_key_down, KeyCode};
use macroquad::math::Vec2;
use crate::constants::GRAVITY;
pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub mass: f32,
}


impl Ball {
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            velocity: Vec2::new(200.0, 0.0),
            radius,
            mass: radius * radius,
        }
    }

    pub fn update(&mut self, dt: f32) {
        // dt = delta time -> time in seconds ( time between previous and current frame ) P.S I know this from unity, lol o_o
        self.velocity.y += GRAVITY * dt; // increasing vertical speed
        self.position += self.velocity * dt;
    }

    pub fn constrain_to_screen(&mut self, screen_width: f32, screen_height: f32) {
        // you cant really use match here :/
        if self.position.y > screen_height - self.radius {
            self.position.y = screen_height - self.radius;
            self.velocity.y = -self.velocity.y * 0.75; // *1 bouncing ball at other direction with 25% energy loss
        } else if self.position.y < self.radius {
            self.position.y = self.radius;
            self.velocity.y = -self.velocity.y * 0.75; // *1
        }

        if self.position.x < self.radius {
            self.position.x = self.radius;
            self.velocity.x = -self.velocity.x * 0.75; // *1
        } else if self.position.x > screen_width - self.radius {
            self.position.x = screen_width - self.radius;
            self.velocity.x = -self.velocity.x * 0.75; // *1
        }
    }

    pub fn handle_input(
        &mut self,
        dt: f32,
        const_mode: bool,
        accel_speed: f32,
        const_speed: f32,
        keys: (KeyCode, KeyCode, KeyCode, KeyCode), // (Up, Down, Left, Right)
        dir_string: &mut String,
    ) {
        let (up, down, left, right) = keys;

        if const_mode {
            if is_key_down(up) { self.velocity.y = -const_speed; dir_string.push('^'); }
            if is_key_down(down) { self.velocity.y = const_speed; dir_string.push('v'); }
            if is_key_down(left) { self.velocity.x = -const_speed; dir_string.push('<'); }
            if is_key_down(right) { self.velocity.x = const_speed; dir_string.push('>'); }

            self.position += self.velocity * dt;
        } else {
            if is_key_down(up) { self.velocity.y -= accel_speed * dt; dir_string.push('^'); }
            if is_key_down(down) { self.velocity.y += accel_speed * dt; dir_string.push('v'); }
            if is_key_down(left) { self.velocity.x -= accel_speed * dt; dir_string.push('<'); }
            if is_key_down(right) { self.velocity.x += accel_speed * dt; dir_string.push('>'); }

            self.update(dt);
        }
    }

    pub fn resolve_collision(&mut self, other: &mut Ball) {
        let delta = other.position - self.position;
        let dist = delta.length();
        let radius = self.radius + other.radius;

        if dist < radius {
            let overlap = radius - dist;
            let normal = delta.normalize_or_zero();

            let t_mass = self.mass + other.mass;

            let s_shift = (other.mass / t_mass) * overlap;
            let o_shift = (self.mass / t_mass) * overlap;

            self.position -= normal * s_shift;
            other.position += normal * o_shift;

            let rel_vel = other.velocity - self.velocity;
            let vel_normal = rel_vel.dot(normal);

            if vel_normal < 0.0 {
                let res = 0.75;

                let imp = -(1.0 + res) * vel_normal / ((1.0 / self.mass) + (1.0 / other.mass));

                // (f = m * a)
                self.velocity -= normal * (imp / self.mass);
                other.velocity += normal * (imp / other.mass);
            }
        }
    }
}
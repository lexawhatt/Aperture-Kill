mod ball;
mod portal;
mod input;
mod constants;

use macroquad::prelude::*;

pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub mass: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Portal {
    pub id: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    pub normal: Vec2,   // it should be normalized (length - 1) [ when BALL enters portal, this thing helps us to detect in what direction ball enters, knowing that and base math we could calculate speed, angle, etc ]
    pub width: f32,
    pub scale: f32,
    pub color: Color,
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

impl Portal {
    pub fn new(id: usize, x: f32, y: f32, normal: Vec2, width: f32, color: Color) -> Self {
        Self {
            id,
            pos: Vec2::new(x, y),
            vel: Vec2::ZERO,
            normal: normal.normalize(),
            width,
            scale: 1.0,
            color,
        }
    }

    pub fn draw(&self) {
        // We obtain a vector directed along the portal (perpendicular to the normal)
        let dir = Vec2::new(-self.normal.y, self.normal.x); // -> if normal looks (0,1) dir will look at (1,0). basically 90 degree rotation
        let halfw = (self.width * self.scale) / 2.0;
        let start = self.pos + dir * halfw; // pos is center of the portal, to go to the start (or end) we go to one direction which is direction * half width
        let end = self.pos - dir * halfw;

        draw_line(start.x, start.y, end.x, end.y, 4.0, self.color);
    }

    // now, the msot important part of this thign

    pub fn tp_obj(&self, p2: &Portal, ball: &mut Ball) {
        let scale = p2.scale / self.scale;
        ball.radius *= scale;
        ball.mass = ball.radius * ball.radius;

        let rel_vel = ball.velocity - self.vel;
        let inv_normal = -self.normal;
        let tg = Vec2::new(-self.normal.y, self.normal.x);

        let normal_vel_mag = rel_vel.dot(inv_normal);
        let tg_vel_mag = rel_vel.dot(tg);

        let mut ex_tg = Vec2::new(-p2.normal.y, p2.normal.x);

        if self.normal.dot(p2.normal) < -0.9 {
            ex_tg = -ex_tg;
        }

        let mut new_rel_vel = p2.normal * normal_vel_mag + ex_tg * tg_vel_mag;

        new_rel_vel *= scale;
        ball.velocity = new_rel_vel + p2.vel;

        let offset = ball.position - self.pos;
        let offset_tg = offset.dot(tg);

        ball.position = p2.pos + ex_tg * (offset_tg * scale);
        ball.position += p2.normal * (ball.radius + 1.0);
    }

    pub fn check_coll(&self, p2: &Portal, ball: &mut Ball) -> bool {
        let to_obj = ball.position - self.pos; // vec from portal center to BALL
        let tg = Vec2::new(-self.normal.y, self.normal.x);

        let dist_tg = to_obj.dot(tg);
        let halfw = (self.width * self.scale) / 2.0; // check if BALL gets to portal

        if dist_tg.abs() > halfw { // if ball flies beyond the width of the portal, then it flies past
            return false;
        }

        let dist_to_pl = to_obj.dot(self.normal);

        let rel_vel = ball.velocity - self.vel;
        let mov_tow = rel_vel.dot(self.normal) < 0.0; // Ball should tp only when its looking towards portal

        if dist_to_pl.abs() <= ball.radius && mov_tow {
            self.tp_obj(p2, ball); //Yay! collision detected! tp!
            return true;
        }

        false
    }
}

fn check_char(target: char) -> bool {
    let mut pressed = false;
    while let Some(c) = get_char_pressed() {
        if c.to_lowercase().to_string() == target.to_lowercase().to_string() {
            pressed = true;
        }
    }
    pressed
}

#[macroquad::main("Ball Example")]
async fn main() {
    let mut ball1 = Ball::new(100.0, 100.0, 15.0);
    let mut ball2 = Ball::new(200.0, 100.0, 20.0);

    let mut p1 = Portal::new(1, 750.0, 300.0, Vec2::new(-1.0, 0.0), 100.0, BLUE);
    let mut p2 = Portal::new(2, 50.0, 300.0, Vec2::new(1.0, 0.0), 100.0, ORANGE);

    p1.scale = 4.0;
    p2.scale = 2.0;


    let mut const_mode = false;

    loop {
        let dt = get_frame_time();
        let mut dir = String::new();

        if let Some(c) = get_char_pressed() {
            if c == 'v' || c == 'V' || c == 'м' || c == 'М' {
                const_mode = !const_mode;
            }
        }

        let keys_ball1 = (KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right);
        let keys_ball2 = (KeyCode::Kp5, KeyCode::Kp2, KeyCode::Kp1, KeyCode::Kp3);

        if const_mode {
            ball1.velocity = Vec2::ZERO;
            ball2.velocity = Vec2::ZERO;
        }

        ball1.handle_input(dt, const_mode, ACCEL_SPEED, CONST_SPEED, keys_ball1, &mut dir);
        ball2.handle_input(dt, const_mode, ACCEL_SPEED, CONST_SPEED, keys_ball2, &mut dir);

        ball1.resolve_collision(&mut ball2);

        let _ = p2.check_coll(&p1, &mut ball1);
        let _ = p1.check_coll(&p2, &mut ball1);

        let _ = p2.check_coll(&p1, &mut ball2);
        let _ = p1.check_coll(&p2, &mut ball2);

        ball1.constrain_to_screen(screen_width(), screen_height());
        ball2.constrain_to_screen(screen_width(), screen_height());

        clear_background(BLACK);

        draw_text(&dir, 10.0, 40.0, 20.0, WHITE);

        draw_circle(ball1.position.x, ball1.position.y, ball1.radius, WHITE);
        draw_circle(ball2.position.x, ball2.position.y, ball2.radius, LIGHTGRAY);

        p1.draw();
        p2.draw();

        let mode = if const_mode { "CONSTANT" } else { "INERTIAL" };
        draw_text(
            &format!("B1 Vel: {:.1} {:.1} | Mode: {}", ball1.velocity.x, ball1.velocity.y, mode),
            10.0,
            20.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}
use macroquad::color::Color;
use macroquad::math::Vec2;
use macroquad::prelude::draw_line;
use crate::player::Ball;
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
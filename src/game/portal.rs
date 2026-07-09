use glam::Vec2;

use crate::game::player::Player;

#[derive(Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLUE: Self = Self {
        r: 54,
        g: 139,
        b: 255,
    };
    pub const ORANGE: Self = Self {
        r: 255,
        g: 151,
        b: 42,
    };
}

#[derive(Clone, Copy, PartialEq)]
pub struct Portal {
    pub id: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    pub normal: Vec2, // normalized. tells where the portal "front" looks
    pub width: f32,
    pub scale: f32,
    pub scale_objects: bool,
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
            scale_objects: true,
            color,
        }
    }

    pub fn endpoints(&self) -> (Vec2, Vec2) {
        // perpendicular to normal. portal line lives here
        let dir = Vec2::new(-self.normal.y, self.normal.x);
        let halfw = (self.width * self.scale) / 2.0;

        (self.pos + dir * halfw, self.pos - dir * halfw)
    }

    // now, the msot important part of this thign

    pub fn tp_obj(&self, p2: &Portal, player: &mut Player) {
        let scale = if self.scale_objects && p2.scale_objects {
            p2.scale / self.scale
        } else {
            1.0
        };

        let tg = Vec2::new(-self.normal.y, self.normal.x);
        let mut ex_tg = Vec2::new(-p2.normal.y, p2.normal.x);

        if self.normal.dot(p2.normal) < -0.9 {
            ex_tg = -ex_tg;
        }

        let offset = player.pos - self.pos;
        let offset_tg = offset.dot(tg);

        player.size *= scale;
        player.pos = p2.pos + ex_tg * (offset_tg * scale);
        player.pos += p2.normal * (player.half_size().length() + 1.0);
        player.vel.y = 0.0;
    }

    pub fn check_coll(&self, p2: &Portal, player: &mut Player) -> bool {
        let to_obj = player.pos - self.pos; // vec from portal center to player
        let tg = Vec2::new(-self.normal.y, self.normal.x);

        let dist_tg = to_obj.dot(tg);
        let halfw = (self.width * self.scale) / 2.0; // check if player gets to portal

        if dist_tg.abs() > halfw {
            // if player goes beyond the portal width, then nope
            return false;
        }

        let dist_to_pl = to_obj.dot(self.normal);
        let hitbox_rad = player.half_size().length();

        if dist_to_pl.abs() <= hitbox_rad {
            self.tp_obj(p2, player); // Yay! collision detected! tp!
            return true;
        }

        false
    }
}

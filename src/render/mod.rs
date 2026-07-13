use glam::Vec2;

use crate::game::World;
use crate::game::portal::Color;

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, frame: &mut [u32], width: u32, height: u32, world: &World) {
        let mut canvas = Canvas {
            frame,
            width,
            height,
        };

        canvas.clear(Color::rgb(9, 10, 14));

        // Renderer looks at World, but does not touch it.
        for portal in &world.portals {
            let (a, b) = portal.endpoints();
            canvas.draw_line(a, b, portal.color);
        }

        let player = &world.player;
        let half = player.half_size();
        let player_rect = Rect {
            pos: player.pos - half,
            size: player.size,
        };

        canvas.fill_rect(player_rect, Color::rgb(45, 49, 59));
        canvas.rect_outline(player_rect, Color::rgb(235, 238, 245));

        canvas.draw_line(player.aim_from(), player.aim_pos, Color::rgb(255, 70, 86));
        canvas.fill_rect(
            Rect {
                pos: player.aim_pos - Vec2::splat(3.0),
                size: Vec2::splat(6.0),
            },
            Color::rgb(255, 70, 86),
        );
    }
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

#[derive(Clone, Copy)]
struct Rect {
    pos: Vec2,
    size: Vec2,
}

struct Canvas<'a> {
    frame: &'a mut [u32],
    width: u32,
    height: u32,
}

impl Canvas<'_> {
    fn clear(&mut self, color: Color) {
        self.frame.fill(color.to_u32());
    }

    fn put_px(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        self.frame[(y as u32 * self.width + x as u32) as usize] = color.to_u32();
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x0 = rect.pos.x.max(0.0) as i32;
        let y0 = rect.pos.y.max(0.0) as i32;
        let x1 = (rect.pos.x + rect.size.x).min(self.width as f32) as i32;
        let y1 = (rect.pos.y + rect.size.y).min(self.height as f32) as i32;

        for yy in y0..y1 {
            for xx in x0..x1 {
                self.put_px(xx, yy, color);
            }
        }
    }

    fn rect_outline(&mut self, rect: Rect, color: Color) {
        let a = rect.pos;
        let b = rect.pos + Vec2::new(rect.size.x, 0.0);
        let c = rect.pos + rect.size;
        let d = rect.pos + Vec2::new(0.0, rect.size.y);

        self.draw_line(a, b, color);
        self.draw_line(b, c, color);
        self.draw_line(c, d, color);
        self.draw_line(d, a, color);
    }

    fn draw_line(&mut self, a: Vec2, b: Vec2, color: Color) {
        let mut x0 = a.x.round() as i32;
        let mut y0 = a.y.round() as i32;
        let x1 = b.x.round() as i32;
        let y1 = b.y.round() as i32;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put_px(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

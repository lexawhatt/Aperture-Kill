use glam::Vec2;

use crate::game::World;
use crate::game::portal::Color;

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, frame: &mut [u32], width: u32, height: u32, world: &World) {
        clear(frame, Color::rgb(9, 10, 14));

        // Renderer looks at World, but does not touch it.
        for portal in &world.portals {
            let (a, b) = portal.endpoints();
            draw_line(frame, width, height, a, b, portal.color);
        }

        let player = &world.player;
        let half = player.half_size();
        let left = player.pos.x - half.x;
        let top = player.pos.y - half.y;

        // hitbox (test)
        fill_rect(
            frame,
            width,
            height,
            left,
            top,
            player.size.x,
            player.size.y,
            Color::rgb(45, 49, 59),
        );
        rect_outline(
            frame,
            width,
            height,
            left,
            top,
            player.size.x,
            player.size.y,
            Color::rgb(235, 238, 245),
        );

        draw_line(
            frame,
            width,
            height,
            player.aim_from(),
            player.aim_pos,
            Color::rgb(255, 70, 86),
        );
        fill_rect(
            frame,
            width,
            height,
            player.aim_pos.x - 3.0,
            player.aim_pos.y - 3.0,
            6.0,
            6.0,
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

fn clear(frame: &mut [u32], color: Color) {
    frame.fill(color.to_u32());
}

fn put_px(frame: &mut [u32], width: u32, height: u32, x: i32, y: i32, color: Color) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }

    frame[(y as u32 * width + x as u32) as usize] = color.to_u32();
}

fn fill_rect(
    frame: &mut [u32],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) {
    let x0 = x.max(0.0) as i32;
    let y0 = y.max(0.0) as i32;
    let x1 = (x + w).min(width as f32) as i32;
    let y1 = (y + h).min(height as f32) as i32;

    for yy in y0..y1 {
        for xx in x0..x1 {
            put_px(frame, width, height, xx, yy, color);
        }
    }
}

fn rect_outline(
    frame: &mut [u32],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) {
    let a = Vec2::new(x, y);
    let b = Vec2::new(x + w, y);
    let c = Vec2::new(x + w, y + h);
    let d = Vec2::new(x, y + h);

    draw_line(frame, width, height, a, b, color);
    draw_line(frame, width, height, b, c, color);
    draw_line(frame, width, height, c, d, color);
    draw_line(frame, width, height, d, a, color);
}

fn draw_line(frame: &mut [u32], width: u32, height: u32, a: Vec2, b: Vec2, color: Color) {
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
        put_px(frame, width, height, x0, y0, color);
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

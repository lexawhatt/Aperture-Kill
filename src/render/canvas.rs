use glam::Vec2;

// Canvas draws pixels and converts between world and screen space.
use crate::game::portal::Color;

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub(super) pos: Vec2,
    pub(super) size: Vec2,
}

pub(super) struct Canvas<'a> {
    frame: &'a mut [u32],
    pub(super) width: u32,
    pub(super) height: u32,
    camera: Vec2,
    zoom: f32,
}

impl<'a> Canvas<'a> {
    pub(super) fn new(
        frame: &'a mut [u32],
        width: u32,
        height: u32,
        camera: Vec2,
        zoom: f32,
    ) -> Self {
        Self {
            frame,
            width,
            height,
            camera,
            zoom,
        }
    }

    pub(super) fn clear(&mut self, color: Color) {
        self.frame.fill(color.to_u32());
    }

    pub(super) fn fill_rect(&mut self, rect: Rect, color: Color) {
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

    pub(super) fn fill_world_rect(&mut self, rect: Rect, color: Color) {
        self.fill_rect(
            Rect {
                pos: self.world_to_screen(rect.pos),
                size: rect.size * self.zoom,
            },
            color,
        );
    }

    pub(super) fn rect_outline(&mut self, rect: Rect, color: Color) {
        let a = rect.pos;
        let b = rect.pos + Vec2::new(rect.size.x, 0.0);
        let c = rect.pos + rect.size;
        let d = rect.pos + Vec2::new(0.0, rect.size.y);

        self.draw_line(a, b, color);
        self.draw_line(b, c, color);
        self.draw_line(c, d, color);
        self.draw_line(d, a, color);
    }

    pub(super) fn world_rect_outline(&mut self, rect: Rect, color: Color) {
        let a = rect.pos;
        let b = rect.pos + Vec2::new(rect.size.x, 0.0);
        let c = rect.pos + rect.size;
        let d = rect.pos + Vec2::new(0.0, rect.size.y);

        self.draw_world_line(a, b, color);
        self.draw_world_line(b, c, color);
        self.draw_world_line(c, d, color);
        self.draw_world_line(d, a, color);
    }

    pub(super) fn draw_line(&mut self, a: Vec2, b: Vec2, color: Color) {
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

    pub(super) fn draw_world_line(&mut self, a: Vec2, b: Vec2, color: Color) {
        self.draw_line(self.world_to_screen(a), self.world_to_screen(b), color);
    }

    pub(super) fn world_to_screen(&self, point: Vec2) -> Vec2 {
        (point - self.camera) * self.zoom + self.screen_center()
    }

    pub(super) fn screen_to_world(&self, point: Vec2) -> Vec2 {
        (point - self.screen_center()) / self.zoom + self.camera
    }

    pub(super) fn put_px(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        self.frame[(y as u32 * self.width + x as u32) as usize] = color.to_u32();
    }

    fn screen_center(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) / 2.0
    }
}

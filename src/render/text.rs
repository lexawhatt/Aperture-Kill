use glam::Vec2;

// Tiny bitmap text for debug-style UI labels.
use crate::game::portal::Color;

use super::canvas::{Canvas, Rect};
use super::glyphs::glyph;

impl Canvas<'_> {
    pub(super) fn world_text(&mut self, pos: Vec2, text: &str, scale: i32, color: Color) {
        self.text(self.world_to_screen(pos), text, scale, color);
    }

    pub(super) fn text(&mut self, pos: Vec2, text: &str, scale: i32, color: Color) {
        let mut x = pos.x.round() as i32;
        let y = pos.y.round() as i32;

        for ch in text.chars() {
            if ch == ' ' {
                x += 4 * scale;
                continue;
            }

            self.glyph(Vec2::new(x as f32, y as f32), glyph(ch), scale, color);
            x += 6 * scale;
        }
    }

    fn glyph(&mut self, pos: Vec2, pattern: [u8; 7], scale: i32, color: Color) {
        for (row, bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }

                self.fill_rect(
                    Rect {
                        pos: pos + Vec2::new((col * scale) as f32, (row as i32 * scale) as f32),
                        size: Vec2::splat(scale as f32),
                    },
                    color,
                );
            }
        }
    }
}

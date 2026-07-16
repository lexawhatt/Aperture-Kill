use glam::Vec2;

// World drawing uses camera-transformed coordinates.
use crate::game::level::{Checkpoint, Door, Hazard, Solid};
use crate::game::portal::{Color, Portal};

use super::canvas::{Canvas, Rect};

impl Canvas<'_> {
    pub(super) fn solid(&mut self, solid: Solid, fill: Color, outline: Color) {
        let corners = solid.corners().map(|corner| self.world_to_screen(corner));
        let (min, max) = corners.into_iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(min, max), corner| (min.min(corner), max.max(corner)),
        );
        let x0 = min.x.max(0.0) as i32;
        let y0 = min.y.max(0.0) as i32;
        let x1 = max.x.min(self.width as f32) as i32;
        let y1 = max.y.min(self.height as f32) as i32;

        for yy in y0..y1 {
            for xx in x0..x1 {
                let point = self.screen_to_world(Vec2::new(xx as f32 + 0.5, yy as f32 + 0.5));
                if solid.contains_point(point) {
                    self.put_px(xx, yy, fill);
                }
            }
        }

        self.solid_outline(solid, outline);
    }

    pub(super) fn door(&mut self, door: Door) {
        let closed = door.solid;
        let panel = door.moving_solid();

        self.solid_outline(closed, Color::rgb(116, 132, 155));
        self.solid(panel, Color::rgb(74, 82, 93), Color::rgb(172, 181, 194));
        self.door_slats(panel);

        let local_y = panel.size.y * 0.68;
        let stripe_h = 10.0;
        let stripe = Solid::rotated(
            panel.pos.x,
            panel.pos.y + local_y,
            panel.size.x,
            stripe_h,
            panel.rotation(),
            false,
        );

        self.solid(stripe, Color::rgb(210, 170, 42), Color::rgb(42, 36, 30));
    }

    pub(super) fn hazard(&mut self, hazard: Hazard) {
        let solid = hazard.solid;

        self.solid(solid, Color::rgb(36, 105, 62), Color::rgb(124, 255, 120));
        let wave_count = (solid.size.x / 24.0).ceil().max(2.0) as usize;

        for index in 0..wave_count {
            let x0 = index as f32 / wave_count as f32 * solid.size.x;
            let x1 = (index as f32 + 0.5) / wave_count as f32 * solid.size.x;
            let x2 = (index as f32 + 1.0) / wave_count as f32 * solid.size.x;
            let y_mid = solid.size.y * 0.42;
            let y_peak = solid.size.y * 0.22;

            self.draw_world_line(
                solid.world_from_local(Vec2::new(x0, y_mid)),
                solid.world_from_local(Vec2::new(x1, y_peak)),
                Color::rgb(180, 255, 130),
            );
            self.draw_world_line(
                solid.world_from_local(Vec2::new(x1, y_peak)),
                solid.world_from_local(Vec2::new(x2, y_mid)),
                Color::rgb(180, 255, 130),
            );
        }
    }

    pub(super) fn checkpoint(&mut self, checkpoint: Checkpoint) {
        let solid = checkpoint.solid;
        let center = solid.center();

        self.solid_outline(solid, Color::rgb(80, 190, 255));
        self.draw_world_line(
            center + Vec2::new(0.0, solid.size.y / 2.0),
            center + Vec2::new(0.0, -solid.size.y / 2.0),
            Color::rgb(80, 190, 255),
        );
        self.draw_world_line(
            center + Vec2::new(0.0, -solid.size.y / 2.0),
            center + Vec2::new(solid.size.x * 0.36, -solid.size.y * 0.3),
            Color::rgb(80, 190, 255),
        );
        self.draw_world_line(
            center + Vec2::new(solid.size.x * 0.36, -solid.size.y * 0.3),
            center + Vec2::new(0.0, -solid.size.y * 0.1),
            Color::rgb(80, 190, 255),
        );
    }

    pub(super) fn player_rect(&mut self, rect: Rect, fill: Color, outline: Color) {
        self.fill_world_rect(rect, fill);
        self.world_rect_outline(rect, outline);
    }

    pub(super) fn player_rect_through_portal(
        &mut self,
        rect: Rect,
        source: Portal,
        destination: Portal,
        fill: Color,
        outline: Color,
    ) {
        if let Some(outside) = rect_on_portal_side(rect, source, true) {
            self.player_rect(outside, fill, outline);
        }

        if let Some(inside) = rect_on_portal_side(rect, source, false) {
            let center = inside.pos + inside.size / 2.0;
            let (mapped_center, mapped_size) =
                source.map_body_to(&destination, center, inside.size);
            let mapped = Rect {
                pos: mapped_center - mapped_size / 2.0,
                size: mapped_size,
            };

            self.player_rect(mapped, fill, outline);
        }
    }

    pub(super) fn grid(&mut self, step: f32, color: Color) {
        let min = self.screen_to_world(Vec2::ZERO);
        let max = self.screen_to_world(Vec2::new(self.width as f32, self.height as f32));
        let mut x = (min.x / step).floor() * step;
        while x <= max.x {
            self.draw_world_line(Vec2::new(x, min.y), Vec2::new(x, max.y), color);
            x += step;
        }

        let mut y = (min.y / step).floor() * step;
        while y <= max.y {
            self.draw_world_line(Vec2::new(min.x, y), Vec2::new(max.x, y), color);
            y += step;
        }
    }

    pub(super) fn solid_outline(&mut self, solid: Solid, color: Color) {
        let corners = solid.corners();

        for index in 0..4 {
            self.draw_world_line(corners[index], corners[(index + 1) % 4], color);
        }
    }

    fn door_slats(&mut self, solid: Solid) {
        let count = (solid.size.y / 18.0).floor().max(2.0) as usize;

        for index in 1..count {
            let y = index as f32 / count as f32 * solid.size.y;
            self.draw_world_line(
                solid.world_from_local(Vec2::new(0.0, y)),
                solid.world_from_local(Vec2::new(solid.size.x, y)),
                Color::rgb(42, 48, 58),
            );
        }
    }

    pub(super) fn resize_handles(&mut self, solid: Solid, color: Color) {
        let points = [
            Vec2::ZERO,
            Vec2::new(solid.size.x / 2.0, 0.0),
            Vec2::new(solid.size.x, 0.0),
            Vec2::new(solid.size.x, solid.size.y / 2.0),
            solid.size,
            Vec2::new(solid.size.x / 2.0, solid.size.y),
            Vec2::new(0.0, solid.size.y),
            Vec2::new(0.0, solid.size.y / 2.0),
        ];

        for point in points {
            let point = solid.world_from_local(point);
            self.fill_world_rect(
                Rect {
                    pos: point - Vec2::splat(5.0),
                    size: Vec2::splat(10.0),
                },
                color,
            );
        }
    }

    pub(super) fn rotate_handle(&mut self, solid: Solid, color: Color) {
        let center = solid.center();
        let radius = solid.size.x.max(solid.size.y) / 2.0 + 18.0;
        let mut previous = center + Vec2::new(radius, 0.0);

        for step in 1..=32 {
            let angle = step as f32 / 32.0 * std::f32::consts::TAU;
            let next = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            self.draw_world_line(previous, next, color);
            previous = next;
        }

        let rotation = solid.rotation();
        let handle_dir = Vec2::new(rotation.sin(), -rotation.cos());
        let handle = center + handle_dir * radius;
        self.draw_world_line(center + handle_dir * (radius - 12.0), handle, color);
        self.fill_world_rect(
            Rect {
                pos: handle - Vec2::splat(6.0),
                size: Vec2::splat(12.0),
            },
            color,
        );
    }

    pub(super) fn slam_storage_particles(&mut self, center: Vec2, half_size: Vec2) {
        let color = Color::rgb(255, 224, 102);
        let points = [
            Vec2::new(-0.75, -0.65),
            Vec2::new(0.78, -0.55),
            Vec2::new(-0.95, 0.05),
            Vec2::new(0.95, 0.12),
            Vec2::new(-0.55, 0.78),
            Vec2::new(0.58, 0.72),
        ];

        for (index, point) in points.into_iter().enumerate() {
            let offset = Vec2::new(point.x * half_size.x * 1.55, point.y * half_size.y * 1.22);
            let size = if index % 2 == 0 { 5.0 } else { 4.0 };

            self.fill_world_rect(
                Rect {
                    pos: center + offset - Vec2::splat(size / 2.0),
                    size: Vec2::splat(size),
                },
                color,
            );
        }
    }
}

fn rect_on_portal_side(rect: Rect, portal: Portal, outside: bool) -> Option<Rect> {
    let normal = portal.normal();
    let mut pos = rect.pos;
    let mut size = rect.size;

    if normal.x.abs() > normal.y.abs() {
        let min = pos.x;
        let max = pos.x + size.x;
        let keep_positive_side = (normal.x > 0.0) == outside;

        if keep_positive_side {
            pos.x = min.max(portal.pos.x);
            size.x = max - pos.x;
        } else {
            size.x = max.min(portal.pos.x) - min;
        }
    } else {
        let min = pos.y;
        let max = pos.y + size.y;
        let keep_positive_side = (normal.y > 0.0) == outside;

        if keep_positive_side {
            pos.y = min.max(portal.pos.y);
            size.y = max - pos.y;
        } else {
            size.y = max.min(portal.pos.y) - min;
        }
    }

    if size.x > 0.0 && size.y > 0.0 {
        Some(Rect { pos, size })
    } else {
        None
    }
}

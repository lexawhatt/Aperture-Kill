use glam::Vec2;

// World drawing uses camera-transformed coordinates.
use crate::constants::PORTAL_SURFACE_OFFSET;
use crate::game::enemy::{Enemy, EnemyKind};
use crate::game::level::{Checkpoint, Door, Hazard, Solid, WorldPortal};
use crate::game::portal::{Color, Portal};
use crate::game::{PiercerBeam, World};

use super::canvas::{Canvas, Rect, WorldClip};

const SEAMLESS_CUT_EPSILON: f32 = 2.0;

#[derive(Clone, Copy)]
struct SeamlessCut {
    pos: Vec2,
    normal: Vec2,
    tangent: Vec2,
    half_width: f32,
    depth: f32,
}

fn seamless_cuts_for_solid(solid: Solid, portals: &[WorldPortal]) -> Vec<SeamlessCut> {
    portals
        .iter()
        .filter_map(|world_portal| {
            if !world_portal.seamless || !portal_sits_on_solid(world_portal.portal, solid) {
                return None;
            }

            let portal = world_portal.portal;
            Some(SeamlessCut {
                pos: portal.pos,
                normal: portal.normal(),
                tangent: portal.tangent(),
                half_width: portal.active_width() / 2.0 + SEAMLESS_CUT_EPSILON,
                depth: world_portal.seamless_depth,
            })
        })
        .collect()
}

fn seamless_portal_cuts_point(point: Vec2, cuts: &[SeamlessCut]) -> bool {
    cuts.iter().any(|cut| {
        let offset = point - cut.pos;
        let tangent_distance = offset.dot(cut.tangent).abs();
        let normal_distance = offset.dot(cut.normal);

        tangent_distance <= cut.half_width
            && normal_distance <= PORTAL_SURFACE_OFFSET + SEAMLESS_CUT_EPSILON
            && normal_distance >= -cut.depth
    })
}

fn solid_can_occlude_seamless_view(solid: Solid, source: WorldPortal) -> bool {
    let (min, max) = solid.bounds();
    let center = (min + max) / 2.0;
    let radius = (max - min).length() / 2.0 + SEAMLESS_CUT_EPSILON;
    let offset = center - source.portal.pos;
    let distance = offset.dot(source.portal.normal());

    if distance > radius || distance < -source.seamless_depth - radius {
        return false;
    }

    let half_angle_cos = (source.seamless_angle.clamp(1.0, 360.0).to_radians() * 0.5).cos();
    if half_angle_cos <= -1.0 {
        return true;
    }

    let length = offset.length();
    if length <= radius {
        return true;
    }

    let angular_slack = (radius / length).min(1.0);
    offset.normalize().dot(-source.portal.normal()) >= half_angle_cos - angular_slack
}

impl Canvas<'_> {
    pub(super) fn solid(&mut self, solid: Solid, fill: Color, outline: Color) {
        self.solid_with_holes(solid, fill, outline, &[]);
    }

    pub(super) fn solid_with_seamless_holes(
        &mut self,
        solid: Solid,
        fill: Color,
        outline: Color,
        portals: &[WorldPortal],
    ) {
        self.solid_with_holes(solid, fill, outline, portals);
    }

    fn solid_with_holes(
        &mut self,
        solid: Solid,
        fill: Color,
        outline: Color,
        portals: &[WorldPortal],
    ) {
        let corners = solid.corners().map(|corner| self.world_to_screen(corner));
        let (min, max) = corners.into_iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(min, max), corner| (min.min(corner), max.max(corner)),
        );
        let cuts = seamless_cuts_for_solid(solid, portals);
        let x0 = min.x.max(0.0) as i32;
        let y0 = min.y.max(0.0) as i32;
        let x1 = max.x.min(self.width as f32) as i32;
        let y1 = max.y.min(self.height as f32) as i32;

        for yy in y0..y1 {
            for xx in x0..x1 {
                let point = self.screen_to_world(Vec2::new(xx as f32 + 0.5, yy as f32 + 0.5));
                if solid.contains_point(point) && !seamless_portal_cuts_point(point, &cuts) {
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

        let local_y = panel.size().y * 0.68;
        let stripe_h = 10.0;
        let stripe = Solid::rotated(
            panel.pos().x,
            panel.pos().y + local_y,
            panel.size().x,
            stripe_h,
            panel.rotation(),
            false,
        );

        self.solid(stripe, Color::rgb(210, 170, 42), Color::rgb(42, 36, 30));
    }

    pub(super) fn hazard(&mut self, hazard: Hazard) {
        let solid = hazard.solid;

        self.solid(solid, Color::rgb(36, 105, 62), Color::rgb(124, 255, 120));
        let wave_count = (solid.size().x / 24.0).ceil().max(2.0) as usize;

        for index in 0..wave_count {
            let x0 = index as f32 / wave_count as f32 * solid.size().x;
            let x1 = (index as f32 + 0.5) / wave_count as f32 * solid.size().x;
            let x2 = (index as f32 + 1.0) / wave_count as f32 * solid.size().x;
            let y_mid = solid.size().y * 0.42;
            let y_peak = solid.size().y * 0.22;

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
            center + Vec2::new(0.0, solid.size().y / 2.0),
            center + Vec2::new(0.0, -solid.size().y / 2.0),
            Color::rgb(80, 190, 255),
        );
        self.draw_world_line(
            center + Vec2::new(0.0, -solid.size().y / 2.0),
            center + Vec2::new(solid.size().x * 0.36, -solid.size().y * 0.3),
            Color::rgb(80, 190, 255),
        );
        self.draw_world_line(
            center + Vec2::new(solid.size().x * 0.36, -solid.size().y * 0.3),
            center + Vec2::new(0.0, -solid.size().y * 0.1),
            Color::rgb(80, 190, 255),
        );
    }

    pub(super) fn enemy(&mut self, enemy: &Enemy) {
        match enemy.kind {
            EnemyKind::Filth => self.filth(enemy),
        }
    }

    fn filth(&mut self, enemy: &Enemy) {
        let solid = enemy.solid();
        let flash = enemy.hurt_flash > 0.0;
        let fill = if flash {
            Color::rgb(255, 235, 235)
        } else {
            Color::rgb(112, 20, 24)
        };
        let outline = if flash {
            Color::rgb(255, 64, 64)
        } else {
            Color::rgb(255, 48, 38)
        };
        let center = solid.center();

        self.solid(solid, fill, outline);
        self.draw_world_line(
            center + Vec2::new(-solid.size().x * 0.32, -solid.size().y * 0.12),
            center + Vec2::new(solid.size().x * 0.32, -solid.size().y * 0.12),
            Color::rgb(20, 5, 5),
        );
        self.fill_world_rect(
            Rect {
                pos: center + Vec2::new(-11.0, -15.0),
                size: Vec2::new(6.0, 6.0),
            },
            Color::rgb(255, 220, 120),
        );
        self.fill_world_rect(
            Rect {
                pos: center + Vec2::new(5.0, -15.0),
                size: Vec2::new(6.0, 6.0),
            },
            Color::rgb(255, 220, 120),
        );
    }

    pub(super) fn piercer_beam(&mut self, beam: PiercerBeam) {
        let color = if beam.charged {
            Color::rgb(39, 221, 255)
        } else {
            Color::rgb(255, 226, 80)
        };
        let bloom = if beam.charged {
            Color::rgb(9, 86, 98)
        } else {
            Color::rgb(255, 130, 40)
        };
        let normal = (beam.end - beam.start).normalize_or_zero();
        let side = Vec2::new(-normal.y, normal.x);

        self.draw_world_line(beam.start + side * 2.0, beam.end + side * 2.0, bloom);
        self.draw_world_line(beam.start - side * 2.0, beam.end - side * 2.0, bloom);
        self.draw_world_line(beam.start, beam.end, color);
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

    pub(super) fn seamless_portal_views(&mut self, world: &World) {
        for (source_index, source) in world.level.world_portals.iter().enumerate() {
            if !source.seamless {
                continue;
            }
            let Some(destination_index) =
                seamless_receiver_index(&world.level.world_portals, source_index)
            else {
                continue;
            };
            let Some(destination) = world.level.world_portals.get(destination_index) else {
                continue;
            };

            self.seamless_portal_view(world, *source, *destination);
        }
    }

    fn seamless_portal_view(
        &mut self,
        world: &World,
        source: WorldPortal,
        destination: WorldPortal,
    ) {
        let previous_clip = self.replace_clip(Some(WorldClip::behind_portal(
            source.portal.pos,
            source.portal.normal(),
            source.seamless_depth,
            source.seamless_angle,
            seamless_occluding_walls(world, source),
        )));

        self.transformed_world(world, destination.portal, source.portal);
        self.replace_clip(previous_clip);
    }

    fn transformed_world(&mut self, world: &World, from: Portal, to: Portal) {
        for solid in &world.level.solids {
            let Some(solid) = transformed_solid(from, to, *solid) else {
                continue;
            };
            let fill = if solid.portalable {
                Color::rgb(36, 42, 53)
            } else {
                Color::rgb(55, 45, 47)
            };

            self.solid(solid, fill, Color::rgb(92, 105, 125));
        }

        for door in &world.level.doors {
            let Some(solid) = transformed_solid(from, to, door.solid) else {
                continue;
            };
            let mut door = *door;

            door.solid = solid;
            self.door(door);
        }

        for hazard in &world.level.hazards {
            if let Some(solid) = transformed_solid(from, to, hazard.solid) {
                self.hazard(Hazard { solid });
            }
        }

        for checkpoint in &world.level.checkpoints {
            if let Some(solid) = transformed_solid(from, to, checkpoint.solid) {
                self.checkpoint(Checkpoint { solid });
            }
        }

        for text in &world.level.texts {
            self.world_text(
                from.map_view_point_to(&to, text.pos),
                &text.text,
                2,
                Color::rgb(210, 218, 230),
            );
        }

        for portal in &world.level.world_portals {
            if portal.seamless {
                continue;
            }

            let a = from.map_view_point_to(&to, portal.portal.endpoints().0);
            let b = from.map_view_point_to(&to, portal.portal.endpoints().1);
            let center = from.map_view_point_to(&to, portal.portal.pos);
            let normal_end =
                from.map_view_point_to(&to, portal.portal.pos + portal.portal.normal() * 16.0);

            self.draw_world_line(a, b, portal.portal.color);
            self.draw_world_line(center, normal_end, portal.portal.color);
        }

        for portal in world.portals.iter().flatten() {
            let a = from.map_view_point_to(&to, portal.endpoints().0);
            let b = from.map_view_point_to(&to, portal.endpoints().1);
            let center = from.map_view_point_to(&to, portal.pos);
            let normal_end = from.map_view_point_to(&to, portal.pos + portal.normal() * 16.0);

            self.draw_world_line(a, b, portal.color);
            self.draw_world_line(center, normal_end, portal.color);
        }

        let player = &world.player;
        let player_solid = Solid::new(
            player.pos.x - player.half_size().x,
            player.pos.y - player.half_size().y,
            player.size.x,
            player.size.y,
            false,
        );
        if let Some(player_solid) = transformed_solid(from, to, player_solid) {
            let fill = if player.is_ground_slamming() {
                Color::rgb(255, 224, 102)
            } else if player.is_dashing() {
                Color::rgb(80, 92, 130)
            } else if player.is_wall_sliding() {
                Color::rgb(62, 76, 92)
            } else {
                Color::rgb(45, 49, 59)
            };

            self.solid(player_solid, fill, Color::rgb(235, 238, 245));
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
        let count = (solid.size().y / 18.0).floor().max(2.0) as usize;

        for index in 1..count {
            let y = index as f32 / count as f32 * solid.size().y;
            self.draw_world_line(
                solid.world_from_local(Vec2::new(0.0, y)),
                solid.world_from_local(Vec2::new(solid.size().x, y)),
                Color::rgb(42, 48, 58),
            );
        }
    }

    pub(super) fn resize_handles(&mut self, solid: Solid, color: Color) {
        let points = [
            Vec2::ZERO,
            Vec2::new(solid.size().x / 2.0, 0.0),
            Vec2::new(solid.size().x, 0.0),
            Vec2::new(solid.size().x, solid.size().y / 2.0),
            solid.size(),
            Vec2::new(solid.size().x / 2.0, solid.size().y),
            Vec2::new(0.0, solid.size().y),
            Vec2::new(0.0, solid.size().y / 2.0),
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
        let radius = solid.size().x.max(solid.size().y) / 2.0 + 18.0;
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

fn seamless_receiver_index(portals: &[WorldPortal], source_index: usize) -> Option<usize> {
    let source = portals.get(source_index)?;
    let mut receivers = portals
        .iter()
        .enumerate()
        .filter(|(index, portal)| *index != source_index && portal.id == source.receiver_id)
        .map(|(index, _)| index);
    let receiver = receivers.next()?;

    receivers.next().is_none().then_some(receiver)
}

fn transformed_solid(from: Portal, to: Portal, solid: Solid) -> Option<Solid> {
    let p0 = from.map_view_point_to(&to, solid.world_from_local(Vec2::ZERO));
    let p1 = from.map_view_point_to(&to, solid.world_from_local(Vec2::new(solid.size().x, 0.0)));
    let p2 = from.map_view_point_to(&to, solid.world_from_local(solid.size()));
    let p3 = from.map_view_point_to(&to, solid.world_from_local(Vec2::new(0.0, solid.size().y)));
    let axis_x = p1 - p0;
    let axis_y = p3 - p0;
    let width = axis_x.length();
    let height = axis_y.length();

    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let center = (p0 + p2) / 2.0;
    let rotation = axis_x.to_angle();

    Some(Solid::rotated(
        center.x - width / 2.0,
        center.y - height / 2.0,
        width,
        height,
        rotation,
        solid.portalable,
    ))
}

fn seamless_occluding_walls(world: &World, source: WorldPortal) -> Vec<Solid> {
    if !source.seamless_rely_on_walls {
        return Vec::new();
    }

    world
        .level
        .solids
        .iter()
        .copied()
        .filter(|solid| {
            !portal_sits_on_solid(source.portal, *solid)
                && solid_can_occlude_seamless_view(*solid, source)
        })
        .collect()
}

fn portal_sits_on_solid(portal: Portal, solid: Solid) -> bool {
    let normal = portal.normal();
    let surface_pos = portal.pos - normal * PORTAL_SURFACE_OFFSET;
    let local = solid.local_from_world(surface_pos);
    let axis_x = solid.axis_x();
    let axis_y = solid.axis_y();

    if local.x >= -SEAMLESS_CUT_EPSILON
        && local.y >= -SEAMLESS_CUT_EPSILON
        && local.x <= solid.size().x + SEAMLESS_CUT_EPSILON
        && local.y <= solid.size().y + SEAMLESS_CUT_EPSILON
    {
        let on_left = local.x.abs() < SEAMLESS_CUT_EPSILON && normal.dot(-axis_x) > 0.95;
        let on_right =
            (local.x - solid.size().x).abs() < SEAMLESS_CUT_EPSILON && normal.dot(axis_x) > 0.95;
        let on_top = local.y.abs() < SEAMLESS_CUT_EPSILON && normal.dot(-axis_y) > 0.95;
        let on_bottom =
            (local.y - solid.size().y).abs() < SEAMLESS_CUT_EPSILON && normal.dot(axis_y) > 0.95;

        return on_left || on_right || on_top || on_bottom;
    }

    false
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

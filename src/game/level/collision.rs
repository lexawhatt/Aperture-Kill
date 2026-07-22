use glam::Vec2;

// Collision resolves real solids, but active portals can open a wall.
use crate::constants::CLIMBSTEP_HEIGHT;
use crate::constants::PORTAL_SURFACE_OFFSET;
use crate::game::geometry::projected_extent;
use crate::game::player::Player;
use crate::game::portal::Portal;

use super::{Level, RAY_EPSILON, Solid};

impl Level {
    pub fn resolve_player(&self, player: &mut Player) {
        self.resolve_player_with_portals(player, &[]);
    }

    pub fn resolve_player_with_portals(&self, player: &mut Player, portals: &[Portal]) {
        let was_on_ground = player.on_ground;
        player.clear_contacts();

        for solid in self.solids.iter().copied() {
            if let Some(correction) = body_solid_overlap(player.pos, player.half_size(), solid) {
                if portal_opens_collision(
                    player.pos,
                    player.half_size(),
                    correction,
                    solid,
                    portals,
                ) {
                    continue;
                }

                if self.try_climb_step(player, solid, correction, was_on_ground, portals) {
                    continue;
                }

                resolve_body_push(player, correction, was_on_ground);
            }
        }

        for door in &self.doors {
            if !door.blocks_player() {
                continue;
            }

            let solid = door.moving_solid();
            if let Some(correction) = body_solid_overlap(player.pos, player.half_size(), solid) {
                resolve_body_push(player, correction, was_on_ground);
            }
        }

        self.try_uncrouch_player(player, portals);
    }

    pub fn resolve_actor_body(&self, center: &mut Vec2, half_size: Vec2, vel: &mut Vec2) -> bool {
        let mut on_ground = false;

        for solid in self.solids.iter().copied() {
            if let Some(correction) = body_solid_overlap(*center, half_size, solid) {
                on_ground |= resolve_actor_push(center, vel, correction);
            }
        }

        for door in &self.doors {
            if !door.blocks_player() {
                continue;
            }

            if let Some(correction) = body_solid_overlap(*center, half_size, door.moving_solid()) {
                on_ground |= resolve_actor_push(center, vel, correction);
            }
        }

        on_ground
    }

    fn try_climb_step(
        &self,
        player: &mut Player,
        solid: Solid,
        correction: Vec2,
        was_on_ground: bool,
        portals: &[Portal],
    ) -> bool {
        if !was_on_ground {
            return false;
        }
        if correction.x.abs() <= correction.y.abs() || solid.rotation().abs() > RAY_EPSILON {
            return false;
        }

        let normal = correction.normalize_or_zero();
        if player.vel.dot(normal) >= 0.0 {
            return false;
        }

        let bottom = player.pos.y + player.half_size().y;
        let step = bottom - solid.pos.y;
        if !(0.0..=CLIMBSTEP_HEIGHT).contains(&step) {
            return false;
        }

        let candidate = player.pos - Vec2::Y * (step + RAY_EPSILON);
        if self.body_blocked(candidate, player.half_size(), portals) {
            return false;
        }

        player.pos = candidate;
        player.touch_ground();
        true
    }

    fn try_uncrouch_player(&self, player: &mut Player, portals: &[Portal]) {
        let Some((center, half_size)) = player.standing_body() else {
            return;
        };

        if !self.body_blocked(center, half_size, portals) {
            player.stand_up();
        }
    }

    fn body_blocked(&self, center: Vec2, half_size: Vec2, portals: &[Portal]) -> bool {
        for solid in self.solids.iter().copied() {
            let Some(correction) = body_solid_overlap(center, half_size, solid) else {
                continue;
            };
            if !portal_opens_collision(center, half_size, correction, solid, portals) {
                return true;
            }
        }

        self.doors.iter().any(|door| {
            door.blocks_player()
                && body_solid_overlap(center, half_size, door.moving_solid()).is_some()
        })
    }
}

fn resolve_body_push(player: &mut Player, correction: Vec2, was_on_ground: bool) {
    player.pos += correction;
    let normal = correction.normalize_or_zero();
    let into_surface = player.vel.dot(normal);
    let incoming_speed = player.vel.dot(-normal);
    let ground_contact = normal.y < -0.55;
    let down_speed = if ground_contact {
        player.vel.y.max(0.0)
    } else {
        0.0
    };

    if into_surface < 0.0 {
        player.vel -= normal * into_surface;
    }
    if ground_contact {
        player.touch_ground_contact(down_speed, !was_on_ground);
    } else if normal.x.abs() > 0.55 && !player.try_wall_slam(normal) {
        player.set_wall_contact_with_speed(-normal.x.signum(), incoming_speed);
    }
}

fn resolve_actor_push(center: &mut Vec2, vel: &mut Vec2, correction: Vec2) -> bool {
    *center += correction;
    let normal = correction.normalize_or_zero();
    let into_surface = vel.dot(normal);

    if into_surface < 0.0 {
        *vel -= normal * into_surface;
    }

    normal.y < -0.55
}

fn portal_opens_collision(
    center: Vec2,
    half_size: Vec2,
    correction: Vec2,
    solid: Solid,
    portals: &[Portal],
) -> bool {
    let correction_normal = correction.normalize_or_zero();

    portals.iter().any(|portal| {
        // A portal opens only its front face; the back side of the same wall still blocks.
        portal_sits_on_solid(*portal, solid)
            && correction_normal.dot(portal.normal()) > 0.92
            && portal.opens_for_body(center, half_size)
    })
}

fn portal_sits_on_solid(portal: Portal, solid: Solid) -> bool {
    let normal = portal.normal();
    let surface_pos = portal.pos - normal * PORTAL_SURFACE_OFFSET;
    let local = solid.local_from_world(surface_pos);
    let axis_x = solid.axis_x();
    let axis_y = solid.axis_y();

    if local.x >= -RAY_EPSILON
        && local.y >= -RAY_EPSILON
        && local.x <= solid.size.x + RAY_EPSILON
        && local.y <= solid.size.y + RAY_EPSILON
    {
        let on_left = local.x.abs() < RAY_EPSILON && normal.dot(-axis_x) > 0.95;
        let on_right = (local.x - solid.size.x).abs() < RAY_EPSILON && normal.dot(axis_x) > 0.95;
        let on_top = local.y.abs() < RAY_EPSILON && normal.dot(-axis_y) > 0.95;
        let on_bottom = (local.y - solid.size.y).abs() < RAY_EPSILON && normal.dot(axis_y) > 0.95;

        return on_left || on_right || on_top || on_bottom;
    }

    false
}

fn body_solid_overlap(center: Vec2, half_size: Vec2, solid: Solid) -> Option<Vec2> {
    let solid_center = solid.center();
    let (axis_x, axis_y) = solid.basis();
    let axes = [Vec2::X, Vec2::Y, axis_x, axis_y];
    let mut smallest_overlap = f32::INFINITY;
    let mut correction_axis = Vec2::ZERO;

    for axis in axes {
        let player_extent = projected_extent(half_size, axis);
        let solid_extent =
            projected_extent(solid.size / 2.0, axis_from_solid(axis, axis_x, axis_y));
        let delta = center.dot(axis) - solid_center.dot(axis);
        let overlap = player_extent + solid_extent - delta.abs();

        if overlap <= 0.0 {
            return None;
        }

        // The minimum separating axis is the shortest correction that fully clears the overlap.
        if overlap < smallest_overlap {
            smallest_overlap = overlap;
            let direction = if delta >= 0.0 { 1.0 } else { -1.0 };
            correction_axis = axis * direction;
        }
    }

    Some(correction_axis * smallest_overlap)
}

fn axis_from_solid(axis: Vec2, solid_axis_x: Vec2, solid_axis_y: Vec2) -> Vec2 {
    Vec2::new(axis.dot(solid_axis_x).abs(), axis.dot(solid_axis_y).abs())
}

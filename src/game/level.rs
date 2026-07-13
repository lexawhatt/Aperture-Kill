use glam::Vec2;

use crate::game::player::Player;

const RAY_EPSILON: f32 = 0.001;

#[derive(Clone, Copy)]
pub struct Solid {
    pub pos: Vec2,
    pub size: Vec2,
    pub portalable: bool,
}

impl Solid {
    pub const fn new(x: f32, y: f32, w: f32, h: f32, portalable: bool) -> Self {
        Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
            portalable,
        }
    }

    fn center(self) -> Vec2 {
        self.pos + self.size / 2.0
    }
}

pub struct Level {
    pub solids: Vec<Solid>,
}

#[derive(Clone, Copy)]
pub struct RayHit {
    pub point: Vec2,
    pub normal: Vec2,
    // Span of the hit surface along the portal line.
    pub surface_span: f32,
}

impl Level {
    pub fn test_level() -> Self {
        Self {
            solids: vec![
                Solid::new(0.0, 560.0, 900.0, 40.0, true),
                Solid::new(0.0, 0.0, 30.0, 600.0, true),
                Solid::new(870.0, 0.0, 30.0, 600.0, true),
                Solid::new(0.0, 0.0, 900.0, 30.0, true),
                Solid::new(170.0, 430.0, 210.0, 28.0, true),
                Solid::new(520.0, 390.0, 220.0, 28.0, true),
                Solid::new(430.0, 250.0, 32.0, 170.0, true),
                Solid::new(250.0, 150.0, 180.0, 26.0, true),
                Solid::new(610.0, 160.0, 32.0, 160.0, true),
            ],
        }
    }

    pub fn resolve_player(&self, player: &mut Player) {
        player.clear_contacts();

        for solid in &self.solids {
            if let Some(overlap) = aabb_overlap(player.pos, player.half_size(), *solid) {
                if overlap.x < overlap.y {
                    let dir = (player.pos.x - solid.center().x).signum();
                    player.pos.x += overlap.x * dir;
                    if (dir < 0.0 && player.vel.x > 0.0) || (dir > 0.0 && player.vel.x < 0.0) {
                        player.vel.x = 0.0;
                    }
                    player.set_wall_contact(-dir);
                } else {
                    let dir = (player.pos.y - solid.center().y).signum();
                    player.pos.y += overlap.y * dir;
                    if dir < 0.0 {
                        player.land();
                    } else if player.vel.y < 0.0 {
                        player.vel.y = 0.0;
                    }
                }
            }
        }
    }

    pub fn raycast_portalable(&self, origin: Vec2, target: Vec2) -> Option<RayHit> {
        let dir = target - origin;
        if dir.length_squared() <= 1.0 {
            return None;
        }

        let max_distance = dir.length();
        let ray_dir = dir / max_distance;
        self.solids
            .iter()
            .filter(|solid| solid.portalable)
            .filter_map(|solid| raycast_solid(origin, ray_dir, max_distance, *solid))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, hit)| hit)
    }
}

fn aabb_overlap(center: Vec2, half_size: Vec2, solid: Solid) -> Option<Vec2> {
    let solid_half = solid.size / 2.0;
    let delta = center - solid.center();
    let overlap = half_size + solid_half - delta.abs();

    if overlap.x > 0.0 && overlap.y > 0.0 {
        Some(overlap)
    } else {
        None
    }
}

fn raycast_solid(
    origin: Vec2,
    dir: Vec2,
    max_distance: f32,
    solid: Solid,
) -> Option<(f32, RayHit)> {
    let min = solid.pos;
    let max = solid.pos + solid.size;
    let mut t_min = 0.0;
    let mut t_max = max_distance;
    let mut normal = Vec2::ZERO;

    for axis in 0..2 {
        let origin_axis = origin[axis];
        let dir_axis = dir[axis];
        let min_axis = min[axis];
        let max_axis = max[axis];

        if dir_axis.abs() < RAY_EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }

        let inv_dir = 1.0 / dir_axis;
        let t1 = (min_axis - origin_axis) * inv_dir;
        let t2 = (max_axis - origin_axis) * inv_dir;
        let near = t1.min(t2);
        let far = t1.max(t2);
        let mut axis_normal = Vec2::ZERO;
        axis_normal[axis] = if t1 < t2 { -1.0 } else { 1.0 };

        if near > t_min {
            t_min = near;
            normal = axis_normal;
        }
        t_max = t_max.min(far);

        if t_min > t_max {
            return None;
        }
    }

    if t_min < 0.0 || t_min > max_distance {
        return None;
    }

    Some((
        t_min,
        RayHit {
            point: origin + dir * t_min,
            normal,
            surface_span: if normal.x != 0.0 {
                solid.size.y
            } else {
                solid.size.x
            },
        },
    ))
}

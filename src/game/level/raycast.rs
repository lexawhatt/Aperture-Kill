use glam::Vec2;

// Raycasts find portalable surfaces and slide portals away from blocked edges.
use crate::constants::PORTAL_SURFACE_OFFSET;

use super::{Level, PORTAL_CLEARANCE, RAY_EPSILON, RayHit, Solid};

impl Level {
    pub fn raycast_portalable(&self, origin: Vec2, target: Vec2) -> Option<RayHit> {
        let dir = target - origin;
        if dir.length_squared() <= 1.0 {
            return None;
        }

        // Keep nearest hit only; later walls are hidden behind it.
        let max_distance = dir.length();
        let ray_dir = dir / max_distance;
        self.solids
            .iter()
            .enumerate()
            .filter(|(_, solid)| solid.portalable)
            .filter_map(|(index, solid)| {
                raycast_solid(origin, ray_dir, max_distance, *solid, index)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, hit)| hit)
    }

    pub fn portal_center(&self, hit: RayHit, portal_width: f32) -> Option<Vec2> {
        hit.portal_center(portal_width)?;
        let half_width = portal_width / 2.0;
        let min_center = hit.surface_min + half_width;
        let max_center = hit.surface_max - half_width;
        let desired = hit.surface_coord.clamp(min_center, max_center);
        let step = 4.0;
        let search_steps = ((max_center - min_center) / step).ceil() as i32;

        for offset_index in 0..=search_steps {
            for direction in [1.0, -1.0] {
                if offset_index == 0 && direction < 0.0 {
                    continue;
                }

                let axis_pos = (desired + direction * offset_index as f32 * step)
                    .clamp(min_center, max_center);
                if portal_segment_clear(hit, axis_pos, half_width, &self.solids) {
                    let surface_center = hit.point + hit.tangent * (axis_pos - hit.surface_coord);

                    return Some(surface_center + hit.normal * PORTAL_SURFACE_OFFSET);
                }
            }
        }

        None
    }
}

fn portal_segment_clear(hit: RayHit, axis_pos: f32, half_width: f32, solids: &[Solid]) -> bool {
    let surface_center = hit.point + hit.tangent * (axis_pos - hit.surface_coord);
    let samples = 12;

    solids
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != hit.solid_index)
        .all(|(_, solid)| {
            (0..=samples).all(|sample| {
                let t = sample as f32 / samples as f32;
                let along = (t - 0.5) * half_width * 2.0;
                let point =
                    surface_center + hit.tangent * along + hit.normal * PORTAL_SURFACE_OFFSET;

                !solid.contains_point(point)
                    && !solid.contains_point(point + hit.normal * PORTAL_CLEARANCE)
                    && !solid.contains_point(point - hit.normal * PORTAL_CLEARANCE)
            })
        })
}

fn raycast_solid(
    origin: Vec2,
    dir: Vec2,
    max_distance: f32,
    solid: Solid,
    solid_index: usize,
) -> Option<(f32, RayHit)> {
    let local_origin = solid.local_from_world(origin);
    let (axis_x, axis_y) = solid.basis();
    let local_dir = Vec2::new(dir.dot(axis_x), dir.dot(axis_y));
    let min = Vec2::ZERO;
    let max = solid.size;
    let mut t_min = 0.0;
    let mut t_max = max_distance;
    let mut normal = Vec2::ZERO;
    let mut hit_axis = 0;

    for axis in 0..2 {
        let origin_axis = local_origin[axis];
        let dir_axis = local_dir[axis];
        let min_axis = min[axis];
        let max_axis = max[axis];

        if dir_axis.abs() < RAY_EPSILON {
            // Parallel rays only hit if they start inside this slab.
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
            // The latest near plane is the surface the ray actually enters.
            t_min = near;
            normal = axis_normal;
            hit_axis = axis;
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
        ray_hit(origin, dir, t_min, hit_axis, normal, solid, solid_index),
    ))
}

fn ray_hit(
    origin: Vec2,
    dir: Vec2,
    t_min: f32,
    hit_axis: usize,
    normal: Vec2,
    solid: Solid,
    solid_index: usize,
) -> RayHit {
    let surface_axis = if hit_axis == 0 { 1 } else { 0 };
    let local_origin = solid.local_from_world(origin);
    let (axis_x, axis_y) = solid.basis();
    let local_dir = Vec2::new(dir.dot(axis_x), dir.dot(axis_y));
    let local_point = local_origin + local_dir * t_min;
    let world_normal = axis_x * normal.x + axis_y * normal.y;
    let tangent = if surface_axis == 0 { axis_x } else { axis_y };

    // Surface bounds let portal placement slide away from edges.
    RayHit {
        point: solid.world_from_local(local_point),
        normal: world_normal,
        tangent,
        solid_index,
        surface_coord: local_point[surface_axis],
        surface_min: 0.0,
        surface_max: solid.size[surface_axis],
        surface_span: solid.size[surface_axis],
    }
}

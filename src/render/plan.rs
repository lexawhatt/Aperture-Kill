use glam::Vec2;

use crate::constants::PORTAL_SURFACE_OFFSET;
use crate::game::World;
use crate::game::level::{Solid, WorldPortal};

pub(super) const SEAMLESS_CUT_EPSILON: f32 = 2.0;

#[derive(Clone, Default)]
pub(super) struct RenderPlan {
    pub(super) world_portals: WorldPortalRenderPlan,
}

impl RenderPlan {
    pub(super) fn build(world: &World) -> Self {
        Self {
            world_portals: WorldPortalRenderPlan::build(world),
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct WorldPortalRenderPlan {
    solid_cuts: Vec<Vec<SeamlessCut>>,
    pub(super) seamless_views: Vec<SeamlessPortalViewPlan>,
}

impl WorldPortalRenderPlan {
    fn build(world: &World) -> Self {
        let seamless_portals = world
            .level
            .world_portals
            .iter()
            .copied()
            .filter(|portal| portal.seamless)
            .collect::<Vec<_>>();
        let solid_cuts = world
            .level
            .solids
            .iter()
            .copied()
            .map(|solid| seamless_cuts_for_solid(solid, &seamless_portals))
            .collect();
        let seamless_views = world
            .level
            .world_portals
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(source_index, source)| {
                if !source.seamless {
                    return None;
                }

                let destination_index =
                    WorldPortal::unique_receiver_index(&world.level.world_portals, source_index)?;
                let destination = *world.level.world_portals.get(destination_index)?;

                Some(SeamlessPortalViewPlan {
                    #[cfg(test)]
                    source_index,
                    #[cfg(test)]
                    destination_index,
                    source,
                    destination,
                    occluding_walls: seamless_occluding_walls(world, source),
                })
            })
            .collect();

        Self {
            solid_cuts,
            seamless_views,
        }
    }

    pub(super) fn cuts_for_solid(&self, solid_index: usize) -> &[SeamlessCut] {
        self.solid_cuts.get(solid_index).map_or(&[], Vec::as_slice)
    }

    #[cfg(test)]
    fn solid_cut_count(&self, solid_index: usize) -> usize {
        self.cuts_for_solid(solid_index).len()
    }
}

#[derive(Clone)]
pub(super) struct SeamlessPortalViewPlan {
    #[cfg(test)]
    pub(super) source_index: usize,
    #[cfg(test)]
    pub(super) destination_index: usize,
    pub(super) source: WorldPortal,
    pub(super) destination: WorldPortal,
    pub(super) occluding_walls: Vec<Solid>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SeamlessCut {
    pos: Vec2,
    normal: Vec2,
    tangent: Vec2,
    half_width: f32,
    depth: f32,
}

pub(super) fn seamless_portal_cuts_point(point: Vec2, cuts: &[SeamlessCut]) -> bool {
    cuts.iter().any(|cut| {
        let offset = point - cut.pos;
        let tangent_distance = offset.dot(cut.tangent).abs();
        let normal_distance = offset.dot(cut.normal);

        tangent_distance <= cut.half_width
            && normal_distance <= PORTAL_SURFACE_OFFSET + SEAMLESS_CUT_EPSILON
            && normal_distance >= -cut.depth
    })
}

fn seamless_cuts_for_solid(solid: Solid, portals: &[WorldPortal]) -> Vec<SeamlessCut> {
    portals
        .iter()
        .filter_map(|world_portal| {
            if !solid.supports_portal(world_portal.portal, SEAMLESS_CUT_EPSILON) {
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
            !solid.supports_portal(source.portal, SEAMLESS_CUT_EPSILON)
                && solid_can_occlude_seamless_view(*solid, source)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;
    use crate::game::World;
    use crate::game::level::{Solid, WorldPortal};

    #[test]
    fn seamless_plan_builds_unique_receiver_views() {
        let mut world = World::new();
        let mut source = WorldPortal::new(100.0, 560.0, Vec2::new(0.0, -1.0), 64.0, 1);
        source.receiver_id = 2;
        source.seamless = true;
        let receiver = WorldPortal::new(300.0, 560.0, Vec2::new(0.0, -1.0), 64.0, 2);

        world.level.world_portals = vec![source, receiver];

        let plan = RenderPlan::build(&world);

        assert_eq!(plan.world_portals.seamless_views.len(), 1);
        assert_eq!(plan.world_portals.seamless_views[0].source_index, 0);
        assert_eq!(plan.world_portals.seamless_views[0].destination_index, 1);
    }

    #[test]
    fn seamless_plan_rejects_ambiguous_receivers() {
        let mut world = World::new();
        let mut source = WorldPortal::new(100.0, 560.0, Vec2::new(0.0, -1.0), 64.0, 1);
        source.receiver_id = 2;
        source.seamless = true;
        let receiver = WorldPortal::new(300.0, 560.0, Vec2::new(0.0, -1.0), 64.0, 2);
        let duplicate = WorldPortal::new(500.0, 560.0, Vec2::new(0.0, -1.0), 64.0, 2);

        world.level.world_portals = vec![source, receiver, duplicate];

        let plan = RenderPlan::build(&world);

        assert!(plan.world_portals.seamless_views.is_empty());
    }

    #[test]
    fn seamless_plan_precomputes_solid_cuts() {
        let mut world = World::new();
        world.level.solids = vec![Solid::new(0.0, 540.0, 400.0, 40.0, true)];
        let mut source = WorldPortal::new(100.0, 538.0, Vec2::new(0.0, -1.0), 64.0, 1);
        source.seamless = true;
        let receiver = WorldPortal::new(300.0, 538.0, Vec2::new(0.0, -1.0), 64.0, 1);

        world.level.world_portals = vec![source, receiver];

        let plan = RenderPlan::build(&world);

        assert_eq!(plan.world_portals.solid_cut_count(0), 1);
        assert!(seamless_portal_cuts_point(
            Vec2::new(100.0, 555.0),
            plan.world_portals.cuts_for_solid(0)
        ));
    }

    #[test]
    fn seamless_plan_tracks_occluding_walls_once_per_view() {
        let mut world = World::new();
        world.level.solids = vec![
            Solid::new(0.0, 540.0, 400.0, 40.0, true),
            Solid::new(92.0, 620.0, 16.0, 40.0, true),
        ];
        let mut source = WorldPortal::new(100.0, 538.0, Vec2::new(0.0, -1.0), 64.0, 1);
        source.receiver_id = 2;
        source.seamless = true;
        source.seamless_rely_on_walls = true;
        let receiver = WorldPortal::new(300.0, 538.0, Vec2::new(0.0, -1.0), 64.0, 2);

        world.level.world_portals = vec![source, receiver];

        let plan = RenderPlan::build(&world);

        assert_eq!(plan.world_portals.seamless_views.len(), 1);
        assert_eq!(
            plan.world_portals.seamless_views[0].occluding_walls.len(),
            1
        );
    }
}

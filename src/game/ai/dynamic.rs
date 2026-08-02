use glam::Vec2;

use super::PortalLink;
use crate::constants::{FILTH_SIGHT_RANGE, PORTAL_SURFACE_OFFSET};
use crate::game::enemy::Enemy;
use crate::game::geometry::projected_extent;
use crate::game::level::{CollisionGeometry, Level, Solid};
use crate::game::portal::Portal;

const PORTAL_EXIT_OFFSET: f32 = PORTAL_SURFACE_OFFSET + 0.25;
const NAV_EDGE_MARGIN: f32 = 4.0;
const NAV_SURFACE_SNAP: f32 = 12.0;
const NAV_DROP_COST_SCALE: f32 = 0.15;
const PORTAL_ROUTE_COST: f32 = 24.0;

#[derive(Clone, Copy, Debug)]
pub(in crate::game) struct EnemyTarget {
    pub pos: Vec2,
    pub can_attack: bool,
}

#[derive(Clone, Copy, Debug)]
struct EnemyRouteTarget {
    pos: Vec2,
    can_attack: bool,
}

#[derive(Clone, Copy, Debug)]
struct NavSurface {
    min_x: f32,
    max_x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct NavState {
    cost: f32,
    x: f32,
    first_x: f32,
    visited: bool,
}

#[derive(Clone, Copy, Debug)]
struct PortalNavEdge {
    from_surface: usize,
    from_x: f32,
    to_surface: usize,
    to_x: f32,
    cost: f32,
}

#[derive(Clone, Copy, Debug)]
struct SurfaceRouteRequest {
    start_surface: usize,
    start_x: f32,
    target_surface: usize,
    target_x: f32,
    half_width: f32,
}

#[derive(Clone, Copy, Debug)]
struct NavRelax {
    current_pos: usize,
    next_pos: usize,
    first_step_x: f32,
    next_x: f32,
    cost: f32,
}

pub(in crate::game) fn enemy_targets(
    level: &Level,
    enemies: &[Enemy],
    player_pos: Vec2,
    portal_links: &[PortalLink],
) -> Vec<EnemyTarget> {
    let surfaces = nav_surfaces(level);
    let mut portal_edges = Vec::new();
    let mut portal_edge_half_size = None;

    enemies
        .iter()
        .map(|enemy| {
            if !enemy.is_active() {
                return EnemyTarget {
                    pos: enemy.pos,
                    can_attack: false,
                };
            }

            let half_size = enemy.half_size();
            if portal_edge_half_size != Some(half_size) {
                portal_edges = portal_nav_edges(&surfaces, portal_links, half_size);
                portal_edge_half_size = Some(half_size);
            }

            enemy_target(
                level,
                &surfaces,
                &portal_edges,
                enemy.pos,
                half_size,
                player_pos,
                portal_links,
            )
        })
        .collect()
}

pub(in crate::game) fn teleport_enemy_through_portals(
    enemy: &mut Enemy,
    portal_links: &[PortalLink],
) {
    let half_size = enemy.half_size();
    let mut best = None;

    for link in portal_links {
        let Some(time) = link
            .source
            .crossing_time(enemy.prev_pos, enemy.pos, half_size)
        else {
            continue;
        };

        if best.is_none_or(|(best_time, _)| time < best_time) {
            best = Some((time, *link));
        }
    }

    let Some((_, link)) = best else {
        return;
    };
    let size = enemy.size();

    link.source.teleport_actor_to(
        &link.destination,
        &mut enemy.prev_pos,
        &mut enemy.pos,
        size,
        &mut enemy.vel,
    );
}

pub(in crate::game) fn separate_enemies(
    enemies: &mut [Enemy],
    collision: CollisionGeometry<'_>,
    portals: &[Portal],
) {
    if enemies
        .iter()
        .filter(|enemy| enemy.is_active())
        .take(2)
        .count()
        < 2
    {
        return;
    }

    for _ in 0..3 {
        let mut moved = false;

        for left_index in 0..enemies.len() {
            let (left, right) = enemies.split_at_mut(left_index + 1);
            let a = &mut left[left_index];
            if !a.is_active() {
                continue;
            }

            for b in right {
                if !b.is_active() {
                    continue;
                }

                let Some(push) = enemy_overlap_push(a, b) else {
                    continue;
                };

                let normal = push.normalize_or_zero();

                a.pos -= push * 0.5;
                b.pos += push * 0.5;

                let a_into_b = a.vel.dot(normal);
                if a_into_b > 0.0 {
                    a.vel -= normal * a_into_b;
                }

                let b_into_a = b.vel.dot(-normal);
                if b_into_a > 0.0 {
                    b.vel += normal * b_into_a;
                }

                moved = true;
            }
        }

        if !moved {
            break;
        }

        for enemy in enemies.iter_mut().filter(|enemy| enemy.is_active()) {
            let half_size = enemy.half_size();

            enemy.on_ground = collision.resolve_actor_body_with_portals(
                &mut enemy.pos,
                half_size,
                &mut enemy.vel,
                portals,
            );
        }
    }
}

fn enemy_target(
    level: &Level,
    surfaces: &[NavSurface],
    portal_edges: &[PortalNavEdge],
    enemy_pos: Vec2,
    enemy_half_size: Vec2,
    player_pos: Vec2,
    portal_links: &[PortalLink],
) -> EnemyTarget {
    let direct_clear = line_clear(level, enemy_pos, player_pos);
    let direct_visible = direct_clear && enemy_pos.distance(player_pos) <= FILTH_SIGHT_RANGE;
    let platform_visible = horizontal_line_clear(level, enemy_pos, player_pos)
        && enemy_pos.distance(player_pos) <= FILTH_SIGHT_RANGE;
    let portal_target = portal_visible_enemy_target(level, enemy_pos, player_pos, portal_links);

    if !direct_visible && !platform_visible && portal_target.is_none() {
        return EnemyTarget {
            pos: enemy_pos,
            can_attack: false,
        };
    }

    enemy_route_target(
        surfaces,
        portal_edges,
        enemy_pos,
        enemy_half_size,
        player_pos,
        direct_clear,
    )
    .map(|target| EnemyTarget {
        pos: target.pos,
        can_attack: target.can_attack,
    })
    .unwrap_or_else(|| {
        portal_target.unwrap_or(EnemyTarget {
            pos: player_pos,
            can_attack: direct_visible,
        })
    })
}

fn enemy_route_target(
    surfaces: &[NavSurface],
    portal_edges: &[PortalNavEdge],
    enemy_pos: Vec2,
    enemy_half_size: Vec2,
    player_pos: Vec2,
    direct_clear: bool,
) -> Option<EnemyRouteTarget> {
    let start_surface = nav_surface_at(surfaces, enemy_pos, enemy_half_size)?;
    let target_surface = nav_surface_at(surfaces, player_pos, enemy_half_size)?;

    if target_surface == start_surface && direct_clear {
        return Some(EnemyRouteTarget {
            pos: player_pos,
            can_attack: direct_clear,
        });
    } else if target_surface == start_surface {
        return None;
    }

    let route = surface_route_next_x(
        surfaces,
        portal_edges,
        SurfaceRouteRequest {
            start_surface,
            start_x: enemy_pos.x,
            target_surface,
            target_x: player_pos.x,
            half_width: enemy_half_size.x,
        },
    )?;

    let route_target = Vec2::new(route.0, enemy_pos.y);

    Some(EnemyRouteTarget {
        pos: route_target,
        can_attack: false,
    })
}

fn nav_surfaces(level: &Level) -> Vec<NavSurface> {
    level
        .solids
        .iter()
        .copied()
        .chain(
            level
                .doors
                .iter()
                .filter(|door| door.blocks_player())
                .map(|door| door.moving_solid()),
        )
        .filter_map(nav_surface)
        .collect()
}

fn nav_surface(solid: Solid) -> Option<NavSurface> {
    if solid.rotation().abs() > 0.001 {
        return None;
    }

    let pos = solid.pos();
    let size = solid.size();

    (size.x >= 8.0 && size.y >= 4.0).then_some(NavSurface {
        min_x: pos.x,
        max_x: pos.x + size.x,
        y: pos.y,
    })
}

fn nav_surface_at(surfaces: &[NavSurface], pos: Vec2, half_size: Vec2) -> Option<usize> {
    let foot_y = pos.y + half_size.y;

    surfaces
        .iter()
        .enumerate()
        .filter(|(_, surface)| {
            pos.x >= surface.min_x - half_size.x
                && pos.x <= surface.max_x + half_size.x
                && foot_y <= surface.y + NAV_SURFACE_SNAP
                && surface.y >= foot_y - NAV_SURFACE_SNAP
        })
        .min_by(|(_, a), (_, b)| (a.y - foot_y).abs().total_cmp(&(b.y - foot_y).abs()))
        .map(|(index, _)| index)
}

fn surface_route_next_x(
    surfaces: &[NavSurface],
    portal_edges: &[PortalNavEdge],
    request: SurfaceRouteRequest,
) -> Option<(f32, f32)> {
    let mut states = vec![
        NavState {
            cost: f32::INFINITY,
            x: 0.0,
            first_x: 0.0,
            visited: false,
        };
        surfaces.len()
    ];

    states[request.start_surface] = NavState {
        cost: 0.0,
        x: request.start_x,
        first_x: request.start_x,
        visited: false,
    };

    while let Some(current_pos) = states
        .iter()
        .enumerate()
        .filter(|(_, state)| !state.visited && state.cost.is_finite())
        .min_by(|(_, a), (_, b)| a.cost.total_cmp(&b.cost))
        .map(|(index, _)| index)
    {
        if current_pos == request.target_surface {
            let state = states[current_pos];
            let cost = state.cost + (request.target_x - state.x).abs();

            return Some((state.first_x, cost));
        }

        states[current_pos].visited = true;
        let current = surfaces[current_pos];
        let current_state = states[current_pos];

        for direction in [-1.0, 1.0] {
            let edge_x = if direction < 0.0 {
                current.min_x - request.half_width - NAV_EDGE_MARGIN
            } else {
                current.max_x + request.half_width + NAV_EDGE_MARGIN
            };
            let Some(next_pos) = drop_surface_below(surfaces, current_pos, edge_x) else {
                continue;
            };

            let walk_cost = (edge_x - current_state.x).abs();
            let drop_cost = (surfaces[next_pos].y - current.y).max(0.0) * NAV_DROP_COST_SCALE;
            let cost = current_state.cost + walk_cost + drop_cost;

            relax_nav_state(
                &mut states,
                NavRelax {
                    current_pos,
                    next_pos,
                    first_step_x: edge_x,
                    next_x: edge_x,
                    cost,
                },
                request.start_surface,
            );
        }

        for edge in portal_edges
            .iter()
            .filter(|edge| edge.from_surface == current_pos)
        {
            let cost = current_state.cost + (edge.from_x - current_state.x).abs() + edge.cost;

            relax_nav_state(
                &mut states,
                NavRelax {
                    current_pos,
                    next_pos: edge.to_surface,
                    first_step_x: edge.from_x,
                    next_x: edge.to_x,
                    cost,
                },
                request.start_surface,
            );
        }
    }

    None
}

fn relax_nav_state(states: &mut [NavState], step: NavRelax, start_surface: usize) {
    if step.cost >= states[step.next_pos].cost {
        return;
    }

    states[step.next_pos] = NavState {
        cost: step.cost,
        x: step.next_x,
        first_x: if step.current_pos == start_surface {
            step.first_step_x
        } else {
            states[step.current_pos].first_x
        },
        visited: false,
    };
}

fn drop_surface_below(surfaces: &[NavSurface], source_pos: usize, drop_x: f32) -> Option<usize> {
    let source = surfaces[source_pos];

    surfaces
        .iter()
        .enumerate()
        .filter(|(index, surface)| {
            *index != source_pos
                && surface.y > source.y + 1.0
                && drop_x >= surface.min_x
                && drop_x <= surface.max_x
        })
        .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y))
        .map(|(index, _)| index)
}

fn portal_nav_edges(
    surfaces: &[NavSurface],
    portal_links: &[PortalLink],
    half_size: Vec2,
) -> Vec<PortalNavEdge> {
    portal_links
        .iter()
        .filter_map(|link| portal_nav_edge(surfaces, *link, half_size))
        .collect()
}

fn portal_nav_edge(
    surfaces: &[NavSurface],
    link: PortalLink,
    half_size: Vec2,
) -> Option<PortalNavEdge> {
    let source_center = portal_entry_center(link.source, half_size)?;
    let from_surface = nav_surface_at(surfaces, source_center, half_size)?;
    let exit_center = portal_exit_center(link, source_center, half_size);
    let (to_surface, to_x, fall_cost) = nav_landing_at_or_below(surfaces, exit_center, half_size)?;

    Some(PortalNavEdge {
        from_surface,
        from_x: source_center.x,
        to_surface,
        to_x,
        cost: PORTAL_ROUTE_COST + fall_cost,
    })
}

fn portal_entry_center(portal: Portal, half_size: Vec2) -> Option<Vec2> {
    let normal = portal.normal();

    if normal.x.abs() >= normal.y.abs() {
        Some(portal.pos)
    } else if normal.y < 0.0 {
        Some(portal.pos + normal * projected_extent(half_size, normal))
    } else {
        None
    }
}

fn portal_exit_center(link: PortalLink, source_center: Vec2, half_size: Vec2) -> Vec2 {
    let source = link.source;
    let destination = link.destination;
    let extent = projected_extent(half_size, source.normal()) + 1.0;
    let mut previous = source_center + source.normal() * extent;
    let mut current = source_center - source.normal() * extent;
    let mut velocity = -source.normal() * 120.0;

    source.teleport_actor_to(
        &destination,
        &mut previous,
        &mut current,
        half_size * 2.0,
        &mut velocity,
    );

    current
}

fn nav_landing_at_or_below(
    surfaces: &[NavSurface],
    pos: Vec2,
    half_size: Vec2,
) -> Option<(usize, f32, f32)> {
    if let Some(surface) = nav_surface_at(surfaces, pos, half_size) {
        return Some((surface, pos.x, 0.0));
    }

    let foot_y = pos.y + half_size.y;

    surfaces
        .iter()
        .enumerate()
        .filter(|(_, surface)| {
            pos.x >= surface.min_x - half_size.x
                && pos.x <= surface.max_x + half_size.x
                && surface.y > foot_y
        })
        .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y))
        .map(|(index, surface)| {
            (
                index,
                pos.x.clamp(surface.min_x, surface.max_x),
                (surface.y - foot_y).max(0.0) * NAV_DROP_COST_SCALE,
            )
        })
}

fn line_clear(level: &Level, origin: Vec2, target: Vec2) -> bool {
    let distance = origin.distance(target);

    distance <= 1.0
        || level
            .raycast_any_solid(origin, target)
            .is_none_or(|hit| hit.point.distance(origin) + 1.0 >= distance)
}

fn horizontal_line_clear(level: &Level, origin: Vec2, target: Vec2) -> bool {
    let target = Vec2::new(target.x, origin.y);

    line_clear(level, origin, target)
}

fn line_clear_to_portal(level: &Level, origin: Vec2, portal: Portal) -> bool {
    let distance = origin.distance(portal.pos);

    distance <= 1.0
        || level
            .raycast_any_solid(origin, portal.pos)
            .is_none_or(|hit| hit.point.distance(origin) + PORTAL_EXIT_OFFSET >= distance)
}

fn line_clear_from_portal(level: &Level, portal: Portal, target: Vec2) -> bool {
    let origin = portal.pos + portal.normal() * PORTAL_EXIT_OFFSET;

    line_clear(level, origin, target)
}

fn portal_visible_enemy_target(
    level: &Level,
    enemy_pos: Vec2,
    player_pos: Vec2,
    portal_links: &[PortalLink],
) -> Option<EnemyTarget> {
    portal_links
        .iter()
        .filter_map(|link| {
            let route_distance =
                enemy_pos.distance(link.source.pos) + link.destination.pos.distance(player_pos);
            if route_distance > FILTH_SIGHT_RANGE
                || !line_clear_to_portal(level, enemy_pos, link.source)
                || !line_clear_from_portal(level, link.destination, player_pos)
            {
                return None;
            }

            let target = EnemyTarget {
                pos: link.destination.map_view_point_to(&link.source, player_pos),
                can_attack: true,
            };

            Some((enemy_pos.distance(target.pos), target))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, target)| target)
}

fn enemy_overlap_push(a: &Enemy, b: &Enemy) -> Option<Vec2> {
    let delta = b.pos - a.pos;
    let overlap_x = a.half_size().x + b.half_size().x - delta.x.abs();
    let overlap_y = a.half_size().y + b.half_size().y - delta.y.abs();

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let dir = if delta.x.abs() > 0.001 {
        delta.x.signum()
    } else {
        1.0
    };

    Some(Vec2::new(overlap_x * dir, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PORTAL_WIDTH;
    use crate::game::portal::Color;

    fn floor_level() -> Level {
        Level {
            solids: vec![Solid::new(0.0, 200.0, 420.0, 24.0, false)],
            ..Level::empty()
        }
    }

    fn only_target(level: &Level, enemy: Enemy, player_pos: Vec2) -> EnemyTarget {
        let targets = enemy_targets(level, &[enemy], player_pos, &[]);

        targets[0]
    }

    #[test]
    fn inactive_enemy_keeps_idle_target() {
        let level = Level::empty();
        let enemy = Enemy::filth_spawn(120.0, 80.0, 1, 1);

        let target = only_target(&level, enemy.clone(), Vec2::new(180.0, 80.0));

        assert_eq!(target.pos, enemy.pos);
        assert!(!target.can_attack);
    }

    #[test]
    fn wall_blocked_player_does_not_become_target() {
        let level = Level {
            solids: vec![
                Solid::new(0.0, 200.0, 420.0, 24.0, false),
                Solid::new(190.0, 80.0, 24.0, 120.0, false),
            ],
            ..Level::empty()
        };
        let enemy = Enemy::filth(150.0, 171.0);

        let target = only_target(&level, enemy.clone(), Vec2::new(260.0, 164.0));

        assert_eq!(target.pos, enemy.pos);
        assert!(!target.can_attack);
    }

    #[test]
    fn visible_player_on_same_surface_is_attack_target() {
        let level = floor_level();
        let enemy = Enemy::filth(150.0, 171.0);
        let player_pos = Vec2::new(260.0, 171.0);

        let target = only_target(&level, enemy, player_pos);

        assert_eq!(target.pos, player_pos);
        assert!(target.can_attack);
    }

    #[test]
    fn portal_route_returns_first_portal_waypoint() {
        let level = Level {
            solids: vec![
                Solid::new(0.0, 200.0, 220.0, 24.0, false),
                Solid::new(300.0, 100.0, 260.0, 24.0, false),
            ],
            ..Level::empty()
        };
        let enemy = Enemy::filth(150.0, 171.0);
        let enemy_pos = enemy.pos;
        let player_pos = Vec2::new(470.0, 64.0);
        let lower_portal = Portal::new(50.0, 171.0, Vec2::new(1.0, 0.0), PORTAL_WIDTH, Color::BLUE);
        let upper_portal = Portal::new(
            350.0,
            71.0,
            Vec2::new(1.0, 0.0),
            PORTAL_WIDTH,
            Color::ORANGE,
        );
        let links = [PortalLink {
            source: lower_portal,
            destination: upper_portal,
        }];

        let targets = enemy_targets(&level, &[enemy], player_pos, &links);
        let target = targets[0];

        assert!(target.pos.x < enemy_pos.x);
        assert!(!target.can_attack);
    }
}

use glam::Vec2;

use super::World;
use super::level::{Level, Solid};
use super::player::Player;
use super::portal::{Color, Portal};
use crate::constants::{DASH_SPEED, MAX_DASH_CHARGES, PLAYER_SIZE, PLAYER_SPEED};
use crate::platform::input::{GameKey, Input};

fn test_portal() -> Portal {
    Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE)
}

#[test]
fn raycast_hits_first_portalable_wall() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 100.0, true)],
    };

    let hit = level
        .raycast_portalable(Vec2::new(0.0, 50.0), Vec2::new(300.0, 50.0))
        .unwrap();

    assert_eq!(hit.point, Vec2::new(100.0, 50.0));
    assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
}

#[test]
fn jump_uses_buffered_press_on_ground() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_key(GameKey::Jump, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.y < 0.0);
    assert!(!player.on_ground);
}

#[test]
fn dash_follows_aim_direction() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::Dash, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.dashing);
    assert!(player.vel.x > DASH_SPEED * 0.9);
    assert!(player.vel.y.abs() < 1.0);
}

#[test]
fn dash_spends_one_charge() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::Dash, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.dash_charges < MAX_DASH_CHARGES);
    assert!(player.dash_charges > MAX_DASH_CHARGES - 1.1);
}

#[test]
fn slide_applies_ground_boost() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    player.vel.x = PLAYER_SPEED;
    input.set_aim_pos(Vec2::new(200.0, 100.0));
    input.set_key(GameKey::Slide, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.sliding);
    assert!(player.size.y < PLAYER_SIZE.1);
    assert!(player.vel.x > PLAYER_SPEED);
}

#[test]
fn slide_locks_direction_against_movement_input() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_aim_pos(Vec2::new(200.0, 100.0));
    input.set_key(GameKey::Slide, true);
    input.set_key(GameKey::Left, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x > 0.0);
}

#[test]
fn dash_charge_does_not_recover_while_sliding() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    player.dash_charges = 1.5;
    input.set_aim_pos(Vec2::new(200.0, 100.0));
    input.set_key(GameKey::Slide, true);
    input.update();

    player.update(1.0, &input, 900.0, 600.0);

    assert!((player.dash_charges - 1.5).abs() < f32::EPSILON);
}

#[test]
fn wall_jump_pushes_player_away_from_wall() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.set_wall_contact(1.0);
    input.set_key(GameKey::Jump, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x < 0.0);
    assert!(player.vel.y < 0.0);
}

#[test]
fn fourth_wall_jump_requires_landing() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    for _ in 0..3 {
        player.set_wall_contact(1.0);
        input.set_key(GameKey::Jump, true);
        input.update();
        player.update(1.0 / 60.0, &input, 900.0, 600.0);
        input.set_key(GameKey::Jump, false);
        input.update();
    }

    player.set_wall_contact(1.0);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.vel = Vec2::ZERO;
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.y >= 0.0);
}

#[test]
fn sweep_hits_when_player_enters_front_face() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert!(portal.intersects_sweep(Vec2::new(79.9, 50.0), Vec2::new(90.1, 50.0), half_size));
}

#[test]
fn sweep_hits_when_player_enters_back_face() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert!(portal.intersects_sweep(Vec2::new(120.1, 50.0), Vec2::new(109.9, 50.0), half_size));
}

#[test]
fn sweep_ignores_objects_outside_portal_width() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert!(!portal.intersects_sweep(Vec2::new(79.9, 120.0), Vec2::new(90.1, 120.0), half_size));
}

#[test]
fn teleport_preserves_velocity_in_destination_space() {
    let source = Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE);
    let destination = Portal::new(20.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
    let mut player = Player::new(90.0, 50.0);

    player.vel = Vec2::new(100.0, 25.0);
    source.teleport_player_to(&destination, &mut player);

    assert!(player.pos.x > destination.pos.x);
    assert_eq!(player.vel, Vec2::new(100.0, 25.0));
}

#[test]
fn blue_portal_shot_places_portal_on_wall() {
    let mut world = World::new();
    let mut input = Input::new();

    input.set_aim_pos(world.player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::BluePortal, true);
    input.update();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.portals[0].is_some());
}

#[test]
fn portal_does_not_place_on_too_small_surface() {
    let mut world = World::new();
    let mut input = Input::new();

    world.level.solids = vec![Solid::new(200.0, 80.0, 20.0, 40.0, true)];
    input.set_aim_pos(world.player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::BluePortal, true);
    input.update();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.portals[0].is_none());
}

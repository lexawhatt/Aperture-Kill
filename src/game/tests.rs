use glam::Vec2;

use super::World;
use super::level::{Checkpoint, Door, Hazard, Level, Solid, WorldPortal};
use super::player::{Player, PlayerEvent};
use super::portal::{Color, Portal};
use crate::constants::{
    AIR_SLIDE_WALL_KICK_Y, DASH_SPEED, DIVE_BOOST, GRAVITY, JUMP_VELOCITY, MAX_DASH_CHARGES,
    PLAYER_SIZE, PLAYER_SPEED, PORTAL_SURFACE_OFFSET, PORTAL_WIDTH, SLAM_NORMAL_HEIGHT_GAIN,
    SLIDE_BOOST, WALL_JUMP_Y, WALL_SLIDE_SPEED,
};
use crate::platform::input::{GameKey, Input};

fn test_portal() -> Portal {
    Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE)
}

#[test]
fn automatic_door_opens_near_player() {
    let mut door = Door::new(100.0, 100.0, 48.0, 112.0);
    let closed_y = door.solid.pos().y;

    door.update(door.solid.center(), 0.5, |_| {});

    assert!(door.open > 0.0);
    assert!(door.moving_solid().pos().y < closed_y);
}

#[test]
fn raycast_hits_first_portalable_wall() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 100.0, true)],
        ..Level::empty()
    };

    let hit = level
        .raycast_portalable(Vec2::new(0.0, 50.0), Vec2::new(300.0, 50.0))
        .unwrap();

    assert_eq!(hit.point, Vec2::new(100.0, 50.0));
    assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
}

#[test]
fn raycast_hits_rotated_portalable_wall() {
    let solid = Solid::rotated(100.0, 100.0, 120.0, 20.0, std::f32::consts::FRAC_PI_4, true);
    let level = Level {
        solids: vec![solid],
        ..Level::empty()
    };
    let origin = solid.center() - solid.axis_y() * 100.0;
    let target = solid.center();
    let hit = level.raycast_portalable(origin, target).unwrap();

    assert!(hit.normal.dot(-solid.axis_y()) > 0.99);
}

#[test]
fn portal_shot_places_on_rotated_portalable_block() {
    let solid = Solid::rotated(320.0, 240.0, 180.0, 28.0, std::f32::consts::FRAC_PI_6, true);
    let mut world = World::new();
    let mut input = Input::new();
    let origin = solid.center() - solid.axis_y() * 180.0;
    let target = solid.center();

    world.level = Level {
        solids: vec![solid],
        ..Level::empty()
    };
    world.player = Player::new(origin.x, origin.y + 20.0);
    input.set_aim_pos(target);
    input.set_key(GameKey::BluePortal, true);
    input.update();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    let portal = world.portals[0].unwrap();
    assert!(portal.normal().dot(-solid.axis_y()) > 0.99);
    assert!(portal.tangent().dot(solid.axis_x()).abs() > 0.99);
}

#[test]
fn portal_shot_places_on_rotated_block_inside_level() {
    let solid = Solid::rotated(370.0, 290.0, 160.0, 32.0, std::f32::consts::FRAC_PI_4, true);
    let mut world = World::new();
    let mut input = Input::new();
    let origin = solid.center() - solid.axis_y() * 180.0;
    let target = solid.center();

    world.level.solids = vec![
        Solid::new(0.0, 560.0, 900.0, 40.0, true),
        Solid::new(0.0, 0.0, 30.0, 600.0, true),
        Solid::new(870.0, 0.0, 30.0, 600.0, true),
        Solid::new(0.0, 0.0, 900.0, 30.0, true),
        solid,
    ];
    world.player = Player::new(origin.x, origin.y + 20.0);
    input.set_aim_pos(target);
    input.set_key(GameKey::BluePortal, true);
    input.update();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    let portal = world.portals[0].unwrap();
    assert!(portal.normal().dot(-solid.axis_y()) > 0.99);
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
fn jump_emits_sound_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::Jump)
    );
}

#[test]
fn dash_follows_aim_direction() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::Dash, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_dashing());
    assert!(player.vel.x > DASH_SPEED * 0.9);
    assert!(player.vel.y.abs() < 1.0);
}

#[test]
fn dash_emits_sound_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::Dash, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::DashStart)
    );
}

#[test]
fn dash_end_emits_sound_stop_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::Dash, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.drain_events().for_each(drop);
    input.update();
    player.update(1.0, &input, 900.0, 600.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::DashEnd)
    );
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
fn dash_without_charge_flashes_stamina() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.dash_charges = 0.0;
    input.set_key(GameKey::Dash, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.dash_deny_flash() > 0.0);
    assert!(!player.is_dashing());
}

#[test]
fn screen_floor_does_not_zero_fall_speed() {
    let mut player = Player::new(100.0, 550.0);
    let input = Input::new();

    player.vel.y = 1_000.0;
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.y > 1_000.0);
    assert!(player.pos.y > 564.0);
}

#[test]
fn high_speed_player_is_not_clamped_to_level_bounds() {
    let mut world = World::new();
    let input = Input::new();

    world.level.solids.clear();
    world.player.pos = Vec2::new(450.0, 300.0);
    world.player.prev_pos = world.player.pos;
    world.player.vel = Vec2::new(0.0, 50_000.0);
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.player.pos.y > 600.0);
    assert!(world.player.vel.y > 50_000.0);
}

#[test]
fn hazard_respawns_player_at_last_checkpoint() {
    let mut world = World::new();
    let input = Input::new();
    let checkpoint = Checkpoint::new(180.0, 200.0, 40.0, 80.0);
    let checkpoint_center = checkpoint.center();
    let hazard = Hazard::new(380.0, 200.0, 80.0, 80.0);

    world.level = Level {
        checkpoints: vec![checkpoint],
        hazards: vec![hazard],
        ..Level::empty()
    };
    world.player = Player::new(checkpoint_center.x, checkpoint_center.y);
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    world.player = Player::new(hazard.solid.center().x, hazard.solid.center().y);
    world.portals = [
        Some(Portal::new(
            120.0,
            220.0,
            Vec2::new(1.0, 0.0),
            PORTAL_WIDTH,
            Color::BLUE,
        )),
        None,
    ];
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.player.pos.distance(checkpoint_center) < 0.001);
    assert!(world.portals.iter().all(Option::is_none));
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

    assert!(player.is_sliding());
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
fn ctrl_in_air_starts_ground_slam() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_ground_slamming());
    assert!(player.vel.y > 100.0);
}

#[test]
fn ground_slam_emits_sound_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::GroundSlamStart)
    );
}

#[test]
fn ground_slam_end_emits_sound_stop_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.drain_events().for_each(drop);
    player.touch_ground_with_impact(500.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::GroundSlamEnd)
    );
}

#[test]
fn slide_end_emits_sound_stop_event() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.drain_events().for_each(drop);
    input.set_key(GameKey::Slide, false);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(
        player
            .drain_events()
            .any(|event| event == PlayerEvent::SlideEnd)
    );
}

#[test]
fn normal_slam_bounce_gives_small_boost() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.touch_ground_with_impact(500.0);

    input.set_key(GameKey::Slide, false);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.y < -JUMP_VELOCITY);
    assert!(player.vel.y > -JUMP_VELOCITY * 1.4);
}

#[test]
fn long_fall_slam_bounce_returns_impact_height() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.vel.y = 1_800.0;
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    let natural_speed = (1_800.0_f32.powi(2) + 2.0 * GRAVITY * (player.pos.y - 100.0)).sqrt();
    player.touch_ground_with_impact(2_600.0);

    input.set_key(GameKey::Slide, false);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    let expected_speed = natural_speed * SLAM_NORMAL_HEIGHT_GAIN.sqrt();
    let gravity_after_jump = GRAVITY / 60.0;
    assert!((player.vel.y + expected_speed - gravity_after_jump).abs() < 1.0);
}

#[test]
fn slam_bounce_uses_height_not_slam_acceleration() {
    let mut player = Player::new(100.0, 343.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.pos.y = 520.0;
    player.touch_ground_with_impact(2_600.0);

    input.set_key(GameKey::Slide, false);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    let fall_height = 520.0 - 343.0;
    let expected_speed = (2.0 * GRAVITY * fall_height * SLAM_NORMAL_HEIGHT_GAIN).sqrt();
    let gravity_after_jump = GRAVITY / 60.0;
    assert!((player.vel.y + expected_speed - gravity_after_jump).abs() < 1.0);
}

#[test]
fn stored_slam_dive_converts_energy_forward() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(Vec2::new(200.0, 100.0));
    player.vel.y = 1_800.0;
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.touch_ground_with_impact(2_600.0);

    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x > SLIDE_BOOST * 2.0);
    assert!(player.vel.y < 0.0);
}

#[test]
fn wall_jump_after_slam_stores_energy_without_fast_fall() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.set_wall_contact(1.0);
    input.set_key(GameKey::Slide, true);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(!player.is_ground_slamming());
    assert!(player.vel.y <= -WALL_JUMP_Y + 50.0);

    player.touch_ground_with_impact(100.0);
    input.set_key(GameKey::Slide, false);
    input.set_key(GameKey::Jump, false);
    input.update();
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.y < -JUMP_VELOCITY * 1.3);
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
fn wall_jump_keeps_direction_after_contact_frame() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.set_wall_contact(1.0);
    player.clear_contacts();
    input.set_key(GameKey::Jump, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x < 0.0);
    assert!(player.vel.y < 0.0);
}

#[test]
fn wall_slide_caps_fall_speed() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.vel.y = WALL_SLIDE_SPEED * 2.0;
    player.set_wall_contact(1.0);
    input.set_key(GameKey::Right, true);
    input.update();

    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_wall_sliding());
    assert_eq!(player.vel.y, WALL_SLIDE_SPEED);
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
fn sweep_waits_until_center_crosses_front_face() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert_eq!(
        portal.crossing_time(Vec2::new(79.9, 50.0), Vec2::new(90.1, 50.0), half_size),
        None
    );
}

#[test]
fn sweep_hits_when_center_crosses_front_face() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert_eq!(
        portal.crossing_time(Vec2::new(90.0, 50.0), Vec2::new(110.0, 50.0), half_size),
        Some(0.5)
    );
}

#[test]
fn sweep_hits_when_center_crosses_back_face() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert_eq!(
        portal.crossing_time(Vec2::new(110.0, 50.0), Vec2::new(90.0, 50.0), half_size),
        Some(0.5)
    );
}

#[test]
fn sweep_ignores_objects_outside_portal_width() {
    let portal = test_portal();
    let half_size = Vec2::new(10.0, 20.0);

    assert_eq!(
        portal.crossing_time(Vec2::new(90.0, 120.0), Vec2::new(110.0, 120.0), half_size),
        None
    );
}

#[test]
fn teleport_preserves_velocity_in_destination_space() {
    let source = Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE);
    let destination = Portal::new(20.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
    let mut player = Player::new(102.0, 50.0);

    player.vel = Vec2::new(100.0, 25.0);
    source.teleport_player_to(&destination, &mut player);

    assert!(player.pos.x > destination.pos.x);
    assert_eq!(player.vel, Vec2::new(100.0, 25.0));
}

#[test]
fn teleport_places_player_clear_of_destination_wall() {
    let source = Portal::new(100.0, 50.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE);
    let destination = Portal::new(20.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
    let mut player = Player::new(100.5, 50.0);

    source.teleport_player_to(&destination, &mut player);

    assert!(player.pos.x >= destination.pos.x + player.half_size().x);
}

#[test]
fn teleport_through_rotated_portals_preserves_speed() {
    let source = Portal::with_tangent(
        100.0,
        100.0,
        Vec2::new(0.0, -1.0),
        Vec2::X,
        96.0,
        Color::BLUE,
    );
    let angle = std::f32::consts::FRAC_PI_4;
    let normal = Vec2::new(angle.cos(), angle.sin());
    let tangent = Vec2::new(-angle.sin(), angle.cos());
    let destination = Portal::with_tangent(300.0, 200.0, normal, tangent, 96.0, Color::ORANGE);
    let mut player = Player::new(100.0, 104.0);

    player.prev_pos = Vec2::new(100.0, 90.0);
    player.vel = Vec2::new(120.0, 900.0);
    let speed = player.vel.length();
    source.teleport_player_to(&destination, &mut player);

    assert!((player.vel.length() - speed).abs() < 0.01);
    assert!(player.vel.dot(destination.normal()) > 0.0);
}

#[test]
fn portal_scale_uses_destination_over_source_ratio() {
    let mut source = Portal::new(100.0, 100.0, Vec2::new(-1.0, 0.0), 80.0, Color::BLUE);
    let mut destination = Portal::new(260.0, 100.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
    let mut player = Player::new(99.0, 100.0);

    source.scale = 1.0;
    destination.scale = 2.0;
    player.prev_pos = Vec2::new(101.0, 100.0);
    source.teleport_player_to(&destination, &mut player);

    assert_eq!(player.size, Vec2::new(PLAYER_SIZE.0, PLAYER_SIZE.1) * 2.0);

    let mut reverse_player = Player::new(261.0, 100.0);
    reverse_player.prev_pos = Vec2::new(259.0, 100.0);
    destination.teleport_player_to(&source, &mut reverse_player);

    assert_eq!(
        reverse_player.size,
        Vec2::new(PLAYER_SIZE.0, PLAYER_SIZE.1) * 0.5
    );
}

#[test]
fn world_portal_exit_from_rotated_block_preserves_momentum() {
    let ramp = Solid::rotated(520.0, 330.0, 180.0, 32.0, std::f32::consts::FRAC_PI_4, true);
    let ramp_surface = ramp.world_from_local(Vec2::new(ramp.size().x / 2.0, 0.0));
    let ramp_normal = -ramp.axis_y();
    let mut world = World::new();
    let input = Input::new();

    world.level.solids = vec![
        Solid::new(0.0, 560.0, 900.0, 40.0, true),
        Solid::new(0.0, 0.0, 30.0, 600.0, true),
        Solid::new(870.0, 0.0, 30.0, 600.0, true),
        Solid::new(0.0, 0.0, 900.0, 30.0, true),
        ramp,
    ];
    world.portals = [
        Some(Portal::with_tangent(
            350.0,
            560.0 - PORTAL_SURFACE_OFFSET,
            Vec2::new(0.0, -1.0),
            Vec2::X,
            PORTAL_WIDTH,
            Color::BLUE,
        )),
        Some(Portal::with_tangent(
            ramp_surface.x + ramp_normal.x * PORTAL_SURFACE_OFFSET,
            ramp_surface.y + ramp_normal.y * PORTAL_SURFACE_OFFSET,
            ramp_normal,
            ramp.axis_x(),
            PORTAL_WIDTH,
            Color::ORANGE,
        )),
    ];
    world.player = Player::new(350.0, 548.0);
    world.player.vel = Vec2::new(0.0, 1_100.0);
    let speed = world.player.vel.length();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    let exit = world.portals[1].unwrap();
    assert!(world.player.vel.length() >= speed * 0.98);
    assert!(world.player.vel.dot(exit.normal()) > 0.0);
}

#[test]
fn mapped_body_keeps_only_passed_slice_size() {
    let source = Portal::new(100.0, 560.0, Vec2::new(0.0, -1.0), 80.0, Color::BLUE);
    let destination = Portal::new(20.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE);
    let slice_center = Vec2::new(100.0, 561.0);
    let slice_size = Vec2::new(34.0, 2.0);
    let (_, mapped_size) = source.map_body_to(&destination, slice_center, slice_size);

    assert_eq!(mapped_size, Vec2::new(2.0, 34.0));
}

#[test]
fn active_portal_opens_wall_collision() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 120.0, true)],
        ..Level::empty()
    };
    let portal = Portal::new(
        100.0 - PORTAL_SURFACE_OFFSET,
        50.0,
        Vec2::new(-1.0, 0.0),
        80.0,
        Color::BLUE,
    );
    let mut player = Player::new(105.0, 50.0);

    level.resolve_player_with_portals(&mut player, &[portal]);

    assert_eq!(player.pos, Vec2::new(105.0, 50.0));
}

#[test]
fn world_portal_opens_wall_collision() {
    let mut world = World::new();
    let input = Input::new();

    world.level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 120.0, true)],
        world_portals: vec![
            WorldPortal {
                id: 1,
                receiver_id: 2,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    100.0 - PORTAL_SURFACE_OFFSET,
                    50.0,
                    Vec2::new(-1.0, 0.0),
                    80.0,
                    Color::BLUE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(260.0, 50.0, Vec2::new(1.0, 0.0), 80.0, Color::ORANGE),
            },
        ],
        ..Level::empty()
    };
    world.player = Player::new(105.0, 50.0);

    world.update(0.0, &input, 900.0, 600.0);

    assert_eq!(world.player.pos, Vec2::new(105.0, 50.0));
}

#[test]
fn portal_does_not_open_back_side_collision() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 120.0, true)],
        ..Level::empty()
    };
    let portal = Portal::new(
        100.0 - PORTAL_SURFACE_OFFSET,
        50.0,
        Vec2::new(-1.0, 0.0),
        80.0,
        Color::BLUE,
    );
    let mut player = Player::new(125.0, 50.0);
    let before = player.pos;

    level.resolve_player_with_portals(&mut player, &[portal]);

    assert!(player.pos.x > before.x);
}

#[test]
fn rotated_portal_opens_only_its_surface() {
    let solid = Solid::rotated(250.0, 250.0, 180.0, 36.0, std::f32::consts::FRAC_PI_6, true);
    let surface = solid.world_from_local(Vec2::new(solid.size().x / 2.0, 0.0));
    let normal = -solid.axis_y();
    let portal = Portal::new(
        surface.x + normal.x * PORTAL_SURFACE_OFFSET,
        surface.y + normal.y * PORTAL_SURFACE_OFFSET,
        normal,
        96.0,
        Color::BLUE,
    );
    let mut player = Player::new(surface.x + normal.x * 12.0, surface.y + normal.y * 12.0);
    let before = player.pos;
    let level = Level {
        solids: vec![solid],
        ..Level::empty()
    };

    level.resolve_player_with_portals(&mut player, &[portal]);

    assert_eq!(player.pos, before);
}

#[test]
fn portal_does_not_open_rotated_solid_edges() {
    let solid = Solid::rotated(250.0, 250.0, 180.0, 36.0, std::f32::consts::FRAC_PI_6, true);
    let surface = solid.world_from_local(Vec2::new(solid.size().x / 2.0, 0.0));
    let normal = -solid.axis_y();
    let portal = Portal::new(
        surface.x + normal.x * PORTAL_SURFACE_OFFSET,
        surface.y + normal.y * PORTAL_SURFACE_OFFSET,
        normal,
        220.0,
        Color::BLUE,
    );
    let side = solid.world_from_local(Vec2::new(solid.size().x, solid.size().y / 2.0));
    let mut player = Player::new(
        side.x + solid.axis_x().x * 8.0,
        side.y + solid.axis_x().y * 8.0,
    );
    let before = player.pos;
    let level = Level {
        solids: vec![solid],
        ..Level::empty()
    };

    level.resolve_player_with_portals(&mut player, &[portal]);

    assert!(player.pos.distance(before) > 0.1);
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

#[test]
fn portal_shot_near_surface_edge_slides_inward() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 200.0, 20.0, true)],
        ..Level::empty()
    };

    let hit = level
        .raycast_portalable(Vec2::new(105.0, 100.0), Vec2::new(105.0, 0.0))
        .unwrap();
    let center = hit.portal_center(PORTAL_WIDTH).unwrap();

    assert_eq!(center.x, 100.0 + PORTAL_WIDTH / 2.0);
}

#[test]
fn rotated_portal_placement_slides_in_surface_space() {
    let solid = Solid::rotated(220.0, 160.0, 180.0, 28.0, std::f32::consts::FRAC_PI_6, true);
    let level = Level {
        solids: vec![solid],
        ..Level::empty()
    };
    let shot = solid.world_from_local(Vec2::new(8.0, -120.0));
    let target = solid.world_from_local(Vec2::new(8.0, 16.0));
    let hit = level.raycast_portalable(shot, target).unwrap();
    let center = level.portal_center(hit, PORTAL_WIDTH).unwrap();
    let surface = center - hit.normal * PORTAL_SURFACE_OFFSET;
    let local = solid.local_from_world(surface);

    assert!((local.x - PORTAL_WIDTH / 2.0).abs() < 0.01);
    assert!(local.y.abs() < 0.01);
}

#[test]
fn placed_portal_sits_outside_surface() {
    let level = Level {
        solids: vec![Solid::new(100.0, 0.0, 20.0, 100.0, true)],
        ..Level::empty()
    };

    let hit = level
        .raycast_portalable(Vec2::new(0.0, 50.0), Vec2::new(300.0, 50.0))
        .unwrap();
    let center = level.portal_center(hit, PORTAL_WIDTH).unwrap();

    assert_eq!(center.x, 100.0 - PORTAL_SURFACE_OFFSET);
}

#[test]
fn portal_placement_avoids_adjacent_walls() {
    let level = Level {
        solids: vec![
            Solid::new(0.0, 560.0, 900.0, 40.0, true),
            Solid::new(0.0, 0.0, 30.0, 600.0, true),
        ],
        ..Level::empty()
    };

    let hit = level
        .raycast_portalable(Vec2::new(50.0, 500.0), Vec2::new(50.0, 590.0))
        .unwrap();
    let center = level.portal_center(hit, PORTAL_WIDTH).unwrap();

    assert!(center.x - PORTAL_WIDTH / 2.0 >= 31.0);
}

#[test]
fn portal_does_not_place_too_close_to_other_portal() {
    let mut world = World::new();
    let mut input = Input::new();

    input.set_aim_pos(world.player.aim_from() + Vec2::X * 100.0);
    input.set_key(GameKey::BluePortal, true);
    input.update();
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    input.set_key(GameKey::BluePortal, false);
    input.set_key(GameKey::OrangePortal, true);
    input.update();
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.portals[0].is_some());
    assert!(world.portals[1].is_none());
}

#[test]
fn slide_pressed_in_air_starts_ground_slam() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.vel.y = -500.0;
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_ground_slamming());
    assert!(!player.is_air_sliding());
}

#[test]
fn portal_slide_starts_air_slide_on_exit() {
    let mut player = Player::new(100.0, 100.0);
    let input = super::player::MovementInput {
        move_x: 0.0,
        aim_pos: Vec2::ZERO,
        dash_pressed: false,
        jump_pressed: false,
        slide_down: true,
        slide_pressed: false,
    };

    player.on_player_portal_exit(Vec2::X, input);

    assert!(player.is_air_sliding());
    assert!(player.size.y < PLAYER_SIZE.1);
}

#[test]
fn dash_slide_preserves_dash_speed_into_slide() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_aim_pos(Vec2::new(300.0, 100.0));
    input.set_key(GameKey::Dash, true);
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_sliding());
    assert!(player.vel.x > DASH_SPEED * 0.95);
}

#[test]
fn dash_storage_jump_preserves_dash_speed_out_of_slide() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_aim_pos(Vec2::new(300.0, 100.0));
    input.set_key(GameKey::Dash, true);
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    input.set_key(GameKey::Dash, false);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(!player.on_ground);
    assert!(player.vel.x > DASH_SPEED * 0.95);
    assert!(player.vel.y < 0.0);
}

#[test]
fn dash_canceling_ground_slam_emits_slam_end() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    player.drain_events().for_each(drop);

    input.set_key(GameKey::Slide, false);
    input.set_key(GameKey::Dash, true);
    input.set_aim_pos(player.aim_from() + Vec2::X * 100.0);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    let events: Vec<_> = player.drain_events().collect();
    assert!(events.contains(&PlayerEvent::GroundSlamEnd));
    assert!(events.contains(&PlayerEvent::DashStart));
    assert!(!player.is_ground_slamming());
}

#[test]
fn slide_after_recent_jump_starts_dive_instead_of_slam() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_aim_pos(Vec2::new(300.0, 100.0));
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    input.set_key(GameKey::Jump, false);
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.is_air_sliding());
    assert!(!player.is_ground_slamming());
    assert!(player.vel.x > DIVE_BOOST * 0.95);
}

#[test]
fn ordinary_dive_falls_under_normal_gravity() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    player.land();
    input.set_aim_pos(Vec2::new(300.0, 100.0));
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    input.set_key(GameKey::Jump, false);
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    input.update();
    player.update(1.0, &input, 900.0, 600.0);

    assert!(player.is_air_sliding());
    assert!(player.vel.y > 700.0);
}

#[test]
fn short_slide_hops_build_speed() {
    let mut player = Player::new(100.0, 100.0);
    let mut input = Input::new();

    input.set_aim_pos(Vec2::new(300.0, 100.0));
    player.land();

    let mut previous_speed = 0.0;
    for _ in 0..3 {
        input.set_key(GameKey::Jump, false);
        input.set_key(GameKey::Slide, true);
        input.update();
        player.update(1.0 / 120.0, &input, 900.0, 600.0);
        let slide_speed = player.vel.x.abs();

        input.set_key(GameKey::Jump, true);
        input.update();
        player.update(1.0 / 120.0, &input, 900.0, 600.0);

        assert!(slide_speed > previous_speed);
        previous_speed = slide_speed;
        player.touch_ground_with_impact(200.0);
    }

    assert!(previous_speed > SLIDE_BOOST + 200.0);
}

#[test]
fn releasing_slide_under_low_ceiling_keeps_crouch_state() {
    let mut player = Player::new(100.0, 164.0);
    let mut input = Input::new();
    let blocked = Level {
        solids: vec![
            Solid::new(0.0, 200.0, 300.0, 40.0, true),
            Solid::new(0.0, 120.0, 300.0, 44.0, true),
        ],
        ..Level::empty()
    };
    let open = Level {
        solids: vec![Solid::new(0.0, 200.0, 300.0, 40.0, true)],
        ..Level::empty()
    };

    player.land();
    input.set_key(GameKey::Slide, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    input.set_key(GameKey::Slide, false);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);
    blocked.resolve_player(&mut player);

    assert!(player.is_crouching());
    assert!(player.size.y < PLAYER_SIZE.1);

    open.resolve_player(&mut player);

    assert!(!player.is_crouching());
    assert_eq!(player.size.y, PLAYER_SIZE.1);
}

#[test]
fn grounded_player_climbsteps_small_ledge() {
    let level = Level {
        solids: vec![
            Solid::new(0.0, 200.0, 300.0, 40.0, true),
            Solid::new(130.0, 176.0, 40.0, 24.0, true),
        ],
        ..Level::empty()
    };
    let mut player = Player::new(116.0, 164.0);
    let before_y = player.pos.y;

    player.land();
    player.vel.x = 300.0;
    level.resolve_player(&mut player);

    assert!(player.pos.y < before_y - 20.0);
    assert!(player.on_ground);
}

#[test]
fn air_slide_wall_kick_preserves_slide_momentum() {
    let level = Level {
        solids: vec![Solid::new(130.0, 40.0, 40.0, 140.0, true)],
        ..Level::empty()
    };
    let mut player = Player::new(120.0, 100.0);
    let mut input = Input::new();
    let portal_input = super::player::MovementInput {
        move_x: 0.0,
        aim_pos: Vec2::new(300.0, 100.0),
        dash_pressed: false,
        jump_pressed: false,
        slide_down: true,
        slide_pressed: false,
    };

    player.vel.x = 1_050.0;
    player.on_player_portal_exit(Vec2::X, portal_input);
    level.resolve_player(&mut player);
    input.set_key(GameKey::Slide, true);
    input.update();
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x < -1_000.0);
    assert!(player.vel.y < -AIR_SLIDE_WALL_KICK_Y + 80.0);
}

#[test]
fn high_speed_wall_hit_enables_vertical_wall_jump() {
    let level = Level {
        solids: vec![Solid::new(130.0, 40.0, 40.0, 140.0, true)],
        ..Level::empty()
    };
    let mut player = Player::new(120.0, 100.0);
    let mut input = Input::new();

    player.vel.x = 1_100.0;
    level.resolve_player(&mut player);
    input.set_key(GameKey::Jump, true);
    input.update();
    player.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(player.vel.x.abs() < 1.0);
    assert!(player.vel.y < -800.0);
}

#[test]
fn portal_kick_adds_exit_normal_impulse() {
    let mut player = Player::new(100.0, 100.0);
    let input = super::player::MovementInput {
        move_x: 0.0,
        aim_pos: Vec2::ZERO,
        dash_pressed: false,
        jump_pressed: true,
        slide_down: false,
        slide_pressed: false,
    };

    player.on_player_portal_exit(Vec2::X, input);

    assert!(player.vel.x > 300.0);
}

#[test]
fn world_portal_exit_supports_slide_air_slide() {
    let mut world = World::new();
    let mut input = Input::new();

    world.level = Level {
        world_portals: vec![
            WorldPortal {
                id: 1,
                receiver_id: 2,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    100.0,
                    100.0,
                    Vec2::new(-1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::BLUE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    260.0,
                    100.0,
                    Vec2::new(1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::ORANGE,
                ),
            },
        ],
        ..Level::empty()
    };
    world.player = Player::new(130.0, 100.0);
    world.player.land();
    world.player.vel = Vec2::new(-2_400.0, 0.0);
    input.set_key(GameKey::Slide, true);
    input.update();

    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.player.is_air_sliding());
}

#[test]
fn world_portal_uses_highest_priority_receiver() {
    let mut world = World::new();
    let input = Input::new();

    world.level = Level {
        world_portals: vec![
            WorldPortal {
                id: 1,
                receiver_id: 2,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    100.0,
                    100.0,
                    Vec2::new(-1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::BLUE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    260.0,
                    100.0,
                    Vec2::new(1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::ORANGE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 10,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    420.0,
                    100.0,
                    Vec2::new(1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::ORANGE,
                ),
            },
        ],
        ..Level::empty()
    };
    world.player = Player::new(130.0, 100.0);
    world.player.vel = Vec2::new(-2400.0, 0.0);
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.player.pos.x > 390.0);
}

#[test]
fn seamless_world_portal_requires_single_receiver() {
    let mut world = World::new();
    let input = Input::new();

    world.level = Level {
        world_portals: vec![
            WorldPortal {
                id: 1,
                receiver_id: 2,
                priority: 0,
                seamless: true,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    100.0,
                    100.0,
                    Vec2::new(-1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::BLUE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 0,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    260.0,
                    100.0,
                    Vec2::new(1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::ORANGE,
                ),
            },
            WorldPortal {
                id: 2,
                receiver_id: 1,
                priority: 10,
                seamless: false,
                seamless_depth: 256.0,
                seamless_angle: 180.0,
                seamless_rely_on_walls: false,
                portal: Portal::new(
                    420.0,
                    100.0,
                    Vec2::new(1.0, 0.0),
                    PORTAL_WIDTH,
                    Color::ORANGE,
                ),
            },
        ],
        ..Level::empty()
    };
    world.player = Player::new(130.0, 100.0);
    world.player.vel = Vec2::new(-2400.0, 0.0);
    world.update(1.0 / 60.0, &input, 900.0, 600.0);

    assert!(world.player.pos.x < 130.0);
}

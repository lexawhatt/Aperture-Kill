mod player;
mod portal;
mod input;
mod constants;
use macroquad::prelude::*;
use constants::{ACCEL_SPEED, CONST_SPEED};
use player::Ball;
use portal::Portal;



fn check_char(target: char) -> bool {
    let mut pressed = false;
    while let Some(c) = get_char_pressed() {
        if c.to_lowercase().to_string() == target.to_lowercase().to_string() {
            pressed = true;
        }
    }
    pressed
}

#[macroquad::main("Ball Example")]
async fn main() {
    let mut ball1 = Ball::new(100.0, 100.0, 15.0);
    let mut ball2 = Ball::new(200.0, 100.0, 20.0);

    let mut p1 = Portal::new(1, 750.0, 300.0, Vec2::new(-1.0, 0.0), 100.0, BLUE);
    let mut p2 = Portal::new(2, 50.0, 300.0, Vec2::new(1.0, 0.0), 100.0, ORANGE);

    p1.scale = 4.0;
    p2.scale = 2.0;


    let mut const_mode = false;

    loop {
        let dt = get_frame_time();
        let mut dir = String::new();

        if let Some(c) = get_char_pressed() {
            if c == 'v' || c == 'V' || c == 'м' || c == 'М' {
                const_mode = !const_mode;
            }
        }

        let keys_ball1 = (KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right);
        let keys_ball2 = (KeyCode::Kp5, KeyCode::Kp2, KeyCode::Kp1, KeyCode::Kp3);

        if const_mode {
            ball1.velocity = Vec2::ZERO;
            ball2.velocity = Vec2::ZERO;
        }

        ball1.handle_input(dt, const_mode, ACCEL_SPEED, CONST_SPEED, keys_ball1, &mut dir);
        ball2.handle_input(dt, const_mode, ACCEL_SPEED, CONST_SPEED, keys_ball2, &mut dir);

        ball1.resolve_collision(&mut ball2);

        let _ = p2.check_coll(&p1, &mut ball1);
        let _ = p1.check_coll(&p2, &mut ball1);

        let _ = p2.check_coll(&p1, &mut ball2);
        let _ = p1.check_coll(&p2, &mut ball2);

        ball1.constrain_to_screen(screen_width(), screen_height());
        ball2.constrain_to_screen(screen_width(), screen_height());

        clear_background(BLACK);

        draw_text(&dir, 10.0, 40.0, 20.0, WHITE);

        draw_circle(ball1.position.x, ball1.position.y, ball1.radius, WHITE);
        draw_circle(ball2.position.x, ball2.position.y, ball2.radius, LIGHTGRAY);

        p1.draw();
        p2.draw();

        let mode = if const_mode { "CONSTANT" } else { "INERTIAL" };
        draw_text(
            &format!("B1 Vel: {:.1} {:.1} | Mode: {}", ball1.velocity.x, ball1.velocity.y, mode),
            10.0,
            20.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}
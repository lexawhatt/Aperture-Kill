use glam::Vec2;

use crate::game::level::Solid;
use crate::game::portal::Color;

pub(super) fn yes_no(value: bool) -> &'static str {
    if value { "YES" } else { "NO" }
}

pub(super) fn grid_mode_text(grid_snap: bool) -> &'static str {
    if grid_snap {
        "H GRID MODE"
    } else {
        "H FREE MODE"
    }
}

pub(super) fn editor_tool_label(index: usize) -> &'static str {
    match index {
        1 => "SOLID",
        2 => "PORTAL",
        3 => "DOOR",
        4 => "TEXT",
        5 => "ACID",
        6 => "CHECK",
        7 => "W PORT",
        _ => "",
    }
}

pub(super) fn solids_bounds(solids: &[Solid]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let mut any = false;

    for solid in solids {
        for corner in solid.corners() {
            min = min.min(corner);
            max = max.max(corner);
            any = true;
        }
    }

    any.then_some((min, max))
}

pub(super) fn text_size(text: &str) -> Vec2 {
    Vec2::new((text.chars().count().max(1) as f32 * 12.0).max(24.0), 14.0)
}

pub(super) fn text_pixel_width(text: &str, scale: i32) -> f32 {
    text.chars()
        .map(|ch| if ch == ' ' { 4 } else { 6 })
        .sum::<i32>() as f32
        * scale as f32
}

pub(super) fn menu_left(width: f32) -> f32 {
    (width * 0.068).clamp(42.0, 132.0)
}

pub(super) fn menu_button_gap(height: f32) -> f32 {
    (height * 0.017).clamp(14.0, 22.0)
}

pub(super) fn social_y(height: f32) -> f32 {
    height - 46.0
}

pub(super) fn options_side_width(width: f32) -> f32 {
    (width * 0.125).clamp(190.0, 320.0)
}

pub(super) fn options_back_button_rect(width: f32, height: f32) -> (Vec2, Vec2) {
    let side_w = options_side_width(width);
    let side_left = options_left(width);
    let button_h = (height * 0.056).clamp(48.0, 62.0);

    (
        Vec2::new(side_left, height - 82.0),
        Vec2::new(side_w, button_h),
    )
}

pub(super) fn options_left(width: f32) -> f32 {
    (width * 0.07).clamp(24.0, 168.0)
}

pub(super) fn options_sidebar_top(height: f32) -> f32 {
    (height * 0.13).clamp(64.0, 150.0)
}

pub(super) fn options_button_height(height: f32) -> f32 {
    (height * 0.056).clamp(42.0, 62.0)
}

pub(super) fn options_button_gap(height: f32) -> f32 {
    (height * 0.012).clamp(6.0, 14.0)
}

pub(super) fn options_show_scrollbar(width: f32, height: f32) -> bool {
    width >= 1180.0 && height >= 720.0
}

pub(super) fn red_failure_tint(raw: u32, intensity: f32, x: i32, y: i32, seed: i32) -> u32 {
    let noise = ((x * 7 + y * 11 + seed).rem_euclid(13) - 6) as f32;
    let red = ((raw >> 16) & 0xff) as f32;
    let green = ((raw >> 8) & 0xff) as f32;
    let blue = (raw & 0xff) as f32;
    let failure = intensity.powf(1.25) * 0.58;
    let scanline = if (y + seed).rem_euclid(11) == 0 {
        0.82
    } else {
        1.0
    };

    let r = (red + 88.0 * failure + noise * 0.45).clamp(0.0, 255.0) as u32;
    let g = (green * (1.0 - failure * 0.38) * scanline).clamp(0.0, 255.0) as u32;
    let b = (blue * (1.0 - failure * 0.52) * scanline).clamp(0.0, 255.0) as u32;

    (r << 16) | (g << 8) | b
}

pub(super) fn red_damage_pulse(raw: u32, amount: f32) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let red = ((raw >> 16) & 0xff) as f32;
    let green = ((raw >> 8) & 0xff) as f32;
    let blue = (raw & 0xff) as f32;

    let r = (red + (255.0 - red) * amount * 0.72).clamp(0.0, 255.0) as u32;
    let g = (green * (1.0 - amount * 0.62)).clamp(0.0, 255.0) as u32;
    let b = (blue * (1.0 - amount * 0.72)).clamp(0.0, 255.0) as u32;

    (r << 16) | (g << 8) | b
}

pub(super) fn mix_color(a: Color, b: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);

    Color::rgb(
        mix_channel(a.r, b.r, amount),
        mix_channel(a.g, b.g, amount),
        mix_channel(a.b, b.b, amount),
    )
}

fn mix_channel(a: u8, b: u8, amount: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * amount).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_width_accounts_for_spaces_and_scale() {
        assert_eq!(text_pixel_width("AB", 2), 24.0);
        assert_eq!(text_pixel_width("A B", 3), 48.0);
    }

    #[test]
    fn mix_color_clamps_amount() {
        let a = Color::rgb(10, 20, 30);
        let b = Color::rgb(110, 120, 130);

        assert_eq!(mix_color(a, b, -1.0), a);
        assert_eq!(mix_color(a, b, 2.0), b);
        assert_eq!(mix_color(a, b, 0.5), Color::rgb(60, 70, 80));
    }
}

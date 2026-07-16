use glam::Vec2;

use crate::settings::{GAME_ACTIONS, OptionsClick, OptionsTab, Settings, VolumeKind};

pub(super) const SOCIAL_LINKS: [(&str, &str, &str); 3] = [
    ("X", "@LEXAWHATT", "https://x.com/LexaWhatt"),
    ("YOUTUBE", "@LEXAWHAT", "https://www.youtube.com/@LexaWhat"),
    ("GITHUB", "LEXAWHATT", "https://github.com/lexawhatt"),
];

pub(super) fn menu_hit(pos: Vec2, width: f32, height: f32) -> Option<usize> {
    let buttons = menu_buttons(width, height);

    buttons.iter().position(|(button_pos, button_size)| {
        pos.x >= button_pos.x
            && pos.x <= button_pos.x + button_size.x
            && pos.y >= button_pos.y
            && pos.y <= button_pos.y + button_size.y
    })
}

pub(super) fn social_hit(pos: Vec2, width: f32, height: f32) -> Option<&'static str> {
    let y = social_y(height);
    let mut x = menu_left(width);

    for (network, handle, url) in SOCIAL_LINKS {
        let link_width = text_width(network, 2) + 12.0 + text_width(handle, 1);
        let link_height = 20.0;

        if pos.x >= x && pos.x <= x + link_width && pos.y >= y && pos.y <= y + link_height {
            return Some(url);
        }

        x += link_width + 30.0;
    }

    None
}

pub(super) fn options_back_hit(pos: Vec2, width: f32, height: f32) -> bool {
    let button = options_back_button(width, height);
    rect_hit(pos, button.0, button.1)
}

pub(super) fn options_hit(
    pos: Vec2,
    width: f32,
    height: f32,
    active_tab: OptionsTab,
    settings: &Settings,
    resolution_dropdown: bool,
) -> OptionsClick {
    if options_back_hit(pos, width, height) {
        return OptionsClick::Back;
    }

    if let Some(tab) = options_tab_hit(pos, width, height) {
        return if tab.enabled() {
            OptionsClick::Tab(tab)
        } else {
            OptionsClick::None
        };
    }

    let layout = options_content_layout(width, height);
    match active_tab {
        OptionsTab::General => {
            let row = Vec2::new(layout.left, layout.y(96.0));
            if rect_hit(pos, row, Vec2::new(layout.content_w, 42.0)) {
                OptionsClick::ToggleFps
            } else {
                OptionsClick::None
            }
        }
        OptionsTab::Controls => controls_hit(pos, &layout),
        OptionsTab::Graphics => graphics_hit(pos, &layout, settings, resolution_dropdown),
        OptionsTab::Audio => audio_hit(pos, &layout),
        OptionsTab::Assist | OptionsTab::Saves | OptionsTab::Hud | OptionsTab::Colors => {
            OptionsClick::None
        }
    }
}

pub(super) fn options_drag_volume(
    pos: Vec2,
    width: f32,
    height: f32,
    kind: VolumeKind,
) -> Option<u8> {
    let layout = options_content_layout(width, height);
    let index = match kind {
        VolumeKind::Master => 0,
        VolumeKind::Sfx => 1,
        VolumeKind::Music => 2,
    };
    let slider_y = layout.y(96.0 + index as f32 * 84.0) - 10.0;
    if pos.y < slider_y - 18.0 || pos.y > slider_y + 64.0 {
        return None;
    }

    let control_w = audio_control_w(layout.control_w);
    let amount = ((pos.x - layout.control_x) / control_w).clamp(0.0, 1.0);
    Some((amount * 100.0).round() as u8)
}

fn menu_buttons(width: f32, height: f32) -> [(Vec2, Vec2); 4] {
    let button_size = Vec2::new(
        (width * 0.34).clamp(320.0, 660.0),
        (height * 0.058).clamp(54.0, 70.0),
    );
    let gap = menu_button_gap(height);
    let total_height = button_size.y * 4.0 + gap * 3.0;
    let target_y = height * 0.42;
    let max_y = height - 100.0 - total_height;
    let min_y = height * 0.34;
    let start = Vec2::new(menu_left(width), target_y.min(max_y).max(min_y));

    [
        (start, button_size),
        (start + Vec2::new(0.0, button_size.y + gap), button_size),
        (
            start + Vec2::new(0.0, (button_size.y + gap) * 2.0),
            button_size,
        ),
        (
            start + Vec2::new(0.0, (button_size.y + gap) * 3.0),
            button_size,
        ),
    ]
}

fn options_back_button(width: f32, height: f32) -> (Vec2, Vec2) {
    let side_w = options_side_width(width);
    let side_left = options_left(width);
    let button_h = (height * 0.056).clamp(48.0, 62.0);
    (
        Vec2::new(side_left, height - 82.0),
        Vec2::new(side_w, button_h),
    )
}

fn options_tab_hit(pos: Vec2, width: f32, height: f32) -> Option<OptionsTab> {
    let left = options_left(width);
    let side_w = options_side_width(width);
    let top = options_sidebar_top(height);
    let button_h = options_button_height(height);
    let gap = options_button_gap(height);

    let general = [
        OptionsTab::General,
        OptionsTab::Controls,
        OptionsTab::Graphics,
        OptionsTab::Audio,
        OptionsTab::Assist,
        OptionsTab::Saves,
    ];
    for (index, tab) in general.iter().enumerate() {
        let button_pos = Vec2::new(left, top + index as f32 * (button_h + gap));
        if rect_hit(pos, button_pos, Vec2::new(side_w, button_h)) {
            return Some(*tab);
        }
    }

    let back = options_back_button(width, height);
    let custom_y = top + general.len() as f32 * (button_h + gap) + gap * 5.0;
    let custom_bottom = custom_y + 44.0 + 2.0 * button_h + gap;
    if custom_bottom >= back.0.y - 14.0 {
        return None;
    }
    for (index, tab) in [OptionsTab::Hud, OptionsTab::Colors].iter().enumerate() {
        let button_pos = Vec2::new(left, custom_y + 44.0 + index as f32 * (button_h + gap));
        if rect_hit(pos, button_pos, Vec2::new(side_w, button_h)) {
            return Some(*tab);
        }
    }

    None
}

fn controls_hit(pos: Vec2, layout: &OptionsLayout) -> OptionsClick {
    for (index, key) in GAME_ACTIONS.iter().enumerate() {
        let row_y = layout.y(96.0 + index as f32 * 68.0);
        if rect_hit(
            pos,
            Vec2::new(layout.control_x, row_y - 10.0),
            Vec2::new(layout.control_w, 46.0),
        ) {
            return OptionsClick::Bind(*key);
        }
    }

    OptionsClick::None
}

fn graphics_hit(
    pos: Vec2,
    layout: &OptionsLayout,
    settings: &Settings,
    resolution_dropdown: bool,
) -> OptionsClick {
    if resolution_dropdown {
        let dropdown_y = layout.y(154.0) + 44.0;
        for (index, _) in settings.resolutions.iter().enumerate() {
            let choice_pos = Vec2::new(layout.control_x, dropdown_y + index as f32 * 30.0);
            if rect_hit(pos, choice_pos, Vec2::new(layout.control_w, 30.0)) {
                return OptionsClick::ResolutionChoice(index);
            }
        }
    }

    if rect_hit(
        pos,
        Vec2::new(layout.control_x, layout.y(96.0) - 10.0),
        Vec2::new(layout.control_w, 46.0),
    ) {
        return OptionsClick::DisplayMode;
    }
    if rect_hit(
        pos,
        Vec2::new(layout.control_x, layout.y(166.0) - 10.0),
        Vec2::new(layout.control_w, 46.0),
    ) {
        return OptionsClick::ToggleResolutionDropdown;
    }

    OptionsClick::None
}

fn audio_hit(pos: Vec2, layout: &OptionsLayout) -> OptionsClick {
    for (index, kind) in [VolumeKind::Master, VolumeKind::Sfx, VolumeKind::Music]
        .iter()
        .enumerate()
    {
        let slider_pos = Vec2::new(
            layout.control_x,
            layout.y(96.0 + index as f32 * 84.0) - 10.0,
        );
        let slider_size = Vec2::new(audio_control_w(layout.control_w), 46.0);
        if rect_hit(pos, slider_pos, slider_size) {
            let amount = ((pos.x - slider_pos.x) / slider_size.x).clamp(0.0, 1.0);
            return OptionsClick::Volume(*kind, (amount * 100.0).round() as u8);
        }
    }

    OptionsClick::None
}

fn options_content_layout(width: f32, height: f32) -> OptionsLayout {
    let left = options_left(width);
    let side_w = options_side_width(width);
    let content_left = (width * 0.29).max(left + side_w + (width * 0.075).clamp(42.0, 92.0));
    let content_right = if options_show_scrollbar(width, height) {
        (width * 0.745).min(width - options_left(width) - 96.0)
    } else {
        width - options_left(width)
    };
    let content_w = (content_right - content_left).clamp(260.0, 920.0);
    let content_top = (height * 0.085).clamp(58.0, 92.0);
    let vertical = ((height - content_top - 80.0) / 920.0).clamp(0.68, 1.0);
    let control_x = content_left + content_w * 0.48;
    let max_control_w = (content_w * 0.54).max(160.0);
    let min_control_w = 160.0_f32.min(max_control_w);
    let control_w = (content_right - control_x).clamp(min_control_w, max_control_w);

    OptionsLayout {
        left: content_left,
        top: content_top,
        content_w,
        control_x,
        control_w,
        vertical,
    }
}

struct OptionsLayout {
    left: f32,
    top: f32,
    content_w: f32,
    control_x: f32,
    control_w: f32,
    vertical: f32,
}

impl OptionsLayout {
    fn y(&self, offset: f32) -> f32 {
        self.top + offset * self.vertical
    }
}

fn text_width(text: &str, scale: i32) -> f32 {
    text.chars()
        .map(|ch| if ch == ' ' { 4 } else { 6 })
        .sum::<i32>() as f32
        * scale as f32
}

fn menu_left(width: f32) -> f32 {
    (width * 0.068).clamp(42.0, 132.0)
}

fn menu_button_gap(height: f32) -> f32 {
    (height * 0.017).clamp(14.0, 22.0)
}

fn social_y(height: f32) -> f32 {
    height - 46.0
}

fn options_left(width: f32) -> f32 {
    (width * 0.07).clamp(24.0, 168.0)
}

fn options_side_width(width: f32) -> f32 {
    (width * 0.125).clamp(190.0, 320.0)
}

fn options_sidebar_top(height: f32) -> f32 {
    (height * 0.13).clamp(64.0, 150.0)
}

fn options_button_height(height: f32) -> f32 {
    (height * 0.056).clamp(42.0, 62.0)
}

fn options_button_gap(height: f32) -> f32 {
    (height * 0.012).clamp(6.0, 14.0)
}

fn options_show_scrollbar(width: f32, height: f32) -> bool {
    width >= 1180.0 && height >= 720.0
}

fn rect_hit(pos: Vec2, rect_pos: Vec2, rect_size: Vec2) -> bool {
    pos.x >= rect_pos.x
        && pos.x <= rect_pos.x + rect_size.x
        && pos.y >= rect_pos.y
        && pos.y <= rect_pos.y + rect_size.y
}

fn audio_control_w(control_w: f32) -> f32 {
    (control_w - 76.0).max(120.0)
}

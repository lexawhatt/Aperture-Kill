use glam::Vec2;

use crate::game::Difficulty;
use crate::game::progression::{
    CHAPTERS, chapter_level, chapter_level_count, custom_level_indices, find_level_by_code,
    is_custom_chapter, level_code,
};
use crate::settings::{GAME_ACTIONS, OptionsClick, OptionsTab, Settings, VolumeKind};

use super::App;

pub(super) const SOCIAL_LINKS: [(&str, &str, &str); 3] = [
    ("X", "@LEXAWHATT", "https://x.com/LexaWhatt"),
    ("YOUTUBE", "@LEXAWHAT", "https://www.youtube.com/@LexaWhat"),
    ("GITHUB", "LEXAWHATT", "https://github.com/lexawhatt"),
];

pub(super) const PAUSE_ACTION_COUNT: usize = 5;

pub(super) fn pause_hit(pos: Vec2, width: f32, height: f32) -> Option<usize> {
    (0..PAUSE_ACTION_COUNT).position(|index| {
        let (button_pos, button_size) = pause_button_rect(index, width, height);

        rect_hit(pos, button_pos, button_size)
    })
}

pub(super) fn menu_hit(pos: Vec2, width: f32, height: f32) -> Option<usize> {
    let buttons = menu_buttons(width, height);

    buttons.iter().position(|(button_pos, button_size)| {
        pos.x >= button_pos.x
            && pos.x <= button_pos.x + button_size.x
            && pos.y >= button_pos.y
            && pos.y <= button_pos.y + button_size.y
    })
}

pub(super) fn difficulty_hit(pos: Vec2, width: f32, height: f32) -> Option<usize> {
    (0..Difficulty::COUNT).position(|index| {
        let (button_pos, button_size) = difficulty_button_rect(index, width, height);
        rect_hit(pos, button_pos, button_size)
    })
}

pub(super) fn chapter_hit(pos: Vec2, width: f32, height: f32) -> Option<usize> {
    (0..CHAPTERS.len()).position(|index| {
        let (button_pos, button_size) = chapter_button_rect(index, width, height);
        rect_hit(pos, button_pos, button_size)
    })
}

pub(super) fn layer_level_hit(
    pos: Vec2,
    width: f32,
    height: f32,
    chapter_index: usize,
) -> Option<usize> {
    let mut cursor = 0;
    for (layer_index, layer) in CHAPTERS.get(chapter_index)?.layers.iter().enumerate() {
        for level_index in 0..layer.levels.len() {
            let (button_pos, button_size) =
                layer_level_rect(layer_index, level_index, width, height);
            if rect_hit(pos, button_pos, button_size) {
                return Some(cursor);
            }
            cursor += 1;
        }
    }

    None
}

pub(super) fn custom_level_hit(
    pos: Vec2,
    width: f32,
    height: f32,
    item_count: usize,
) -> Option<usize> {
    (0..item_count).position(|index| {
        let (button_pos, button_size) = custom_level_rect(index, width, height);
        rect_hit(pos, button_pos, button_size)
    })
}

pub(super) fn social_hit(pos: Vec2, width: f32, height: f32) -> Option<&'static str> {
    let y = social_y(height);
    let mut x = menu_left(width);

    for (index, (network, _, url)) in SOCIAL_LINKS.iter().enumerate() {
        let link_width = text_width(network, 2);
        let link_height = 34.0;

        if pos.x >= x && pos.x <= x + link_width && pos.y >= y && pos.y <= y + link_height {
            return Some(url);
        }

        x += link_width
            + if index == SOCIAL_LINKS.len() - 1 {
                0.0
            } else {
                34.0
            };
    }

    None
}

impl App {
    pub(in crate::app) fn displayed_progress_label(&self, difficulty_index: usize) -> String {
        let level_index = self.displayed_progress_index(difficulty_index);

        self.levels
            .get(level_index)
            .and_then(|level| level_code(&level.name))
            .unwrap_or("0-1")
            .to_string()
    }

    pub(in crate::app) fn selected_level_code(&self) -> Option<&'static str> {
        chapter_level(self.chapter_cursor, self.level_cursor).map(|level| level.code)
    }

    pub(in crate::app) fn selected_catalog_level_index(&self) -> Option<usize> {
        if is_custom_chapter(self.chapter_cursor) {
            let custom_index = self.level_cursor.checked_sub(1)?;
            return self.custom_level_indices().get(custom_index).copied();
        }

        let code = self.selected_level_code()?;

        find_level_by_code(&self.levels, code)
    }

    pub(in crate::app) fn custom_level_indices(&self) -> Vec<usize> {
        custom_level_indices(&self.levels)
    }

    pub(in crate::app) fn menu_level_count(&self) -> usize {
        if is_custom_chapter(self.chapter_cursor) {
            self.custom_level_indices().len() + 1
        } else {
            chapter_level_count(self.chapter_cursor)
        }
    }

    pub(in crate::app) fn clamp_menu_cursors(&mut self) {
        self.main_menu_cursor = self.main_menu_cursor.min(3);
        self.difficulty_cursor = self.difficulty_cursor.min(Difficulty::COUNT - 1);
        self.chapter_cursor = self.chapter_cursor.min(CHAPTERS.len() - 1);

        let count = self.menu_level_count();
        if count == 0 {
            self.level_cursor = 0;
        } else {
            self.level_cursor = self.level_cursor.min(count - 1);
        }
    }

    fn displayed_progress_index(&self, difficulty_index: usize) -> usize {
        let max_index = self.levels.len().saturating_sub(1);

        self.difficulty_progress
            .get(difficulty_index..)
            .and_then(|progress| progress.iter().max().copied())
            .unwrap_or(0)
            .min(max_index)
    }
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
        (height * 0.084).clamp(68.0, 104.0),
    );
    let gap = menu_button_gap(height);
    let total_height = button_size.y * 4.0 + gap * 3.0;
    let target_y = height * 0.405;
    let max_y = height - 156.0 - total_height;
    let min_y = height * 0.35;
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

fn pause_button_rect(index: usize, width: f32, height: f32) -> (Vec2, Vec2) {
    let size = Vec2::new(
        (width * 0.17).clamp(220.0, 330.0),
        (height * 0.052).clamp(42.0, 56.0),
    );
    let gap = (height * 0.018).clamp(12.0, 18.0);
    let total_height = size.y * PAUSE_ACTION_COUNT as f32 + gap * (PAUSE_ACTION_COUNT - 1) as f32;
    let top = (height * 0.5 - total_height * 0.5 + (height * 0.035).clamp(20.0, 38.0))
        .clamp(132.0, (height - total_height - 34.0).max(132.0));

    (
        Vec2::new((width - size.x) * 0.5, top + index as f32 * (size.y + gap)),
        size,
    )
}

fn difficulty_button_rect(index: usize, width: f32, height: f32) -> (Vec2, Vec2) {
    let button_size = Vec2::new(
        (width * 0.31).clamp(360.0, 580.0),
        (height * 0.058).clamp(46.0, 64.0),
    );
    let gap = (height * 0.012).clamp(8.0, 14.0);
    let group_gap = (height * 0.07).clamp(52.0, 84.0);
    let top = (height * 0.185).clamp(104.0, 178.0);
    let group_offset = if index >= 4 {
        group_gap * 2.0
    } else if index >= 2 {
        group_gap
    } else {
        0.0
    };
    let y = top + index as f32 * (button_size.y + gap) + group_offset;

    (Vec2::new(menu_left(width), y), button_size)
}

fn chapter_button_rect(index: usize, width: f32, height: f32) -> (Vec2, Vec2) {
    let button_size = Vec2::new(
        (width * 0.35).clamp(430.0, 650.0),
        (height * 0.058).clamp(48.0, 64.0),
    );
    let top = (height * 0.26).clamp(154.0, 260.0);
    let gap = (height * 0.017).clamp(10.0, 18.0);
    let section_gap = (height * 0.048).clamp(36.0, 54.0);
    let extra = if index >= 4 { section_gap } else { 0.0 };
    let y = top + index as f32 * (button_size.y + gap) + extra;

    (Vec2::new((width - button_size.x) * 0.5, y), button_size)
}

fn layer_level_rect(
    layer_index: usize,
    level_index: usize,
    width: f32,
    height: f32,
) -> (Vec2, Vec2) {
    let margin = (width * 0.038).clamp(36.0, 76.0);
    let gap = (width * 0.035).clamp(30.0, 66.0);
    let columns = 4.0;
    let card_w = ((width - margin * 2.0 - gap * (columns - 1.0)) / columns).clamp(190.0, 330.0);
    let card_h = (height * 0.145).clamp(98.0, 158.0);
    let row_step = (height * 0.28).clamp(182.0, 272.0);
    let top = (height * 0.26).clamp(150.0, 242.0);
    let x = margin + level_index as f32 * (card_w + gap);
    let y = top + layer_index as f32 * row_step;

    (Vec2::new(x, y), Vec2::new(card_w, card_h))
}

fn custom_level_rect(index: usize, width: f32, height: f32) -> (Vec2, Vec2) {
    let columns = if width < 920.0 { 2 } else { 4 };
    let columns_f = columns as f32;
    let margin = (width * 0.055).clamp(42.0, 96.0);
    let gap = (width * 0.028).clamp(24.0, 54.0);
    let card_w = ((width - margin * 2.0 - gap * (columns_f - 1.0)) / columns_f).clamp(210.0, 360.0);
    let card_h = (height * 0.15).clamp(104.0, 160.0);
    let col = (index % columns) as f32;
    let row = (index / columns) as f32;
    let top = (height * 0.26).clamp(148.0, 238.0);

    (
        Vec2::new(margin + col * (card_w + gap), top + row * (card_h + 36.0)),
        Vec2::new(card_w, card_h),
    )
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
    let key_w = (layout.control_w * 0.39).clamp(122.0, 172.0);
    let gap = 10.0;
    for (index, key) in GAME_ACTIONS.iter().enumerate() {
        let row_y = layout.y(96.0 + index as f32 * 68.0);
        let primary_pos = Vec2::new(layout.control_x, row_y - 12.0);
        let secondary_pos = primary_pos + Vec2::new(key_w + gap, 0.0);
        let key_size = Vec2::new(key_w, 46.0);

        if rect_hit(pos, primary_pos, key_size) || rect_hit(pos, secondary_pos, key_size) {
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
    let content_left = (width * 0.29).max(left + side_w + (width * 0.06).clamp(44.0, 78.0));
    let content_right = if options_show_scrollbar(width, height) {
        (width * 0.745).min(width - options_left(width) - 96.0)
    } else {
        width - options_left(width)
    };
    let content_w = (content_right - content_left).clamp(260.0, 920.0);
    let content_top = (height * 0.085).clamp(58.0, 92.0);
    let vertical = ((height - content_top - 80.0) / 920.0).clamp(0.68, 1.0);
    let control_x = content_left + content_w * 0.33;
    let max_control_w = (content_w * 0.42).clamp(190.0, 420.0);
    let min_control_w = 160.0_f32.min(max_control_w);
    let control_w = (content_right - control_x - 12.0).clamp(min_control_w, max_control_w);

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
    (height * 0.012).clamp(10.0, 14.0)
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

mod audio;
mod camera;
mod editor;
mod editor_geometry;
mod events;
mod frame;
mod input;
mod menu;

// Owns OS resources and connects input, simulation, camera, and rendering.
use std::sync::Arc;
use std::time::Instant;

use crate::settings::{OptionsTab, Settings, VolumeKind};
use audio::Audio;
use camera::Camera;
use editor::Editor;
use game::level_store::{LevelSpec, load_levels, save_level};
use game::{Difficulty, World};
use glam::Vec2;
use platform::input::Input;
use render::Renderer;
use render::backend::RenderBackend;
use winit::keyboard::ModifiersState;
use winit::window::Window;

use crate::{game, platform, render};

pub struct App {
    window: Option<Arc<Window>>,
    render_backend: Option<RenderBackend>,
    input: Input,
    world: World,
    levels: Vec<LevelSpec>,
    current_level: usize,
    mode: AppMode,
    menu_screen: MenuScreen,
    main_menu_cursor: usize,
    selected_difficulty: Difficulty,
    difficulty_cursor: usize,
    chapter_cursor: usize,
    level_cursor: usize,
    difficulty_progress: [usize; Difficulty::COUNT],
    pause_cursor: usize,
    pause_keyboard_focus: bool,
    settings: Settings,
    options_tab: OptionsTab,
    options_return_to_pause: bool,
    binding_capture: Option<platform::input::GameKey>,
    resolution_dropdown: bool,
    volume_drag: Option<VolumeKind>,
    editor_inspector_open: bool,
    editor: Editor,
    renderer: Renderer,
    audio: Audio,
    camera: Camera,
    cursor_screen: Vec2,
    cursor_world: Vec2,
    debug_gui: bool,
    fps: f32,
    modifiers: ModifiersState,
    last_frame: Instant,
}

impl App {
    pub fn new() -> Self {
        Self::new_with_audio(Audio::new())
    }

    fn new_with_audio(audio: Audio) -> Self {
        let levels = load_levels();
        let world = World::from_level(&levels[0]);
        let camera = Camera::new(world.player.pos);

        Self {
            window: None,
            render_backend: None,
            input: Input::new(),
            world,
            levels,
            current_level: 0,
            mode: AppMode::LevelMenu,
            menu_screen: MenuScreen::Main,
            main_menu_cursor: 0,
            selected_difficulty: Difficulty::Standard,
            difficulty_cursor: Difficulty::Standard.index(),
            chapter_cursor: 0,
            level_cursor: 0,
            difficulty_progress: [0; Difficulty::COUNT],
            pause_cursor: 0,
            pause_keyboard_focus: false,
            settings: Settings::new(),
            options_tab: OptionsTab::General,
            options_return_to_pause: false,
            binding_capture: None,
            resolution_dropdown: false,
            volume_drag: None,
            editor_inspector_open: false,
            editor: Editor::new(),
            renderer: Renderer::new(),
            audio,
            camera,
            cursor_screen: Vec2::ZERO,
            cursor_world: Vec2::ZERO,
            debug_gui: false,
            fps: 0.0,
            modifiers: ModifiersState::empty(),
            last_frame: Instant::now(),
        }
    }

    fn load_current_level(&mut self) {
        if let Some(level) = self.levels.get(self.current_level) {
            self.world
                .load_level_with_difficulty(level, self.selected_difficulty);
            self.camera.center = self.world.player.pos;
            self.editor = Editor::new();
            self.input.release_gameplay();
            self.audio.stop_actions();
        }
    }

    fn complete_current_level(&mut self) {
        let difficulty = self.selected_difficulty.index();
        self.difficulty_progress[difficulty] =
            self.difficulty_progress[difficulty].max(self.current_level);
        self.menu_screen = MenuScreen::Layer;
        self.mode = AppMode::LevelMenu;
        self.input.release_gameplay();
        self.audio.stop_actions();
    }

    fn save_current_level(&mut self) {
        let Some(level) = self.levels.get_mut(self.current_level) else {
            return;
        };

        level.replace_world(&self.world.level);
        if save_level(level).is_ok() {
            self.editor.mark_saved();
        }
    }

    fn create_custom_level(&mut self) {
        let mut level = LevelSpec::custom_template(self.next_custom_level_name());
        if save_level(&mut level).is_err() {
            return;
        }

        self.levels.push(level);
        self.current_level = self.levels.len() - 1;
        self.level_cursor = self.custom_level_indices().len();
        self.load_current_level();
        self.mode = AppMode::Editor;
        self.editor.mark_saved();
    }

    fn next_custom_level_name(&self) -> String {
        for index in 1..10_000 {
            let name = format!("CUSTOM LEVEL {index}");
            if self.levels.iter().all(|level| level.name != name) {
                return name;
            }
        }

        "CUSTOM LEVEL".to_string()
    }

    fn refresh_cursor_world(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();

        self.refresh_cursor_world_for(size.width.max(1), size.height.max(1));
    }

    fn refresh_cursor_world_for(&mut self, width: u32, height: u32) {
        self.cursor_world =
            self.camera
                .screen_to_world(self.cursor_screen, width as f32, height as f32);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Playing,
    Pause,
    LevelMenu,
    Changelog,
    Options,
    Editor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuScreen {
    Main,
    Difficulty,
    Chapter,
    Layer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::progression::CUSTOM_LEVELS_CHAPTER_INDEX;
    use crate::platform::input::GameKey;
    use winit::keyboard::KeyCode;

    fn test_app() -> App {
        App::new_with_audio(Audio::silent())
    }

    #[test]
    fn menu_keyboard_selection_stays_in_bounds() {
        let mut app = test_app();

        app.handle_menu_key(KeyCode::ArrowUp);
        assert_eq!(app.main_menu_cursor, 0);

        app.handle_menu_key(KeyCode::ArrowDown);
        assert!(app.main_menu_cursor < 4);
    }

    #[test]
    fn options_escape_clears_binding_before_leaving_menu() {
        let mut app = test_app();

        app.mode = AppMode::Options;
        app.binding_capture = Some(GameKey::Jump);
        app.handle_options_key(KeyCode::Escape);

        assert!(app.binding_capture.is_none());
        assert!(matches!(app.mode, AppMode::Options));

        app.handle_options_key(KeyCode::Escape);
        assert!(matches!(app.mode, AppMode::LevelMenu));
    }

    #[test]
    fn changelog_confirm_returns_to_level_menu() {
        let mut app = test_app();

        app.mode = AppMode::Changelog;
        app.handle_changelog_key(KeyCode::Enter);

        assert!(matches!(app.mode, AppMode::LevelMenu));
    }

    #[test]
    fn pause_resume_returns_to_playing() {
        let mut app = test_app();

        app.mode = AppMode::Playing;
        app.open_pause_menu();
        assert!(matches!(app.mode, AppMode::Pause));
        assert!(!app.pause_keyboard_focus);

        app.resume_from_pause();
        assert!(matches!(app.mode, AppMode::Playing));
    }

    #[test]
    fn pause_keyboard_focus_starts_after_navigation() {
        let mut app = test_app();

        app.open_pause_menu();
        app.handle_pause_key(KeyCode::ArrowDown);

        assert!(app.pause_keyboard_focus);
        assert_eq!(app.pause_cursor, 0);
    }

    #[test]
    fn pause_options_return_to_pause() {
        let mut app = test_app();

        app.mode = AppMode::Pause;
        app.options_return_to_pause = true;
        app.mode = AppMode::Options;
        app.close_options();

        assert!(matches!(app.mode, AppMode::Pause));
    }

    #[test]
    fn custom_levels_chapter_opens_layer_screen() {
        let mut app = test_app();

        app.menu_screen = MenuScreen::Chapter;
        app.chapter_cursor = CUSTOM_LEVELS_CHAPTER_INDEX;
        app.handle_menu_key(KeyCode::Enter);

        assert_eq!(app.menu_screen, MenuScreen::Layer);
        assert_eq!(app.menu_level_count(), app.custom_level_indices().len() + 1);
    }
}

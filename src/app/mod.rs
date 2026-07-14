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

use audio::Audio;
use camera::Camera;
use editor::Editor;
use game::World;
use game::levels::{LevelSpec, load_levels, save_level};
use glam::Vec2;
use platform::input::Input;
use render::Renderer;
use softbuffer::{Context, Surface};
use winit::keyboard::ModifiersState;
use winit::window::Window;

use crate::{game, platform, render};

pub struct App {
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    input: Input,
    world: World,
    levels: Vec<LevelSpec>,
    current_level: usize,
    mode: AppMode,
    editor: Editor,
    renderer: Renderer,
    audio: Audio,
    camera: Camera,
    cursor_screen: Vec2,
    cursor_world: Vec2,
    debug_gui: bool,
    modifiers: ModifiersState,
    last_frame: Instant,
}

impl App {
    pub fn new() -> Self {
        let levels = load_levels();
        let world = World::from_level(&levels[0]);
        let camera = Camera::new(world.player.pos);

        Self {
            window: None,
            context: None,
            surface: None,
            input: Input::new(),
            world,
            levels,
            current_level: 0,
            mode: AppMode::Playing,
            editor: Editor::new(),
            renderer: Renderer::new(),
            audio: Audio::new(),
            camera,
            cursor_screen: Vec2::ZERO,
            cursor_world: Vec2::ZERO,
            debug_gui: false,
            modifiers: ModifiersState::empty(),
            last_frame: Instant::now(),
        }
    }

    fn load_current_level(&mut self) {
        if let Some(level) = self.levels.get(self.current_level) {
            self.world.load_level(level);
            self.camera.center = self.world.player.pos;
            self.editor = Editor::new();
            self.input.release_gameplay();
            self.audio.stop_actions();
        }
    }

    fn save_current_level(&mut self) {
        let Some(level) = self.levels.get_mut(self.current_level) else {
            return;
        };

        *level = LevelSpec::from_world(
            level.name.clone(),
            level.spawn,
            self.world.level.solids.clone(),
            self.world.level.doors.clone(),
            self.world.level.hazards.clone(),
            self.world.level.checkpoints.clone(),
            self.world.level.texts.clone(),
            level.path.clone(),
        );
        if save_level(level).is_ok() {
            self.editor.mark_saved();
        }
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
    LevelMenu,
    Editor,
}

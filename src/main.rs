mod constants;
mod game;
mod platform;
mod render;

use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use game::World;
use glam::Vec2;
use platform::input::{GameKey, Input};
use render::Renderer;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();

    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    input: Input,
    world: World,
    renderer: Renderer,
    last_frame: Instant,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            context: None,
            surface: None,
            input: Input::new(),
            world: World::new(),
            renderer: Renderer::new(),
            last_frame: Instant::now(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Portals")
            .with_inner_size(LogicalSize::new(900.0, 600.0));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.input
                    .set_aim_pos(Vec2::new(position.x as f32, position.y as f32));
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let down = event.state == ElementState::Pressed;

                match event.physical_key {
                    // Physical keys = layout does not matter
                    PhysicalKey::Code(KeyCode::KeyA | KeyCode::ArrowLeft) => {
                        self.input.set_key(GameKey::Left, down);
                    }
                    PhysicalKey::Code(KeyCode::KeyD | KeyCode::ArrowRight) => {
                        self.input.set_key(GameKey::Right, down);
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        self.input.set_key(GameKey::Jump, down);
                    }
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {
                        self.input.set_key(GameKey::Dash, down);
                    }
                    PhysicalKey::Code(KeyCode::ControlLeft) => {
                        self.input.set_key(GameKey::Slide, down);
                    }
                    PhysicalKey::Code(KeyCode::Escape) if down => event_loop.exit(),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl App {
    fn redraw(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(1.0 / 20.0);
        self.last_frame = now;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.input.update();
        self.world
            .update(dt, &self.input, width as f32, height as f32);

        surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        let mut buffer = surface.buffer_mut().unwrap();
        self.renderer.draw(&mut buffer, width, height, &self.world);
        buffer.present().unwrap();
    }
}

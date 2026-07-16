use std::sync::Arc;

// Winit callback layer; behavior is routed to input/frame modules.
use glam::Vec2;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Fullscreen, Window, WindowId};

use crate::app::{App, AppMode};
use crate::settings::{DisplayMode, Resolution};

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Aperture Kill")
            .with_inner_size(LogicalSize::new(900.0, 600.0))
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("Failed to create the game window: {err}");
                event_loop.exit();
                return;
            }
        };
        self.settings
            .set_resolutions(available_resolutions(window.as_ref()));
        let context = match Context::new(window.clone()) {
            Ok(context) => context,
            Err(err) => {
                eprintln!("Failed to create the rendering context: {err}");
                event_loop.exit();
                return;
            }
        };
        let surface = match Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("Failed to create the rendering surface: {err}");
                event_loop.exit();
                return;
            }
        };

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
                self.cursor_screen = Vec2::new(position.x as f32, position.y as f32);
                self.refresh_cursor_world();
                self.input.set_aim_pos(self.cursor_world);
                if self.mode == AppMode::Editor {
                    self.editor
                        .drag_to(self.cursor_world, &mut self.world.level);
                }
                if self.mode == AppMode::Options {
                    self.drag_options_volume();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                self.handle_mouse(button, down, event_loop);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Focused(false) => {
                self.input.release_gameplay();
                self.volume_drag = None;
                self.editor.cancel_drag();
                self.modifiers = Default::default();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let down = event.state == ElementState::Pressed;

                self.handle_key(event.physical_key, down, event_loop);
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
    pub(super) fn apply_display_settings(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        self.settings
            .set_resolutions(available_resolutions(window.as_ref()));
        apply_window_display(window, self.settings.display_mode, self.settings.resolution);
    }
}

fn apply_window_display(window: &Window, mode: DisplayMode, resolution: Resolution) {
    match mode {
        DisplayMode::Borderless => {
            window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
        }
        DisplayMode::Fullscreen => {
            // Exclusive mode is unavailable on some compositors; borderless preserves fullscreen.
            let fullscreen = window.current_monitor().and_then(|monitor| {
                monitor
                    .video_modes()
                    .find(|video_mode| {
                        let size = video_mode.size();
                        size.width == resolution.width && size.height == resolution.height
                    })
                    .map(Fullscreen::Exclusive)
            });
            window.set_fullscreen(
                fullscreen.or_else(|| Some(Fullscreen::Borderless(window.current_monitor()))),
            );
        }
        DisplayMode::Windowed => {
            window.set_fullscreen(None);
            let _ = window.request_inner_size(LogicalSize::new(
                resolution.width as f64,
                resolution.height as f64,
            ));
        }
    }
}

fn available_resolutions(window: &Window) -> Vec<Resolution> {
    let Some(monitor) = window.current_monitor() else {
        return Vec::new();
    };

    let monitor_size = monitor.size();
    let mut resolutions = monitor
        .video_modes()
        .map(|video_mode| {
            let size = video_mode.size();
            Resolution {
                width: size.width,
                height: size.height,
            }
        })
        .collect::<Vec<_>>();

    // Video modes omit useful windowed sizes on several platforms, so merge a bounded fallback set.
    for resolution in common_resolutions(monitor_size.width, monitor_size.height) {
        resolutions.push(resolution);
    }

    if resolutions.is_empty() {
        resolutions.push(Resolution {
            width: monitor_size.width,
            height: monitor_size.height,
        });
    }

    resolutions
}

fn common_resolutions(max_width: u32, max_height: u32) -> Vec<Resolution> {
    [
        (3840, 2160),
        (3440, 1440),
        (3200, 1800),
        (2560, 1440),
        (2560, 1080),
        (2304, 1296),
        (2048, 1152),
        (1920, 1200),
        (1920, 1080),
        (1920, 800),
        (1680, 1050),
        (1600, 900),
        (1440, 1080),
        (1440, 960),
        (1440, 900),
        (1366, 768),
        (1280, 1024),
        (1280, 720),
        (1024, 768),
        (960, 540),
        (800, 600),
        (640, 480),
        (320, 240),
        (320, 200),
    ]
    .into_iter()
    .filter(|(width, height)| *width <= max_width && *height <= max_height)
    .map(|(width, height)| Resolution { width, height })
    .collect()
}

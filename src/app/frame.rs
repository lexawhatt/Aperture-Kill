use std::num::NonZeroU32;
use std::time::Instant;

// Per-frame order: input, simulation/camera, then render.
use crate::app::{App, AppMode};
use crate::render::{DebugOverlay, EditorOverlay, RenderMode};

impl App {
    pub(super) fn redraw(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };

        let dt = self.frame_dt();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.update_frame(dt, width, height);
        self.render_frame(width, height);
    }

    fn frame_dt(&mut self) -> f32 {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(1.0 / 20.0);
        self.last_frame = now;
        dt
    }

    fn update_frame(&mut self, dt: f32, width: u32, height: u32) {
        self.refresh_cursor_world_for(width, height);
        self.input.set_aim_pos(self.cursor_world);
        self.input.update();
        self.editor.update(dt);

        match self.mode {
            AppMode::Playing => self.update_playing(dt, width, height),
            AppMode::Editor => self.update_editor(dt, width, height),
            AppMode::LevelMenu => {}
        }
    }

    fn update_playing(&mut self, dt: f32, width: u32, height: u32) {
        self.world
            .update(dt, &self.input, width as f32, height as f32);
        let sound_events = self.world.drain_sound_events().collect::<Vec<_>>();
        let listener = self.world.player.pos;
        for event in sound_events {
            self.audio.play(event, listener);
        }
        self.camera.follow(self.world.player.pos, dt);
        self.refresh_cursor_world_for(width, height);
        self.input.set_aim_pos(self.cursor_world);
    }

    fn update_editor(&mut self, dt: f32, width: u32, height: u32) {
        self.camera.pan(self.editor.pan_direction(), dt);
        self.refresh_cursor_world_for(width, height);
        self.editor
            .drag_to(self.cursor_world, &mut self.world.level);
    }

    fn render_frame(&mut self, width: u32, height: u32) {
        let render_mode = match self.mode {
            AppMode::Playing => RenderMode::Playing,
            AppMode::LevelMenu => RenderMode::LevelMenu {
                levels: &self.levels,
                selected: self.current_level,
            },
            AppMode::Editor => RenderMode::Editor(EditorOverlay {
                selected_solids: self.editor.selected_solids(),
                selected_doors: self.editor.selected_doors(),
                selected_hazards: self.editor.selected_hazards(),
                selected_checkpoints: self.editor.selected_checkpoints(),
                selected_texts: self.editor.selected_texts(),
                selection_count: self.editor.selection_count(),
                text_editing: self.editor.text_editing(),
                marquee: self.editor.marquee_rect(),
                active_tool: self.editor.tool.index(),
                rotate_ui: self.editor.rotate_ui,
                grid_snap: self.editor.grid_snap(),
                dirty: self.editor.dirty,
                saved_flash: self.editor.status_timer > 0.0,
            }),
        };
        let renderer = &self.renderer;
        let world = &self.world;
        let camera = self.camera.center;
        let zoom = self.camera.zoom;
        let debug = self.debug_overlay();
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        let mut buffer = surface.buffer_mut().unwrap();

        renderer.draw(
            &mut buffer,
            width,
            height,
            world,
            render_mode,
            camera,
            zoom,
            debug,
        );
        buffer.present().unwrap();
    }

    fn debug_overlay(&self) -> Option<DebugOverlay> {
        if !self.debug_gui {
            return None;
        }

        Some(DebugOverlay {
            mode: match self.mode {
                AppMode::Playing => "PLAY",
                AppMode::LevelMenu => "MENU",
                AppMode::Editor => "EDIT",
            },
            player_pos: self.world.player.pos,
            player_vel: self.world.player.vel,
            camera: self.camera.center,
            zoom: self.camera.zoom,
            cursor_world: self.cursor_world,
            on_ground: self.world.player.on_ground,
            sliding: self.world.player.sliding,
            dashing: self.world.player.dashing,
            slamming: self.world.player.ground_slamming,
            solid_count: self.world.level.solids.len(),
            portal_count: self.world.portals.iter().flatten().count(),
        })
    }
}

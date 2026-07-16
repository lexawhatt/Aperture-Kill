use std::num::NonZeroU32;
use std::time::Instant;

// Per-frame order: input, simulation/camera, then render.
use crate::app::{App, AppMode};
use crate::render::{
    DebugOverlay, EditorDoorInspector, EditorInspector, EditorOverlay, EditorWorldPortalInspector,
    RenderFrame, RenderMode,
};

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
        // A monotonic clock should not move backwards, but saturating keeps a platform clock
        // anomaly from turning physics state into NaN.
        let dt = now
            .saturating_duration_since(self.last_frame)
            .as_secs_f32()
            .min(1.0 / 20.0);
        self.last_frame = now;
        if dt > 0.0 {
            let instant_fps = 1.0 / dt;
            self.fps = if self.fps == 0.0 {
                instant_fps
            } else {
                self.fps * 0.9 + instant_fps * 0.1
            };
        }
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
            AppMode::LevelMenu | AppMode::Changelog | AppMode::Options => {}
        }
        self.sync_mode_audio();
    }

    fn update_playing(&mut self, dt: f32, width: u32, height: u32) {
        self.world
            .update(dt, &self.input, width as f32, height as f32);
        let listener = self.world.player.pos;
        for event in self.world.drain_sound_events() {
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
        let editor_overlay = (self.mode == AppMode::Editor).then(|| self.editor_overlay());
        let render_mode = match self.mode {
            AppMode::Playing => RenderMode::Playing,
            AppMode::LevelMenu => RenderMode::LevelMenu {
                levels: &self.levels,
                selected: self.current_level,
            },
            AppMode::Changelog => RenderMode::Changelog,
            AppMode::Options => RenderMode::Options {
                settings: &self.settings,
                active_tab: self.options_tab,
                capture: self.binding_capture,
                resolution_dropdown: self.resolution_dropdown,
            },
            AppMode::Editor => {
                let Some(overlay) = editor_overlay.as_ref() else {
                    return;
                };
                RenderMode::Editor(overlay)
            }
        };
        let renderer = &self.renderer;
        let world = &self.world;
        let camera = self.camera.center;
        let zoom = self.camera.zoom;
        let debug = self.debug_overlay();
        let fps = self.settings.show_fps.then_some(self.fps);
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let Some(width) = NonZeroU32::new(width) else {
            return;
        };
        let Some(height) = NonZeroU32::new(height) else {
            return;
        };
        if surface.resize(width, height).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        renderer.draw(RenderFrame {
            frame: &mut buffer,
            width: width.get(),
            height: height.get(),
            world,
            mode: render_mode,
            camera,
            zoom,
            debug,
            fps,
        });
        let _ = buffer.present();
    }

    fn editor_overlay(&self) -> EditorOverlay {
        let inspector = match (
            self.editor.primary_door_index(),
            self.editor.primary_world_portal_index(),
        ) {
            (Some(index), _) => self
                .world
                .level
                .doors
                .get(index)
                .map(|door| {
                    EditorInspector::Door(EditorDoorInspector {
                        automatic: door.automatic,
                        trigger_radius: door.trigger_radius,
                        speed: door.speed,
                    })
                })
                .unwrap_or(EditorInspector::None),
            (_, Some(index)) => self
                .world
                .level
                .world_portals
                .get(index)
                .map(|portal| {
                    EditorInspector::WorldPortal(EditorWorldPortalInspector {
                        id: portal.id,
                        receiver_id: portal.receiver_id,
                        priority: portal.priority,
                        width: portal.portal.width,
                    })
                })
                .unwrap_or(EditorInspector::None),
            _ => EditorInspector::None,
        };

        EditorOverlay {
            selected_solids: self.editor.selected_solids(),
            selected_doors: self.editor.selected_doors(),
            selected_hazards: self.editor.selected_hazards(),
            selected_checkpoints: self.editor.selected_checkpoints(),
            selected_texts: self.editor.selected_texts(),
            selected_world_portals: self.editor.selected_world_portals(),
            selection_count: self.editor.selection_count(),
            text_editing: self.editor.text_editing(),
            marquee: self.editor.marquee_rect(),
            active_tool: self.editor.tool.index(),
            active_tool_label: self.editor.tool.label(),
            selection_kind: self.editor.primary_selection_kind().label(),
            inspector,
            inspector_open: self.editor_inspector_open,
            rotate_ui: self.editor.rotate_ui,
            grid_snap: self.editor.grid_snap(),
            dirty: self.editor.dirty,
            saved_flash: self.editor.status_timer > 0.0,
        }
    }

    fn debug_overlay(&self) -> Option<DebugOverlay> {
        if !self.debug_gui {
            return None;
        }

        Some(DebugOverlay {
            mode: match self.mode {
                AppMode::Playing => "PLAY",
                AppMode::LevelMenu => "MENU",
                AppMode::Changelog => "CHANGELOG",
                AppMode::Options => "OPTIONS",
                AppMode::Editor => "EDIT",
            },
            player_pos: self.world.player.pos,
            player_vel: self.world.player.vel,
            camera: self.camera.center,
            zoom: self.camera.zoom,
            cursor_world: self.cursor_world,
            on_ground: self.world.player.on_ground,
            sliding: self.world.player.is_sliding(),
            dashing: self.world.player.is_dashing(),
            slamming: self.world.player.is_ground_slamming(),
            solid_count: self.world.level.solids.len(),
            portal_count: self.world.portals.iter().flatten().count(),
        })
    }

    fn sync_mode_audio(&mut self) {
        if matches!(
            self.mode,
            AppMode::LevelMenu | AppMode::Changelog | AppMode::Options
        ) {
            self.audio.start_menu_ambience();
        } else {
            self.audio.stop_menu_ambience();
        }
    }
}

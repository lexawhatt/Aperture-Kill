use std::time::Instant;

// Per-frame order: input, simulation/camera, then render.
use crate::app::menu::{difficulty_hit, pause_hit};
use crate::app::{App, AppMode, MenuScreen};
use crate::game::level::LevelTriggerKind;
use crate::game::progression::{custom_level_indices, level_code};
use crate::render::{
    DebugOverlay, EditorDoorInspector, EditorEnemyInspector, EditorEnemySpawnTriggerInspector,
    EditorInspector, EditorObjectMeta, EditorOverlay, EditorWorldPortalInspector, LevelMenuOverlay,
    LevelMenuScreen, PauseOverlay, RenderMode, RenderScene,
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
            AppMode::Pause | AppMode::LevelMenu | AppMode::Changelog | AppMode::Options => {}
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
        if self.world.take_level_completed() {
            self.complete_current_level();
            return;
        }
        self.camera.center += self.world.take_camera_shift();
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
        let pause_overlay =
            (self.mode == AppMode::Pause).then(|| self.pause_overlay(width, height));
        let level_menu_overlay =
            (self.mode == AppMode::LevelMenu).then(|| self.level_menu_overlay(width, height));
        let render_mode = match self.mode {
            AppMode::Playing => RenderMode::Playing,
            AppMode::Pause => {
                let Some(overlay) = pause_overlay.as_ref() else {
                    return;
                };
                RenderMode::Pause(overlay)
            }
            AppMode::LevelMenu => {
                let Some(overlay) = level_menu_overlay.as_ref() else {
                    return;
                };
                RenderMode::LevelMenu(overlay)
            }
            AppMode::Changelog => RenderMode::Changelog,
            AppMode::Options => RenderMode::Options {
                settings: &self.settings,
                active_tab: self.options_tab,
                capture: self.binding_capture,
                resolution_dropdown: self.resolution_dropdown,
                dim_level_background: self.options_return_to_pause,
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
        let camera = self
            .world
            .death
            .map(|death| self.camera.center + death.camera_offset())
            .unwrap_or(self.camera.center);
        let zoom = self
            .world
            .death
            .map(|death| self.camera.zoom * death.camera_zoom())
            .unwrap_or(self.camera.zoom);
        let debug = self.debug_overlay();
        let fps = self.settings.show_fps.then_some(self.fps);
        let Some(render_backend) = self.render_backend.as_mut() else {
            return;
        };
        let scene = RenderScene {
            width,
            height,
            world,
            mode: render_mode,
            camera,
            zoom,
            debug,
            fps,
        };

        if let Err(err) = render_backend.render(renderer, scene) {
            eprintln!("Failed to render frame: {err}");
        }
    }

    fn editor_overlay(&self) -> EditorOverlay {
        let inspector = match (
            self.editor.primary_door_index(),
            self.editor.primary_enemy_index(),
            self.editor.primary_trigger_index(),
            self.editor.primary_world_portal_index(),
        ) {
            (Some(index), _, _, _) => self
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
            (_, Some(index), _, _) => self
                .world
                .level
                .enemies
                .get(index)
                .map(|enemy| {
                    EditorInspector::Enemy(EditorEnemyInspector {
                        spawn_id: enemy.spawn_id.max(1),
                        spawn_wave: enemy.spawn_wave.max(1),
                    })
                })
                .unwrap_or(EditorInspector::None),
            (_, _, Some(index), _) => self
                .world
                .level
                .triggers
                .get(index)
                .and_then(|trigger| match trigger.kind {
                    LevelTriggerKind::EnemySpawn { enemy_id } => {
                        Some(EditorInspector::EnemySpawnTrigger(
                            EditorEnemySpawnTriggerInspector { enemy_id },
                        ))
                    }
                    _ => None,
                })
                .unwrap_or(EditorInspector::None),
            (_, _, _, Some(index)) => self
                .world
                .level
                .world_portals
                .get(index)
                .map(|portal| {
                    EditorInspector::WorldPortal(EditorWorldPortalInspector {
                        id: portal.id,
                        receiver_id: portal.receiver_id,
                        priority: portal.priority,
                        scale: portal.portal.scale,
                        seamless: portal.seamless,
                        seamless_depth: portal.seamless_depth,
                        seamless_angle: portal.seamless_angle,
                        seamless_rely_on_walls: portal.seamless_rely_on_walls,
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
            selected_enemies: self.editor.selected_enemies(),
            selected_triggers: self.editor.selected_triggers(),
            selected_texts: self.editor.selected_texts(),
            selected_world_portals: self.editor.selected_world_portals(),
            selection_count: self.editor.selection_count(),
            text_editing: self.editor.text_editing(),
            marquee: self.editor.marquee_rect(),
            active_tool: self.editor.tool.index(),
            active_tool_label: if self.editor.category().contains_tool(self.editor.tool) {
                self.editor.tool.label()
            } else {
                self.editor.category().label()
            },
            active_category: self.editor.category().index(),
            active_category_label: self.editor.category().label(),
            active_layer: self.editor.active_layer(&self.world.level),
            editor_mode: self.editor.mode().index(),
            editor_mode_label: self.editor.mode().label(),
            selection_kind: self.editor.primary_selection_kind().label(),
            object_meta: self
                .editor
                .primary_object_meta(&self.world.level)
                .map(|meta| EditorObjectMeta {
                    id: meta.id,
                    layer: meta.layer,
                }),
            inspector,
            inspector_open: self.editor_inspector_open,
            rotate_ui: self.editor.rotate_ui,
            grid_snap: self.editor.grid_snap(),
            dirty: self.editor.dirty,
            saved_flash: self.editor.status_timer > 0.0,
        }
    }

    fn level_menu_overlay(&self, width: u32, height: u32) -> LevelMenuOverlay {
        let difficulty_hover = difficulty_hit(self.cursor_screen, width as f32, height as f32)
            .filter(|index| {
                self.menu_screen == MenuScreen::Difficulty
                    && *index < crate::game::Difficulty::COUNT
            });

        LevelMenuOverlay {
            screen: match self.menu_screen {
                MenuScreen::Main => LevelMenuScreen::Main,
                MenuScreen::Difficulty => LevelMenuScreen::Difficulty,
                MenuScreen::Chapter => LevelMenuScreen::Chapter,
                MenuScreen::Layer => LevelMenuScreen::Layer,
            },
            main_cursor: self.main_menu_cursor,
            difficulty_cursor: self.difficulty_cursor,
            selected_difficulty: self.selected_difficulty.index(),
            difficulty_hover,
            difficulty_progress: std::array::from_fn(|index| self.displayed_progress_label(index)),
            chapter_cursor: self.chapter_cursor,
            level_cursor: self.level_cursor,
            available_level_codes: self
                .levels
                .iter()
                .filter_map(|level| level_code(&level.name))
                .map(str::to_string)
                .collect(),
            custom_level_names: custom_level_indices(&self.levels)
                .into_iter()
                .filter_map(|index| self.levels.get(index))
                .map(|level| level.name.clone())
                .collect(),
        }
    }

    fn debug_overlay(&self) -> Option<DebugOverlay> {
        if !self.debug_gui {
            return None;
        }

        Some(DebugOverlay {
            mode: match self.mode {
                AppMode::Playing => "PLAY",
                AppMode::Pause => "PAUSE",
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

    fn pause_overlay(&self, width: u32, height: u32) -> PauseOverlay {
        PauseOverlay {
            keyboard_focus: self.pause_keyboard_focus.then_some(self.pause_cursor),
            hover: pause_hit(self.cursor_screen, width as f32, height as f32),
        }
    }

    fn sync_mode_audio(&mut self) {
        if matches!(self.mode, AppMode::LevelMenu | AppMode::Changelog)
            || (self.mode == AppMode::Options && !self.options_return_to_pause)
        {
            self.audio.start_menu_ambience();
        } else {
            self.audio.stop_menu_ambience();
        }
    }
}

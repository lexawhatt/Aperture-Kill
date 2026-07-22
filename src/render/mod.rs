mod canvas;
mod glyphs;
mod text;
mod ui;
mod world;

// Renderer is a facade; Canvas owns low-level pixel drawing.
use glam::Vec2;

use crate::game::World;
use crate::game::level::WorldPortal;
use crate::game::levels::LevelSpec;
use crate::game::portal::{Color, Portal};
use crate::platform::input::GameKey;
use crate::settings::{OptionsTab, Settings};

use canvas::{Canvas, Rect};

pub struct Renderer;

pub enum RenderMode<'a> {
    Playing,
    LevelMenu {
        levels: &'a [LevelSpec],
        selected: usize,
    },
    Changelog,
    Options {
        settings: &'a Settings,
        active_tab: OptionsTab,
        capture: Option<GameKey>,
        resolution_dropdown: bool,
    },
    Editor(&'a EditorOverlay),
}

pub struct RenderFrame<'frame, 'data> {
    pub frame: &'frame mut [u32],
    pub width: u32,
    pub height: u32,
    pub world: &'data World,
    pub mode: RenderMode<'data>,
    pub camera: Vec2,
    pub zoom: f32,
    pub debug: Option<DebugOverlay>,
    pub fps: Option<f32>,
}

pub struct EditorOverlay {
    pub selected_solids: Vec<usize>,
    pub selected_doors: Vec<usize>,
    pub selected_hazards: Vec<usize>,
    pub selected_checkpoints: Vec<usize>,
    pub selected_texts: Vec<usize>,
    pub selected_world_portals: Vec<usize>,
    pub selection_count: usize,
    pub text_editing: bool,
    pub marquee: Option<(Vec2, Vec2)>,
    pub active_tool: usize,
    pub active_tool_label: &'static str,
    pub selection_kind: &'static str,
    pub inspector: EditorInspector,
    pub inspector_open: bool,
    pub rotate_ui: bool,
    pub grid_snap: bool,
    pub dirty: bool,
    pub saved_flash: bool,
}

#[derive(Clone, Copy)]
pub enum EditorInspector {
    None,
    Door(EditorDoorInspector),
    WorldPortal(EditorWorldPortalInspector),
}

#[derive(Clone, Copy)]
pub struct EditorDoorInspector {
    pub automatic: bool,
    pub trigger_radius: f32,
    pub speed: f32,
}

#[derive(Clone, Copy)]
pub struct EditorWorldPortalInspector {
    pub id: u16,
    pub receiver_id: u16,
    pub priority: i16,
    pub scale: f32,
    pub seamless: bool,
    pub seamless_depth: f32,
    pub seamless_angle: f32,
    pub seamless_rely_on_walls: bool,
}

#[derive(Clone, Copy)]
pub struct DebugOverlay {
    pub mode: &'static str,
    pub player_pos: Vec2,
    pub player_vel: Vec2,
    pub camera: Vec2,
    pub zoom: f32,
    pub cursor_world: Vec2,
    pub on_ground: bool,
    pub sliding: bool,
    pub dashing: bool,
    pub slamming: bool,
    pub solid_count: usize,
    pub portal_count: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, render: RenderFrame<'_, '_>) {
        let RenderFrame {
            frame,
            width,
            height,
            world,
            mode,
            camera,
            zoom,
            debug,
            fps,
        } = render;
        let mut canvas = Canvas::new(frame, width, height, camera, zoom);
        let editor_mode = matches!(&mode, RenderMode::Editor(_));

        canvas.clear(Color::rgb(9, 10, 14));
        self.draw_world(&mut canvas, world, editor_mode);
        canvas.hud(
            world.player.health,
            world.player.health_percent(),
            world.player.dash_charges,
            world.player.dash_deny_flash(),
        );
        canvas.damage_pulse(world.damage_pulse.amount());
        if let Some(death) = world.death {
            canvas.death_overlay(death);
        }

        match mode {
            RenderMode::Playing => {}
            RenderMode::LevelMenu { levels, selected } => {
                canvas.level_menu(levels, selected);
            }
            RenderMode::Changelog => {
                canvas.changelog_menu();
            }
            RenderMode::Options {
                settings,
                active_tab,
                capture,
                resolution_dropdown,
            } => {
                canvas.options_menu(settings, active_tab, capture, resolution_dropdown);
            }
            RenderMode::Editor(overlay) => canvas.editor_overlay(world, overlay),
        }

        if let Some(debug) = debug {
            canvas.debug_overlay(debug);
        }
        if let Some(fps) = fps {
            canvas.fps_counter(fps);
        }
    }

    fn draw_world(&self, canvas: &mut Canvas<'_>, world: &World, editor_mode: bool) {
        canvas.seamless_portal_views(world);
        let seamless_portals = world
            .level
            .world_portals
            .iter()
            .copied()
            .filter(|portal| portal.seamless)
            .collect::<Vec<_>>();

        // Static geometry is drawn first so portals and actors stay readable.
        for solid in &world.level.solids {
            let fill = if solid.portalable {
                Color::rgb(36, 42, 53)
            } else {
                Color::rgb(55, 45, 47)
            };
            canvas.solid_with_seamless_holes(
                *solid,
                fill,
                Color::rgb(92, 105, 125),
                &seamless_portals,
            );
        }

        for door in &world.level.doors {
            canvas.door(*door);
        }

        for hazard in &world.level.hazards {
            canvas.hazard(*hazard);
        }

        for checkpoint in &world.level.checkpoints {
            canvas.checkpoint(*checkpoint);
        }

        for text in &world.level.texts {
            canvas.world_text(text.pos, &text.text, 2, Color::rgb(210, 218, 230));
        }

        for portal in &world.level.world_portals {
            if portal.seamless && !editor_mode {
                continue;
            }

            let (a, b) = portal.portal.endpoints();
            canvas.draw_world_line(a, b, portal.portal.color);
            canvas.draw_world_line(
                portal.portal.pos,
                portal.portal.pos + portal.portal.normal() * 16.0,
                portal.portal.color,
            );
            canvas.world_text(
                portal.portal.pos + Vec2::new(8.0, -18.0),
                &format!("{}>{}:{}", portal.id, portal.receiver_id, portal.priority),
                1,
                Color::rgb(210, 198, 255),
            );
        }

        // Portals are visual-only here; physics is owned by World.
        for portal in world.portals.iter().flatten() {
            let (a, b) = portal.endpoints();
            canvas.draw_world_line(a, b, portal.color);
            canvas.draw_world_line(
                portal.pos,
                portal.pos + portal.normal() * 16.0,
                portal.color,
            );
        }

        for enemy in &world.level.enemies {
            canvas.enemy(enemy);
        }

        if let Some(beam) = world.piercer.beam {
            canvas.piercer_beam(beam);
        }

        self.draw_player(canvas, world);
        self.draw_aim(canvas, world);
    }

    fn draw_player(&self, canvas: &mut Canvas<'_>, world: &World) {
        let player = &world.player;
        let rect = Rect {
            pos: player.pos - player.half_size(),
            size: player.size,
        };
        let fill = if player.is_ground_slamming() {
            Color::rgb(255, 224, 102)
        } else if player.is_dashing() {
            Color::rgb(80, 92, 130)
        } else if player.is_wall_sliding() {
            Color::rgb(62, 76, 92)
        } else {
            Color::rgb(45, 49, 59)
        };
        let outline = Color::rgb(235, 238, 245);

        if let [Some(source), Some(destination)] = world.portals {
            if source.opens_for_body(player.pos, player.half_size()) {
                canvas.player_rect_through_portal(rect, source, destination, fill, outline);
            } else if destination.opens_for_body(player.pos, player.half_size()) {
                canvas.player_rect_through_portal(rect, destination, source, fill, outline);
            } else if let Some((source, destination)) = active_world_portal_for_body(world) {
                canvas.player_rect_through_portal(rect, source, destination, fill, outline);
            } else {
                canvas.player_rect(rect, fill, outline);
            }
        } else if let Some((source, destination)) = active_world_portal_for_body(world) {
            canvas.player_rect_through_portal(rect, source, destination, fill, outline);
        } else {
            canvas.player_rect(rect, fill, outline);
        }

        if player.slam_storage_ready() {
            canvas.slam_storage_particles(player.pos, player.half_size());
        }
    }

    fn draw_aim(&self, canvas: &mut Canvas<'_>, world: &World) {
        let player = &world.player;
        let aim = canvas.world_to_screen(player.aim_pos);
        canvas.fill_rect(
            Rect {
                pos: aim - Vec2::splat(3.0),
                size: Vec2::splat(7.0),
            },
            Color::rgb(4, 8, 12),
        );
        canvas.fill_rect(
            Rect {
                pos: aim - Vec2::splat(1.0),
                size: Vec2::splat(3.0),
            },
            Color::rgb(255, 70, 86),
        );
    }
}

fn active_world_portal_for_body(world: &World) -> Option<(Portal, Portal)> {
    let player = &world.player;

    world
        .level
        .world_portals
        .iter()
        .enumerate()
        .find_map(|(source_index, source)| {
            if !source.portal.opens_for_body(player.pos, player.half_size()) {
                return None;
            }
            let destination_index =
                world_portal_receiver_index(&world.level.world_portals, source_index)?;
            let destination = world.level.world_portals.get(destination_index)?;

            Some((source.portal, destination.portal))
        })
}

fn world_portal_receiver_index(portals: &[WorldPortal], source_index: usize) -> Option<usize> {
    let source = portals.get(source_index)?;
    if source.seamless {
        let mut receivers = portals
            .iter()
            .enumerate()
            .filter(|(index, portal)| *index != source_index && portal.id == source.receiver_id)
            .map(|(index, _)| index);
        let receiver = receivers.next()?;

        return receivers.next().is_none().then_some(receiver);
    }

    portals
        .iter()
        .enumerate()
        .filter(|(index, portal)| *index != source_index && portal.id == source.receiver_id)
        .max_by_key(|(_, portal)| portal.priority)
        .map(|(index, _)| index)
}

impl Color {
    pub(super) fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_playing_frame_touches_pixels() {
        let renderer = Renderer::new();
        let world = World::new();
        let mut frame = vec![0; 320 * 180];

        renderer.draw(RenderFrame {
            frame: &mut frame,
            width: 320,
            height: 180,
            world: &world,
            mode: RenderMode::Playing,
            camera: world.player.pos,
            zoom: 0.5,
            debug: None,
            fps: None,
        });

        assert!(frame.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn draw_level_menu_frame_touches_pixels() {
        let renderer = Renderer::new();
        let world = World::new();
        let levels = vec![LevelSpec::fallback()];
        let mut frame = vec![0; 320 * 180];

        renderer.draw(RenderFrame {
            frame: &mut frame,
            width: 320,
            height: 180,
            world: &world,
            mode: RenderMode::LevelMenu {
                levels: &levels,
                selected: 0,
            },
            camera: world.player.pos,
            zoom: 0.5,
            debug: None,
            fps: None,
        });

        assert!(frame.iter().any(|pixel| *pixel != 0));
    }
}

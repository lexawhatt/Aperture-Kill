mod canvas;
mod glyphs;
mod text;
mod ui;
mod world;

// Renderer is a facade; Canvas owns low-level pixel drawing.
use glam::Vec2;

use crate::game::World;
use crate::game::levels::LevelSpec;
use crate::game::portal::Color;

use canvas::{Canvas, Rect};

pub struct Renderer;

pub enum RenderMode<'a> {
    Playing,
    LevelMenu {
        levels: &'a [LevelSpec],
        selected: usize,
    },
    Editor(EditorOverlay),
}

pub struct EditorOverlay {
    pub selected_solids: Vec<usize>,
    pub selected_doors: Vec<usize>,
    pub selected_hazards: Vec<usize>,
    pub selected_checkpoints: Vec<usize>,
    pub selected_texts: Vec<usize>,
    pub selection_count: usize,
    pub text_editing: bool,
    pub marquee: Option<(Vec2, Vec2)>,
    pub active_tool: usize,
    pub rotate_ui: bool,
    pub grid_snap: bool,
    pub dirty: bool,
    pub saved_flash: bool,
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

    pub fn draw(
        &self,
        frame: &mut [u32],
        width: u32,
        height: u32,
        world: &World,
        mode: RenderMode<'_>,
        camera: Vec2,
        zoom: f32,
        debug: Option<DebugOverlay>,
    ) {
        let mut canvas = Canvas::new(frame, width, height, camera, zoom);

        canvas.clear(Color::rgb(9, 10, 14));
        self.draw_world(&mut canvas, world);
        canvas.draw_dash_charges(world.player.dash_charges);

        match mode {
            RenderMode::Playing => {}
            RenderMode::LevelMenu { levels, selected } => {
                canvas.level_menu(levels, selected);
            }
            RenderMode::Editor(overlay) => {
                canvas.editor_overlay(world, overlay);
            }
        }

        if let Some(debug) = debug {
            canvas.debug_overlay(debug);
        }
    }

    fn draw_world(&self, canvas: &mut Canvas<'_>, world: &World) {
        // Static geometry is drawn first so portals and actors stay readable.
        for solid in &world.level.solids {
            let fill = if solid.portalable {
                Color::rgb(36, 42, 53)
            } else {
                Color::rgb(55, 45, 47)
            };
            canvas.solid(*solid, fill, Color::rgb(92, 105, 125));
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

        // Portals are visual-only here; physics is owned by World.
        for portal in world.portals.iter().flatten() {
            let (a, b) = portal.endpoints();
            canvas.draw_world_line(a, b, portal.color);
            canvas.draw_world_line(portal.pos, portal.pos + portal.normal * 16.0, portal.color);
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
        let fill = if player.ground_slamming {
            Color::rgb(255, 224, 102)
        } else if player.dashing {
            Color::rgb(80, 92, 130)
        } else if player.wall_sliding {
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
            } else {
                canvas.player_rect(rect, fill, outline);
            }
        } else {
            canvas.player_rect(rect, fill, outline);
        }

        if player.slam_storage_ready() {
            canvas.slam_storage_particles(player.pos, player.half_size());
        }
    }

    fn draw_aim(&self, canvas: &mut Canvas<'_>, world: &World) {
        let player = &world.player;
        canvas.draw_world_line(player.aim_from(), player.aim_pos, Color::rgb(255, 70, 86));
        canvas.fill_world_rect(
            Rect {
                pos: player.aim_pos - Vec2::splat(3.0),
                size: Vec2::splat(6.0),
            },
            Color::rgb(255, 70, 86),
        );
    }
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub(super) fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

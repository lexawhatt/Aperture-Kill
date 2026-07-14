use glam::Vec2;

// UI drawing stays in screen space.
use crate::constants::MAX_DASH_CHARGES;
use crate::game::World;
use crate::game::levels::LevelSpec;
use crate::game::portal::Color;

use super::canvas::{Canvas, Rect};
use super::{DebugOverlay, EditorOverlay};

impl Canvas<'_> {
    pub(super) fn draw_dash_charges(&mut self, charges: f32) {
        let start = Vec2::new(18.0, 18.0);
        let size = Vec2::new(42.0, 8.0);
        let gap = 7.0;

        // Fractional fill makes recharge timing visible without text.
        for index in 0..MAX_DASH_CHARGES as usize {
            let pos = start + Vec2::new(index as f32 * (size.x + gap), 0.0);
            let filled = (charges - index as f32).clamp(0.0, 1.0);
            let outline = Rect { pos, size };

            self.rect_outline(outline, Color::rgb(180, 190, 205));
            if filled > 0.0 {
                self.fill_rect(
                    Rect {
                        pos: pos + Vec2::splat(1.0),
                        size: Vec2::new((size.x - 2.0) * filled, size.y - 2.0),
                    },
                    Color::rgb(80, 190, 255),
                );
            }
        }
    }

    pub(super) fn level_menu(&mut self, levels: &[LevelSpec], selected: usize) {
        let panel = Rect {
            pos: Vec2::new(250.0, 110.0),
            size: Vec2::new(400.0, 310.0),
        };
        self.fill_rect(panel, Color::rgb(20, 24, 31));
        self.rect_outline(panel, Color::rgb(106, 124, 150));
        self.text(
            Vec2::new(280.0, 130.0),
            "LEVELS",
            3,
            Color::rgb(235, 238, 245),
        );
        self.text(
            Vec2::new(280.0, 380.0),
            "ENTER / CLICK LOAD",
            2,
            Color::rgb(150, 163, 184),
        );

        for (index, level) in levels.iter().enumerate() {
            self.level_menu_item(index, level, index == selected);
        }
    }

    pub(super) fn editor_overlay(&mut self, world: &World, overlay: EditorOverlay) {
        self.grid(16.0, Color::rgb(22, 27, 36));
        self.selected_solid_overlay(world, &overlay);
        self.selected_door_overlay(world, &overlay);
        self.selected_hazard_overlay(world, &overlay);
        self.selected_checkpoint_overlay(world, &overlay);
        self.selected_text_overlay(world, &overlay);
        self.marquee_overlay(&overlay);
        self.editor_panel(&overlay);
    }

    pub(super) fn debug_overlay(&mut self, debug: DebugOverlay) {
        let panel = Rect {
            pos: Vec2::new(14.0, 196.0),
            size: Vec2::new(260.0, 154.0),
        };

        self.fill_rect(panel, Color::rgb(12, 15, 20));
        self.rect_outline(panel, Color::rgb(255, 224, 102));
        self.text(
            panel.pos + Vec2::new(10.0, 10.0),
            "DEBUG F7",
            1,
            Color::rgb(255, 224, 102),
        );

        self.debug_line(1, &format!("MODE {}", debug.mode));
        self.debug_line(
            2,
            &format!(
                "POS X {} Y {}",
                debug.player_pos.x as i32, debug.player_pos.y as i32
            ),
        );
        self.debug_line(
            3,
            &format!(
                "VEL X {} Y {}",
                debug.player_vel.x as i32, debug.player_vel.y as i32
            ),
        );
        self.debug_line(
            4,
            &format!(
                "CAM X {} Y {} Z {}",
                debug.camera.x as i32,
                debug.camera.y as i32,
                (debug.zoom * 100.0) as i32
            ),
        );
        self.debug_line(
            5,
            &format!(
                "CUR X {} Y {}",
                debug.cursor_world.x as i32, debug.cursor_world.y as i32
            ),
        );
        self.debug_line(6, &format!("GROUND {}", yes_no(debug.on_ground)));
        self.debug_line(
            7,
            &format!(
                "SLIDE {} DASH {}",
                yes_no(debug.sliding),
                yes_no(debug.dashing)
            ),
        );
        self.debug_line(8, &format!("SLAM {}", yes_no(debug.slamming)));
        self.debug_line(
            9,
            &format!(
                "SOLIDS {} PORTALS {}",
                debug.solid_count, debug.portal_count
            ),
        );
    }

    fn selected_solid_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, solid) in world.level.solids.iter().enumerate() {
            if !overlay.selected_solids.contains(&index) {
                continue;
            }

            self.solid_outline(*solid, Color::rgb(255, 224, 102));
            if overlay.selection_count == 1 {
                self.resize_handles(*solid, Color::rgb(255, 224, 102));
                if overlay.rotate_ui {
                    self.rotate_handle(*solid, Color::rgb(255, 224, 102));
                }

                let label = if solid.portalable {
                    "PORTALABLE"
                } else {
                    "SOLID"
                };
                self.world_text(
                    solid.world_from_local(Vec2::ZERO) + Vec2::new(0.0, -18.0),
                    label,
                    2,
                    Color::rgb(255, 224, 102),
                );
            }
        }

        if overlay.selection_count > 1 {
            let selected = overlay
                .selected_solids
                .iter()
                .filter_map(|index| world.level.solids.get(*index))
                .copied()
                .collect::<Vec<_>>();
            if let Some((min, max)) = solids_bounds(&selected) {
                self.world_rect_outline(
                    Rect {
                        pos: min,
                        size: max - min,
                    },
                    Color::rgb(107, 221, 144),
                );
                self.world_text(
                    min + Vec2::new(0.0, -18.0),
                    &format!("{} SELECTED", overlay.selection_count),
                    2,
                    Color::rgb(107, 221, 144),
                );
            }
        }
    }

    fn selected_door_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, door) in world.level.doors.iter().enumerate() {
            if !overlay.selected_doors.contains(&index) {
                continue;
            }

            self.solid_outline(door.solid, Color::rgb(107, 221, 144));
            if overlay.selection_count == 1 {
                self.resize_handles(door.solid, Color::rgb(107, 221, 144));
                self.world_text(
                    door.solid.world_from_local(Vec2::ZERO) + Vec2::new(0.0, -18.0),
                    "AUTO DOOR",
                    2,
                    Color::rgb(107, 221, 144),
                );
                self.trigger_ring(door.solid.center(), door.trigger_radius);
            }
        }
    }

    fn selected_hazard_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, hazard) in world.level.hazards.iter().enumerate() {
            if !overlay.selected_hazards.contains(&index) {
                continue;
            }

            self.solid_outline(hazard.solid, Color::rgb(124, 255, 120));
            if overlay.selection_count == 1 {
                self.resize_handles(hazard.solid, Color::rgb(124, 255, 120));
                self.world_text(
                    hazard.solid.world_from_local(Vec2::ZERO) + Vec2::new(0.0, -18.0),
                    "ACID",
                    2,
                    Color::rgb(124, 255, 120),
                );
            }
        }
    }

    fn selected_checkpoint_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, checkpoint) in world.level.checkpoints.iter().enumerate() {
            if !overlay.selected_checkpoints.contains(&index) {
                continue;
            }

            self.solid_outline(checkpoint.solid, Color::rgb(80, 190, 255));
            if overlay.selection_count == 1 {
                self.resize_handles(checkpoint.solid, Color::rgb(80, 190, 255));
                self.world_text(
                    checkpoint.solid.world_from_local(Vec2::ZERO) + Vec2::new(0.0, -18.0),
                    "CHECKPOINT",
                    2,
                    Color::rgb(80, 190, 255),
                );
            }
        }
    }

    fn selected_text_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, text) in world.level.texts.iter().enumerate() {
            if !overlay.selected_texts.contains(&index) {
                continue;
            }

            let rect = Rect {
                pos: text.pos,
                size: text_size(&text.text),
            };
            let color = if overlay.text_editing {
                Color::rgb(80, 190, 255)
            } else {
                Color::rgb(255, 224, 102)
            };

            self.world_rect_outline(rect, color);
            if overlay.text_editing {
                self.world_text(
                    text.pos + Vec2::new(0.0, -18.0),
                    "EDIT TEXT",
                    2,
                    Color::rgb(80, 190, 255),
                );
            }
        }
    }

    fn marquee_overlay(&mut self, overlay: &EditorOverlay) {
        let Some((pos, size)) = overlay.marquee else {
            return;
        };
        if size.length_squared() < 4.0 {
            return;
        }

        self.world_rect_outline(Rect { pos, size }, Color::rgb(80, 190, 255));
    }

    fn editor_panel(&mut self, overlay: &EditorOverlay) {
        let panel = Rect {
            pos: Vec2::new(14.0, 64.0),
            size: Vec2::new(382.0, 194.0),
        };
        let title = if overlay.dirty {
            "EDITOR UNSAVED"
        } else {
            "EDITOR"
        };

        self.fill_rect(panel, Color::rgb(16, 19, 25));
        self.rect_outline(panel, Color::rgb(92, 105, 125));
        self.text(Vec2::new(26.0, 76.0), title, 2, Color::rgb(245, 247, 250));
        self.editor_tool_tabs(overlay.active_tool);
        self.text(
            Vec2::new(26.0, 120.0),
            "DRAG MOVE / EDGE RESIZE",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 136.0),
            "SHIFT CLICK / DRAG EMPTY SELECT",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 152.0),
            "3 DOOR / 4 TEXT / 5 ACID / 6 CHECK",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 168.0),
            "CTRL DRAG MOVE SELECTED",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 184.0),
            "CTRL A C X V D Z / DEL",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 200.0),
            grid_mode_text(overlay.grid_snap),
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 216.0),
            "P TYPE / G SPAWN / R ROTATE",
            1,
            Color::rgb(180, 190, 205),
        );
        self.text(
            Vec2::new(26.0, 232.0),
            "1 SOLID / 2 PORTALABLE / F5 SAVE",
            1,
            Color::rgb(180, 190, 205),
        );

        if overlay.saved_flash {
            self.text(
                Vec2::new(310.0, 76.0),
                "SAVED",
                1,
                Color::rgb(107, 221, 144),
            );
        }
    }

    fn level_menu_item(&mut self, index: usize, level: &LevelSpec, selected: bool) {
        let y = 150.0 + index as f32 * 42.0;
        let item = Rect {
            pos: Vec2::new(280.0, y),
            size: Vec2::new(340.0, 34.0),
        };
        let fill = if selected {
            Color::rgb(54, 139, 255)
        } else {
            Color::rgb(32, 38, 49)
        };

        self.fill_rect(item, fill);
        self.rect_outline(item, Color::rgb(116, 132, 155));
        self.text(
            item.pos + Vec2::new(12.0, 9.0),
            &level.name,
            2,
            Color::rgb(245, 247, 250),
        );
    }

    fn editor_tool_tabs(&mut self, active_tool: usize) {
        let solid = Rect {
            pos: Vec2::new(26.0, 96.0),
            size: Vec2::new(76.0, 16.0),
        };
        let portalable = Rect {
            pos: Vec2::new(110.0, 96.0),
            size: Vec2::new(130.0, 16.0),
        };
        let solid_active = active_tool == 1;
        let portalable_active = active_tool == 2;

        self.tool_tab(solid, "1 SOLID", solid_active, Color::rgb(255, 224, 102));
        self.tool_tab(
            portalable,
            "2 PORTALABLE",
            portalable_active,
            Color::rgb(54, 139, 255),
        );
    }

    fn tool_tab(&mut self, rect: Rect, label: &str, active: bool, active_fill: Color) {
        let fill = if active {
            active_fill
        } else {
            Color::rgb(32, 38, 49)
        };
        let text = if active {
            Color::rgb(16, 19, 25)
        } else {
            Color::rgb(180, 190, 205)
        };

        self.fill_rect(rect, fill);
        self.rect_outline(rect, Color::rgb(92, 105, 125));
        self.text(rect.pos + Vec2::new(5.0, 4.0), label, 1, text);
    }

    fn debug_line(&mut self, row: usize, text: &str) {
        self.text(
            Vec2::new(24.0, 206.0 + row as f32 * 14.0),
            text,
            1,
            Color::rgb(180, 190, 205),
        );
    }

    fn trigger_ring(&mut self, center: Vec2, radius: f32) {
        let mut previous = center + Vec2::new(radius, 0.0);

        for step in 1..=48 {
            let angle = step as f32 / 48.0 * std::f32::consts::TAU;
            let next = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            self.draw_world_line(previous, next, Color::rgb(80, 190, 255));
            previous = next;
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "YES" } else { "NO" }
}

fn grid_mode_text(grid_snap: bool) -> &'static str {
    if grid_snap {
        "H GRID MODE"
    } else {
        "H FREE MODE"
    }
}

fn solids_bounds(solids: &[crate::game::level::Solid]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let mut any = false;

    for solid in solids {
        for corner in solid.corners() {
            min = min.min(corner);
            max = max.max(corner);
            any = true;
        }
    }

    any.then_some((min, max))
}

fn text_size(text: &str) -> Vec2 {
    Vec2::new((text.chars().count().max(1) as f32 * 12.0).max(24.0), 14.0)
}

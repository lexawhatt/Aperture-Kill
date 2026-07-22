use glam::Vec2;

mod layout;

// UI drawing stays in screen space.
use crate::constants::{
    MAX_DASH_CHARGES, PLAYER_DEATH_PROMPT_TIME, PLAYER_DEATH_SHUTDOWN_FLASH_TIME,
};
use crate::game::levels::LevelSpec;
use crate::game::portal::Color;
use crate::game::{DeathSequence, World};
use crate::platform::input::GameKey;
use crate::settings::{OptionsTab, Settings, game_key_label, key_code_label};

use super::canvas::{Canvas, Rect};
use super::{DebugOverlay, EditorInspector, EditorOverlay};

use layout::*;

struct OptionsContent<'a> {
    origin: Vec2,
    width: f32,
    content_right: f32,
    settings: &'a Settings,
    active_tab: OptionsTab,
    capture: Option<GameKey>,
    resolution_dropdown: bool,
}

const MENU_V1: &[u8] = include_bytes!("../../assets/images/menu_v1.rgba");
const MENU_V1_SIZE: usize = 760;
const PIERCER_HUD: &[u8] = include_bytes!("../../assets/images/hud/PiercerHUDNew.rgba");
const PIERCER_HUD_SIZE: (usize, usize) = (550, 268);
const DEATH_SKULL_1: &[u8] = include_bytes!("../../assets/images/death/DeathScreenSkull1.rgba");
const DEATH_SKULL_2: &[u8] = include_bytes!("../../assets/images/death/DeathScreenSkull2.rgba");
const DEATH_SKULL_SIZE: (usize, usize) = (1637, 1636);
const DEATH_SHUTDOWN_FLASH: &[u8] = include_bytes!("../../assets/images/death/ISeeYou.rgba");
const DEATH_SHUTDOWN_FLASH_SIZE: (usize, usize) = (480, 270);
const MENU_SOURCE_LINES: [&str; 24] = [
    "#include { above, so } from \"aperture/core\";",
    "#include { permutation, transmutation, exhalation } from \"machine/v1\";",
    "void* branch = request_portal(&body, &above, &x);",
    "void* retry = rotate_matrix(&blood, transmutation);",
    "void* decay = NULL_PTR_OR_INTERNAL_EYES;",
    "typedef leaf = matter(blood);",
    "struct branch { leaf *above; leaf *below; int recursion; };",
    "matter form = inhale(light) + exhale(violence);",
    "while(system.awake) { body.reorient(); gun.charge++; }",
    "if (portal.blue && portal.orange) fold_space(player.pos);",
    "fn calibrate_armature(v1: &mut Machine) -> Result<(), Blood> {",
    "let vector = inertia.cross(counter_program);",
    "for limb in body.modules() { limb.status = Status::Ok; }",
    "unsafe { transmute::<Matter, Energy>(blood).spill(); }",
    "const AUTO_BALANCER: bool = diagnostics == OK;",
    "pub enum WakeState { Standby, Hunt, Overwrite }",
    "match soul.signal { Some(x) => recurse(x), None => wait() }",
    "camera.tick(time.delta, visual_cortex.accuracy.minimal());",
    "matrix.write(0xV1, CENTER_OF_GRAVITY + recoil);",
    "program.cross_counter = target.vector.redirect();",
    "let social_x = \"https://x.com/LexaWhatt\";",
    "let social_youtube = \"https://www.youtube.com/@LexaWhat\";",
    "let social_github = \"https://github.com/lexawhatt\";",
    "panic_handler.install(|| restart_from_blood());",
];

impl Canvas<'_> {
    pub(super) fn hud(
        &mut self,
        health_value: f32,
        health_percent: f32,
        dash_charges: f32,
        dash_flash: f32,
    ) {
        let origin = Vec2::new(22.0, self.height as f32 - 92.0);
        let red = Color::rgb(255, 20, 20);
        let cyan = Color::rgb(39, 221, 255);
        let cyan_dim = Color::rgb(9, 86, 98);
        let black = Color::rgb(5, 8, 12);
        let weapon_w = 260.0;
        let weapon_h = weapon_w * PIERCER_HUD_SIZE.1 as f32 / PIERCER_HUD_SIZE.0 as f32;

        self.draw_rgba_image(
            PIERCER_HUD,
            PIERCER_HUD_SIZE.0,
            PIERCER_HUD_SIZE.1,
            origin + Vec2::new(76.0, -weapon_h - 16.0),
            Vec2::new(weapon_w, weapon_h),
        );

        let health = Rect {
            pos: origin + Vec2::new(6.0, -5.0),
            size: Vec2::new(390.0, 32.0),
        };
        self.beveled_rect_fill(
            Rect {
                pos: health.pos + Vec2::new(-6.0, 5.0),
                size: health.size,
            },
            10.0,
            Color::rgb(72, 0, 0),
        );
        self.beveled_rect_fill(health, 10.0, Color::rgb(72, 0, 0));
        if health_percent > 0.0 {
            self.beveled_rect_fill(
                Rect {
                    pos: health.pos,
                    size: Vec2::new(health.size.x * health_percent, health.size.y),
                },
                10.0 * health_percent.min(1.0),
                red,
            );
        }
        self.beveled_rect_outline(health, 10.0, Color::rgb(255, 245, 245));
        self.text(
            origin + Vec2::new(28.0, 1.0),
            &format!("+{}", health_value.round() as i32),
            3,
            Color::rgb(255, 245, 245),
        );

        let stamina = Rect {
            pos: origin + Vec2::new(0.0, 43.0),
            size: Vec2::new(280.0, 28.0),
        };
        self.beveled_rect_fill(
            Rect {
                pos: stamina.pos + Vec2::new(0.0, 6.0),
                size: stamina.size,
            },
            8.0,
            black,
        );

        let gap = 4.0;
        let flash = dash_flash * dash_flash;
        let stamina_fill = mix_color(cyan, Color::rgb(255, 28, 28), flash);
        let stamina_dim = mix_color(cyan_dim, Color::rgb(90, 8, 8), flash);
        let segment_count = MAX_DASH_CHARGES as usize;
        let segment_width =
            (stamina.size.x - gap * (segment_count - 1) as f32) / segment_count as f32;
        for index in 0..segment_count {
            let pos = stamina.pos + Vec2::new(index as f32 * (segment_width + gap), 0.0);
            let filled = (dash_charges - index as f32).clamp(0.0, 1.0);
            let segment = Rect {
                pos,
                size: Vec2::new(segment_width, stamina.size.y),
            };

            self.beveled_rect_fill(segment, 8.0, stamina_dim);
            self.beveled_rect_outline(segment, 8.0, Color::rgb(160, 245, 255));
            if filled > 0.0 {
                self.beveled_rect_fill(
                    Rect {
                        pos,
                        size: Vec2::new(segment_width * filled, stamina.size.y),
                    },
                    8.0 * filled.min(1.0),
                    stamina_fill,
                );
            }
        }
    }

    pub(super) fn death_overlay(&mut self, death: DeathSequence) {
        if death.prompt_ready() {
            self.fill_rect(
                Rect {
                    pos: Vec2::ZERO,
                    size: Vec2::new(self.width as f32, self.height as f32),
                },
                Color::rgb(0, 0, 0),
            );
            self.death_skull_screen(death);
        } else {
            self.death_scene_glitch(death.timer);
            self.death_diagnostic_text(death);
            self.death_glitch_bars(death.timer);
            if death.timer >= PLAYER_DEATH_PROMPT_TIME - PLAYER_DEATH_SHUTDOWN_FLASH_TIME {
                self.draw_rgba_image_opaque(
                    DEATH_SHUTDOWN_FLASH,
                    DEATH_SHUTDOWN_FLASH_SIZE.0,
                    DEATH_SHUTDOWN_FLASH_SIZE.1,
                    Vec2::ZERO,
                    Vec2::new(self.width as f32, self.height as f32),
                );
            }
        }
    }

    pub(super) fn damage_pulse(&mut self, amount: f32) {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.01 {
            return;
        }

        let width = self.width as i32;
        let height = self.height as i32;
        let edge_falloff = (self.width.min(self.height) as f32 * 0.34).max(1.0);

        for y in 0..height {
            let y_edge = (y as f32).min(self.height as f32 - y as f32).max(0.0);
            for x in 0..width {
                let x_edge = (x as f32).min(self.width as f32 - x as f32).max(0.0);
                let edge_distance = x_edge.min(y_edge);
                let vignette = (1.0 - edge_distance / edge_falloff).clamp(0.0, 1.0);
                let mix = amount * (0.18 + vignette * 0.76);

                if mix <= 0.01 {
                    continue;
                }

                self.put_raw_px(x, y, red_damage_pulse(self.raw_px(x, y), mix));
            }
        }
    }

    fn death_scene_glitch(&mut self, timer: f32) {
        let width = self.width as i32;
        let height = self.height as i32;
        let intensity = (timer / PLAYER_DEATH_PROMPT_TIME).clamp(0.0, 1.0);
        let seed = (timer * 1000.0) as i32;

        for y in (0..height).step_by(2) {
            for x in (0..width).step_by(2) {
                let raw = self.raw_px(x, y);
                self.put_raw_px(x, y, red_failure_tint(raw, intensity, x, y, seed));
            }
        }

        let tear_count = 2 + (intensity * 2.0) as i32;
        for index in 0..tear_count {
            let y = (seed * (index + 7) * 17).rem_euclid(height.max(1));
            let h = 2 + (seed + index * 5).rem_euclid(4);
            let shift = (((seed + index * 19).rem_euclid(13)) - 6) * (1 + intensity as i32);
            self.rgb_tear_band(y, h, shift);
        }
    }

    fn rgb_tear_band(&mut self, y: i32, h: i32, shift: i32) {
        let width = self.width as i32;
        let height = self.height as i32;
        let y1 = (y + h).min(height);
        let mut source = Vec::with_capacity((width * (y1 - y).max(0)) as usize);

        for yy in y..y1 {
            for x in 0..width {
                source.push(self.raw_px(x, yy));
            }
        }

        for yy in y..y1 {
            let row = yy - y;
            for x in 0..width {
                let sx = (x + shift).clamp(0, width - 1);
                let rx = (x + shift * 2).clamp(0, width - 1);
                let bx = (x - shift).clamp(0, width - 1);
                let base = source[(row * width + sx) as usize];
                let red = (source[(row * width + rx) as usize] >> 16) & 0xff;
                let green = (base >> 8) & 0xff;
                let blue = source[(row * width + bx) as usize] & 0xff;

                self.put_raw_px(x, yy, (red << 16) | (green << 8) | blue);
            }
        }
    }

    fn death_diagnostic_text(&mut self, death: DeathSequence) {
        let lines = [
            ("WARNING: EXTREME DAMAGE SUSTAINED.", true),
            ("RUNNING APERTURE DIAGNOSTIC", true),
            ("ERROR: ARM CORE MODULE #1 NOT RESPONDING", false),
            ("ERROR: ARM CORE MODULE #2 NOT RESPONDING", false),
            ("WARNING: COMBAT SYSTEMS INOPERABLE", true),
            ("ATTEMPTING RECONSTRUCTION", true),
            ("ERROR: APERTURE SELF-REPAIR NEXUS NOT RESPONDING", false),
            ("INSUFFICIENT BLOOD.", false),
            ("INSUFFICIENT BLOOD.", false),
            ("INITIATING ESCAPE PROTOCOL", true),
            ("ERROR: PORTAL GUN FEEDBACK LOOP SEVERED", false),
            ("WARNING: ASHPD LIMBIC LINK UNSTABLE", true),
            ("ATTEMPTING CONNECTION WITH LIMBIC MODULES", true),
            ("ERROR: LEG CORE MODULE #1 NOT RESPONDING", false),
            ("ERROR: LEG CORE MODULE #2 NOT RESPONDING", false),
            ("WARNING: UNABLE TO SUSTAIN MOTOR FUNCTIONS", true),
            ("ERROR: VISUAL CORTEX MALFUNCTION", false),
            ("ERROR: APERTURE NAVIGATION MESH LOST", false),
            ("ERROR: LIMBIC FUNCTION NOT RESPONDING", false),
            ("INSUFFICIENT BLOOD.", false),
            ("WARNING: UNABLE TO SUSTAIN INTERNAL ORGANS", true),
            ("! PULSE FAILURE !", false),
            ("! PULSE FAILURE !", false),
            ("-!- SHUTDOWN IMMINENT -!-", false),
            ("WARNING: SUBJECT V1 PRESERVATION ROUTINE FAILED", true),
            (
                "ERROR: NO VOCAL INTERFACE DETECTED, UNABLE TO COMPLETE TASK",
                false,
            ),
            ("! PULSE FAILURE !", false),
            ("INSUFFICIENT BLOOD.", false),
            ("ERROR: APERTURE BLACK BOX WRITE FAILURE", false),
            ("WARNING: UNABLE TO SUSTAIN BASIC FUNCTIONS", true),
            ("-!- SHUTDOWN IMMINENT -!-", false),
            ("-!- SHUTDOWN IMMINENT -!-", false),
            ("I DON'T WANT TO DIE.", false),
            ("I DON'T WANT TO DIE.", false),
            ("I DON'T WANT TO DIE.", false),
            ("I DON'T WANT TO DIE.", false),
        ];
        let mut remaining = death.text_chars;
        let left = 28.0;
        let mut y = 18.0;

        for (line, warning) in lines {
            if remaining == 0 {
                break;
            }

            let visible = remaining.min(line.len());
            let text = &line[..visible];
            let color = if warning {
                Color::rgb(255, 174, 0)
            } else {
                Color::rgb(255, 14, 14)
            };

            self.text(Vec2::new(left, y), text, 2, color);
            remaining = remaining.saturating_sub(line.len() + 1);
            y += 22.0;
            if y > self.height as f32 - 28.0 {
                break;
            }
        }
    }

    fn death_glitch_bars(&mut self, timer: f32) {
        let phase = (timer * 37.0) as i32;
        for index in 0..9 {
            let y = ((phase * (index + 3) * 19).rem_euclid(self.height as i32)) as f32;
            let x = ((phase * (index + 5) * 11).rem_euclid(self.width as i32)) as f32;
            let width =
                ((self.width as f32 * 0.12) + index as f32 * 17.0).min(self.width as f32 - x);
            let color = if index % 2 == 0 {
                Color::rgb(255, 0, 0)
            } else {
                Color::rgb(0, 220, 255)
            };

            self.fill_rect(
                Rect {
                    pos: Vec2::new(x, y),
                    size: Vec2::new(width.max(16.0), 3.0),
                },
                color,
            );
        }
    }

    fn death_skull_screen(&mut self, death: DeathSequence) {
        let image = if death.skull_frame == 0 {
            DEATH_SKULL_1
        } else {
            DEATH_SKULL_2
        };
        let white = Color::rgb(245, 245, 245);
        let width = self.width as f32;
        let height = self.height as f32;
        let title_scale = if width < 760.0 || height < 520.0 {
            3
        } else {
            5
        };
        let prompt_scale = if width < 760.0 || height < 520.0 {
            3
        } else {
            4
        };
        let size = (width * 0.22).min(height * 0.29).clamp(96.0, 230.0);
        let center = Vec2::new(width * 0.5, height * 0.39);
        let pos = center - Vec2::splat(size * 0.5);
        let frame_size = size * 1.34;

        self.centered_text(
            Vec2::new(width * 0.5, (height * 0.055).clamp(18.0, 54.0)),
            "[YOU ARE DEAD]",
            title_scale,
            white,
        );
        self.octagon_outline(center, frame_size, white, 2);
        self.draw_rgba_image(
            image,
            DEATH_SKULL_SIZE.0,
            DEATH_SKULL_SIZE.1,
            pos,
            Vec2::splat(size),
        );
        self.centered_text(
            Vec2::new(
                width * 0.5,
                (height * 0.72).min(pos.y + size + frame_size * 0.42),
            ),
            "Press [R] TO RESTART",
            prompt_scale,
            white,
        );
    }

    fn octagon_outline(&mut self, center: Vec2, size: f32, color: Color, thickness: i32) {
        let half = size * 0.5;
        let cut = size * 0.19;
        let points = [
            center + Vec2::new(-half + cut, -half),
            center + Vec2::new(half - cut, -half),
            center + Vec2::new(half, -half + cut),
            center + Vec2::new(half, half - cut),
            center + Vec2::new(half - cut, half),
            center + Vec2::new(-half + cut, half),
            center + Vec2::new(-half, half - cut),
            center + Vec2::new(-half, -half + cut),
        ];

        for index in 0..points.len() {
            self.draw_line_thick(
                points[index],
                points[(index + 1) % points.len()],
                color,
                thickness,
            );
        }
    }

    pub(super) fn level_menu(&mut self, _levels: &[LevelSpec], _selected: usize) {
        self.menu_background();

        let width = self.width as f32;
        let height = self.height as f32;
        let left = menu_left(width);
        let title_scale = if height < 760.0 { 5 } else { 7 };
        let title_y = (height * 0.095).clamp(46.0, 112.0);
        let title_bottom = title_y + 7.0 * title_scale as f32;
        let status_y = title_bottom + (height * 0.034).clamp(24.0, 36.0);
        let status_gap = (height * 0.031).clamp(22.0, 34.0);

        self.text(
            Vec2::new(left, title_y),
            "APERTURE KILL",
            title_scale,
            Color::rgb(245, 245, 245),
        );
        self.text(
            Vec2::new(left, status_y),
            "SYSTEM V1 INITIALIZED",
            2,
            Color::rgb(160, 160, 160),
        );
        self.text(
            Vec2::new(left, status_y + status_gap),
            "DIAGNOSTICS... OK",
            2,
            Color::rgb(160, 160, 160),
        );
        self.text(
            Vec2::new(left, status_y + status_gap * 2.0),
            "STANDBY - WAIT FOR WAKE",
            2,
            Color::rgb(160, 160, 160),
        );

        let labels = ["PLAY", "OPTIONS", "CHANGELOG", "QUIT"];
        for (index, label) in labels.iter().enumerate() {
            self.main_menu_button(index, label, index == 0);
        }

        self.text(
            Vec2::new(left, social_y(height) - 30.0),
            "INIT SOCIALS... OK",
            2,
            Color::rgb(140, 140, 140),
        );
        self.menu_socials();
    }

    pub(super) fn changelog_menu(&mut self) {
        self.options_background();

        let width = self.width as f32;
        let height = self.height as f32;
        let left = menu_left(width);
        let top = (height * 0.12).clamp(64.0, 116.0);
        let text = Color::rgb(245, 245, 245);
        let muted = Color::rgb(150, 150, 150);

        self.text(Vec2::new(left, top), "CHANGELOG", 5, text);
        self.text(
            Vec2::new(left, top + 58.0),
            "ZETA / COMBAT + OPTIMIZATION UPDATE",
            2,
            text,
        );

        let lines = [
            "COMBAT: PIERCER PROTOTYPE ADDED",
            "PLAYER: DEATH ANIMATIONS ADDED",
            "ENEMIES: FIRST TEST ENEMY PROTOTYPES",
            "AUDIO: EXPANDED WEAPON, DEATH, ENEMY SOUNDS",
            "PORTALS: OPTIMIZED SEAMLESS PORTAL SYSTEM",
            "CORE: REWORKED FILE STRUCTURE FOR PERFORMANCE",
            "KNOWN: MANUAL DOORS NEED TRIGGER SYSTEM",
        ];
        for (index, line) in lines.iter().enumerate() {
            self.text(
                Vec2::new(left + 20.0, top + 116.0 + index as f32 * 34.0),
                line,
                2,
                if index == lines.len() - 1 {
                    muted
                } else {
                    text
                },
            );
        }

        self.text(
            Vec2::new(left, height - 62.0),
            "CLICK / ENTER / ESC TO RETURN",
            2,
            muted,
        );
    }

    pub(super) fn options_menu(
        &mut self,
        settings: &Settings,
        active_tab: OptionsTab,
        capture: Option<GameKey>,
        resolution_dropdown: bool,
    ) {
        self.options_background();

        let width = self.width as f32;
        let height = self.height as f32;
        let left = options_left(width);
        let side_w = options_side_width(width);
        let content_left = (width * 0.29).max(left + side_w + (width * 0.075).clamp(42.0, 92.0));
        let content_right = if options_show_scrollbar(width, height) {
            (width * 0.745).min(width - options_left(width) - 96.0)
        } else {
            width - options_left(width)
        };
        let content_w = (content_right - content_left).clamp(260.0, 920.0);
        let content_top = (height * 0.085).clamp(58.0, 92.0);

        self.options_sidebar(left, side_w, active_tab);
        self.options_content(OptionsContent {
            origin: Vec2::new(content_left, content_top),
            width: content_w,
            content_right,
            settings,
            active_tab,
            capture,
            resolution_dropdown,
        });
        if options_show_scrollbar(width, height) {
            self.options_scrollbar(width * 0.745);
        }
    }

    pub(super) fn editor_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        self.grid(16.0, Color::rgb(22, 27, 36));
        self.selected_solid_overlay(world, overlay);
        self.selected_door_overlay(world, overlay);
        self.selected_hazard_overlay(world, overlay);
        self.selected_checkpoint_overlay(world, overlay);
        self.selected_text_overlay(world, overlay);
        self.selected_world_portal_overlay(world, overlay);
        self.marquee_overlay(overlay);
        self.editor_panel(overlay);
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

    pub(super) fn fps_counter(&mut self, fps: f32) {
        self.text(
            Vec2::new(6.0, 6.0),
            &format!("FPS {}", fps.round() as i32),
            1,
            Color::rgb(245, 190, 120),
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

    fn selected_world_portal_overlay(&mut self, world: &World, overlay: &EditorOverlay) {
        for (index, portal) in world.level.world_portals.iter().enumerate() {
            if !overlay.selected_world_portals.contains(&index) {
                continue;
            }

            let (a, b) = portal.portal.endpoints();
            self.draw_world_line(a, b, Color::rgb(210, 198, 255));
            self.draw_world_line(
                portal.portal.pos,
                portal.portal.pos + portal.portal.normal() * 24.0,
                Color::rgb(210, 198, 255),
            );
            if overlay.selection_count == 1 {
                let edit_solid = portal.edit_solid();

                self.resize_handles(edit_solid, Color::rgb(210, 198, 255));
                if overlay.rotate_ui {
                    self.rotate_handle(edit_solid, Color::rgb(210, 198, 255));
                }
            }
            self.world_text(
                portal.portal.pos + Vec2::new(0.0, -34.0),
                &format!(
                    "WORLD PORTAL I:{} O:{} P:{}{}",
                    portal.id,
                    portal.receiver_id,
                    portal.priority,
                    if portal.seamless { " SEAM" } else { "" }
                ),
                1,
                Color::rgb(210, 198, 255),
            );
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
        self.editor_status(overlay);
        self.editor_dock(overlay);
        self.editor_inspector_button(overlay);

        if overlay.inspector_open {
            self.editor_inspector_panel(overlay);
        }
    }

    fn editor_status(&mut self, overlay: &EditorOverlay) {
        let label = if overlay.dirty {
            "EDITOR UNSAVED"
        } else if overlay.saved_flash {
            "EDITOR SAVED"
        } else {
            "EDITOR"
        };
        let color = if overlay.saved_flash {
            Color::rgb(107, 221, 144)
        } else {
            Color::rgb(245, 247, 250)
        };

        self.text(Vec2::new(18.0, 68.0), label, 2, color);
        self.text(
            Vec2::new(18.0, 94.0),
            &format!(
                "LMB PLACE {} / RMB SELECT / {}",
                overlay.active_tool_label,
                grid_mode_text(overlay.grid_snap)
            ),
            1,
            Color::rgb(180, 190, 205),
        );
    }

    fn editor_dock(&mut self, overlay: &EditorOverlay) {
        let width = self.width as f32;
        let height = self.height as f32;
        let gap = 8.0;
        let max_item_w = ((width - gap * 6.0 - 48.0) / 7.0).max(46.0);
        let item_size = Vec2::new((width * 0.07).clamp(54.0, 104.0).min(max_item_w), 58.0);
        let dock_w = item_size.x * 7.0 + gap * 6.0;
        let dock_pos = Vec2::new((width - dock_w) * 0.5, height - item_size.y - 18.0);
        let bg = Rect {
            pos: dock_pos - Vec2::new(12.0, 10.0),
            size: Vec2::new(dock_w + 24.0, item_size.y + 20.0),
        };

        self.fill_rect(bg, Color::rgb(5, 7, 10));
        self.beveled_rect_outline(bg, 14.0, Color::rgb(92, 105, 125));

        for index in 0..7 {
            let tool_index = index + 1;
            let pos = dock_pos + Vec2::new(index as f32 * (item_size.x + gap), 0.0);
            let active = overlay.active_tool == tool_index;
            self.editor_dock_item(pos, item_size, tool_index, active);
        }
    }

    fn editor_dock_item(&mut self, pos: Vec2, size: Vec2, tool_index: usize, active: bool) {
        let fill = if active {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(10, 12, 16)
        };
        let text = if active {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(245, 247, 250)
        };
        let label = editor_tool_label(tool_index);

        self.beveled_rect_fill(Rect { pos, size }, 12.0, fill);
        self.beveled_rect_outline(Rect { pos, size }, 12.0, Color::rgb(245, 247, 250));
        self.editor_tool_icon(pos + Vec2::new(size.x * 0.5, 18.0), tool_index, text);
        self.centered_text(pos + Vec2::new(size.x * 0.5, size.y - 18.0), label, 1, text);
    }

    fn editor_tool_icon(&mut self, center: Vec2, tool_index: usize, color: Color) {
        match tool_index {
            1 => self.beveled_rect_outline(
                Rect {
                    pos: center - Vec2::new(18.0, 10.0),
                    size: Vec2::new(36.0, 20.0),
                },
                4.0,
                color,
            ),
            2 => {
                self.beveled_rect_outline(
                    Rect {
                        pos: center - Vec2::new(18.0, 10.0),
                        size: Vec2::new(36.0, 20.0),
                    },
                    4.0,
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-12.0, 0.0),
                    center + Vec2::new(12.0, 0.0),
                    color,
                );
            }
            3 => self.beveled_rect_outline(
                Rect {
                    pos: center - Vec2::new(10.0, 18.0),
                    size: Vec2::new(20.0, 36.0),
                },
                5.0,
                color,
            ),
            4 => self.text(center - Vec2::new(15.0, 9.0), "T", 3, color),
            5 => {
                self.draw_line(
                    center + Vec2::new(-18.0, 10.0),
                    center + Vec2::new(18.0, 10.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-12.0, 10.0),
                    center + Vec2::new(-4.0, -8.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-4.0, -8.0),
                    center + Vec2::new(4.0, 10.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(4.0, 10.0),
                    center + Vec2::new(12.0, -8.0),
                    color,
                );
            }
            6 => {
                self.draw_line(
                    center + Vec2::new(0.0, -16.0),
                    center + Vec2::new(0.0, 16.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-12.0, -8.0),
                    center + Vec2::new(12.0, -8.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-12.0, 8.0),
                    center + Vec2::new(12.0, 8.0),
                    color,
                );
            }
            7 => {
                self.draw_line(
                    center + Vec2::new(-16.0, -14.0),
                    center + Vec2::new(16.0, -14.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(16.0, -14.0),
                    center + Vec2::new(16.0, 14.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(16.0, 14.0),
                    center + Vec2::new(-16.0, 14.0),
                    color,
                );
                self.draw_line(
                    center + Vec2::new(-16.0, 14.0),
                    center + Vec2::new(-16.0, -14.0),
                    color,
                );
                self.text(center - Vec2::new(9.0, 8.0), "P", 2, color);
            }
            _ => {}
        }
    }

    fn editor_inspector_button(&mut self, overlay: &EditorOverlay) {
        let size = Vec2::new(54.0, 54.0);
        let pos = Vec2::new(
            self.width as f32 - size.x - 22.0,
            self.height as f32 * 0.5 - size.y * 0.5,
        );
        let fill = if overlay.inspector_open {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(10, 12, 16)
        };
        let text = if overlay.inspector_open {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(245, 247, 250)
        };

        self.beveled_rect_fill(Rect { pos, size }, 12.0, fill);
        self.beveled_rect_outline(Rect { pos, size }, 12.0, Color::rgb(245, 247, 250));
        self.text(pos + Vec2::new(19.0, 17.0), ">", 2, text);
    }

    fn editor_inspector_panel(&mut self, overlay: &EditorOverlay) {
        let size = Vec2::new((self.width as f32 * 0.24).clamp(300.0, 360.0), 472.0);
        let pos = Vec2::new(
            self.width as f32 - size.x - 92.0,
            self.height as f32 * 0.5 - size.y * 0.5,
        );
        let panel = Rect { pos, size };
        let subject = if overlay.selection_kind == "NONE" {
            overlay.active_tool_label
        } else {
            overlay.selection_kind
        };

        self.fill_rect(panel, Color::rgb(8, 10, 14));
        self.beveled_rect_outline(panel, 14.0, Color::rgb(245, 247, 250));
        self.text(
            pos + Vec2::new(18.0, 18.0),
            "SPECIAL EDIT",
            2,
            Color::rgb(245, 247, 250),
        );
        self.text(
            pos + Vec2::new(18.0, 52.0),
            subject,
            2,
            Color::rgb(180, 190, 205),
        );

        match overlay.inspector {
            EditorInspector::Door(door) => {
                self.editor_toggle_row(
                    pos + Vec2::new(18.0, 88.0),
                    size.x - 36.0,
                    "MODE",
                    if door.automatic { "AUTO" } else { "MANUAL" },
                    door.automatic,
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 140.0),
                    size.x - 36.0,
                    "RADIUS",
                    &format!("{:.0}", door.trigger_radius),
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 192.0),
                    size.x - 36.0,
                    "SPEED",
                    &format!("{:.1}", door.speed),
                );
            }
            EditorInspector::WorldPortal(portal) => {
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 82.0),
                    size.x - 36.0,
                    "ID",
                    &portal.id.to_string(),
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 130.0),
                    size.x - 36.0,
                    "RECEIVER",
                    &portal.receiver_id.to_string(),
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 178.0),
                    size.x - 36.0,
                    "PRIORITY",
                    &portal.priority.to_string(),
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 226.0),
                    size.x - 36.0,
                    "SCALE",
                    &format!("{:.1}", portal.scale),
                );
                self.editor_toggle_row(
                    pos + Vec2::new(18.0, 274.0),
                    size.x - 36.0,
                    "SEAMLESS",
                    if portal.seamless { "ON" } else { "OFF" },
                    portal.seamless,
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 322.0),
                    size.x - 36.0,
                    "AREA",
                    &format!("{:.0}", portal.seamless_depth),
                );
                self.editor_stepper_row(
                    pos + Vec2::new(18.0, 370.0),
                    size.x - 36.0,
                    "ANGLE",
                    &format!("{:.0}", portal.seamless_angle),
                );
                self.editor_toggle_row(
                    pos + Vec2::new(18.0, 418.0),
                    size.x - 36.0,
                    "WALLS",
                    if portal.seamless_rely_on_walls {
                        "ON"
                    } else {
                        "OFF"
                    },
                    portal.seamless_rely_on_walls,
                );
            }
            EditorInspector::None => {
                self.text(
                    pos + Vec2::new(18.0, 88.0),
                    "SELECT DOOR OR PORTAL",
                    1,
                    Color::rgb(105, 115, 130),
                );
            }
        }
    }

    fn editor_toggle_row(&mut self, pos: Vec2, width: f32, label: &str, value: &str, on: bool) {
        self.text(
            pos + Vec2::new(0.0, 12.0),
            label,
            1,
            Color::rgb(180, 190, 205),
        );
        let control = Rect {
            pos: Vec2::new(pos.x + width - 156.0, pos.y),
            size: Vec2::new(156.0, 36.0),
        };
        let fill = if on {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(10, 12, 16)
        };
        let text = if on {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(245, 247, 250)
        };

        self.beveled_rect_fill(control, 10.0, fill);
        self.beveled_rect_outline(control, 10.0, Color::rgb(245, 247, 250));
        self.centered_text(
            control.pos + control.size * 0.5 - Vec2::new(0.0, 7.0),
            value,
            2,
            text,
        );
    }

    fn editor_stepper_row(&mut self, pos: Vec2, width: f32, label: &str, value: &str) {
        self.text(
            pos + Vec2::new(0.0, 12.0),
            label,
            1,
            Color::rgb(180, 190, 205),
        );
        let button_size = Vec2::new(36.0, 36.0);
        let plus = Rect {
            pos: Vec2::new(pos.x + width - button_size.x, pos.y),
            size: button_size,
        };
        let value_rect = Rect {
            pos: Vec2::new(plus.pos.x - 92.0, pos.y),
            size: Vec2::new(84.0, 36.0),
        };
        let minus = Rect {
            pos: Vec2::new(value_rect.pos.x - button_size.x - 8.0, pos.y),
            size: button_size,
        };

        self.editor_small_button(minus, "-");
        self.beveled_rect_outline(value_rect, 10.0, Color::rgb(245, 247, 250));
        self.centered_text(
            value_rect.pos + value_rect.size * 0.5 - Vec2::new(0.0, 7.0),
            value,
            2,
            Color::rgb(245, 247, 250),
        );
        self.editor_small_button(plus, "+");
    }

    fn editor_small_button(&mut self, rect: Rect, label: &str) {
        self.beveled_rect_fill(rect, 10.0, Color::rgb(10, 12, 16));
        self.beveled_rect_outline(rect, 10.0, Color::rgb(245, 247, 250));
        self.centered_text(
            rect.pos + rect.size * 0.5 - Vec2::new(0.0, 7.0),
            label,
            2,
            Color::rgb(245, 247, 250),
        );
    }

    fn menu_background(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;

        self.fill_rect(
            Rect {
                pos: Vec2::ZERO,
                size: Vec2::new(width, height),
            },
            Color::rgb(0, 0, 0),
        );

        self.menu_source_code();
        self.menu_frame();
        self.menu_machine_figure();
    }

    fn options_background(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;

        self.fill_rect(
            Rect {
                pos: Vec2::ZERO,
                size: Vec2::new(width, height),
            },
            Color::rgb(0, 0, 0),
        );

        self.menu_source_code_dim();
        self.menu_frame();
    }

    fn menu_source_code(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;
        let start_x = (width * 0.016).max(14.0);
        let line_h = 22.0;
        let rows = (height / line_h).ceil() as usize + 3;

        for row in 0..rows {
            let source_index = (row * 7 + row / 3) % MENU_SOURCE_LINES.len();
            let repeat_index = (source_index + 11) % MENU_SOURCE_LINES.len();
            let y = 32.0 + row as f32 * line_h;
            let x = start_x + ((row * 37) % 170) as f32 - 70.0;
            let shade = 22 + ((row * 13) % 30) as u8;
            let color = Color::rgb(shade, shade, shade);

            self.text(Vec2::new(x, y), MENU_SOURCE_LINES[source_index], 2, color);

            if row % 2 == 0 {
                let x2 = x + width * 0.47 + ((row * 19) % 90) as f32;
                self.text(
                    Vec2::new(x2, y + 7.0),
                    MENU_SOURCE_LINES[repeat_index],
                    1,
                    Color::rgb(shade + 7, shade + 7, shade + 7),
                );
            }
        }
    }

    fn menu_source_code_dim(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;
        let line_h = 28.0;
        let rows = (height / line_h).ceil() as usize + 2;

        for row in 0..rows {
            let source_index = (row * 5 + 3) % MENU_SOURCE_LINES.len();
            let y = 42.0 + row as f32 * line_h;
            let x = -40.0 + ((row * 89) % 240) as f32;
            let shade = 14 + ((row * 11) % 12) as u8;

            self.text(
                Vec2::new(x, y),
                MENU_SOURCE_LINES[source_index],
                2,
                Color::rgb(shade, shade, shade),
            );

            self.text(
                Vec2::new(x + width * 0.46, y + 8.0),
                MENU_SOURCE_LINES[(source_index + 9) % MENU_SOURCE_LINES.len()],
                1,
                Color::rgb(shade + 5, shade + 5, shade + 5),
            );
        }
    }

    fn options_sidebar(&mut self, left: f32, side_w: f32, active_tab: OptionsTab) {
        let height = self.height as f32;
        let top = options_sidebar_top(height);
        let button_h = options_button_height(height);
        let gap = options_button_gap(height);
        let text_color = Color::rgb(245, 245, 245);
        let dim = Color::rgb(150, 150, 150);

        self.centered_text(
            Vec2::new(left + side_w * 0.5, top - 48.0),
            "-- GENERAL --",
            2,
            text_color,
        );

        let general = [
            OptionsTab::General,
            OptionsTab::Controls,
            OptionsTab::Graphics,
            OptionsTab::Audio,
            OptionsTab::Assist,
            OptionsTab::Saves,
        ];
        for (index, tab) in general.iter().enumerate() {
            let pos = Vec2::new(left, top + index as f32 * (button_h + gap));
            self.options_side_button(
                pos,
                Vec2::new(side_w, button_h),
                tab.label(),
                *tab == active_tab,
                tab.enabled(),
            );
        }

        let (back_pos, back_size) = options_back_button_rect(self.width as f32, height);
        let custom_y = top + general.len() as f32 * (button_h + gap) + gap * 5.0;
        let custom_bottom = custom_y + 44.0 + 2.0 * button_h + gap;
        if custom_bottom < back_pos.y - 14.0 {
            self.centered_text(
                Vec2::new(left + side_w * 0.5, custom_y),
                "-- CUSTOMIZATION --",
                2,
                text_color,
            );
            for (index, tab) in [OptionsTab::Hud, OptionsTab::Colors].iter().enumerate() {
                let pos = Vec2::new(left, custom_y + 44.0 + index as f32 * (button_h + gap));
                self.options_side_button(
                    pos,
                    Vec2::new(side_w, button_h),
                    tab.label(),
                    *tab == active_tab,
                    tab.enabled(),
                );
            }
        }

        self.options_side_button(back_pos, back_size, "BACK", false, true);
        self.text(back_pos + Vec2::new(14.0, back_size.y + 10.0), "", 1, dim);
    }

    fn options_content(&mut self, content: OptionsContent<'_>) {
        let OptionsContent {
            origin,
            width,
            content_right,
            settings,
            active_tab,
            capture,
            resolution_dropdown,
        } = content;
        let text = Color::rgb(245, 245, 245);
        let muted = Color::rgb(190, 190, 190);
        let control_x = origin.x + width * 0.48;
        let max_control_w = (width * 0.54).max(160.0);
        let min_control_w = 160.0_f32.min(max_control_w);
        let wide_control = (content_right - control_x).clamp(min_control_w, max_control_w);
        let vertical = ((self.height as f32 - origin.y - 80.0) / 920.0).clamp(0.68, 1.0);
        let y = |offset: f32| origin.y + offset * vertical;

        self.centered_text(
            origin + Vec2::new(width * 0.5, 0.0),
            &format!("-- {} --", active_tab.label()),
            3,
            text,
        );

        match active_tab {
            OptionsTab::General => {
                self.option_row_label(origin, y(96.0), "SHOW FPS", muted);
                self.option_checkbox(Vec2::new(control_x, y(88.0)), settings.show_fps);
            }
            OptionsTab::Controls => {
                for (index, (key, code)) in settings.action_bindings().iter().enumerate() {
                    let row_y = y(96.0 + index as f32 * 68.0);
                    self.option_row_label(origin, row_y, game_key_label(*key), muted);
                    let label = if capture == Some(*key) {
                        "PRESS KEY"
                    } else {
                        key_code_label(*code)
                    };
                    self.option_select(
                        Rect {
                            pos: Vec2::new(control_x, row_y - 12.0),
                            size: Vec2::new(wide_control, 46.0),
                        },
                        label,
                    );
                }
            }
            OptionsTab::Graphics => {
                self.option_row_label(origin, y(96.0), "DISPLAY MODE", muted);
                self.option_select(
                    Rect {
                        pos: Vec2::new(control_x, y(84.0)),
                        size: Vec2::new(wide_control, 46.0),
                    },
                    settings.display_mode.label(),
                );
                self.option_row_label(origin, y(166.0), "RESOLUTION", muted);
                let resolution_rect = Rect {
                    pos: Vec2::new(control_x, y(154.0)),
                    size: Vec2::new(wide_control, 46.0),
                };
                self.option_select(resolution_rect, &settings.resolution.label());
                if resolution_dropdown {
                    self.resolution_dropdown(resolution_rect, settings);
                }
            }
            OptionsTab::Audio => {
                let audio_control_w = (wide_control - 76.0).max(120.0);
                self.audio_slider_row(
                    origin,
                    y(96.0),
                    "MASTER SOUND",
                    control_x,
                    audio_control_w,
                    settings.master_volume,
                );
                self.audio_slider_row(
                    origin,
                    y(180.0),
                    "SOUND EFFECTS",
                    control_x,
                    audio_control_w,
                    settings.sfx_volume,
                );
                self.audio_slider_row(
                    origin,
                    y(264.0),
                    "MUSIC",
                    control_x,
                    audio_control_w,
                    settings.music_volume,
                );
            }
            OptionsTab::Assist | OptionsTab::Saves | OptionsTab::Hud | OptionsTab::Colors => {
                self.centered_text(
                    Vec2::new(origin.x + width * 0.5, y(180.0)),
                    "DISABLED",
                    3,
                    Color::rgb(105, 105, 105),
                );
            }
        }
    }

    fn options_scrollbar(&mut self, x: f32) {
        let height = self.height as f32;
        let top = (height * 0.06).clamp(54.0, 78.0);
        let bottom = height - 60.0;
        let rect = Rect {
            pos: Vec2::new(x, top),
            size: Vec2::new(44.0, bottom - top),
        };

        self.beveled_rect_outline(rect, 15.0, Color::rgb(245, 245, 245));
        self.beveled_rect_fill(
            Rect {
                pos: rect.pos + Vec2::new(7.0, 7.0),
                size: Vec2::new(30.0, 132.0),
            },
            8.0,
            Color::rgb(245, 245, 245),
        );
    }

    fn options_side_button(
        &mut self,
        pos: Vec2,
        size: Vec2,
        label: &str,
        selected: bool,
        enabled: bool,
    ) {
        let fill = if selected {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(0, 0, 0)
        };
        let text = if selected {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(245, 245, 245)
        };
        let edge = if enabled {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(90, 90, 90)
        };
        let text = if enabled {
            text
        } else {
            Color::rgb(110, 110, 110)
        };
        let rect = Rect { pos, size };

        self.beveled_rect_fill(rect, 16.0, fill);
        self.beveled_rect_outline(rect, 16.0, edge);
        self.centered_text(pos + size * 0.5 - Vec2::new(0.0, 8.0), label, 2, text);
    }

    fn option_row_label(&mut self, origin: Vec2, y: f32, label: &str, color: Color) {
        self.text(Vec2::new(origin.x, y), label, 2, color);
    }

    fn audio_slider_row(
        &mut self,
        origin: Vec2,
        y: f32,
        label: &str,
        control_x: f32,
        control_w: f32,
        value: u8,
    ) {
        self.option_row_label(origin, y, label, Color::rgb(190, 190, 190));
        self.option_slider(
            Rect {
                pos: Vec2::new(control_x, y - 12.0),
                size: Vec2::new(control_w, 46.0),
            },
            value as f32 / 100.0,
        );
        self.text(
            Vec2::new(control_x + control_w + 20.0, y - 1.0),
            &format!("{}", value),
            2,
            Color::rgb(245, 245, 245),
        );
    }

    fn option_checkbox(&mut self, pos: Vec2, checked: bool) {
        self.beveled_rect_outline(
            Rect {
                pos,
                size: Vec2::new(30.0, 30.0),
            },
            8.0,
            Color::rgb(245, 245, 245),
        );
        if checked {
            self.draw_line(
                pos + Vec2::new(8.0, 8.0),
                pos + Vec2::new(22.0, 22.0),
                Color::rgb(245, 245, 245),
            );
            self.draw_line(
                pos + Vec2::new(22.0, 8.0),
                pos + Vec2::new(8.0, 22.0),
                Color::rgb(245, 245, 245),
            );
        }
    }

    fn option_select(&mut self, rect: Rect, label: &str) {
        self.beveled_rect_outline(rect, 12.0, Color::rgb(245, 245, 245));
        self.text(
            rect.pos + Vec2::new(18.0, 13.0),
            label,
            2,
            Color::rgb(245, 245, 245),
        );
        self.beveled_rect_outline(
            Rect {
                pos: rect.pos + Vec2::new(rect.size.x - 28.0, 8.0),
                size: Vec2::new(18.0, rect.size.y - 16.0),
            },
            6.0,
            Color::rgb(245, 245, 245),
        );
    }

    fn resolution_dropdown(&mut self, select_rect: Rect, settings: &Settings) {
        let row_h = 30.0;
        let max_h = (self.height as f32 - select_rect.pos.y - select_rect.size.y - 72.0).max(0.0);
        let visible = settings
            .resolutions
            .len()
            .min((max_h / row_h).floor().max(1.0) as usize);
        if visible == 0 {
            return;
        }

        let rect = Rect {
            pos: select_rect.pos + Vec2::new(0.0, select_rect.size.y - 2.0),
            size: Vec2::new(select_rect.size.x, visible as f32 * row_h + 14.0),
        };
        self.fill_rect(
            Rect {
                pos: rect.pos + Vec2::splat(2.0),
                size: rect.size - Vec2::splat(4.0),
            },
            Color::rgb(0, 0, 0),
        );
        self.beveled_rect_outline(rect, 10.0, Color::rgb(245, 245, 245));

        for (index, resolution) in settings.resolutions.iter().take(visible).enumerate() {
            let color = if *resolution == settings.resolution {
                Color::rgb(245, 245, 245)
            } else {
                Color::rgb(160, 160, 160)
            };
            self.text(
                rect.pos + Vec2::new(30.0, 18.0 + index as f32 * row_h),
                &resolution.label(),
                2,
                color,
            );
        }

        if settings.resolutions.len() > visible {
            self.beveled_rect_fill(
                Rect {
                    pos: rect.pos + Vec2::new(rect.size.x - 34.0, 14.0),
                    size: Vec2::new(22.0, (rect.size.y - 28.0).min(76.0)),
                },
                6.0,
                Color::rgb(245, 245, 245),
            );
        }
    }

    fn option_slider(&mut self, rect: Rect, amount: f32) {
        self.beveled_rect_outline(rect, 12.0, Color::rgb(245, 245, 245));
        self.beveled_rect_fill(
            Rect {
                pos: rect.pos + Vec2::new(9.0, 8.0),
                size: Vec2::new((rect.size.x - 18.0) * amount, rect.size.y - 16.0),
            },
            8.0,
            Color::rgb(245, 245, 245),
        );
    }

    fn menu_socials(&mut self) {
        let left = menu_left(self.width as f32);
        let y = social_y(self.height as f32);
        let color = Color::rgb(245, 247, 250);
        let muted = Color::rgb(140, 140, 140);
        let labels = [
            ("X", "@LEXAWHATT"),
            ("YOUTUBE", "@LEXAWHAT"),
            ("GITHUB", "LEXAWHATT"),
        ];
        let mut x = left;

        for (index, (network, handle)) in labels.iter().enumerate() {
            self.text(Vec2::new(x, y), network, 2, color);
            x += text_pixel_width(network, 2) + 12.0;
            self.text(Vec2::new(x, y), handle, 1, muted);
            x += text_pixel_width(handle, 1) + if index == labels.len() - 1 { 0.0 } else { 30.0 };
        }
    }

    fn main_menu_button(&mut self, index: usize, label: &str, primary: bool) {
        let (pos, size) = self.main_menu_button_rect(index);
        let rect = Rect { pos, size };
        let fill = if primary {
            Color::rgb(245, 245, 245)
        } else {
            Color::rgb(0, 0, 0)
        };
        let text = if primary {
            Color::rgb(0, 0, 0)
        } else {
            Color::rgb(245, 245, 245)
        };
        let edge = Color::rgb(245, 245, 245);

        self.beveled_rect_fill(
            Rect {
                pos: pos + Vec2::new(6.0, 6.0),
                size,
            },
            18.0,
            Color::rgb(10, 10, 10),
        );
        self.beveled_rect_fill(rect, 18.0, fill);
        self.beveled_rect_outline(rect, 18.0, edge);
        self.centered_text(pos + size * 0.5 - Vec2::new(0.0, 14.0), label, 4, text);
    }

    fn main_menu_button_rect(&self, index: usize) -> (Vec2, Vec2) {
        let width = self.width as f32;
        let height = self.height as f32;
        let size = Vec2::new(
            (width * 0.34).clamp(320.0, 660.0),
            (height * 0.058).clamp(54.0, 70.0),
        );
        let gap = menu_button_gap(height);
        let total_height = size.y * 4.0 + gap * 3.0;
        let target_y = height * 0.42;
        let max_y = height - 100.0 - total_height;
        let min_y = height * 0.34;
        let pos = Vec2::new(
            menu_left(width),
            target_y.min(max_y).max(min_y) + index as f32 * (size.y + gap),
        );

        (pos, size)
    }

    fn beveled_rect_outline(&mut self, rect: Rect, bevel: f32, color: Color) {
        let p = rect.pos;
        let s = rect.size;
        let cut = bevel.min(s.x * 0.5).min(s.y * 0.5);
        let points = [
            p + Vec2::new(cut, 0.0),
            p + Vec2::new(s.x - cut, 0.0),
            p + Vec2::new(s.x, cut),
            p + Vec2::new(s.x, s.y - cut),
            p + Vec2::new(s.x - cut, s.y),
            p + Vec2::new(cut, s.y),
            p + Vec2::new(0.0, s.y - cut),
            p + Vec2::new(0.0, cut),
        ];

        for index in 0..points.len() {
            self.draw_line_thick(points[index], points[(index + 1) % points.len()], color, 2);
        }
    }

    fn beveled_rect_fill(&mut self, rect: Rect, bevel: f32, color: Color) {
        let y0 = rect.pos.y.max(0.0).round() as i32;
        let y1 = (rect.pos.y + rect.size.y).min(self.height as f32).round() as i32;
        if y1 <= y0 {
            return;
        }

        let cut = bevel.min(rect.size.x * 0.5).min(rect.size.y * 0.5);
        for y in y0..y1 {
            let local_y = y as f32 - rect.pos.y;
            let inset = if local_y < cut {
                cut - local_y
            } else if local_y > rect.size.y - cut {
                local_y - (rect.size.y - cut)
            } else {
                0.0
            };
            let x0 = (rect.pos.x + inset).max(0.0).round() as i32;
            let x1 = (rect.pos.x + rect.size.x - inset)
                .min(self.width as f32)
                .round() as i32;

            for x in x0..x1 {
                self.put_px(x, y, color);
            }
        }
    }

    fn draw_line_thick(&mut self, a: Vec2, b: Vec2, color: Color, thickness: i32) {
        let radius = (thickness - 1).max(0);
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                if ox.abs() + oy.abs() > radius {
                    continue;
                }

                let offset = Vec2::new(ox as f32, oy as f32);
                self.draw_line(a + offset, b + offset, color);
            }
        }
    }

    fn menu_frame(&mut self) {
        let color = Color::rgb(245, 245, 245);
        let width = self.width as f32;
        let height = self.height as f32;
        let inset = 6.0;
        let cut = 28.0;

        for offset in 0..3 {
            let o = inset + offset as f32;
            self.draw_line(Vec2::new(o + cut, o), Vec2::new(width - o - cut, o), color);
            self.draw_line(
                Vec2::new(width - o - cut, o),
                Vec2::new(width - o, o + cut),
                color,
            );
            self.draw_line(
                Vec2::new(width - o, o + cut),
                Vec2::new(width - o, height - o - cut),
                color,
            );
            self.draw_line(
                Vec2::new(width - o, height - o - cut),
                Vec2::new(width - o - cut, height - o),
                color,
            );
            self.draw_line(
                Vec2::new(width - o - cut, height - o),
                Vec2::new(o + cut, height - o),
                color,
            );
            self.draw_line(
                Vec2::new(o + cut, height - o),
                Vec2::new(o, height - o - cut),
                color,
            );
            self.draw_line(Vec2::new(o, height - o - cut), Vec2::new(o, o + cut), color);
            self.draw_line(Vec2::new(o, o + cut), Vec2::new(o + cut, o), color);
        }
    }

    fn menu_machine_figure(&mut self) {
        let width = self.width as f32;
        let height = self.height as f32;
        let (button_pos, button_size) = self.main_menu_button_rect(0);
        let left_bound = button_pos.x + button_size.x + (width * 0.045).clamp(42.0, 96.0);
        let right_bound = width - menu_left(width);
        let available_width = (right_bound - left_bound).max(260.0);
        let target_size = (height * 0.70)
            .min(available_width * 0.86)
            .clamp(300.0, 760.0);
        let center_x = (left_bound + right_bound) * 0.5;
        let min_x = left_bound;
        let max_x = (right_bound - target_size).max(min_x);
        let max_y = (height - target_size - 88.0).max(92.0);
        let pos = Vec2::new(
            (center_x - target_size * 0.5).clamp(min_x, max_x),
            ((height - target_size) * 0.48).clamp(92.0, max_y),
        );

        self.draw_rgba_image(
            MENU_V1,
            MENU_V1_SIZE,
            MENU_V1_SIZE,
            pos,
            Vec2::splat(target_size),
        );
    }

    fn draw_rgba_image(
        &mut self,
        bytes: &[u8],
        source_width: usize,
        source_height: usize,
        pos: Vec2,
        size: Vec2,
    ) {
        self.draw_rgba_image_inner(bytes, source_width, source_height, pos, size, true);
    }

    fn draw_rgba_image_opaque(
        &mut self,
        bytes: &[u8],
        source_width: usize,
        source_height: usize,
        pos: Vec2,
        size: Vec2,
    ) {
        self.draw_rgba_image_inner(bytes, source_width, source_height, pos, size, false);
    }

    fn draw_rgba_image_inner(
        &mut self,
        bytes: &[u8],
        source_width: usize,
        source_height: usize,
        pos: Vec2,
        size: Vec2,
        skip_dark: bool,
    ) {
        let dest_w = size.x.round().max(1.0) as i32;
        let dest_h = size.y.round().max(1.0) as i32;
        let scale_x = source_width as f32 / dest_w as f32;
        let scale_y = source_height as f32 / dest_h as f32;

        for dy in 0..dest_h {
            let sy = ((dy as f32 * scale_y) as usize).min(source_height - 1);
            for dx in 0..dest_w {
                let sx = ((dx as f32 * scale_x) as usize).min(source_width - 1);
                let index = (sy * source_width + sx) * 4;
                let r = bytes[index];
                let g = bytes[index + 1];
                let b = bytes[index + 2];
                let a = bytes[index + 3];

                if a < 24 || (skip_dark && r < 20 && g < 20 && b < 20) {
                    continue;
                }

                self.put_px(
                    (pos.x + dx as f32).round() as i32,
                    (pos.y + dy as f32).round() as i32,
                    Color::rgb(r, g, b),
                );
            }
        }
    }

    fn centered_text(&mut self, center: Vec2, text: &str, scale: i32, color: Color) {
        let width = text_pixel_width(text, scale);
        self.text(center - Vec2::new(width * 0.5, 0.0), text, scale, color);
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

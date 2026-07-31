use winit::event::MouseButton;
use winit::keyboard::KeyCode;

// Editor input mutates solids and pans the world camera.
use crate::app::App;
use crate::app::editor::{EditorCategory, EditorMode, EditorTool};
use crate::game::level::{LevelTrigger, LevelTriggerKind};
use crate::game::player::Player;

impl App {
    pub(super) fn handle_editor_key(&mut self, code: KeyCode, down: bool) {
        if down && self.modifiers.control_key() && self.handle_editor_shortcut(code) {
            return;
        }
        if down
            && self
                .editor
                .handle_text_key(code, self.modifiers.shift_key(), &mut self.world.level)
        {
            return;
        }
        if self.editor.set_pan_key(code, down) {
            return;
        }
        if !down {
            return;
        }

        match code {
            KeyCode::Digit1 => self.create_editor_block(EditorTool::Portalable),
            KeyCode::Digit2 => self.create_editor_block(EditorTool::Solid),
            KeyCode::Digit3 => self
                .editor
                .create_hazard(self.cursor_world, &mut self.world.level),
            KeyCode::Digit4 => self
                .editor
                .create_door(self.cursor_world, &mut self.world.level),
            KeyCode::Digit5 => self
                .editor
                .create_checkpoint(self.cursor_world, &mut self.world.level),
            KeyCode::Digit6 => self
                .editor
                .create_world_portal(self.cursor_world, &mut self.world.level),
            KeyCode::Digit7 => self
                .editor
                .create_text(self.cursor_world, &mut self.world.level),
            KeyCode::Digit8 => self
                .editor
                .create_filth(self.cursor_world, &mut self.world.level),
            KeyCode::Digit9 => self
                .editor
                .create_level_start(self.cursor_world, &mut self.world.level),
            KeyCode::Digit0 => self
                .editor
                .create_level_end(self.cursor_world, &mut self.world.level),
            KeyCode::Delete | KeyCode::Backspace => {
                self.editor.delete_selected(&mut self.world.level);
            }
            KeyCode::Enter => {
                self.editor.toggle_text_editing();
            }
            KeyCode::KeyP => {
                self.editor.toggle_portalable(&mut self.world.level);
            }
            KeyCode::KeyR => {
                self.editor.rotate_ui = !self.editor.rotate_ui;
            }
            KeyCode::KeyG => self.set_editor_spawn(),
            KeyCode::KeyH => self.editor.toggle_grid_snap(&mut self.world.level),
            KeyCode::KeyI => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 1, 0, 0);
            }
            KeyCode::KeyO => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 0, 1, 0);
            }
            KeyCode::KeyU => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 0, 0, 1);
            }
            KeyCode::KeyJ => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 0, 0, -1);
            }
            KeyCode::KeyN => self.create_editor_block(self.editor.tool),
            KeyCode::PageUp => {
                self.editor
                    .adjust_selected_object_meta(&mut self.world.level, 0, 1);
            }
            KeyCode::PageDown => {
                self.editor
                    .adjust_selected_object_meta(&mut self.world.level, 0, -1);
            }
            KeyCode::F5 => self.save_current_level(),
            _ => {}
        }
    }

    pub(super) fn handle_editor_mouse(&mut self, button: MouseButton, down: bool) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        let screen_w = size.width as f32;
        let screen_h = size.height as f32;

        match (button, down) {
            (MouseButton::Left, true) => {
                if let Some(action) = editor_top_action_hit(self.cursor_screen, screen_w, screen_h)
                {
                    self.apply_editor_top_action(action);
                    return;
                }
                if let Some(mode) = editor_mode_hit(self.cursor_screen, screen_w, screen_h) {
                    self.editor.set_mode(mode);
                    return;
                }
                if let Some(category) = editor_category_hit(self.cursor_screen, screen_w, screen_h)
                {
                    self.editor.set_category(category);
                    return;
                }
                if let Some(tool) = editor_dock_hit(
                    self.cursor_screen,
                    screen_w,
                    screen_h,
                    self.editor.category().index(),
                ) {
                    self.editor.set_tool(tool);
                    return;
                }
                if let Some(delta) = editor_layer_hit(self.cursor_screen, screen_w, screen_h) {
                    self.editor
                        .adjust_selected_object_meta(&mut self.world.level, 0, delta);
                    return;
                }
                if let Some(action) =
                    editor_quick_action_hit(self.cursor_screen, screen_w, screen_h)
                {
                    self.apply_editor_quick_action(action);
                    return;
                }
                if self.editor_inspector_open {
                    let selection_kind = self.editor.primary_selection_kind().label();
                    if let Some(action) = editor_inspector_action_hit(
                        self.cursor_screen,
                        screen_w,
                        screen_h,
                        selection_kind,
                    ) {
                        self.apply_editor_inspector_action(action);
                        return;
                    }
                }
                if self.editor_inspector_open
                    && editor_inspector_panel_hit(self.cursor_screen, screen_w, screen_h)
                {
                    return;
                }
                if editor_ui_hit(self.cursor_screen, screen_w, screen_h) {
                    return;
                }
                match self.editor.mode() {
                    EditorMode::Build => {
                        if !self
                            .editor
                            .has_object_at(self.cursor_world, &self.world.level)
                        {
                            self.editor
                                .create_active_tool(self.cursor_world, &mut self.world.level);
                        }
                    }
                    EditorMode::Edit => self.editor.begin_left_drag(
                        self.cursor_world,
                        self.modifiers.shift_key(),
                        self.modifiers.control_key(),
                        &mut self.world.level,
                    ),
                    EditorMode::Delete => {
                        self.editor
                            .delete_at(self.cursor_world, &mut self.world.level);
                    }
                }
            }
            (MouseButton::Left, false) if self.editor.mode() == EditorMode::Edit => {
                self.editor.end_drag(&self.world.level);
            }
            (MouseButton::Right, true) => {
                if self.editor_inspector_open
                    && editor_inspector_panel_hit(self.cursor_screen, screen_w, screen_h)
                {
                    return;
                }
                if editor_ui_hit(self.cursor_screen, screen_w, screen_h) {
                    return;
                }
                self.editor.begin_left_drag(
                    self.cursor_world,
                    self.modifiers.shift_key(),
                    self.modifiers.control_key(),
                    &mut self.world.level,
                );
            }
            (MouseButton::Right, false) => self.editor.end_drag(&self.world.level),
            _ => {}
        }
    }

    fn create_editor_block(&mut self, tool: EditorTool) {
        self.editor
            .create_block(self.cursor_world, tool, &mut self.world.level);
    }

    fn apply_editor_inspector_action(&mut self, action: EditorInspectorAction) {
        match action {
            EditorInspectorAction::DoorMode => {
                self.editor
                    .toggle_selected_door_automatic(&mut self.world.level);
            }
            EditorInspectorAction::DoorRadius(delta) => {
                self.editor
                    .adjust_selected_door_radius(&mut self.world.level, delta);
            }
            EditorInspectorAction::DoorSpeed(delta) => {
                self.editor
                    .adjust_selected_door_speed(&mut self.world.level, delta);
            }
            EditorInspectorAction::EnemyId(delta) => {
                self.editor
                    .adjust_selected_enemy_spawn(&mut self.world.level, delta, 0);
            }
            EditorInspectorAction::EnemyWave(delta) => {
                self.editor
                    .adjust_selected_enemy_spawn(&mut self.world.level, 0, delta);
            }
            EditorInspectorAction::SpawnTriggerEnemyId(delta) => {
                self.editor
                    .adjust_selected_enemy_spawn_trigger(&mut self.world.level, delta);
            }
            EditorInspectorAction::PortalId(delta) => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, delta, 0, 0);
            }
            EditorInspectorAction::PortalReceiver(delta) => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 0, delta, 0);
            }
            EditorInspectorAction::PortalPriority(delta) => {
                self.editor
                    .adjust_selected_world_portal(&mut self.world.level, 0, 0, delta);
            }
            EditorInspectorAction::PortalScale(delta) => {
                self.editor
                    .adjust_selected_world_portal_scale(&mut self.world.level, delta);
            }
            EditorInspectorAction::PortalSeamless => {
                self.editor
                    .toggle_selected_world_portal_seamless(&mut self.world.level);
            }
            EditorInspectorAction::PortalArea(delta) => {
                self.editor
                    .adjust_selected_world_portal_seamless_depth(&mut self.world.level, delta);
            }
            EditorInspectorAction::PortalAngle(delta) => {
                self.editor
                    .adjust_selected_world_portal_seamless_angle(&mut self.world.level, delta);
            }
            EditorInspectorAction::PortalWalls => {
                self.editor
                    .toggle_selected_world_portal_rely_on_walls(&mut self.world.level);
            }
            EditorInspectorAction::ObjectId(delta) => {
                self.editor
                    .adjust_selected_object_meta(&mut self.world.level, delta, 0);
            }
            EditorInspectorAction::ObjectLayer(delta) => {
                self.editor
                    .adjust_selected_object_meta(&mut self.world.level, 0, delta);
            }
        }
    }

    fn apply_editor_top_action(&mut self, action: EditorTopAction) {
        match action {
            EditorTopAction::Undo => self.editor.undo(&mut self.world.level),
            EditorTopAction::Delete => self.editor.delete_selected(&mut self.world.level),
            EditorTopAction::Special => self.editor_inspector_open = !self.editor_inspector_open,
            EditorTopAction::Save => self.save_current_level(),
        }
    }

    fn apply_editor_quick_action(&mut self, action: EditorQuickAction) {
        match action {
            EditorQuickAction::Rotate => self.editor.rotate_ui = !self.editor.rotate_ui,
            EditorQuickAction::Snap => self.editor.toggle_grid_snap(&mut self.world.level),
            EditorQuickAction::Special => self.editor_inspector_open = !self.editor_inspector_open,
            EditorQuickAction::Save => self.save_current_level(),
        }
    }

    fn handle_editor_shortcut(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::KeyA => {
                self.editor.select_all(&self.world.level);
                true
            }
            KeyCode::KeyC => {
                self.editor.copy_selected(&self.world.level);
                true
            }
            KeyCode::KeyD => {
                self.editor.duplicate_selected(&mut self.world.level);
                true
            }
            KeyCode::KeyV => {
                self.editor
                    .paste_clipboard(self.cursor_world, &mut self.world.level);
                true
            }
            KeyCode::KeyX => {
                self.editor.cut_selected(&mut self.world.level);
                true
            }
            KeyCode::KeyZ => {
                self.editor.undo(&mut self.world.level);
                true
            }
            _ => false,
        }
    }

    fn set_editor_spawn(&mut self) {
        let spawn = self.editor.snap_point(self.cursor_world);
        let trigger_size = glam::Vec2::new(48.0, 80.0);

        if let Some(level) = self.levels.get_mut(self.current_level) {
            level.spawn = spawn;
        }
        if let Some(trigger) = self
            .world
            .level
            .triggers
            .iter_mut()
            .find(|trigger| trigger.kind == LevelTriggerKind::LevelStart)
        {
            trigger.solid.set_pos(spawn - trigger.solid.size() / 2.0);
        } else {
            self.world.level.triggers.push(LevelTrigger::level_start(
                spawn.x - trigger_size.x / 2.0,
                spawn.y - trigger_size.y / 2.0,
                trigger_size.x,
                trigger_size.y,
            ));
        }
        self.world.player = Player::new_with_max_health(
            spawn.x,
            spawn.y,
            self.world.difficulty.player_max_health(),
        );
        self.editor.mark_dirty();
    }
}

fn editor_dock_hit(
    pos: glam::Vec2,
    width: f32,
    height: f32,
    category_index: usize,
) -> Option<EditorTool> {
    let tools = editor_palette_real_tools(category_index);
    let slot_count = editor_palette_slot_count(category_index);

    for (slot, tool_index) in tools.iter().copied().enumerate() {
        let (item_pos, item_size) = editor_tool_rect(slot, slot_count, width, height);
        if rect_hit(pos, item_pos, item_size) {
            return EditorTool::from_index(tool_index);
        }
    }

    None
}

fn editor_category_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<EditorCategory> {
    for index in 0..EditorCategory::COUNT {
        let (tab_pos, tab_size) = editor_category_tab_rect(index, width, height);
        if rect_hit(pos, tab_pos, tab_size) {
            return EditorCategory::from_index(index);
        }
    }

    None
}

fn editor_mode_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<EditorMode> {
    for index in 0..EditorMode::COUNT {
        let (button_pos, button_size) = editor_mode_button_rect(index, width, height);
        if rect_hit(pos, button_pos, button_size) {
            return EditorMode::from_index(index);
        }
    }

    None
}

fn editor_quick_action_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<EditorQuickAction> {
    for index in 0..4 {
        let (button_pos, button_size) = editor_quick_button_rect(index, width, height);
        if rect_hit(pos, button_pos, button_size) {
            return EditorQuickAction::from_index(index);
        }
    }

    None
}

fn editor_layer_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<i16> {
    let (control_pos, control_size) = editor_layer_control_rect(width, height);
    if !rect_hit(pos, control_pos, control_size) {
        return None;
    }

    let button = glam::Vec2::new(36.0, 36.0);
    let left_pos = control_pos + glam::Vec2::new(8.0, (control_size.y - button.y) * 0.5);
    let right_pos = control_pos
        + glam::Vec2::new(
            control_size.x - button.x - 8.0,
            (control_size.y - button.y) * 0.5,
        );

    if rect_hit(pos, left_pos, button) {
        Some(-1)
    } else if rect_hit(pos, right_pos, button) {
        Some(1)
    } else {
        None
    }
}

fn editor_top_action_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<EditorTopAction> {
    for index in 0..4 {
        let (button_pos, button_size) = editor_top_button_rect(index, width, height);
        if rect_hit(pos, button_pos, button_size) {
            return EditorTopAction::from_index(index);
        }
    }

    None
}

fn editor_ui_hit(pos: glam::Vec2, width: f32, height: f32) -> bool {
    let (bottom_pos, bottom_size) = editor_bottom_panel_rect(width, height);

    rect_hit(pos, bottom_pos, bottom_size) || editor_top_bar_hit(pos, width, height)
}

fn editor_inspector_panel_hit(pos: glam::Vec2, width: f32, height: f32) -> bool {
    let (panel_pos, size) = editor_inspector_layout(width, height);

    rect_hit(pos, panel_pos, size)
}

fn editor_inspector_action_hit(
    pos: glam::Vec2,
    width: f32,
    height: f32,
    selection_kind: &str,
) -> Option<EditorInspectorAction> {
    let (panel_pos, panel_size) = editor_inspector_layout(width, height);
    if !rect_hit(pos, panel_pos, panel_size) {
        return None;
    }

    let row_width = panel_size.x - 36.0;
    let row_pos = |y: f32| panel_pos + glam::Vec2::new(18.0, y);

    match selection_kind {
        "DOOR" => {
            if editor_toggle_hit(pos, row_pos(88.0), row_width) {
                return Some(EditorInspectorAction::DoorMode);
            }
            editor_stepper_hit(pos, row_pos(140.0), row_width)
                .map(|direction| EditorInspectorAction::DoorRadius(16.0 * direction as f32))
                .or_else(|| {
                    editor_stepper_hit(pos, row_pos(192.0), row_width)
                        .map(|direction| EditorInspectorAction::DoorSpeed(0.2 * direction as f32))
                })
        }
        "FILTH" => editor_stepper_hit(pos, row_pos(88.0), row_width)
            .map(EditorInspectorAction::EnemyId)
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(140.0), row_width)
                    .map(EditorInspectorAction::EnemyWave)
            }),
        "TRIGGER" => editor_stepper_hit(pos, row_pos(88.0), row_width)
            .map(EditorInspectorAction::SpawnTriggerEnemyId),
        "WORLD PORTAL" => editor_stepper_hit(pos, row_pos(74.0), row_width)
            .map(EditorInspectorAction::PortalId)
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(116.0), row_width)
                    .map(EditorInspectorAction::PortalReceiver)
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(158.0), row_width)
                    .map(EditorInspectorAction::PortalPriority)
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(200.0), row_width)
                    .map(|direction| EditorInspectorAction::PortalScale(0.1 * direction as f32))
            })
            .or_else(|| {
                editor_toggle_hit(pos, row_pos(242.0), row_width)
                    .then_some(EditorInspectorAction::PortalSeamless)
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(284.0), row_width)
                    .map(|direction| EditorInspectorAction::PortalArea(16.0 * direction as f32))
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(326.0), row_width)
                    .map(|direction| EditorInspectorAction::PortalAngle(15.0 * direction as f32))
            })
            .or_else(|| {
                editor_toggle_hit(pos, row_pos(368.0), row_width)
                    .then_some(EditorInspectorAction::PortalWalls)
            }),
        _ => None,
    }
    .or_else(|| {
        let common_y = match selection_kind {
            "NONE" => return None,
            "DOOR" => 244.0,
            "FILTH" => 192.0,
            "TRIGGER" => 140.0,
            "WORLD PORTAL" => 410.0,
            _ => 88.0,
        };

        editor_stepper_hit(pos, row_pos(common_y), row_width)
            .map(EditorInspectorAction::ObjectId)
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(common_y + 52.0), row_width)
                    .map(EditorInspectorAction::ObjectLayer)
            })
    })
}

fn editor_toggle_hit(pos: glam::Vec2, row_pos: glam::Vec2, row_width: f32) -> bool {
    rect_hit(
        pos,
        glam::Vec2::new(row_pos.x + row_width - 156.0, row_pos.y),
        glam::Vec2::new(156.0, 36.0),
    )
}

fn editor_stepper_hit(pos: glam::Vec2, row_pos: glam::Vec2, row_width: f32) -> Option<i16> {
    let button_size = glam::Vec2::new(36.0, 36.0);
    let plus_pos = glam::Vec2::new(row_pos.x + row_width - button_size.x, row_pos.y);
    let value_pos = glam::Vec2::new(plus_pos.x - 92.0, row_pos.y);
    let minus_pos = glam::Vec2::new(value_pos.x - button_size.x - 8.0, row_pos.y);

    if rect_hit(pos, minus_pos, button_size) {
        Some(-1)
    } else if rect_hit(pos, plus_pos, button_size) {
        Some(1)
    } else {
        None
    }
}

fn editor_inspector_layout(width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let size = glam::Vec2::new((width * 0.24).clamp(300.0, 360.0), 520.0);
    let ideal_y = height * 0.5 - size.y * 0.5;
    let min_y = 84.0;
    let max_y = (height - size.y - 18.0).max(min_y);
    let pos = glam::Vec2::new(width - size.x - 22.0, ideal_y.clamp(min_y, max_y));

    (pos, size)
}

fn editor_bottom_panel_rect(width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let panel_h = (height * 0.24).clamp(154.0, 186.0);
    let margin = 12.0;

    (
        glam::Vec2::new(margin, height - panel_h - 10.0),
        glam::Vec2::new((width - margin * 2.0).max(320.0), panel_h),
    )
}

fn editor_mode_button_rect(index: usize, width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let (panel_pos, panel_size) = editor_bottom_panel_rect(width, height);
    let gap = 9.0;
    let size = glam::Vec2::new(
        (width * 0.125).clamp(118.0, 158.0),
        ((panel_size.y - 32.0 - gap * 2.0) / 3.0).clamp(34.0, 44.0),
    );
    let pos = panel_pos + glam::Vec2::new(14.0, 16.0 + index as f32 * (size.y + gap));

    (pos, size)
}

fn editor_quick_button_rect(index: usize, width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let (panel_pos, panel_size) = editor_bottom_panel_rect(width, height);
    let gap = 10.0;
    let size = glam::Vec2::new(78.0, ((panel_size.y - 34.0 - gap) / 2.0).clamp(48.0, 62.0));
    let col = index % 2;
    let row = index / 2;
    let x = panel_pos.x + panel_size.x - 14.0 - size.x * 2.0 - gap;
    let y = panel_pos.y + 18.0;

    (
        glam::Vec2::new(
            x + col as f32 * (size.x + gap),
            y + row as f32 * (size.y + gap),
        ),
        size,
    )
}

fn editor_tool_rect(
    index: usize,
    slot_count: usize,
    width: f32,
    height: f32,
) -> (glam::Vec2, glam::Vec2) {
    let (palette_pos, palette_size, gap, columns) =
        editor_palette_layout(width, height, slot_count);
    let row = index / columns;
    let col = index % columns;
    let rows = slot_count.max(1).div_ceil(columns);
    let item_w = ((palette_size.x - gap * (columns - 1) as f32) / columns as f32).clamp(48.0, 62.0);
    let item_h = ((palette_size.y - gap * (rows - 1) as f32) / rows as f32).clamp(48.0, 54.0);
    let used_w = item_w * columns as f32 + gap * (columns - 1) as f32;
    let used_h = item_h * rows as f32 + gap * (rows - 1) as f32;
    let start = palette_pos + glam::Vec2::new((palette_size.x - used_w) * 0.5, 0.0);

    (
        start
            + glam::Vec2::new(col as f32 * (item_w + gap), row as f32 * (item_h + gap))
            + glam::Vec2::new(0.0, (palette_size.y - used_h) * 0.5),
        glam::Vec2::new(item_w, item_h),
    )
}

fn editor_palette_layout(
    width: f32,
    height: f32,
    slot_count: usize,
) -> (glam::Vec2, glam::Vec2, f32, usize) {
    let (panel_pos, panel_size) = editor_bottom_panel_rect(width, height);
    let mode_w = (width * 0.125).clamp(118.0, 158.0);
    let quick_w = 78.0 * 2.0 + 10.0;
    let layer_w = 146.0;
    let palette_left = panel_pos.x + 14.0 + mode_w + 26.0;
    let palette_right = panel_pos.x + panel_size.x - 14.0 - quick_w - layer_w - 44.0;
    let palette_w = (palette_right - palette_left).max(220.0);
    let slot_count = slot_count.max(1);
    let columns = if palette_w >= 740.0 {
        slot_count
    } else {
        slot_count.min(6)
    };

    (
        glam::Vec2::new(palette_left, panel_pos.y + 52.0),
        glam::Vec2::new(palette_w, panel_size.y - 68.0),
        8.0,
        columns,
    )
}

fn editor_category_tab_rect(index: usize, width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let (palette_pos, palette_size, _, _) = editor_palette_layout(width, height, 6);
    let gap = 8.0;
    let count = EditorCategory::COUNT;
    let tab_w = (palette_size.x - gap * (count - 1) as f32) / count as f32;

    (
        glam::Vec2::new(
            palette_pos.x + index as f32 * (tab_w + gap),
            palette_pos.y - 34.0,
        ),
        glam::Vec2::new(tab_w, 24.0),
    )
}

fn editor_layer_control_rect(width: f32, height: f32) -> (glam::Vec2, glam::Vec2) {
    let (panel_pos, panel_size) = editor_bottom_panel_rect(width, height);
    let (quick_pos, _) = editor_quick_button_rect(0, width, height);
    let size = glam::Vec2::new(136.0, 44.0);

    (
        glam::Vec2::new(
            quick_pos.x - 18.0 - size.x,
            panel_pos.y + panel_size.y * 0.5 - size.y * 0.5,
        ),
        size,
    )
}

fn editor_palette_real_tools(category_index: usize) -> &'static [usize] {
    match category_index {
        0 => &[1, 2, 3],
        1 => &[4, 5, 6, 7],
        2 => &[8],
        3 => &[9, 10, 11],
        _ => &[],
    }
}

fn editor_palette_slot_count(category_index: usize) -> usize {
    match category_index {
        0 => 6,
        1 => 7,
        2 => 5,
        3 => 7,
        4 => 6,
        _ => 6,
    }
}

fn editor_top_bar_hit(pos: glam::Vec2, width: f32, height: f32) -> bool {
    (0..4).any(|index| {
        let (button_pos, button_size) = editor_top_button_rect(index, width, height);

        rect_hit(pos, button_pos, button_size)
    })
}

fn editor_top_button_rect(index: usize, width: f32, _height: f32) -> (glam::Vec2, glam::Vec2) {
    let size = glam::Vec2::splat(54.0);
    let gap = 12.0;
    let y = 16.0;
    let x = match index {
        0 => 18.0,
        1 => 18.0 + size.x + gap,
        2 => width - 18.0 - size.x * 2.0 - gap,
        3 => width - 18.0 - size.x,
        _ => 0.0,
    };

    (glam::Vec2::new(x, y), size)
}

enum EditorInspectorAction {
    DoorMode,
    DoorRadius(f32),
    DoorSpeed(f32),
    EnemyId(i16),
    EnemyWave(i16),
    SpawnTriggerEnemyId(i16),
    PortalId(i16),
    PortalReceiver(i16),
    PortalPriority(i16),
    PortalScale(f32),
    PortalSeamless,
    PortalArea(f32),
    PortalAngle(f32),
    PortalWalls,
    ObjectId(i16),
    ObjectLayer(i16),
}

enum EditorQuickAction {
    Rotate,
    Snap,
    Special,
    Save,
}

impl EditorQuickAction {
    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Rotate),
            1 => Some(Self::Snap),
            2 => Some(Self::Special),
            3 => Some(Self::Save),
            _ => None,
        }
    }
}

enum EditorTopAction {
    Undo,
    Delete,
    Special,
    Save,
}

impl EditorTopAction {
    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Undo),
            1 => Some(Self::Delete),
            2 => Some(Self::Special),
            3 => Some(Self::Save),
            _ => None,
        }
    }
}

fn rect_hit(pos: glam::Vec2, rect_pos: glam::Vec2, rect_size: glam::Vec2) -> bool {
    pos.x >= rect_pos.x
        && pos.x <= rect_pos.x + rect_size.x
        && pos.y >= rect_pos.y
        && pos.y <= rect_pos.y + rect_size.y
}

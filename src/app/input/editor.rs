use winit::event::MouseButton;
use winit::keyboard::KeyCode;

// Editor input mutates solids and pans the world camera.
use crate::app::App;
use crate::app::editor::EditorTool;
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
            KeyCode::Digit1 => self.create_editor_block(EditorTool::Solid),
            KeyCode::Digit2 => self.create_editor_block(EditorTool::Portalable),
            KeyCode::Digit3 => self
                .editor
                .create_door(self.cursor_world, &mut self.world.level),
            KeyCode::Digit4 => self
                .editor
                .create_text(self.cursor_world, &mut self.world.level),
            KeyCode::Digit5 => self
                .editor
                .create_hazard(self.cursor_world, &mut self.world.level),
            KeyCode::Digit6 => self
                .editor
                .create_checkpoint(self.cursor_world, &mut self.world.level),
            KeyCode::Digit7 => self
                .editor
                .create_world_portal(self.cursor_world, &mut self.world.level),
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
                if let Some(tool) = editor_dock_hit(self.cursor_screen, screen_w, screen_h) {
                    self.editor.set_tool(tool);
                    return;
                }
                if editor_inspector_button_hit(self.cursor_screen, screen_w, screen_h) {
                    self.editor_inspector_open = !self.editor_inspector_open;
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
                if !self
                    .editor
                    .has_object_at(self.cursor_world, &self.world.level)
                {
                    self.editor
                        .create_active_tool(self.cursor_world, &mut self.world.level);
                }
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
            EditorInspectorAction::PortalWidth(delta) => {
                self.editor
                    .adjust_selected_world_portal_width(&mut self.world.level, delta);
            }
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

        if let Some(level) = self.levels.get_mut(self.current_level) {
            level.spawn = spawn;
        }
        self.world.player = Player::new(spawn.x, spawn.y);
        self.editor.mark_dirty();
    }
}

fn editor_dock_hit(pos: glam::Vec2, width: f32, height: f32) -> Option<EditorTool> {
    let (dock_pos, item_size, gap) = editor_dock_layout(width, height);
    for index in 0..7 {
        let item_pos = dock_pos + glam::Vec2::new(index as f32 * (item_size.x + gap), 0.0);
        if rect_hit(pos, item_pos, item_size) {
            return EditorTool::from_index(index + 1);
        }
    }

    None
}

fn editor_ui_hit(pos: glam::Vec2, width: f32, height: f32) -> bool {
    let (dock_pos, item_size, gap) = editor_dock_layout(width, height);
    let dock_size = glam::Vec2::new(item_size.x * 7.0 + gap * 6.0, item_size.y);

    rect_hit(pos, dock_pos, dock_size) || editor_inspector_button_hit(pos, width, height)
}

fn editor_inspector_button_hit(pos: glam::Vec2, width: f32, height: f32) -> bool {
    let size = glam::Vec2::new(54.0, 54.0);
    let button_pos = glam::Vec2::new(width - size.x - 22.0, height * 0.5 - size.y * 0.5);

    rect_hit(pos, button_pos, size)
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
        "WORLD PORTAL" => editor_stepper_hit(pos, row_pos(82.0), row_width)
            .map(EditorInspectorAction::PortalId)
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(130.0), row_width)
                    .map(EditorInspectorAction::PortalReceiver)
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(178.0), row_width)
                    .map(EditorInspectorAction::PortalPriority)
            })
            .or_else(|| {
                editor_stepper_hit(pos, row_pos(226.0), row_width)
                    .map(|direction| EditorInspectorAction::PortalWidth(8.0 * direction as f32))
            }),
        _ => None,
    }
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
    let size = glam::Vec2::new((width * 0.24).clamp(300.0, 360.0), 276.0);
    let pos = glam::Vec2::new(width - size.x - 92.0, height * 0.5 - size.y * 0.5);

    (pos, size)
}

fn editor_dock_layout(width: f32, height: f32) -> (glam::Vec2, glam::Vec2, f32) {
    let gap = 8.0;
    let max_item_w = ((width - gap * 6.0 - 48.0) / 7.0).max(46.0);
    let item_size = glam::Vec2::new((width * 0.07).clamp(54.0, 104.0).min(max_item_w), 58.0);
    let dock_w = item_size.x * 7.0 + gap * 6.0;
    let pos = glam::Vec2::new((width - dock_w) * 0.5, height - item_size.y - 18.0);

    (pos, item_size, gap)
}

enum EditorInspectorAction {
    DoorMode,
    DoorRadius(f32),
    DoorSpeed(f32),
    PortalId(i16),
    PortalReceiver(i16),
    PortalPriority(i16),
    PortalWidth(f32),
}

fn rect_hit(pos: glam::Vec2, rect_pos: glam::Vec2, rect_size: glam::Vec2) -> bool {
    pos.x >= rect_pos.x
        && pos.x <= rect_pos.x + rect_size.x
        && pos.y >= rect_pos.y
        && pos.y <= rect_pos.y + rect_size.y
}

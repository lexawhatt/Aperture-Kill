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
            KeyCode::KeyN => self.create_editor_block(self.editor.tool),
            KeyCode::F5 => self.save_current_level(),
            _ => {}
        }
    }

    pub(super) fn handle_editor_mouse(&mut self, button: MouseButton, down: bool) {
        match (button, down) {
            (MouseButton::Left, true) => self.editor.begin_left_drag(
                self.cursor_world,
                self.modifiers.shift_key(),
                self.modifiers.control_key(),
                &mut self.world.level,
            ),
            (MouseButton::Left, false) => self.editor.end_drag(&self.world.level),
            _ => {}
        }
    }

    fn create_editor_block(&mut self, tool: EditorTool) {
        self.editor
            .create_block(self.cursor_world, tool, &mut self.world.level);
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

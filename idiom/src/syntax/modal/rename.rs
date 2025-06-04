use super::ModalMessage;
use crate::{configs::EditorAction, global_state::GlobalState, workspace::CursorPosition};

use idiom_ui::{layout::Rect, text_field::TextField};

pub struct RenameVariable {
    new_name: TextField,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        Self { new_name: TextField::new(title.to_owned()), cursor, title: format!(" Rename: {} ", title) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        2
    }

    #[inline]
    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) {
        area.get_line(0).expect("Checked").render(&self.title, &mut gs.backend);
        self.new_name.widget(area.get_line(1).expect("Checked"), &mut gs.backend);
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        // self.new_name.map_actions(action, &mut gs.clipboard);
        match action {
            EditorAction::Copy => {
                if let Some(clip) = self.new_name.copy() {
                    gs.clipboard.push(clip);
                }
            }
            EditorAction::Cut => {
                if let Some(clip) = self.new_name.cut() {
                    gs.clipboard.push(clip);
                };
            }
            EditorAction::Paste => {
                if let Some(clip) = gs.clipboard.pull() {
                    _ = self.new_name.paste_passthrough(clip);
                };
            }
            EditorAction::Char(ch) => {
                self.new_name.push_char(ch);
            }
            EditorAction::Delete => {
                self.new_name.del();
            }
            EditorAction::Backspace => {
                self.new_name.backspace();
            }
            EditorAction::EndOfLine | EditorAction::EndOfFile => {
                self.new_name.go_to_end_of_line();
            }
            EditorAction::Left => {
                self.new_name.go_left();
            }
            EditorAction::Right => {
                self.new_name.go_right();
            }
            EditorAction::SelectLeft => {
                self.new_name.select_left();
            }
            EditorAction::SelectRight => {
                self.new_name.select_right();
            }
            EditorAction::JumpLeft => {
                self.new_name.jump_left();
            }
            EditorAction::JumpRight => {
                self.new_name.jump_right();
            }
            EditorAction::JumpLeftSelect => {
                self.new_name.select_jump_left();
            }
            EditorAction::JumpRightSelect => {
                self.new_name.select_jump_right();
            }
            EditorAction::NewLine => return ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => (),
        }
        ModalMessage::Taken
    }
}

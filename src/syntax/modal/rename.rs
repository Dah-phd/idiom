use super::ModalMessage;
use crate::{global_state::GlobalState, render::TextField, utils::BORDERED_BLOCK, workspace::CursorPosition};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::Rect, Frame};

pub struct RenameVariable {
    new_name: TextField<()>,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        Self { new_name: TextField::basic(title.to_owned()), cursor, title: format!("Rename: {} ", title) }
    }

    pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let block = BORDERED_BLOCK.title(self.title.as_str());
        frame.render_widget(self.new_name.widget().block(block), area);
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        self.new_name.map(key, &mut gs.clipboard);
        match key.code {
            KeyCode::Enter => ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => ModalMessage::Taken,
        }
    }
}

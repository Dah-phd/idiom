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

    pub fn len(&self) -> usize {
        2
    }

    pub fn render(&mut self, area: &crate::render::layout::Rect, gs: &mut GlobalState) -> std::io::Result<()> {
        area.get_line(0).unwrap().render(" Rename:", &mut gs.writer)?;
        self.new_name.widget(area.get_line(1).unwrap(), &mut gs.writer)
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        self.new_name.map(key, &mut gs.clipboard);
        match key.code {
            KeyCode::Enter => ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => ModalMessage::Taken,
        }
    }
}

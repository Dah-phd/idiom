use super::ModalMessage;
use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    render::{layout::Rect, TextField},
    workspace::CursorPosition,
};

pub struct RenameVariable {
    new_name: TextField<()>,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        Self { new_name: TextField::basic(title.to_owned()), cursor, title: format!(" Rename: {} ", title) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        2
    }

    #[inline]
    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) {
        area.get_line(0).unwrap().render(&self.title, &mut gs.writer);
        self.new_name.widget(area.get_line(1).unwrap(), &mut gs.writer);
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        self.new_name.map_actions(action, &mut gs.clipboard);
        match action {
            EditorAction::NewLine => ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => ModalMessage::Taken,
        }
    }
}

use super::ModalMessage;
use crate::{
    configs::EditorAction, ext_tui::text_field::TextField, global_state::GlobalState, workspace::CursorPosition,
};
use idiom_tui::layout::Rect;

pub struct RenameVariable {
    new_name: TextField<()>,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        let mut new_name = TextField::basic(title.to_owned());
        new_name.select_all();
        Self { new_name, cursor, title: format!(" Rename: {} ", title) }
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
        self.new_name.map_actions(action, &mut gs.clipboard);
        match action {
            EditorAction::NewLine => ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => ModalMessage::Taken,
        }
    }

    pub fn mouse_click(&mut self, rel_char: usize) {
        if let Some(checked_rel_char) = rel_char.checked_sub(4) {
            self.new_name.click_char(checked_rel_char);
        }
    }
}

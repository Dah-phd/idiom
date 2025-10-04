use super::{ModalAction, ModalMessage};
use crate::{
    configs::EditorAction,
    ext_tui::{text_field::map_action, StyleExt},
    global_state::GlobalState,
    workspace::CursorPosition,
};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Rect, text_field::TextField};

const TEXT_FIELD_RENDER_OFFSET: usize = 4;

pub struct RenameVariable {
    new_name: TextField,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        let mut new_name = TextField::new(title.to_owned());
        new_name.select_all();
        Self { new_name, cursor, title: format!(" Rename: {title} ") }
    }

    #[inline]
    pub fn len(&self) -> usize {
        2
    }

    #[inline]
    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) {
        area.get_line(0).expect("Checked").render(&self.title, &mut gs.backend);
        self.new_name.widget(
            area.get_line(1).expect("Checked"),
            ContentStyle::reversed(),
            gs.ui_theme.accent_style_reversed(),
            gs.backend(),
        );
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        map_action(&mut self.new_name, action, &mut gs.clipboard);
        match action {
            EditorAction::NewLine => {
                ModalMessage::Action(ModalAction::Rename(self.new_name.as_str().to_owned(), self.cursor))
            }
            _ => ModalMessage::Taken,
        }
    }

    pub fn mouse_click(&mut self, rel_char: usize) {
        if let Some(checked_rel_char) = rel_char.checked_sub(TEXT_FIELD_RENDER_OFFSET) {
            if self.new_name.cursor() == checked_rel_char {
                self.new_name.select_token_at_cursor();
            } else {
                self.new_name.cursor_set(checked_rel_char);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{RenameVariable, TEXT_FIELD_RENDER_OFFSET};
    use crate::workspace::CursorPosition;

    #[test]
    fn mause() {
        let mut modal = RenameVariable::new(CursorPosition::default(), "test_var");
        assert_eq!(modal.new_name.select(), Some((0, 8)));
        modal.mouse_click(4 + TEXT_FIELD_RENDER_OFFSET);
        assert_eq!(4, modal.new_name.cursor());
        assert_eq!(None, modal.new_name.select());
    }
}

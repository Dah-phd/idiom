use super::{ModalAction, ModalMessage};
use crate::{
    configs::EditorAction, ext_tui::text_field::TextField, global_state::GlobalState, workspace::CursorPosition,
};
use idiom_tui::layout::Rect;

const TEXT_FIELD_RENDER_OFFSET: usize = 4;

pub struct RenameVariable {
    new_name: TextField<()>,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    pub fn new(cursor: CursorPosition, title: &str) -> Self {
        let mut new_name = TextField::basic(title.to_owned());
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
        self.new_name.widget(area.get_line(1).expect("Checked"), &mut gs.backend);
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        self.new_name.map_actions(action, &mut gs.clipboard);
        match action {
            EditorAction::NewLine => ModalMessage::Action(ModalAction::Rename(self.new_name.to_string(), self.cursor)),
            _ => ModalMessage::Taken,
        }
    }

    pub fn mouse_click(&mut self, rel_char: usize) {
        if let Some(checked_rel_char) = rel_char.checked_sub(TEXT_FIELD_RENDER_OFFSET) {
            self.new_name.click_char(checked_rel_char);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{RenameVariable, TEXT_FIELD_RENDER_OFFSET};
    use crate::ext_tui::text_field::test::{pull_char, pull_select};
    use crate::workspace::CursorPosition;

    #[test]
    fn mause() {
        let mut modal = RenameVariable::new(CursorPosition::default(), "test_var");
        assert_eq!(pull_select(&modal.new_name), Some((0, 8)));
        modal.mouse_click(4 + TEXT_FIELD_RENDER_OFFSET);
        assert_eq!(4, pull_char(&modal.new_name));
        assert_eq!(None, pull_select(&modal.new_name));
    }
}

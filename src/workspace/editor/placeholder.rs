use crate::{global_state::GlobalState, render};

use super::{plain::TextEditor, Editor};

pub enum SelectedEditor {
    Code(Editor),
    Plain(TextEditor),
    None,
}

impl SelectedEditor {
    fn select_text(&mut self, new: TextEditor) -> Self {
        std::mem::replace(self, Self::Plain(new))
    }

    fn select_code(&mut self, new: Editor) -> Self {
        std::mem::replace(self, Self::Code(new))
    }

    fn map(&mut self, gs: &mut GlobalState) {
        todo!()
    }

    fn redner(&mut self, gs: &mut GlobalState) {
        match self {
            Self::Code(editor) => editor.render(gs),
            Self::Plain(editor) => editor.render(gs),
            _ => (),
        }
    }
}

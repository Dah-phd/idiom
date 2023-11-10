use crate::{
    configs::EditorConfigs,
    syntax::{Lexer, Theme},
};

use super::{action::ActionLogger, clipboard::Clipboard, CursorPosition, Editor};

pub fn mock_editor(content: Vec<String>) -> Editor {
    let file_type = crate::configs::FileType::Rust;
    Editor {
        cursor: super::CursorPosition::default(),
        lexer: Lexer::from_type(&file_type, Theme::default()),
        file_type,
        path: "".into(),
        at_line: 0,
        select: super::Select::None,
        configs: EditorConfigs::default(),
        clipboard: Clipboard::default(),
        action_logger: ActionLogger::default(),
        max_rows: 0,
        content,
    }
}

pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &Editor) -> bool {
    if let Some((from, to)) = editor.select.get() {
        return &select.0 == from && &select.1 == to;
    }
    false
}

pub fn pull_line(editor: &Editor, idx: usize) -> Option<&String> {
    editor.content.get(idx)
}

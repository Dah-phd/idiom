use super::super::{
    cursor::{Cursor, CursorPosition},
    Editor,
};
use crate::configs::FileType;
use crate::global_state::GlobalState;
use crate::syntax::Lexer;
use crate::workspace::actions::Actions;
use std::path::PathBuf;

pub fn mock_editor(content: Vec<String>) -> Editor {
    let ft = FileType::Unknown;
    let path = PathBuf::from("");
    let mut gs = GlobalState::new(80, 120);
    Editor {
        lexer: Lexer::with_context(ft, &path, &content, &mut gs),
        file_type: ft,
        display: "".to_string(),
        path,
        timestamp: None,
        cursor: Cursor::default(),
        actions: Actions::default(),
        content,
    }
}

pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &Editor) -> bool {
    if let Some((p1, p2)) = editor.cursor.select_get() {
        return p1 == select.0 && p2 == select.1;
    }
    false
}

pub fn pull_line(editor: &Editor, idx: usize) -> Option<&String> {
    editor.content.get(idx)
}

use super::actions::Actions;
use super::CursorPosition;
use super::{cursor::Cursor, Editor};
use crate::configs::FileType;
use crate::events::Events;
use crate::syntax::Theme;
use crate::{configs::EditorConfigs, syntax::Lexer};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub fn mock_editor(content: Vec<String>) -> Editor {
    let ft = FileType::Unknown;
    Editor {
        lexer: Lexer::with_context(ft, Theme::default(), &Rc::new(RefCell::new(Events::default()))),
        file_type: ft,
        display: "".to_string(),
        path: PathBuf::from(""),
        cursor: Cursor::default(),
        actions: Actions::default(),
        max_rows: 0,
        content,
    }
}

pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &Editor) -> bool {
    if let Some((p1, p2)) = editor.cursor.select_get() {
        return p1 == &select.0 && p2 == &select.1;
    }
    false
}

pub fn pull_line(editor: &Editor, idx: usize) -> Option<&String> {
    editor.content.get(idx)
}

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
    let cfg = EditorConfigs::default();
    let ft = FileType::Unknown;
    let theme = Theme::default();
    Editor {
        lexer: Lexer::with_context(ft, theme, &Rc::new(RefCell::new(Events::default()))),
        at_line: 0,
        file_type: ft,
        display: "".to_string(),
        path: PathBuf::from(""),
        cursor: Cursor::new(cfg),
        max_rows: 0,
        content,
    }
}
pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &Editor) -> bool {
    if let Some((p1, p2)) = editor.cursor.select.get() {
        return p1 == &select.0 && p2 == &select.1;
    }
    false
}
pub fn pull_line(editor: &Editor, idx: usize) -> Option<&String> {
    editor.content.get(idx)
}

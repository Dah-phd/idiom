use super::super::{
    cursor::{Cursor, CursorPosition},
    CodeEditor,
};
use crate::global_state::GlobalState;
use crate::render::backend::{Backend, BackendProtocol};
use crate::syntax::Lexer;
use crate::workspace::{actions::Actions, line::CodeLine};
use crate::{configs::FileType, workspace::editor::build_display};
use std::path::PathBuf;

pub fn mock_editor(content: Vec<String>) -> CodeEditor {
    let ft = FileType::Rust;
    let path = PathBuf::from("");
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let content: Vec<CodeLine> = content.into_iter().map(CodeLine::from).collect();
    CodeEditor {
        line_number_offset: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::with_context(ft, &path, &mut gs),
        file_type: ft,
        display: "".to_string(),
        path,
        timestamp: None,
        cursor: Cursor::default(),
        actions: Actions::default(),
        content,
        last_render_at_line: None,
    }
}

pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &CodeEditor) -> bool {
    if let Some((p1, p2)) = editor.cursor.select_get() {
        return p1 == select.0 && p2 == select.1;
    }
    false
}

pub fn pull_line(editor: &CodeEditor, idx: usize) -> Option<String> {
    editor.content.get(idx).map(|line| line.to_string())
}

#[test]
fn test_update_path() {
    let mut editor = mock_editor(vec![]);
    editor.path = PathBuf::from("./src/workspace/editor/mod.rs");
    assert!(editor.update_path(PathBuf::from("./src/workspace/editor/test.rs")).is_ok());
    assert_eq!(editor.display, "editor/test.rs");
}

#[test]
fn test_display() {
    let buf = PathBuf::from("./src/workspace/editor/mod.rs").canonicalize().unwrap();
    assert_eq!(build_display(buf.as_path()), "editor/mod.rs");
    assert_eq!(build_display(PathBuf::from("bumba").as_path()), "bumba");
}

use super::super::{
    cursor::{Cursor, CursorPosition},
    editor::{utils::build_display, FileUpdate},
    Editor,
};
use crate::global_state::GlobalState;
use crate::render::backend::{Backend, BackendProtocol};
use crate::syntax::Lexer;
use crate::workspace::{actions::Actions, line::EditorLine};
use crate::{configs::FileType, workspace::renderer::Renderer};
use std::path::PathBuf;

pub fn mock_editor(content: Vec<String>) -> Editor {
    let ft = FileType::Rust;
    let path = PathBuf::from("");
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let content: Vec<EditorLine> = content.into_iter().map(EditorLine::from).collect();
    Editor {
        line_number_offset: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::with_context(ft, &path, &mut gs),
        file_type: ft,
        display: "".to_string(),
        path,
        update_status: FileUpdate::None,
        cursor: Cursor::default(),
        actions: Actions::default(),
        content,
        renderer: Renderer::code(),
        last_render_at_line: None,
    }
}

pub fn select_eq(select: (CursorPosition, CursorPosition), editor: &Editor) -> bool {
    if let Some((p1, p2)) = editor.cursor.select_get() {
        return p1 == select.0 && p2 == select.1;
    }
    false
}

pub fn pull_line(editor: &Editor, idx: usize) -> Option<String> {
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

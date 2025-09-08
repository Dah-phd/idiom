use super::super::{
    cursor::{Cursor, CursorPosition},
    editor::{utils::build_display, FileUpdate},
    Editor,
};
use super::{calc_line_number_offset, controls};
use crate::{
    configs::FileType,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{actions::Actions, line::EditorLine, renderer::Renderer},
};
use idiom_tui::{layout::Rect, Backend};
use std::path::PathBuf;

pub fn mock_editor(content: Vec<String>) -> Editor {
    let ft = FileType::Rust;
    let path = PathBuf::from("test-path");
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), crate::ext_tui::CrossTerm::init());
    let content: Vec<EditorLine> = content.into_iter().map(EditorLine::from).collect();
    Editor {
        line_number_offset: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::with_context(ft, &path, &mut gs),
        file_type: ft,
        display: "".to_string(),
        path,
        update_status: FileUpdate::None,
        cursor: Cursor::default(),
        multi_positions: vec![],
        actions: Actions::default(),
        controls: controls::ControlMap::default(),
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

#[test]
fn test_line_number_calcs() {
    let content = (0..3).collect::<Vec<_>>();
    let expect = if content.is_empty() { 1 } else { (content.len().ilog10() + 1) as usize }; // 1
    let result = calc_line_number_offset(content.len());
    assert_eq!(result, 1);
    assert_eq!(result, expect);
    let bigger_content = (0..10).collect::<Vec<_>>(); // over 10 elements
    let expect = if bigger_content.is_empty() { 1 } else { (bigger_content.len().ilog10() + 1) as usize }; // 2
    let result = calc_line_number_offset(bigger_content.len());
    assert_eq!(result, 2);
    assert_eq!(result, expect);
}

#[test]
fn merge_multi_cursors() {
    fn make_cursor(line: usize, char: usize) -> Cursor {
        let mut cursor = Cursor::default();
        cursor.at_line = line;
        cursor.line = line;
        cursor.char = char;
        cursor
    }

    let mut editor = mock_editor(vec![]);
    editor.multi_positions.extend([
        Cursor::default(),
        Cursor::default(),
        make_cursor(2, 2),
        make_cursor(2, 2),
        make_cursor(3, 2),
        make_cursor(3, 3),
        make_cursor(4, 2),
    ]);
    controls::consolidate_cursors(&mut editor);
    assert_eq!(
        editor.multi_positions,
        vec![
            make_cursor(4, 2),
            make_cursor(3, 3),
            make_cursor(3, 2),
            make_cursor(2, 2),
            Cursor::default(),
        ]
    );
}

#[test]
fn filter_per_line_if_no_select() {
    fn with_select(from: CursorPosition, to: CursorPosition) -> Cursor {
        let mut cursor = Cursor::default();
        cursor.select_set(from, to);
        cursor
    }

    let mut main_cursor = Cursor::default();
    main_cursor.set_position(CursorPosition { line: 10, char: 9 });
    main_cursor.max_rows = 100;
    let expect_main = main_cursor.clone();

    let mut second_no_select = Cursor::default();
    second_no_select.set_position(CursorPosition { line: 2, char: 12 });
    let mut exepct_second = second_no_select.clone();

    let mut second_cursor = Cursor::default();
    second_cursor.select_set(CursorPosition { line: 1, char: 3 }, CursorPosition { line: 2, char: 10 });
    second_cursor.max_rows = 99;
    exepct_second.max_rows = second_cursor.max_rows;

    let mut editor = mock_editor(vec![]);
    editor.multi_positions = vec![
        with_select(CursorPosition { line: 11, char: 9 }, CursorPosition { line: 11, char: 10 }),
        with_select(CursorPosition { line: 11, char: 3 }, CursorPosition { line: 11, char: 8 }),
        with_select(CursorPosition { line: 10, char: 12 }, CursorPosition { line: 10, char: 15 }),
        main_cursor,
        with_select(CursorPosition { line: 12, char: 2 }, CursorPosition { line: 10, char: 8 }),
        with_select(CursorPosition { line: 6, char: 2 }, CursorPosition { line: 6, char: 8 }),
        with_select(CursorPosition { line: 3, char: 2 }, CursorPosition { line: 3, char: 8 }),
        second_no_select,
        second_cursor,
        Cursor::default(),
    ];

    let cursors = controls::filter_multi_cursors_per_line_if_no_select(&editor);
    assert_eq!(
        cursors,
        vec![
            with_select(CursorPosition { line: 11, char: 9 }, CursorPosition { line: 11, char: 10 }),
            with_select(CursorPosition { line: 11, char: 3 }, CursorPosition { line: 11, char: 8 }),
            expect_main,
            with_select(CursorPosition { line: 6, char: 2 }, CursorPosition { line: 6, char: 8 }),
            with_select(CursorPosition { line: 3, char: 2 }, CursorPosition { line: 3, char: 8 }),
            exepct_second,
            Cursor::default(),
        ]
    );
}

use super::super::editor::{utils::build_display, FileUpdate};
use super::{
    calc_line_number_offset,
    controls::{filter_multi_cursors_per_line_if_no_select, ControlMap},
    Editor, EditorModal, TuiCodec,
};
use crate::{
    actions::Actions,
    configs::FileType,
    cursor::{Cursor, CursorPosition, WordRange},
    editor_line::EditorLine,
    ext_tui::CrossTerm,
    global_state::GlobalState,
    syntax::Lexer,
};
use idiom_tui::{layout::Rect, Backend};
use std::path::PathBuf;

pub fn mock_editor(content: Vec<String>) -> Editor {
    let ft = FileType::Rust;
    let path = PathBuf::from("test-path");
    let content: Vec<EditorLine> = content.into_iter().map(EditorLine::from).collect();
    Editor {
        line_number_padding: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::with_context(ft, &path),
        file_type: ft,
        display: "".to_string(),
        path,
        update_status: FileUpdate::None,
        cursor: Cursor::default(),
        modal: EditorModal::default(),
        actions: Actions::default(),
        controls: ControlMap::default(),
        content,
        renderer: TuiCodec::code(),
        last_render_at_line: None,
    }
}

pub fn mock_editor_text_render(content: Vec<String>) -> Editor {
    let ft = FileType::Text;
    let path = PathBuf::from("test-path");
    let content: Vec<EditorLine> = content.into_iter().map(EditorLine::from).collect();
    Editor {
        line_number_padding: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::text_lexer(&path),
        file_type: ft,
        display: "".to_string(),
        path,
        update_status: FileUpdate::None,
        cursor: Cursor::default(),
        modal: EditorModal::default(),
        actions: Actions::default(),
        controls: ControlMap::default(),
        content,
        renderer: TuiCodec::text(),
        last_render_at_line: None,
    }
}

pub fn mock_editor_md_render(content: Vec<String>) -> Editor {
    let ft = FileType::MarkDown;
    let path = PathBuf::from("test-path");
    let content: Vec<EditorLine> = content.into_iter().map(EditorLine::from).collect();
    Editor {
        line_number_padding: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
        lexer: Lexer::md_lexer(&path),
        file_type: ft,
        display: "".to_string(),
        path,
        update_status: FileUpdate::None,
        cursor: Cursor::default(),
        modal: EditorModal::default(),
        actions: Actions::default(),
        controls: ControlMap::default(),
        content,
        renderer: TuiCodec::markdown(),
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

fn make_cursor(line: usize, char: usize) -> Cursor {
    let mut cursor = Cursor::default();
    cursor.at_line = line;
    cursor.line = line;
    cursor.char = char;
    cursor.text_width = 120;
    cursor
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
    let buf = PathBuf::from("./src/editor/mod.rs").canonicalize().unwrap();
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
    let mut editor = mock_editor(vec![]);
    editor.controls.mock_update_cursors().extend([
        Cursor::default(),
        Cursor::default(),
        make_cursor(2, 2),
        make_cursor(2, 2),
        make_cursor(3, 2),
        make_cursor(3, 3),
        make_cursor(4, 2),
    ]);
    ControlMap::consolidate_cursors(&mut editor);
    assert_eq!(
        editor.controls.cursors(),
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
    editor.controls.mock_cursors(vec![
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
    ]);

    let cursors = filter_multi_cursors_per_line_if_no_select(&editor);
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

#[test]
fn token_if_already_selected() {
    let mut editor = mock_editor(vec![
        String::from("let word = \"bird\";"),
        String::from("println!(\"{:?}\", &word);"),
        String::from("let is_there = word.contins(\"word\");"),
        String::from("if word.starts_with(\"bird\") {"),
        String::from("    println!(\"🦀 end: {}\", &word);"),
        String::from("} // not a __word__"),
    ]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let pos = CursorPosition { line: 3, char: 4 };
    editor.cursor.set_position(pos);
    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    let range = WordRange::find_at(&editor.content, editor.cursor.get_position()).unwrap();
    assert_eq!(Some(range.as_select()), editor.cursor.select_get());

    let mut expected = vec![
        (CursorPosition { line: 4, char: 27 }, CursorPosition { line: 4, char: 31 }),
        (CursorPosition { line: 3, char: 3 }, CursorPosition { line: 3, char: 7 }),
    ];

    // second invoke
    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    assert_eq!(editor.controls.cursors().len(), 2);
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);

    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    expected.push((CursorPosition { line: 0, char: 4 }, CursorPosition { line: 0, char: 8 }));
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);

    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    expected.insert(expected.len() - 1, (CursorPosition { line: 1, char: 18 }, CursorPosition { line: 1, char: 22 }));
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);

    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    expected.insert(expected.len() - 2, (CursorPosition { line: 2, char: 15 }, CursorPosition { line: 2, char: 19 }));
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);

    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    expected.insert(expected.len() - 3, (CursorPosition { line: 2, char: 29 }, CursorPosition { line: 2, char: 33 }));
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);

    _ = editor.map(crate::configs::EditorAction::SelectToken, &mut gs);
    assert_eq!(editor.controls.cursors().iter().flat_map(|c| c.select_get()).collect::<Vec<_>>(), expected);
}

#[test]
fn test_token_next() {
    let text_line = EditorLine::from("a=split here");
    assert_eq!(text_line.get(2, 7), Some("split"));
    assert_eq!(text_line.get_from(7), Some(" here"));
    assert_eq!(text_line.get_to(2), Some("a="));
}

#[test]
fn test_apply() {
    let mut editor = mock_editor(vec![
        "this is line".to_owned(),
        "this is long line with many many cursors".to_owned(),
        "this is line not so long".to_owned(),
        "this is line".to_owned(),
    ]);

    // cursor consolidated
    editor.controls.mock_update_cursors().extend([
        make_cursor(3, 4),
        make_cursor(2, 17),
        make_cursor(2, 4),
        make_cursor(1, 18),
        make_cursor(1, 10),
        make_cursor(1, 4),
        make_cursor(0, 4),
    ]);

    editor.apply(|act, lex, cont, cur| {
        let from = cur.get_position();
        let to = CursorPosition { line: from.line, char: from.char + 2 };
        cur.select_set(from, to);
        let text = crate::workspace::utils::copy_content(from, to, cont).to_uppercase();
        act.replace_select(from, to, text, cur, cont, lex);
    });

    let result = editor.content.iter().map(|el| el.as_str()).collect::<Vec<_>>();

    assert_eq!(
        result,
        vec![
            "this Is line",
            "this Is loNG line WIth many many cursors",
            "this Is line not SO long",
            "this Is line"
        ]
    );
}

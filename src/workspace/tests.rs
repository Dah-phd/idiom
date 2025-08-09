use super::{
    editor::Editor,
    line::EditorLine,
    map_editor,
    utils::{clip_content, copy_content, get_closing_char_from_context, insert_clip, remove_content},
    Workspace,
};
use crate::{
    configs::{test::mock_editor_key_map, EditorConfigs},
    ext_tui::CrossTerm,
    global_state::GlobalState,
    workspace::{
        actions::tests::create_content,
        editor::code_tests::{mock_editor, pull_line, select_eq},
        Cursor, CursorPosition,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Rect, Backend};
use std::collections::HashMap;

pub fn mock_ws(content: Vec<String>) -> Workspace {
    let mut ws = Workspace {
        editors: vec![mock_editor(content)].into(),
        base_configs: EditorConfigs::default(),
        key_map: mock_editor_key_map(),
        lsp_servers: HashMap::default(),
        map_callback: map_editor,
        tab_style: ContentStyle::default(),
    };
    ws.resize_all(60, 90);
    ws
}

pub fn mock_ws_empty() -> Workspace {
    Workspace {
        editors: vec![].into(),
        base_configs: EditorConfigs::default(),
        key_map: mock_editor_key_map(),
        lsp_servers: HashMap::default(),
        map_callback: map_editor,
        tab_style: ContentStyle::default(),
    }
}

fn base_ws() -> Workspace {
    mock_ws(vec![
        "hello world!".to_owned(),
        "next line".to_owned(),
        "     ".to_owned(),
        "really long line here".to_owned(),
        "short one here".to_owned(),
        "test msg".to_owned(),
    ])
}

fn active(ws: &mut Workspace) -> &mut Editor {
    ws.get_active().unwrap()
}

fn press(ws: &mut Workspace, code: KeyCode, gs: &mut GlobalState) {
    ws.map(&KeyEvent::new(code, KeyModifiers::empty()), gs);
}

fn shift_press(ws: &mut Workspace, code: KeyCode, gs: &mut GlobalState) {
    ws.map(&KeyEvent::new(code, KeyModifiers::SHIFT), gs);
}

fn ctrl_press(ws: &mut Workspace, code: KeyCode, gs: &mut GlobalState) {
    ws.map(&KeyEvent::new(code, KeyModifiers::CONTROL), gs);
}

fn ctrl_shift_press(ws: &mut Workspace, code: KeyCode, gs: &mut GlobalState) {
    ws.map(&KeyEvent::new(code, KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)), gs);
}

fn assert_position(ws: &mut Workspace, position: CursorPosition) {
    let current: CursorPosition = (&active(ws).cursor).into();
    assert_eq!(current, position);
}

/// UTILS

#[test]
fn test_insert_clip() {
    // ensure utf8 safety on critical func
    let mut content = vec![EditorLine::new("one🚀line".to_owned())];
    let cursor = CursorPosition { line: 0, char: 4 };
    let cursor = insert_clip("first\nsecond\nrocket🚀", &mut content, cursor);
    assert_eq!(&content[0].to_string(), "one🚀first");
    assert_eq!(&content[1].to_string(), "second");
    assert_eq!(&content[2].to_string(), "rocket🚀line");
    assert_eq!(cursor, CursorPosition { line: 2, char: 7 });
    // single line
    let cursor = insert_clip("single", &mut content, cursor);
    assert_eq!(&content[2].to_string(), "rocket🚀singleline");
    assert_eq!(cursor, CursorPosition { line: 2, char: 13 });
    // end on new line
    let cursor = insert_clip("text\n", &mut content, CursorPosition { line: 0, char: 0 });
    assert_eq!(&content[0].to_string(), "text");
    assert_eq!(&content[1].to_string(), "one🚀first");
    assert_eq!(cursor, CursorPosition { line: 1, char: 0 });
}

#[test]
fn test_remove_content() {
    // ensure utf8 safety on critical func
    let mut content = create_content();
    let from = CursorPosition { line: 4, char: 1 };
    remove_content(from, CursorPosition { line: 5, char: 15 }, &mut content);
    assert_eq!(&content[4].to_string(), "🚀 everywhere in the end");
    // within line
    remove_content(from, CursorPosition { line: 4, char: 10 }, &mut content);
    assert_eq!(&content[4].to_string(), "🚀re in the end");
    // end on new line
    remove_content(from, CursorPosition { line: 5, char: 0 }, &mut content);
    assert_eq!(&content[4].to_string(), "🚀i will have to have some scopes {");
}

#[test]
fn test_clip_content() {
    // ensure utf8 safety on critical func
    let mut content = create_content();
    let from = CursorPosition { line: 4, char: 1 };
    let clip = clip_content(from, CursorPosition { line: 5, char: 15 }, &mut content);
    assert_eq!(&content[4].to_string(), "🚀 everywhere in the end");
    assert_eq!(&clip, " things will get really complicated especially with all the utf8 chars and utf16 pos encoding\nthere will be 🚀");
    // within line
    let clip2 = clip_content(from, CursorPosition { line: 4, char: 10 }, &mut content);
    assert_eq!(&content[4].to_string(), "🚀re in the end");
    assert_eq!(&clip2, " everywhe");
    // end on new line
    let clip3 = clip_content(CursorPosition { line: 0, char: 0 }, CursorPosition { line: 1, char: 0 }, &mut content);
    assert_eq!(&content[0].to_string(), "more lines of code should be here but only text");
    assert_eq!(&clip3, "here comes the text\n");
}

#[test]
fn test_copy_content() {
    let content = create_content();
    let clip = copy_content(CursorPosition { line: 4, char: 1 }, CursorPosition { line: 5, char: 15 }, &content);
    assert_eq!(&clip, " things will get really complicated especially with all the utf8 chars and utf16 pos encoding\nthere will be 🚀");
    // within line
    let clip2 = copy_content(CursorPosition { line: 5, char: 14 }, CursorPosition { line: 5, char: 15 }, &content);
    assert_eq!(&clip2, "🚀");
    // end on new line
    let clip3 = copy_content(CursorPosition { line: 0, char: 0 }, CursorPosition { line: 1, char: 0 }, &content);
    assert_eq!(&clip3, "here comes the text\n");
}

#[test]
fn get_closing_context() {
    for (brack, rev) in "{([\"'".chars().zip("})]\"'".chars()) {
        for next_c in ";:.=, {}[]()".chars() {
            let text = EditorLine::from(format!(" {next_c} "));
            assert_eq!(Some(rev), get_closing_char_from_context(brack, &text, 1));
        }
        for next_c in ";:.=, {}[]()".chars() {
            let text = EditorLine::from(format!("{next_c}{next_c} "));
            assert_eq!(Some(rev), get_closing_char_from_context(brack, &text, 1));
        }
    }
    for next_c in "asdb1234".chars() {
        let text = EditorLine::from(format!(" {next_c} "));
        assert_eq!(None, get_closing_char_from_context('[', &text, 1));
    }
    for next_c in "asdb1234".chars() {
        let text = EditorLine::from(format!("{next_c}  "));
        assert_eq!(Some(')'), get_closing_char_from_context('(', &text, 1));
    }
    for next_c in "asdb1234".chars() {
        let text = EditorLine::from(format!("{next_c}  "));
        assert_eq!(None, get_closing_char_from_context('"', &text, 1));
    }
    for sandwich_c in "bamwqer_235".chars() {
        let text = EditorLine::from(format!("{sandwich_c}{sandwich_c} "));
        assert_eq!(None, get_closing_char_from_context('"', &text, 1));
    }
}

/// ACTIONS

#[test]
fn test_open_scope() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Char(' '), &mut gs);
    press(&mut ws, KeyCode::Left, &mut gs);
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    press(&mut ws, KeyCode::Char('['), &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 4, line: 1 });
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "{([");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "    ");
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "])} hello world!");
}

#[test]
fn test_block_closing() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    press(&mut ws, KeyCode::Char('['), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "{([hello world!");
    assert_position(&mut ws, CursorPosition { char: 3, line: 0 });
}

#[test]
fn test_allow_closing() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    press(&mut ws, KeyCode::Left, &mut gs);
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    press(&mut ws, KeyCode::Char('['), &mut gs);
    press(&mut ws, KeyCode::Char('='), &mut gs);
    press(&mut ws, KeyCode::Left, &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    press(&mut ws, KeyCode::Char(';'), &mut gs);
    press(&mut ws, KeyCode::Char('['), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "{([(;[])=])}{hello world!");
    assert_position(&mut ws, CursorPosition { char: 6, line: 0 });
}

#[test]
fn test_block_quotes() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    press(&mut ws, KeyCode::Char('"'), &mut gs);
    press(&mut ws, KeyCode::Char('\''), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "\"'hello world!");
    assert_position(&mut ws, CursorPosition { char: 2, line: 0 });
}

#[test]
fn test_allow_quotes() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    press(&mut ws, KeyCode::Char(':'), &mut gs);
    press(&mut ws, KeyCode::Char(';'), &mut gs);
    press(&mut ws, KeyCode::Left, &mut gs);
    press(&mut ws, KeyCode::Char('"'), &mut gs);
    press(&mut ws, KeyCode::Char('.'), &mut gs);
    press(&mut ws, KeyCode::Char(','), &mut gs);
    press(&mut ws, KeyCode::Left, &mut gs);
    press(&mut ws, KeyCode::Char('\''), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), ":\".'',\";hello world!");
    assert_position(&mut ws, CursorPosition { char: 4, line: 0 });
}

#[test]
fn test_move() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 0, line: 1 });
    press(&mut ws, KeyCode::End, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 9, line: 1 });
    press(&mut ws, KeyCode::Right, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 0, line: 2 });
    press(&mut ws, KeyCode::Left, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 9, line: 1 });
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::End, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 21, line: 3 });
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 14, line: 4 });
    press(&mut ws, KeyCode::Left, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 13, line: 4 });
    press(&mut ws, KeyCode::Right, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 14, line: 4 });
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 8, line: 5 });
    press(&mut ws, KeyCode::Up, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 14, line: 4 });
}

#[test]
fn move_checks() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    let base_line = active(&mut ws).content[0].content.to_string();
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), base_line);
    press(&mut ws, KeyCode::End, &mut gs);
    assert_position(&mut ws, CursorPosition { char: base_line.len(), line: 0 });
    press(&mut ws, KeyCode::Right, &mut gs);
    assert_position(&mut ws, CursorPosition { char: 0, line: 1 });
    press(&mut ws, KeyCode::Left, &mut gs);
    assert_position(&mut ws, CursorPosition { char: base_line.len(), line: 0 });
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    assert!(select_eq(
        (CursorPosition { char: base_line.len() - 1, line: 0 }, CursorPosition { char: base_line.len(), line: 0 }),
        active(&mut ws)
    ));
    let mut test_cursor = Cursor::default();
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() - 1 },
        CursorPosition { line: 0, char: base_line.len() },
    );
    assert_eq!(test_cursor.char, base_line.len());
    assert!(test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() },
        CursorPosition { line: 0, char: base_line.len() - 1 },
    );
    assert_eq!(test_cursor.char, base_line.len() - 1);
    assert!(test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() },
        CursorPosition { line: 0, char: base_line.len() + 1 },
    );
    assert_eq!(test_cursor.char, base_line.len() + 1);
    assert!(!test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() + 1 },
        CursorPosition { line: 0, char: base_line.len() },
    );
    assert_eq!(test_cursor.char, base_line.len());
    assert!(!test_cursor.matches_content(&active(&mut ws).content));
}

#[test]
fn test_select() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 0, char: 12 }), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Right, &mut gs);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    assert!(select_eq((CursorPosition::default(), CursorPosition { char: 9, line: 1 }), active(&mut ws)));
    press(&mut ws, KeyCode::Left, &mut gs);
    shift_press(&mut ws, KeyCode::Right, &mut gs);
    assert!(select_eq((CursorPosition { char: 8, line: 1 }, CursorPosition { char: 9, line: 1 }), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    assert!(select_eq((CursorPosition { char: 7, line: 1 }, CursorPosition { char: 8, line: 1 }), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Up, &mut gs);
    assert!(select_eq((CursorPosition { char: 7, line: 0 }, CursorPosition { char: 8, line: 1 }), active(&mut ws)));
}

#[test]
fn select_checks() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());

    gs.insert_mode();
    let last_line = active(&mut ws).content.len() - 1;
    let base_line = active(&mut ws).content[last_line].to_string();
    let mut test_cursor = Cursor::default();
    let all = (CursorPosition::default(), CursorPosition { char: base_line.len(), line: last_line });

    ctrl_press(&mut ws, KeyCode::Char('a'), &mut gs);
    assert_position(&mut ws, CursorPosition { char: base_line.len(), line: last_line });
    assert!(select_eq(all, active(&mut ws)));
    active(&mut ws).cursor.select_set(all.1, all.0); // swap select 'from' and 'to'
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    assert!(select_eq((CursorPosition { line: 1, char: 0 }, all.1), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    assert!(select_eq(
        (CursorPosition { line: 0, char: active(&mut ws).content[0].char_len() }, all.1),
        active(&mut ws)
    ));
    test_cursor.select_set(CursorPosition { line: 0, char: active(&mut ws).content[0].char_len() }, all.1);
    assert!(test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(CursorPosition { line: 0, char: active(&mut ws).content[0].char_len() + 1 }, all.1);
    assert!(!test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(all.0, all.1);
    assert!(test_cursor.matches_content(&active(&mut ws).content));
    test_cursor.select_set(all.0, CursorPosition { line: all.1.line + 1, char: 0 });
    assert!(!test_cursor.matches_content(&active(&mut ws).content));
}

#[test]
fn test_chars() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Char('n'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nhello world!");
    press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(ello world!");
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh({ello world!");
    press(&mut ws, KeyCode::Char('['), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh({[ello world!");
    press(&mut ws, KeyCode::Char('"'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh({[\"ello world!");
    press(&mut ws, KeyCode::Char('\''), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh({[\"'ello world!");
}

#[test]
fn test_new_line() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "hello world!");
    press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "h");
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "ello world!");
    press(&mut ws, KeyCode::End, &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "ello world!");
    assert_eq!(pull_line(active(&mut ws), 3).unwrap(), "");
    // scopes
    press(&mut ws, KeyCode::Down, &mut gs);
    ctrl_press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(CursorPosition { line: 5, char: 4 }, (&ws.get_active().unwrap().cursor).into());
    assert_eq!(pull_line(active(&mut ws), 4).unwrap(), "next{");
    assert_eq!(pull_line(active(&mut ws), 5).unwrap(), "    ");
    assert_eq!(pull_line(active(&mut ws), 6).unwrap(), "} line");
    // scopes depth
    press(&mut ws, KeyCode::Char('['), &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(CursorPosition { line: 6, char: 8 }, (&ws.get_active().unwrap().cursor).into());
    assert_eq!(pull_line(active(&mut ws), 5).unwrap(), "    [");
    assert_eq!(pull_line(active(&mut ws), 6).unwrap(), "        ");
    assert_eq!(pull_line(active(&mut ws), 7).unwrap(), "    ]");
}

#[test]
fn test_del() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Delete, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "ello world!");
    press(&mut ws, KeyCode::End, &mut gs);
    press(&mut ws, KeyCode::Delete, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "ello world!next line");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "     ");
    press(&mut ws, KeyCode::End, &mut gs);
    press(&mut ws, KeyCode::Delete, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "really long line here");
}

#[test]
fn test_backspace() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    press(&mut ws, KeyCode::Backspace, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello world!");
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::Backspace, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello world!next line");
    press(&mut ws, KeyCode::Backspace, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello worldnext line");
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::End, &mut gs);
    press(&mut ws, KeyCode::Backspace, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "    ");
    press(&mut ws, KeyCode::Backspace, &mut gs);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "");
}

#[test]
fn test_cut_paste() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    ctrl_press(&mut ws, KeyCode::Char('x'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "next line");
    ctrl_press(&mut ws, KeyCode::Right, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    ctrl_press(&mut ws, KeyCode::Char('x'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nextly long line here");
    shift_press(&mut ws, KeyCode::Down, &mut gs); // with select
    ctrl_press(&mut ws, KeyCode::Char('v'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "next line");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "     ");
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "realt one here");
}

#[test]
fn test_jump_select() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    gs.insert_mode();
    ctrl_shift_press(&mut ws, KeyCode::Right, &mut gs);
    select_eq((CursorPosition::default(), CursorPosition { line: 0, char: 5 }), active(&mut ws));
    ctrl_shift_press(&mut ws, KeyCode::Right, &mut gs);
    select_eq((CursorPosition::default(), CursorPosition { line: 0, char: 11 }), active(&mut ws));
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 9 }), active(&mut ws));
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    select_eq((CursorPosition::default(), CursorPosition { line: 3, char: 11 }), active(&mut ws));
}

#[test]
fn cursor_postion_confirm() {
    let eq = CursorPosition { line: 3, char: 10 };
    assert_eq!(eq, eq);
    assert!(eq >= eq);
    assert!(eq <= eq);
    assert!((eq <= eq));
    assert!((eq >= eq));

    let smol = CursorPosition { line: 3, char: 10 };
    let big = CursorPosition { line: 4, char: 1 };
    assert!(big >= smol);
    assert!(big > smol);
    assert!((big > smol));
    assert!((big >= smol));
    assert_eq!(std::cmp::max(smol, big), big);
    assert_eq!(std::cmp::min(smol, big), smol);

    let smol = CursorPosition { line: 3, char: 10 };
    let big = CursorPosition { line: 3, char: 11 };
    assert!(big >= smol);
    assert!((big > smol));
    assert!((big > smol));
    assert!((big >= smol));
    assert_eq!(std::cmp::max(smol, big), big);
    assert_eq!(std::cmp::min(smol, big), smol);
}

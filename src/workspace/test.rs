use super::{editor::Editor, map_editor, Workspace};
use crate::{
    configs::{test::mock_editor_key_map, EditorConfigs},
    global_state::GlobalState,
    workspace::{
        editor::test::{mock_editor, pull_line, select_eq},
        CursorPosition,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Style;
use std::collections::HashMap;

pub fn mock_ws(content: Vec<String>) -> Workspace {
    let mut ws = Workspace {
        editors: vec![mock_editor(content)],
        base_config: EditorConfigs::default(),
        key_map: mock_editor_key_map(),
        tab_style: Style::default(),
        lsp_servers: HashMap::default(),
        map_callback: map_editor,
    };
    ws.resize_render(60, 90);
    ws
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

#[test]
fn test_move() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(60, 100);
    gs.insert_mode();
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 0, line: 1 });
    press(&mut ws, KeyCode::End, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 9, line: 1 });
    press(&mut ws, KeyCode::Right, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 0, line: 2 });
    press(&mut ws, KeyCode::Left, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 9, line: 1 });
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::Down, &mut gs);
    press(&mut ws, KeyCode::End, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 21, line: 3 });
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
    press(&mut ws, KeyCode::Left, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 13, line: 4 });
    press(&mut ws, KeyCode::Right, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
    press(&mut ws, KeyCode::Down, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 8, line: 5 });
    press(&mut ws, KeyCode::Up, &mut gs);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
}

#[test]
fn test_select() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(60, 100);
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
fn test_chars() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(60, 100);
    gs.insert_mode();
    press(&mut ws, KeyCode::Char('n'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nhello world!");
    press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Char('('), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh()ello world!");
    press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Char('{'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}ello world!");
    press(&mut ws, KeyCode::Right, &mut gs);
    press(&mut ws, KeyCode::Char('['), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[]ello world!");
    press(&mut ws, KeyCode::Char('"'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[\"\"]ello world!");
    press(&mut ws, KeyCode::Char('\''), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[\"''\"]ello world!");
}

#[test]
fn test_new_line() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(60, 100);
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
}

#[test]
fn test_del() {
    let mut ws = base_ws();
    let mut gs = GlobalState::new(60, 100);
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
    let mut gs = GlobalState::new(60, 100);
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
    let mut gs = GlobalState::new(60, 100);
    gs.insert_mode();
    ctrl_press(&mut ws, KeyCode::Char('x'), &mut gs);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "next line");
    ctrl_press(&mut ws, KeyCode::Right, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    ctrl_press(&mut ws, KeyCode::Char('x'), &mut gs);
    press(&mut ws, KeyCode::Up, &mut gs);
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
    let mut gs = GlobalState::new(60, 100);
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

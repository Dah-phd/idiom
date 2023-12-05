use super::{file::Editor, Workspace};
use crate::{
    components::workspace::{
        file::test::{mock_editor, pull_line, select_eq},
        CursorPosition,
    },
    configs::{test::mock_editor_key_map, EditorConfigs, Mode},
    events::Events,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::collections::HashMap;

pub fn mock_ws(content: Vec<String>) -> Workspace {
    let mut state = ListState::default();
    state.select(Some(0));
    Workspace {
        editors: vec![mock_editor(content)],
        state,
        events: Events::new(),
        base_config: EditorConfigs::default(),
        key_map: mock_editor_key_map(),
        lsp_servers: HashMap::default(),
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

fn raw_keypress(ws: &mut Workspace, code: KeyCode) {
    ws.map(&KeyEvent::new(code, KeyModifiers::empty()), &mut Mode::Insert);
}

fn shift_keypress(ws: &mut Workspace, code: KeyCode) {
    ws.map(&KeyEvent::new(code, KeyModifiers::SHIFT), &mut Mode::Insert);
}

#[test]
fn test_move() {
    let mut ws = base_ws();
    raw_keypress(&mut ws, KeyCode::Down);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 0, line: 1 });
    raw_keypress(&mut ws, KeyCode::End);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 9, line: 1 });
    raw_keypress(&mut ws, KeyCode::Right);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 0, line: 2 });
    raw_keypress(&mut ws, KeyCode::Left);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 9, line: 1 });
    raw_keypress(&mut ws, KeyCode::Down);
    raw_keypress(&mut ws, KeyCode::Down);
    raw_keypress(&mut ws, KeyCode::End);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 21, line: 3 });
    raw_keypress(&mut ws, KeyCode::Down);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
    raw_keypress(&mut ws, KeyCode::Left);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 13, line: 4 });
    raw_keypress(&mut ws, KeyCode::Right);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
    raw_keypress(&mut ws, KeyCode::Down);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 8, line: 5 });
    raw_keypress(&mut ws, KeyCode::Up);
    assert_eq!(active(&mut ws).cursor.position(), CursorPosition { char: 14, line: 4 });
}

#[test]
fn test_select() {
    let mut ws = base_ws();
    shift_keypress(&mut ws, KeyCode::Down);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), active(&mut ws)));
    shift_keypress(&mut ws, KeyCode::Left);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 0, char: 12 }), active(&mut ws)));
    shift_keypress(&mut ws, KeyCode::Right);
    assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), active(&mut ws)));
    shift_keypress(&mut ws, KeyCode::Left);
    shift_keypress(&mut ws, KeyCode::Down);
    assert!(select_eq((CursorPosition::default(), CursorPosition { char: 9, line: 1 }), active(&mut ws)));
    raw_keypress(&mut ws, KeyCode::Left);
    shift_keypress(&mut ws, KeyCode::Right);
    assert!(select_eq((CursorPosition { char: 8, line: 1 }, CursorPosition { char: 9, line: 1 }), active(&mut ws)));
    shift_keypress(&mut ws, KeyCode::Left);
    shift_keypress(&mut ws, KeyCode::Left);
    assert!(select_eq((CursorPosition { char: 7, line: 1 }, CursorPosition { char: 8, line: 1 }), active(&mut ws)));
    shift_keypress(&mut ws, KeyCode::Up);
    assert!(select_eq((CursorPosition { char: 7, line: 0 }, CursorPosition { char: 8, line: 1 }), active(&mut ws)));
}

#[test]
fn test_chars() {
    let mut ws = base_ws();
    raw_keypress(&mut ws, KeyCode::Char('n'));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nhello world!");
    raw_keypress(&mut ws, KeyCode::Right);
    raw_keypress(&mut ws, KeyCode::Char('('));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh()ello world!");
    raw_keypress(&mut ws, KeyCode::Right);
    raw_keypress(&mut ws, KeyCode::Char('{'));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}ello world!");
    raw_keypress(&mut ws, KeyCode::Right);
    raw_keypress(&mut ws, KeyCode::Char('['));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[]ello world!");
    raw_keypress(&mut ws, KeyCode::Char('"'));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[\"\"]ello world!");
    raw_keypress(&mut ws, KeyCode::Char('\''));
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "nh(){}[\"''\"]ello world!");
}

#[test]
fn test_new_line() {
    let mut ws = base_ws();
    raw_keypress(&mut ws, KeyCode::Enter);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "hello world!");
    raw_keypress(&mut ws, KeyCode::Right);
    raw_keypress(&mut ws, KeyCode::Enter);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "h");
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "ello world!");
    raw_keypress(&mut ws, KeyCode::End);
    raw_keypress(&mut ws, KeyCode::Enter);
    assert_eq!(pull_line(active(&mut ws), 2).unwrap(), "ello world!");
    assert_eq!(pull_line(active(&mut ws), 3).unwrap(), "");
}

#[test]
fn test_del() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    raw_keypress(&mut ws, KeyCode::Delete);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "ello world!");
    raw_keypress(&mut ws, KeyCode::End);
    raw_keypress(&mut ws, KeyCode::Delete);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "ello world!next line");
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "     ");
    raw_keypress(&mut ws, KeyCode::End);
    raw_keypress(&mut ws, KeyCode::Delete);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "really long line here");
}

#[test]
fn test_backspace() {
    let mut ws = base_ws();
    raw_keypress(&mut ws, KeyCode::Backspace);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello world!");
    raw_keypress(&mut ws, KeyCode::Down);
    raw_keypress(&mut ws, KeyCode::Backspace);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello world!next line");
    raw_keypress(&mut ws, KeyCode::Backspace);
    assert_eq!(pull_line(active(&mut ws), 0).unwrap(), "hello worldnext line");
    raw_keypress(&mut ws, KeyCode::Down);
    raw_keypress(&mut ws, KeyCode::End);
    raw_keypress(&mut ws, KeyCode::Backspace);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "    ");
    raw_keypress(&mut ws, KeyCode::Backspace);
    assert_eq!(pull_line(active(&mut ws), 1).unwrap(), "");
}

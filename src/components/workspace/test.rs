use super::Workspace;
use crate::{
    components::workspace::{
        file::test::{mock_editor, pull_line, select_eq},
        CursorPosition,
    },
    configs::{test::mock_editor_key_map, EditorConfigs},
};
use ratatui::widgets::ListState;
use std::collections::HashMap;

fn mock_ws(content: Vec<String>) -> Workspace {
    let mut state = ListState::default();
    state.select(Some(0));
    Workspace {
        editors: vec![mock_editor(content)],
        state,
        base_config: EditorConfigs::default(),
        key_map: mock_editor_key_map(),
        lsp_servers: HashMap::default(),
    }
}

fn base_ws() -> Workspace {
    mock_ws(vec![
        "hello world!".to_owned(),
        "next line".to_owned(),
        "".to_owned(),
        "really long line here".to_owned(),
        "short one here".to_owned(),
        "test msg".to_owned(),
    ])
}

#[test]
fn test_move() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    if let Some(editor) = ws.get_active() {
        editor.down();
        assert_eq!(editor.cursor, CursorPosition { char: 0, line: 1 });
        editor.end_of_line();
        assert_eq!(editor.cursor, CursorPosition { char: 9, line: 1 });
        editor.right();
        assert_eq!(editor.cursor, CursorPosition { char: 0, line: 2 });
        editor.left();
        assert_eq!(editor.cursor, CursorPosition { char: 9, line: 1 });
        editor.down();
        editor.down();
        editor.end_of_line();
        assert_eq!(editor.cursor, CursorPosition { char: 21, line: 3 });
        editor.down();
        assert_eq!(editor.cursor, CursorPosition { char: 14, line: 4 });
        editor.left();
        assert_eq!(editor.cursor, CursorPosition { char: 13, line: 4 });
        editor.right();
        assert_eq!(editor.cursor, CursorPosition { char: 14, line: 4 });
    }
}

#[test]
fn test_select() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    if let Some(editor) = ws.get_active() {
        editor.select_down();
        assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), editor));
        editor.select_left();
        assert!(select_eq((CursorPosition::default(), CursorPosition { line: 0, char: 12 }), editor));
        editor.select_right();
        assert!(select_eq((CursorPosition::default(), CursorPosition { line: 1, char: 0 }), editor));
        editor.select_left();
        editor.select_down();
        assert!(select_eq((CursorPosition::default(), CursorPosition { char: 9, line: 1 }), editor));
        editor.left();
        editor.select_right();
        assert!(select_eq((CursorPosition { char: 8, line: 1 }, CursorPosition { char: 9, line: 1 }), editor));
        editor.select_left();
        editor.select_left();
        assert!(select_eq((CursorPosition { char: 7, line: 1 }, CursorPosition { char: 8, line: 1 }), editor));
        editor.select_up();
        assert!(select_eq((CursorPosition { char: 7, line: 0 }, CursorPosition { char: 8, line: 1 }), editor));
    }
}

#[test]
fn test_chars() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    if let Some(editor) = ws.get_active() {
        editor.push('n');
        assert_eq!(pull_line(editor, 0).unwrap(), "nhello world!");
        editor.right();
        editor.push('(');
        assert_eq!(pull_line(editor, 0).unwrap(), "nh()ello world!");
        editor.right();
        editor.push('{');
        assert_eq!(pull_line(editor, 0).unwrap(), "nh(){}ello world!");
        editor.right();
        editor.push('[');
        assert_eq!(pull_line(editor, 0).unwrap(), "nh(){}[]ello world!");
        editor.push('"');
        assert_eq!(pull_line(editor, 0).unwrap(), "nh(){}[\"\"]ello world!");
        editor.push('\'');
        assert_eq!(pull_line(editor, 0).unwrap(), "nh(){}[\"''\"]ello world!");
    }
}

#[test]
fn test_new_line() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    if let Some(editor) = ws.get_active() {
        editor.new_line();
        assert_eq!(pull_line(editor, 0).unwrap(), "");
        assert_eq!(pull_line(editor, 1).unwrap(), "hello world!");
        editor.right();
        editor.new_line();
        assert_eq!(pull_line(editor, 1).unwrap(), "h");
        assert_eq!(pull_line(editor, 2).unwrap(), "ello world!");
        editor.end_of_line();
        editor.new_line();
        assert_eq!(pull_line(editor, 2).unwrap(), "ello world!");
        assert_eq!(pull_line(editor, 3).unwrap(), "");
    }
}

#[test]
fn test_del() {
    let mut ws = base_ws();
    assert!(ws.get_active().is_some());
    if let Some(editor) = ws.get_active() {
        editor.del();
        assert_eq!(pull_line(editor, 0).unwrap(), "ello world!");
        editor.end_of_line();
        editor.del();
        assert_eq!(pull_line(editor, 0).unwrap(), "ello world!next line");
        assert_eq!(pull_line(editor, 1).unwrap(), "");
        editor.end_of_line();
        editor.del();
        assert_eq!(pull_line(editor, 1).unwrap(), "really long line here");
    }
}

#[test]
fn test_backspace() {}

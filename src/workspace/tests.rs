use super::{Mode, Workspace};
use crate::{
    configs::{EditorAction, EditorConfigs, EditorKeyMap},
    cursor::{Cursor, CursorPosition},
    editor::{
        Editor,
        tests::{mock_editor, pull_line, select_eq},
    },
    editor_line::EditorLine,
    ext_tui::CrossTerm,
    global_state::GlobalState,
    lsp::servers::LSPServers,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use idiom_tui::{Backend, layout::Rect};

impl Workspace {
    pub fn mocked(editors: Vec<Vec<String>>) -> Self {
        let data = editors.into_iter().map(mock_editor).collect::<Vec<_>>();
        let mut ws = Workspace {
            editors: data.into(),
            base_configs: EditorConfigs::default(),
            key_map: EditorKeyMap::mocked(),
            lsp_servers: LSPServers::default(),
            mode: Mode::new_editor(),
        };
        ws.resize_all(Rect::new(0, 0, 90, 60));
        ws
    }

    pub fn mocked_empty() -> Self {
        Self::mocked(vec![])
    }
}

fn base_ws() -> Workspace {
    Workspace::mocked(vec![vec![
        "hello world!".to_owned(),
        "next line".to_owned(),
        "     ".to_owned(),
        "really long line here".to_owned(),
        "short one here".to_owned(),
        "test msg".to_owned(),
    ]])
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
    let current = active(ws).cursor().get_position();
    assert_eq!(current, position);
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
    let base_line = active(&mut ws).content()[0].to_string();
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
    assert!(test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() },
        CursorPosition { line: 0, char: base_line.len() - 1 },
    );
    assert_eq!(test_cursor.char, base_line.len() - 1);
    assert!(test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() },
        CursorPosition { line: 0, char: base_line.len() + 1 },
    );
    assert_eq!(test_cursor.char, base_line.len() + 1);
    assert!(!test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(
        CursorPosition { line: 0, char: base_line.len() + 1 },
        CursorPosition { line: 0, char: base_line.len() },
    );
    assert_eq!(test_cursor.char, base_line.len());
    assert!(!test_cursor.matches_content(&active(&mut ws).content()));
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
    let last_line = active(&mut ws).content().len() - 1;
    let base_line = active(&mut ws).content()[last_line].to_string();
    let mut test_cursor = Cursor::default();
    let all = (CursorPosition::default(), CursorPosition { char: base_line.len(), line: last_line });

    ctrl_press(&mut ws, KeyCode::Char('a'), &mut gs);
    assert_position(&mut ws, CursorPosition { char: base_line.len(), line: last_line });
    assert!(select_eq(all, active(&mut ws)));
    unsafe { active(&mut ws).cursor_mut().select_set(all.1, all.0) }; // swap select 'from' and 'to'
    shift_press(&mut ws, KeyCode::Down, &mut gs);
    assert!(select_eq((CursorPosition { line: 1, char: 0 }, all.1), active(&mut ws)));
    shift_press(&mut ws, KeyCode::Left, &mut gs);
    assert!(select_eq(
        (CursorPosition { line: 0, char: active(&mut ws).content()[0].char_len() }, all.1),
        active(&mut ws)
    ));
    test_cursor.select_set(CursorPosition { line: 0, char: active(&mut ws).content()[0].char_len() }, all.1);
    assert!(test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(CursorPosition { line: 0, char: active(&mut ws).content()[0].char_len() + 1 }, all.1);
    assert!(!test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(all.0, all.1);
    assert!(test_cursor.matches_content(&active(&mut ws).content()));
    test_cursor.select_set(all.0, CursorPosition { line: all.1.line + 1, char: 0 });
    assert!(!test_cursor.matches_content(&active(&mut ws).content()));
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
    assert_eq!(CursorPosition { line: 5, char: 4 }, ws.get_active().unwrap().cursor().get_position());
    assert_eq!(pull_line(active(&mut ws), 4).unwrap(), "next{");
    assert_eq!(pull_line(active(&mut ws), 5).unwrap(), "    ");
    assert_eq!(pull_line(active(&mut ws), 6).unwrap(), "} line");
    // scopes depth
    press(&mut ws, KeyCode::Char('['), &mut gs);
    press(&mut ws, KeyCode::Enter, &mut gs);
    assert_eq!(CursorPosition { line: 6, char: 8 }, ws.get_active().unwrap().cursor().get_position());
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

#[test]
fn cursor_directions_get() {
    let mut test_cursor = Cursor::default();
    let position_l3 = CursorPosition { line: 3, char: 2 };
    test_cursor.select_set(position_l3, CursorPosition::default());
    let ((from, to), dir) = test_cursor.select_get_direction().unwrap();
    assert_eq!((from, to), (CursorPosition::default(), position_l3));
    dir.apply_ordered(from, to, |first, second| assert_eq!((first, second), (position_l3, CursorPosition::default())));

    test_cursor.select_set(CursorPosition::default(), position_l3);
    let ((from, to), dir) = test_cursor.select_get_direction().unwrap();
    assert_eq!((from, to), (CursorPosition::default(), position_l3));
    dir.apply_ordered(from, to, |first, second| assert_eq!((first, second), (CursorPosition::default(), position_l3)));
}

#[test]
fn cursor_directions_take() {
    let mut test_cursor = Cursor::default();
    let position_l3 = CursorPosition { line: 3, char: 2 };
    test_cursor.select_set(position_l3, CursorPosition::default());
    let ((from, to), dir) = test_cursor.select_take_direction().unwrap();
    assert_eq!((from, to), (CursorPosition::default(), position_l3));
    dir.apply_ordered(from, to, |first, second| assert_eq!((first, second), (position_l3, CursorPosition::default())));

    test_cursor.select_set(CursorPosition::default(), position_l3);
    let ((from, to), dir) = test_cursor.select_take_direction().unwrap();
    assert_eq!((from, to), (CursorPosition::default(), position_l3));
    dir.apply_ordered(from, to, |first, second| assert_eq!((first, second), (CursorPosition::default(), position_l3)));
}

#[test]
fn match_content() {
    let content: Vec<EditorLine> =
        ["test", "test2", "test3", "end line"].into_iter().map(|s| EditorLine::from(String::from(s))).collect();

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 3, char: 5 }, CursorPosition { line: 3, char: 9 });
    cursor.match_content(&content);
    assert_eq!(cursor.select_get(), Some((CursorPosition { line: 3, char: 5 }, CursorPosition { line: 3, char: 8 })));
    cursor.select_set(CursorPosition { line: 3, char: 9 }, CursorPosition { line: 3, char: 9 });
    cursor.match_content(&content);
    assert_eq!(cursor.get_position(), CursorPosition { line: 3, char: 8 });
    assert_eq!(None, cursor.select_get());
    cursor.select_set(CursorPosition { line: 2, char: 3 }, CursorPosition { line: 4, char: 9 });
    cursor.match_content(&content);
    assert_eq!(cursor.get_position(), CursorPosition { line: 3, char: 8 });
    assert_eq!(None, cursor.select_get());
}

#[test]
fn ws_mode_switches_edgecases() {
    let mut ws = Workspace::mocked(vec![vec![], vec![]]);
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    assert!(ws.is_toggled_editor());
    let cancel = ws.key_map.try_pull(&EditorAction::Cancel).unwrap();
    let left = ws.key_map.try_pull(&EditorAction::Left).unwrap();
    assert!(ws.editors.collect_status());
    assert!(ws.map(&cancel, &mut gs));
    assert!(ws.editors.collect_status());
    assert!(ws.is_toggled_tabs());
    assert!(ws.map(&left, &mut gs));
    assert!(ws.is_toggled_tabs());
    assert!(ws.editors.collect_status());
}

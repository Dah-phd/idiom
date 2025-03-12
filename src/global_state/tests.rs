use super::draw::full_rebuild;
use super::GlobalState;
use crate::global_state::draw::solve_components;
use crate::render::backend::{Backend, BackendProtocol};
use crate::render::layout::{Borders, Line, Rect};
use crate::runner::EditorTerminal;
use crate::tree::tests::mock_tree;
use crate::workspace::tests::mock_ws;

#[test]
fn full_rebuild_draw() {
    let backend = Backend::init();
    let mut gs = GlobalState::new(backend).unwrap();
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(80);
    gs.screen_rect = (80, 80).into();
    full_rebuild(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 18, width: 62, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 18, width: 62, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 1, width: 16, height: 78, borders: Borders::LEFT | Borders::RIGHT });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

#[test]
fn full_rebuild_draw_insert() {
    let backend = Backend::init();
    let mut gs = GlobalState::new(backend).unwrap();
    gs.toggle_tree();
    gs.insert_mode();
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(80);
    gs.screen_rect = (80, 80).into();
    full_rebuild(&mut gs, &mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 0, width: 80, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

#[test]
fn solve_components_draw() {
    let backend = Backend::init();
    let mut gs = GlobalState::new(backend).unwrap();
    gs.screen_rect = (80, 80).into();
    solve_components(&mut gs);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 18, width: 62, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 18, width: 62, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 1, width: 16, height: 78, borders: Borders::LEFT | Borders::RIGHT });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

#[test]
fn solve_components_insert() {
    let backend = Backend::init();
    let mut gs = GlobalState::new(backend).unwrap();
    gs.screen_rect = (80, 80).into();
    gs.toggle_tree();
    gs.insert_mode();
    solve_components(&mut gs);
    gs.screen_rect = (80, 80).into();
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 0, width: 80, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

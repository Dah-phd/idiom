use super::GlobalState;
use crate::embeded_term::EditorTerminal;
use crate::ext_tui::CrossTerm;
use crate::tree::tests::mock_tree;
use crate::workspace::tests::mock_ws;
use idiom_tui::{
    layout::{Borders, Line, Rect},
    Backend,
};

#[test]
fn full_rebuild_draw() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(Some(String::new()));
    gs.full_resize(80, 80);
    let editor_rect = gs.calc_editor_rect();
    gs.draw(&mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(editor_rect, gs.editor_area);
    assert_eq!(gs.editor_area, Rect { row: 1, col: 14, width: 66, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 14, width: 66, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 1, width: 12, height: 78, borders: Borders::LEFT | Borders::RIGHT });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

#[test]
fn full_rebuild_draw_insert() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    gs.toggle_tree();
    gs.insert_mode();
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(Some(String::new()));
    gs.full_resize(80, 80);
    let editor_rect = gs.calc_editor_rect();
    gs.draw(&mut ws, &mut tree, &mut term);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(editor_rect, gs.editor_area);
    assert_eq!(gs.editor_area, Rect { row: 1, col: 0, width: 80, height: 78, borders: Borders::empty() });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 80, col: 0, width: 80 });
}

use super::GlobalState;
use crate::embeded_term::EditorTerminal;
use crate::ext_tui::CrossTerm;
use crate::tree::tests::mock_tree;
use crate::workspace::tests::mock_ws;
use idiom_tui::{
    layout::{Borders, Line, Rect},
    Backend,
};

fn full_draw_select(h: u16, w: u16) -> GlobalState {
    let mut gs = GlobalState::new(Rect::new(0, 0, w as usize, h), CrossTerm::init());
    let mut ws = mock_ws(
        ["test line uno - in here", "second line", "last line for the test"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    );
    let mut tree = mock_tree();
    let mut term = EditorTerminal::new(Some(String::new()));
    assert_eq!(gs.tree_area, Rect::default());
    assert_eq!(gs.editor_area, Rect::default());
    gs.full_resize(&mut ws, &mut term, h, w);
    gs.draw(&mut ws, &mut tree, &mut term);
    gs
}

fn full_draw_insert(h: u16, w: u16) -> GlobalState {
    let mut gs = GlobalState::new(Rect::new(0, 0, w as usize, h), CrossTerm::init());
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
    assert_eq!(gs.tree_area, Rect::default());
    assert_eq!(gs.editor_area, Rect::default());
    gs.full_resize(&mut ws, &mut term, h, w);
    gs.draw(&mut ws, &mut tree, &mut term);
    gs
}

fn force_calc_select(h: u16, w: u16) -> GlobalState {
    let mut gs = GlobalState::new(Rect::new(0, 0, w as usize, h), CrossTerm::init());
    let mut ws = mock_ws(vec![]);
    let mut term = EditorTerminal::new(Some(String::new()));
    assert_eq!(gs.tree_area, Rect::default());
    assert_eq!(gs.editor_area, Rect::default());
    gs.full_resize(&mut ws, &mut term, 80, 80);
    gs.force_area_calc();
    gs
}

fn force_calc_insert(h: u16, w: u16) -> GlobalState {
    let mut gs = GlobalState::new(Rect::new(0, 0, w as usize, h), CrossTerm::init());
    let mut ws = mock_ws(vec![]);
    let mut term = EditorTerminal::new(Some(String::new()));
    assert_eq!(gs.tree_area, Rect::default());
    assert_eq!(gs.editor_area, Rect::default());
    gs.toggle_tree();
    gs.insert_mode();
    gs.full_resize(&mut ws, &mut term, 80, 80);
    gs.force_area_calc();
    gs
}

#[test]
fn full_rebuild_draw() {
    let gs = full_draw_select(80, 80);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 15, width: 65, height: 78, borders: Borders::LEFT });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 14, width: 66, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 0, width: 14, height: 78, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 79, col: 0, width: 80 });
}

#[test]
fn full_rebuild_draw_insert() {
    let gs = full_draw_insert(80, 80);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 1, width: 79, height: 78, borders: Borders::LEFT });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 79, col: 0, width: 80 });
}

#[test]
fn force_area_calc() {
    let gs = force_calc_select(80, 80);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 15, width: 65, height: 78, borders: Borders::LEFT });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 14, width: 66, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 1, col: 0, width: 14, height: 78, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 79, col: 0, width: 80 });
}

#[test]
fn force_area_calc_insert() {
    let gs = force_calc_insert(80, 80);
    assert_eq!(gs.screen_rect, Rect::from((80, 80)));
    assert_eq!(gs.editor_area, Rect { row: 1, col: 1, width: 79, height: 78, borders: Borders::LEFT });
    assert_eq!(gs.tab_area, Rect { row: 0, col: 0, width: 80, height: 1, borders: Borders::empty() });
    assert_eq!(gs.tree_area, Rect { row: 0, col: 0, width: 0, height: 79, borders: Borders::empty() });
    assert_eq!(gs.footer_line, Line { row: 79, col: 0, width: 80 });
}

#[test]
fn compare_select() {
    let force_gs = force_calc_select(80, 80);
    let draw_gs = full_draw_select(80, 80);
    assert_eq!(force_gs.screen_rect, draw_gs.screen_rect);
    assert_eq!(force_gs.editor_area, draw_gs.editor_area);
    assert_eq!(force_gs.tab_area, draw_gs.tab_area);
    assert_eq!(force_gs.tree_area, draw_gs.tree_area);
    assert_eq!(force_gs.footer_line, draw_gs.footer_line);
}

#[test]
fn compare_insert() {
    let force_gs = force_calc_insert(80, 80);
    let draw_gs = full_draw_insert(80, 80);
    assert_eq!(force_gs.screen_rect, draw_gs.screen_rect);
    assert_eq!(force_gs.editor_area, draw_gs.editor_area);
    assert_eq!(force_gs.tab_area, draw_gs.tab_area);
    assert_eq!(force_gs.tree_area, draw_gs.tree_area);
    assert_eq!(force_gs.footer_line, draw_gs.footer_line);
}

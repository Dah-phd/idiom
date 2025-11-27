use super::GlobalState;
use crate::{
    editor::EditorStats,
    embeded_term::EditorTerminal,
    ext_tui::{CrossTerm, StyleExt},
    tree::tests::mock_tree,
    workspace::tests::mock_ws,
};
use crossterm::style::{Color, ContentStyle};
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

#[test]
fn footer_render() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 60, 30), CrossTerm::init());
    gs.force_area_calc();
    gs.footer.line = gs.footer();

    gs.footer.render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.render(
        Some(EditorStats { len: 300, select_len: 2, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 20>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 (2 selected) ".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.error("err".into());

    gs.footer.render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.render(
        Some(EditorStats { len: 300, select_len: 0, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 33>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 ".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );
}

#[test]
fn footer_fast_render() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 60, 30), CrossTerm::init());
    gs.force_area_calc();
    gs.footer.line = gs.footer();

    gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert!(gs.backend.drain().is_empty());

    gs.footer.fast_render(
        Some(EditorStats { len: 300, select_len: 2, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 20>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 (2 selected) ".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.fast_render(
        Some(EditorStats { len: 300, select_len: 2, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert!(gs.backend.drain().is_empty());

    gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert!(gs.backend.drain().is_empty());

    gs.footer.error("err".into());

    gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.fast_render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    assert!(gs.backend.drain().is_empty());

    gs.footer.fast_render(
        Some(EditorStats { len: 300, select_len: 0, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 33>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 ".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.fast_render(
        Some(EditorStats { len: 300, select_len: 0, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    assert!(gs.backend.drain().is_empty());
}

#[test]
fn footer_force_rerender() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 60, 30), CrossTerm::init());
    gs.force_area_calc();
    gs.footer.line = gs.footer();

    gs.footer.force_rerender(gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.render(
        Some(EditorStats { len: 300, select_len: 2, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    gs.backend.drain();
    gs.footer.force_rerender(gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 20>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 (2 selected) ".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.error("err".into());
    gs.footer.render(None, gs.ui_theme.accent_style(), &mut gs.backend);
    gs.backend.drain();
    gs.footer.force_rerender(gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );

    gs.footer.render(
        Some(EditorStats { len: 300, select_len: 0, position: (0, 1).into() }),
        gs.ui_theme.accent_style(),
        &mut gs.backend,
    );
    gs.backend.drain();

    gs.footer.force_rerender(gs.ui_theme.accent_style(), &mut gs.backend);
    assert_eq!(
        gs.backend.drain(),
        [
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (ContentStyle::default(), "<<clear EOL>>".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 33>>".into()),
            (gs.ui_theme.accent_style(), "  Doc Len 300, Ln 0, Col 1 ".into()),
            (ContentStyle::default(), "<<go to row: 29 col: 0>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 2>>".into()),
            (gs.ui_theme.accent_style().with_fg(Color::Red), "err".into()),
            (ContentStyle::default(), "<<reset style>>".into())
        ]
    );
}

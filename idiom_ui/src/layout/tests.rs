use super::{Line, Rect};
use crate::{
    backend::{Backend, CrossTerm, StyleExt},
    layout::Borders,
};
use crossterm::style::ContentStyle;

#[test]
fn split_horizont_rel() {
    let rect: Rect = (20, 30).into();
    assert_eq!(
        rect.split_horizont_rel(12),
        (
            Rect { row: 0, col: 0, width: 12, height: 30, borders: Borders::empty() },
            Rect { row: 0, col: 12, width: 8, height: 30, borders: Borders::empty() }
        )
    );
}

#[test]
fn split_horizont_rel_overflow() {
    let rect: Rect = (10, 30).into();
    assert_eq!(
        rect.split_horizont_rel(12),
        (
            Rect { row: 0, col: 0, width: 10, height: 30, borders: Borders::empty() },
            Rect { row: 0, col: 10, width: 0, height: 30, borders: Borders::empty() }
        )
    );
}

#[test]
fn split_vertical_rel() {
    let rect = Rect::from((20, 30));
    assert_eq!(
        rect.split_vertical_rel(12),
        (
            Rect { row: 0, col: 0, width: 20, height: 12, borders: Borders::empty() },
            Rect { row: 12, col: 0, width: 20, height: 18, borders: Borders::empty() }
        )
    );
}

#[test]
fn split_vertical_rel_overflow() {
    let rect = Rect::from((20, 10));
    assert_eq!(
        rect.split_vertical_rel(12),
        (
            Rect { row: 0, col: 0, width: 20, height: 10, borders: Borders::empty() },
            Rect { row: 10, col: 0, width: 20, height: 0, borders: Borders::empty() }
        )
    );
}

#[test]
fn render_centered() {
    let width = 50;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered("idiom", &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::default(), "<<padding: 23>>".to_owned()),
            (ContentStyle::default(), "idiom".to_owned()),
            (ContentStyle::default(), "<<padding: 22>>".to_owned())
        ]
    )
}

#[test]
fn render_centered_maxed() {
    let width = 4;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered("idiom", &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::default(), "idio".to_owned()),
        ]
    )
}

#[test]
fn render_centered_one_pad() {
    let width = 6;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered("idiom", &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::default(), "idiom".to_owned()),
            (ContentStyle::default(), "<<padding: 1>>".to_owned())
        ]
    )
}

#[test]
fn render_centered_styled() {
    let width = 7;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered_styled("idiom", ContentStyle::bold(), &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::bold(), "<<set style>>".to_owned()),
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::bold(), "<<padding: 1>>".to_owned()),
            (ContentStyle::bold(), "idiom".to_owned()),
            (ContentStyle::bold(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "<<set style>>".to_owned())
        ]
    );
}

#[test]
fn render_centered_styled_maxed() {
    let width = 4;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered_styled("idiom", ContentStyle::bold(), &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::bold(), "<<set style>>".to_owned()),
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::bold(), "idio".to_owned()),
            (ContentStyle::default(), "<<set style>>".to_owned())
        ]
    );
}

#[test]
fn render_centered_styled_one_pad() {
    let width = 6;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered_styled("idiom", ContentStyle::bold(), &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::bold(), "<<set style>>".to_owned()),
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::bold(), "idiom".to_owned()),
            (ContentStyle::bold(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "<<set style>>".to_owned())
        ]
    );
}

#[test]
fn render_centered_complex() {
    let width = 50;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered("🔥idiom🔥", &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::default(), "<<padding: 21>>".to_owned()),
            (ContentStyle::default(), "🔥idiom🔥".to_owned()), // 5 + 2 + 2 = 9  >>> 50 - 9 = 21 + 20
            (ContentStyle::default(), "<<padding: 20>>".to_owned()),
        ]
    )
}

#[test]
fn render_centered_complex_maxed() {
    let width = 8;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered("🔥idiom🔥", &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::default(), "🔥idiom".to_owned()), // 5 + 2 >> 8 - 7 = 1 pad
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
        ]
    )
}

#[test]
fn render_centered_complex_style_maxed() {
    let width = 8;
    let line = Line { row: 1, col: 3, width };
    let mut backend = CrossTerm::init();
    line.render_centered_styled("🔥idiom🔥", ContentStyle::bold(), &mut backend);
    assert_eq!(
        backend.drain(),
        [
            (ContentStyle::bold(), "<<set style>>".to_owned()),
            (ContentStyle::default(), "<<go to row: 1 col: 3>>".to_owned()),
            (ContentStyle::bold(), "🔥idiom".to_owned()), // 5 + 2 >> 8 - 7 = 1 pad
            (ContentStyle::bold(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "<<set style>>".to_owned()),
        ]
    )
}

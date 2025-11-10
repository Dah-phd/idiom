use super::super::tests::{expect_select, parse_complex_line};
use super::line;
use crate::{
    configs::FileType,
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    syntax::tests::mock_utf8_lexer,
    workspace::{
        cursor::Cursor,
        editor::tests::mock_editor_md_render,
        line::{EditorLine, LineContext},
    },
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{
    layout::{Borders, Rect},
    Backend,
};

fn generate_lines() -> Vec<EditorLine> {
    [
        "## TADA",
        "- write tests",
        "- lsp server cold start, maybe? \"jedi-language server\" _starts slow_, but __once__ it starts *it* should **continue** end",
    ].into_iter().map(EditorLine::from).collect()
}

fn generate_complex_lines() -> Vec<EditorLine> {
    [
        "## 🔥TADA🔥",
        "- write tests",
        "- lsp server cold start, maybe? \"j🔥di-language server\" _starts slow_, but __once__ it starts *it* should **continue** end",
    ].into_iter().map(EditorLine::from).collect()
}

#[test]
fn simple_line() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let cursor = Cursor::default();

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_lines();

    for text in texts.iter_mut() {
        line(text, None, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["TADA".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec![" > write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec![" > lsp server cold start, maybe? \"jed".into()]));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["i-language server\" ", "starts slow", ", but ", "o"].into_iter().map(String::from).collect())
    );
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["nce", " it starts ", "it", " should ", "continue", " end"].into_iter().map(String::from).collect())
    );
    assert!(rendered.is_empty())
}

#[test]
fn simple_line_select() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((1, 7).into(), (2, 60).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_lines();

    for text in texts.iter_mut() {
        let select = ctx.select_get();
        line(text, select, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    let style_select = gs.theme.selected;
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["TADA".into()]));
    expect_select(7, 14, style_select, ctx.accent_style, &rendered);
    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(2), ["- write", " tests", "~"].into_iter().map(String::from).collect())
    );
    expect_select(0, 37, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"jedi".into()]));
    expect_select(0, 23, style_select, ctx.accent_style, &rendered);
}

#[test]
fn complex_line() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let cursor = Cursor::default();

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_complex_lines();

    for text in texts.iter_mut() {
        line(text, None, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["🔥TADA🔥".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec![" > write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec![" > lsp server cold start, maybe? \"j🔥".into()]));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["di-language server\" ", "starts slow", ", but "].into_iter().map(String::from).collect())
    );
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["once", " it starts ", "it", " should ", "continue", " end"].into_iter().map(String::from).collect())
    );
    assert!(rendered.is_empty())
}

#[test]
fn complex_line_select() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((1, 7).into(), (2, 60).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_complex_lines();

    for text in texts.iter_mut() {
        let select = ctx.select_get();
        line(text, select, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    let style_select = gs.theme.selected;
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["🔥TADA🔥".into()]));
    expect_select(7, 14, style_select, ctx.accent_style, &rendered);
    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(2), ["- write", " tests", "~"].into_iter().map(String::from).collect())
    );
    expect_select(0, 36, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"j🔥d".into()]));
    expect_select(0, 24, style_select, ctx.accent_style, &rendered);
}

// DEPENDENCY TEST
// markdown create testing - it is used only on run time, and changes can cause strange renders

#[test]
fn test_md_editor() {
    let mut editor = mock_editor_md_render(vec![]);
    todo!("add more tests")
}

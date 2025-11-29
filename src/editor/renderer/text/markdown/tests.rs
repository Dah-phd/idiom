use super::{
    super::{
        super::tests::{expect_select, parse_complex_line},
        md_line,
        tests::drain_as_raw_text_qmark_cursor,
    },
    ascii_line_exact, complex_line_exact,
};
use crate::{
    configs::FileType,
    cursor::Cursor,
    editor::syntax::{tests::mock_utf8_lexer, tokens::WrapData},
    editor::tests::mock_editor_md_render,
    editor_line::{EditorLine, LineContext},
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{
    layout::{Borders, IterLines, Rect},
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
        md_line(text, None, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["TADA".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec!["- write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"jedi".into()]));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["-language server\" ", "starts slow", ", but ", "on"].into_iter().map(String::from).collect())
    );
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, ["ce", " it starts ", "it", " should ", "continue", " end"].into_iter().map(String::from).collect())
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
        md_line(text, select, &mut ctx, &mut lines, &mut gs);
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
fn test_complex_line() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let cursor = Cursor::default();

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_complex_lines();

    for text in texts.iter_mut() {
        md_line(text, None, &mut ctx, &mut lines, &mut gs);
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["🔥TADA🔥".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec!["- write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"j🔥d".into()]));
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
        md_line(text, select, &mut ctx, &mut lines, &mut gs);
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
fn test_exact_md_ascii() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 60), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_md_render(vec![
        "content **content** _asdwa_ asdwadasjukhdfajskfhgasjkf".into(),
        "![c](https://codeberg.org)".into(),
        "".into(),
    ]);
    let ea = gs.editor_area();
    editor.resize(ea.width, ea.height as usize);
    editor.cursor.set_position((1, 0).into());

    let mut ctx = LineContext::collect_context(
        &editor.cursor,
        editor.lexer.encoding().char_len,
        2,
        ContentStyle::fg(Color::DarkGrey),
    );
    let mut lines = ea.into_iter();
    let text_width = lines.width() - ctx.line_prefix_len();

    let text = &mut editor.content[0];
    let wd = WrapData::from_text_cached(text, text_width);
    assert_eq!(wd.count(), 3);
    ascii_line_exact(text, &mut lines, &mut ctx, gs.backend());
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(result, [
        "<<go to row: 1 col: 15>>", " 1 ", "<<clear EOL>>", "content ", "<<set style>>", "content", "<<set style>>", " ", "<<set style>>", "asdwa", "<<set style>>", " asdwa",
        "<<go to row: 2 col: 15>>", "   ", "<<clear EOL>>", "dasjukhdfajskfhgasjkf", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "   ", "<<clear EOL>>"  // empty for required len
    ]);

    let text = &mut editor.content[1];
    let wd = WrapData::from_text_cached(text, text_width);
    assert_eq!(wd.count(), 1);
    ascii_line_exact(text, &mut lines, &mut ctx, gs.backend());
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(result, [
        "<<go to row: 4 col: 15>>", " 2 ", "<<clear EOL>>", "<<set style>>", "c", "<<set style>>", "<<padding: 4>>", "https://codeberg.org", "<<reset style>>"
    ]);
}

#[test]
fn test_exact_md_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 60), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_md_render(vec![
        "content **con🦀nt** _a🦀wa_ asdwadasjukhdfajskfhgasjkf".into(),
        "![cb](https://code🦀rg.org/cr🦀ab-empji🦀/)".into(),
        "".into(),
    ]);
    let ea = gs.editor_area();
    editor.resize(ea.width, ea.height as usize);
    editor.cursor.set_position((1, 0).into());

    let mut ctx = LineContext::collect_context(
        &editor.cursor,
        editor.lexer.encoding().char_len,
        2,
        ContentStyle::fg(Color::DarkGrey),
    );
    let mut lines = ea.into_iter();
    let text_width = lines.width() - ctx.line_prefix_len();

    let text = &mut editor.content[0];
    let wd = WrapData::from_text_cached(text, text_width);
    assert_eq!(wd.count(), 3);
    complex_line_exact(text, &mut lines, &mut ctx, gs.backend());
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(result, [
        "<<go to row: 1 col: 15>>", " 1 ", "<<clear EOL>>",
        "c","o","n","t","e","n","t"," ",
        "<<set style>>", "c","o","n","🦀","n","t",
        "<<set style>>", " ",
        "<<set style>>", "a","🦀","w","a",
        "<<set style>>", " ","a","s","d","w","a",
        "<<go to row: 2 col: 15>>", "   ", "<<clear EOL>>",
        "d","a","s","j","u","k","h","d","f","a","j","s","k","f","h","g","a","s","j","k","f", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "   ", "<<clear EOL>>"  // empty for required len
    ]);

    let text = &mut editor.content[1];
    let wd = WrapData::from_text_cached(text, text_width);
    assert_eq!(wd.count(), 2);
    complex_line_exact(text, &mut lines, &mut ctx, gs.backend());
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(result, [
        "<<go to row: 4 col: 15>>", " 2 ", "<<clear EOL>>", "<<set style>>", "c", "b", "<<set style>>",
        "<<padding: 4>>", "https://code🦀rg.org/", "<<reset style>>", // link text
        "<<go to row: 5 col: 15>>", "   ", "<<clear EOL>>" // link is always single line
    ]);
}

#[test]
fn test_md_editor() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 20), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_md_render(vec![
        "content **content** _asdwa_ asdwadasjukhdfajskfhgasjkfad".into(), // multiline
        "![c](https://codeberg.org/ad)".into(),
        "".into(),
        "content **content** _asdwa_ asdwadasjukhdfajskfhgasjkfad".into(), // multi line
        "![c](https://codeberg.org/ad)".into(),
    ]);
    let ea = gs.editor_area();
    editor.resize(ea.width, ea.height as usize);
    editor.cursor.set_position((2, 0).into());

    editor.render(&mut gs);
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(result, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>",
        "content ","<<set style>>","content","<<set style>>"," ","<<set style>>","asdwa","<<set style>>"," asdwad",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "asjukhdfajskfhgasjkfad", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>",
        "<<set style>>","c","<<set style>>","<<padding: 4>>","https://codeberg.org/ad","<<reset style>>",
        "<<go to row: 5 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 6 col: 15>>", "3 ", "<<clear EOL>>", "?",
        "<<go to row: 7 col: 15>>", "4 ", "<<clear EOL>>",
        "content ","<<set style>>","content","<<set style>>"," ","<<set style>>","asdwa","<<set style>>"," asdwad",
        "<<go to row: 8 col: 15>>", "  ", "<<clear EOL>>",
        "asjukhdfajskfhgasjkfad","<<reset style>>",
        "<<go to row: 9 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 10 col: 15>>", "5 ", "<<clear EOL>>",
        "<<set style>>","c","<<set style>>","<<padding: 4>>","https://codeberg.org/ad","<<reset style>>",
        "<<go to row: 11 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 12 col: 15>>", "<<padding: 30>>", "<<go to row: 13 col: 15>>", "<<padding: 30>>",
        "<<go to row: 14 col: 15>>", "<<padding: 30>>", "<<go to row: 15 col: 15>>", "<<padding: 30>>",
        "<<go to row: 16 col: 15>>", "<<padding: 30>>", "<<go to row: 17 col: 15>>", "<<padding: 30>>",
        "<<go to row: 18 col: 15>>", "<<padding: 30>>",
        "<<unfreeze>>",
    ]);
}

#[test]
fn test_md_editor_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 20), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_md_render(vec![
        "content **con🦀nt** _a🦀wa_ asdwadasjukhdfajskfhgasjkfad".into(),
        "![cb](https://code🦀rg.org/cr🦀ab-empji🦀/ad)".into(),
        "".into(),
        "content **con🦀nt** _a🦀wa_ asdwadasjukhdfajskfhgasjkfad".into(),
        "![cb](https://code🦀rg.org/cr🦀ab-empji🦀/ad)".into(),
    ]);
    let ea = gs.editor_area();
    editor.resize(ea.width, ea.height as usize);
    editor.cursor.set_position((2, 0).into());

    editor.render(&mut gs);
    let result = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    let expect = [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "c", "o", "n", "t", "e", "n", "t", " ",
        "<<set style>>", "c", "o", "n", "🦀", "n", "t",
        "<<set style>>", " ",
        "<<set style>>", "a", "🦀", "w", "a",
        "<<set style>>", " ", "a", "s", "d", "w", "a", "d",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>",
        "a","s","j","u","k","h","d","f","a","j","s","k","f","h","g","a","s","j","k","f","a","d","<<reset style>>",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>",
        "<<set style>>", "c", "b", "<<set style>>", "<<padding: 4>>", "https://code🦀rg.org/c", "<<reset style>>",
        "<<go to row: 5 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 6 col: 15>>", "3 ", "<<clear EOL>>", "?",
        "<<go to row: 7 col: 15>>", "4 ", "<<clear EOL>>", "c", "o", "n", "t", "e", "n", "t", " ",
        "<<set style>>", "c", "o", "n", "🦀", "n", "t",
        "<<set style>>", " ",
        "<<set style>>", "a", "🦀", "w", "a",
        "<<set style>>", " ", "a", "s", "d", "w", "a", "d",
        "<<go to row: 8 col: 15>>", "  ", "<<clear EOL>>",
        "a","s","j","u","k","h","d","f","a","j","s","k","f","h","g","a","s","j","k","f","a","d","<<reset style>>",
        "<<go to row: 9 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 10 col: 15>>", "5 ", "<<clear EOL>>",
        "<<set style>>", "c", "b", "<<set style>>", "<<padding: 4>>", "https://code🦀rg.org/c", "<<reset style>>",
        "<<go to row: 11 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 12 col: 15>>", "<<padding: 30>>", "<<go to row: 13 col: 15>>", "<<padding: 30>>",
        "<<go to row: 14 col: 15>>", "<<padding: 30>>", "<<go to row: 15 col: 15>>", "<<padding: 30>>",
        "<<go to row: 16 col: 15>>", "<<padding: 30>>", "<<go to row: 17 col: 15>>", "<<padding: 30>>",
        "<<go to row: 18 col: 15>>", "<<padding: 30>>",
        "<<unfreeze>>",
    ];
    assert_eq!(result, expect);
}

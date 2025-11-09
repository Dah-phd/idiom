use super::{
    super::tests::{expect_cursor, expect_select, parse_complex_line},
    ascii, complex, line,
};
use crate::{
    configs::FileType,
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    syntax::tests::mock_utf8_lexer,
    workspace::{
        cursor::Cursor,
        editor::tests::mock_editor_text_render,
        line::{EditorLine, LineContext},
        CursorPosition,
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
fn cursor_render() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 0, char: 39 });

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in development - so if you want to try it do it with caution.**");
    assert!(text.is_simple());
    ascii::cursor(&mut text, None, 0, &mut lines, &mut ctx, &mut gs);
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in develop";
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()]));
    expect_cursor(cursor.char - first_line.chars().count(), "<<clear EOL>>", &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["ment - so if you want to try it do it".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec![" with caution.** ".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_complex_render() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 0, char: 39 });

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in devel🔥pment - so if you want to try it do it with caution.**");
    assert!(!text.is_simple());
    complex::cursor(&mut text, None, 0, &mut lines, &mut ctx, &mut gs);
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in devel🔥";
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()],));
    expect_cursor(cursor.char - first_line.chars().count(), "<<clear EOL>>", &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["pment - so if you want to try it do i".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["t with caution.** ".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_select() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition::default(), (0, 39).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in development - so if you want to try it do it with caution.**");
    assert!(text.is_simple());
    let select = ctx.select_get();
    ascii::cursor(&mut text, select, 0, &mut lines, &mut ctx, &mut gs);

    let mut rendered = gs.backend().drain();
    let first_line = "**The project is currently in develop";
    let style_select = gs.theme.selected;
    expect_select(0, 39, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()]));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, vec!["me".into(), "nt - so if you want to try it do it".into()])
    );
    assert_eq!(parse_complex_line(&mut rendered), (None, vec![" with caution.**".into(), " ".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_complex_select() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition::default(), (0, 39).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in devel🔥pment - so if you want to try it do it with caution.**");
    assert!(!text.is_simple());
    let select = ctx.select_get();
    complex::cursor(&mut text, select, 0, &mut lines, &mut ctx, &mut gs);
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in devel🔥";
    let style_select = gs.theme.selected;
    expect_select(0, 39, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()],));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, vec!["pme".into(), "nt - so if you want to try it do i".into()])
    );
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["t with caution.**".into(), " ".into()]));
    assert!(rendered.is_empty())
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
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["## TADA".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec!["- write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"jedi".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["-language server\" _starts slow_, but ".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["__once__ it starts *it* should **cont".into()]));
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
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["## TADA".into()]));
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
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["## 🔥TADA🔥".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(2), vec!["- write tests".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"j🔥d".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["i-language server\" _starts slow_, but".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec![" __once__ it starts *it* should **con".into()]));
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
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["## 🔥TADA🔥".into()]));
    expect_select(7, 14, style_select, ctx.accent_style, &rendered);
    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(2), ["- write", " tests", "~"].into_iter().map(String::from).collect())
    );
    expect_select(0, 36, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(3), vec!["- lsp server cold start, maybe? \"j🔥d".into()]));
    expect_select(0, 24, style_select, ctx.accent_style, &rendered);
}

fn drain_as_raw_text_qmark_cursor(gs: &mut GlobalState) -> Vec<String> {
    gs.backend()
        .drain()
        .into_iter()
        .map(|(s, text)| if s == ContentStyle::reversed() { "?".to_owned() } else { text })
        .collect()
}

#[test]
fn test_full_end_line() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 47, 6), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "GlobalState::new(Rect::new(0, 0, 30, 60), CrossTerm::init())".into(), // 60 len
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.render(&mut gs);
    // style is ignored
    let text = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?","l","o","b","a","l","S","t","a","t","e",":",":","n","e","w","(","R","e","c","t",":",":","n","e","w","(","0",","," ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "0",","," ","3","0",","," ","6","0",")",","," ","C","r","o","s","s","T","e","r","m",":",":","i","n","i","t","(",")",")",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", " ", // padding due to prev line filled up
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>", "n/a",
        "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 33>>",
        "<<go to row: 5 col: 22>>", "  Doc Len 2, Ln 1, Col 1 ",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>", "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>", "<<reset style>>", 
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);
    editor.map(crate::configs::EditorAction::Right, &mut gs);
    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "GlobalState::new(Rect::new(0, ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "0, 30, 60), CrossTerm::init())",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>",
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 33>>",
        "<<go to row: 5 col: 22>>", "  Doc Len 2, Ln 2, Col 1 ",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>", "<<reset style>>",
        "<<unfreeze>>"
    ]);
}

#[test]
fn test_cursor_line_oversize() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 25, 5), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "let mut gs = GlobalState::new(Rect::new(0, 0, 30, 60), CrossTerm::init());".into(),
        "n/a".into(),
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "g", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 1 ", "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 0>>", "<<reset style>>",
        "<<reset style>>",  "<<unfreeze>>", 
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "l", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "?", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 9 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", ":", ":", "n", "e", "w", "(", "R", "e",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "c", "t", ":", ":", "n", "e", "w", "(",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "?", ",", " ", "0", ",", " ", "3", "0",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 41 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    let cursor_on_last_wrap = [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "r", "o", "s", "s", "T", "e", "r", "m",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", ":", ":", "i", "n", "i", "t", "(", ")",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", ")", ";", "?",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 75 ",
        "<<reset style>>", "<<unfreeze>>"
    ];

    assert_eq!(text, cursor_on_last_wrap);

    editor.map(crate::configs::EditorAction::Right, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<go to row: 2 col: 15>>", "3 ", "<<clear EOL>>", "n/a",
        "<<go to row: 3 col: 15>>", "<<padding: 10>>", "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 2, Col 1 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Left, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    assert_eq!(text, cursor_on_last_wrap);
}

#[test]
fn test_cursor_line_oversize_full_last_wrap() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 25, 5), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "let mut gs = GlobalState::new(Rect::new(0, 0, 30, 60), CrossTerm::init()); //end".into(),
        "n/a".into(),
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "g", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 1 ", "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 0>>", "<<reset style>>",
        "<<reset style>>",  "<<unfreeze>>",
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "l", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "?", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 9 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "r", "o", "s", "s", "T", "e", "r", "m",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", ":", ":", "i", "n", "i", "t", "(", ")",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "?", ";", " ", "/", "/", "e", "n", "d",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 73 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    let cursor_on_last_wrap = [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", ":", ":", "i", "n", "i", "t", "(", ")",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", ")", ";", " ", "/", "/", "e", "n", "d",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "?",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 81 ",
        "<<reset style>>", "<<unfreeze>>"
    ];

    assert_eq!(text, cursor_on_last_wrap);

    editor.map(crate::configs::EditorAction::Right, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<go to row: 2 col: 15>>", "3 ", "<<clear EOL>>", "n/a",
        "<<go to row: 3 col: 15>>", "<<padding: 10>>",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 2, Col 1 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Left, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    assert_eq!(text, cursor_on_last_wrap);
}

#[test]
fn test_full_end_line_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 47, 6), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "GlobalState::new(Rect::new(0, 0, 30, 60), Cro🦀Term::init())".into(), // 60 len
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.render(&mut gs);
    // style is ignored
    let text = drain_as_raw_text_qmark_cursor(&mut gs);
    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?","l","o","b","a","l","S","t","a","t","e",":",":","n","e","w","(","R","e","c","t",":",":","n","e","w","(","0",","," ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "0",","," ","3","0",","," ","6","0",")",","," ","C","r","o","🦀","T","e","r","m",":",":","i","n","i","t","(",")",")",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", " ", // padding due to prev line filled up
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>", "n/a",
        "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 33>>",
        "<<go to row: 5 col: 22>>", "  Doc Len 2, Ln 1, Col 1 ",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>", "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>", "<<reset style>>", 
        "<<reset style>>",
        "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);
    editor.map(crate::configs::EditorAction::Right, &mut gs);
    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>", "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "GlobalState::new(Rect::new(0, ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "0, 30, 60), Cro🦀Term::init())",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", // last line is filled to end
        "<<go to row: 4 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<set style>>",
        "<<go to row: 5 col: 14>>", "<<padding: 33>>",
        "<<go to row: 5 col: 22>>", "  Doc Len 2, Ln 2, Col 1 ",
        "<<go to row: 5 col: 14>>", "<<padding: 8>>",
        "<<reset style>>", 
        "<<unfreeze>>"
    ]);
}

#[test]
fn test_cursor_line_oversize_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 25, 5), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "let mut gs = GlobalState::new(Rect::new(0, 0, 30, 60), CrossTerm::in🦀());".into(),
        "n/a".into(),
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "g", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 1 ", "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 0>>", "<<reset style>>",
        "<<reset style>>",  "<<unfreeze>>", 
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "l", "e", "t", " ", "m", "u", "t", " ",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "?", "s", " ", "=", " ", "G", "l", "o",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "b", "a", "l", "S", "t", "a", "t", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 9 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "0", ",", " ", "0", ",", " ", "3", "0",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", ",", " ", "6", "0", ")", ",", " ", "C",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "?", "o", "s", "s", "T", "e", "r", "m",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 57 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    let cursor_on_last_wrap = [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "r", "o", "s", "s", "T", "e", "r", "m",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", ":", ":", "i", "n", "🦀", "(", ")",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", ")", ";", "?",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 74 ",
        "<<reset style>>", "<<unfreeze>>"
    ];

    assert_eq!(text, cursor_on_last_wrap);

    editor.map(crate::configs::EditorAction::Right, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<go to row: 2 col: 15>>", "3 ", "<<clear EOL>>", "n/a",
        "<<go to row: 3 col: 15>>", "<<padding: 10>>",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 2, Col 1 ",
        "<<reset style>>", "<<unfreeze>>"    
    ]);

    editor.map(crate::configs::EditorAction::Left, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    assert_eq!(text, cursor_on_last_wrap);
}

#[test]
fn test_cursor_line_oversize_full_last_wrap_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 25, 5), CrossTerm::init());
    gs.force_area_calc();
    let mut editor = mock_editor_text_render(vec![
        "let mut gs = Gl🦀balState::new(Rect::new(0, 0, 30, 60), CrossTerm::🦀it()); //e".into(),
        "n/a".into(),
        "n/a".into(),
    ]);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "?", "e", "t", " ", "m", "u", "t", " ",
        // last skipped due to emoji width
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "g", "s", " ", "=", " ", "G", "l",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "🦀", "b", "a", "l", "S", "t", "a",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 1 ", "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 0>>", "<<reset style>>",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "l", "e", "t", " ", "m", "u", "t", " ",
        // last skipped due to emoji width
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "?", "s", " ", "=", " ", "G", "l",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "🦀", "b", "a", "l", "S", "t", "a",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 1, Col 9 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);
    editor.map(crate::configs::EditorAction::Down, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", " ", "C", "r", "o", "s", "s", "T", "e",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "r", "m", ":", ":", "🦀", "i", "t",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "(", ")", ")", "?", " ", "/", "/", "e",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 73 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::EndOfLine, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    let cursor_on_last_wrap = [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "r", "m", ":", ":", "🦀", "i", "t",
        "<<go to row: 2 col: 15>>", "  ", "<<clear EOL>>", "(", ")", ")", ";", " ", "/", "/", "e",
        "<<go to row: 3 col: 15>>", "  ", "<<clear EOL>>", "?",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", " 1, Col 78 ",
        "<<reset style>>", "<<unfreeze>>"
    ];

    assert_eq!(text, cursor_on_last_wrap);

    editor.map(crate::configs::EditorAction::Right, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    #[rustfmt::skip]
    assert_eq!(text, [
        "<<freeze>>",
        "<<go to row: 1 col: 15>>", "2 ", "<<clear EOL>>", "?", "/", "a", " ",
        "<<go to row: 2 col: 15>>", "3 ", "<<clear EOL>>", "n/a",
        "<<go to row: 3 col: 15>>", "<<padding: 10>>",
        "<<set style>>",
        "<<go to row: 4 col: 14>>", "<<padding: 11>>",
        "<<go to row: 4 col: 14>>", "n 2, Col 1 ",
        "<<reset style>>", "<<unfreeze>>"
    ]);

    editor.map(crate::configs::EditorAction::Left, &mut gs);

    editor.render(&mut gs);
    let text = drain_as_raw_text_qmark_cursor(&mut gs);

    assert_eq!(text, cursor_on_last_wrap);
}

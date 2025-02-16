use crate::{
    configs::FileType,
    global_state::GlobalState,
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        layout::{Borders, Rect},
    },
    syntax::tests::mock_utf8_lexer,
    workspace::{
        cursor::Cursor,
        line::{EditorLine, LineContext},
        CursorPosition,
    },
};

use super::super::tests::parse_complex_line;
use super::{ascii, complex, line};
use crossterm::style::{Color, ContentStyle};
use markdown::{tokenize, Block, ListItem, Span};

fn expect_select(
    mut start_char: usize,
    end_char: usize,
    select: Color,
    accent: ContentStyle,
    rendered: &[(ContentStyle, String)],
) {
    let mut count_to_end = end_char - start_char;
    let tokens = rendered
        .iter()
        .skip_while(|(.., t)| t != "<<clear EOL>>")
        .take_while(|(.., t)| !t.starts_with("<<go to row"))
        .filter(|(c, t)| {
            let is_ui = *c == accent;
            let is_control = t.starts_with("<<") && t.ends_with(">>");
            !is_ui && !is_control
        });

    for (style, text) in tokens {
        if start_char != 0 {
            assert_eq!(style.background_color, None);
            start_char -= text.chars().count();
        } else if count_to_end != 0 {
            assert_eq!(style.background_color, Some(select));
            count_to_end -= text.chars().count();
        } else {
            assert_eq!(style.background_color, None)
        }
    }
}

fn expect_cursor(mut char_idx: usize, rendered: &[(ContentStyle, String)]) {
    let mut skip = true;
    for (style, text) in rendered.iter() {
        if skip {
            skip = text != "<<clear EOL>>";
            continue;
        }

        if char_idx != 0 {
            char_idx -= text.chars().count();
            continue;
        }
        assert_eq!(*style, ContentStyle::reversed());
        return;
    }
}

fn generate_lines() -> Vec<EditorLine> {
    [
        "## TODO",
        "- write tests",
        "- lsp server cold start, maybe? \"jedi-language server\" _starts slow_, but __once__ it starts *it* should **continue** end",
    ].into_iter().map(EditorLine::from).collect()
}

fn generate_complex_lines() -> Vec<EditorLine> {
    [
        "## 🔥TODO🔥",
        "- write tests",
        "- lsp server cold start, maybe? \"j🔥di-language server\" _starts slow_, but __once__ it starts *it* should **continue** end",
    ].into_iter().map(EditorLine::from).collect()
}

#[test]
fn cursor_render() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 0, char: 39 });

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in development - so if you want to try it do it with caution.**");
    assert!(text.is_simple());
    ascii::cursor(&mut text, None, 0, &mut lines, &mut ctx, gs.backend());
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in develop";
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()]));
    expect_cursor(cursor.char - first_line.chars().count(), &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["ment - so if you want to try it do it".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec![" with caution.**".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_complex_render() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 0, char: 39 });

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in devel🔥pment - so if you want to try it do it with caution.**");
    assert!(!text.is_simple());
    complex::cursor(&mut text, None, 0, &mut lines, &mut ctx, gs.backend());
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in devel🔥";
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()],));
    expect_cursor(cursor.char - first_line.chars().count(), &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["pment - so if you want to try it do i".into()]));
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["t with caution.**".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_select() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition::default(), (0, 39).into());

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in development - so if you want to try it do it with caution.**");
    assert!(text.is_simple());
    let select = ctx.get_select_full_line(text.char_len());
    ascii::cursor(&mut text, select, 0, &mut lines, &mut ctx, gs.backend());

    let mut rendered = gs.backend().drain();
    let first_line = "**The project is currently in develop";
    let style_select = ctx.lexer.theme.selected;
    expect_select(0, 39, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()]));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, vec!["me".into(), "nt - so if you want to try it do it".into()])
    );
    assert_eq!(parse_complex_line(&mut rendered), (None, vec![" with caution.**".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn cursor_complex_select() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition::default(), (0, 39).into());

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 3, borders: Borders::empty() }.into_iter();
    let mut text =
        EditorLine::from("**The project is currently in devel🔥pment - so if you want to try it do it with caution.**");
    assert!(!text.is_simple());
    let select = ctx.get_select_full_line(text.char_len());
    complex::cursor(&mut text, select, 0, &mut lines, &mut ctx, gs.backend());
    let mut rendered = gs.backend().drain();

    let first_line = "**The project is currently in devel🔥";
    let style_select = ctx.lexer.theme.selected;
    expect_select(0, 39, style_select, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()],));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, vec!["pme".into(), "nt - so if you want to try it do i".into()])
    );
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["t with caution.**".into()]));
    assert!(rendered.is_empty())
}

#[test]
fn simple_line() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let cursor = Cursor::default();

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_lines();

    for text in texts.iter_mut() {
        line(text, None, &mut ctx, &mut lines, gs.backend());
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["TODO".into()]));
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
fn simple_line_select() {}

#[test]
fn complex_line() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let cursor = Cursor::default();

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_complex_lines();

    for text in texts.iter_mut() {
        line(text, None, &mut ctx, &mut lines, gs.backend());
    }

    let mut rendered = gs.backend().drain();
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec!["🔥TODO🔥".into()]));
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
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((1, 7).into(), (2, 60).into());

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::fg(Color::DarkGrey));
    let mut lines = Rect { row: 0, col: 0, width: 40, height: 5, borders: Borders::empty() }.into_iter();
    let mut texts = generate_complex_lines();

    for text in texts.iter_mut() {
        let select = ctx.get_select_full_line(text.char_len());
        line(text, select, &mut ctx, &mut lines, gs.backend());
    }

    let mut rendered = gs.backend().drain();
    let style_select = ctx.lexer.theme.selected;
    parse_complex_line(&mut rendered);
    expect_select(7, 13, style_select, ctx.accent_style, &rendered);
    parse_complex_line(&mut rendered);
    expect_select(0, 36, style_select, ctx.accent_style, &rendered);
    parse_complex_line(&mut rendered);
    expect_select(0, 24, style_select, ctx.accent_style, &rendered);
}

// DEPENDENCY TEST
// markdown create testing - it is used only on run time, and changes can cause strange renders

#[test]
fn parser_code_snippet() {
    let txt = "```";
    assert_eq!(tokenize(txt), vec![Block::Paragraph(vec![Span::Code(String::from('`'))])]);
}

#[test]
fn example_parsed_code() {
    assert_eq!(
        tokenize("![](/non_dev/screen1.png)"),
        vec![Block::Paragraph(vec![Span::Image(
            String::new(),
            String::from("/non_dev/screen1.png"),
            None
        )])]
    );
    assert_eq!(
        tokenize("## Tested platform"),
        vec![Block::Header(vec![Span::Text(String::from("Tested platform"))], 2)]
    );
    assert_eq!(
        tokenize("- Linux Fedora derivate (Nobara)"),
        vec![Block::UnorderedList(vec![ListItem::Simple(vec![Span::Text(
            String::from("Linux Fedora derivate (Nobara)")
        )])])],
    );
}

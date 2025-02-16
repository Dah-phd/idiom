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
use super::{ascii, complex};
use crossterm::style::{Color, ContentStyle};
use markdown::{tokenize, Block, ListItem, Span};

fn expect_select(start_char: usize, end_char: usize, accent: ContentStyle, rendered: &[(ContentStyle, String)]) {
    let mut counter = start_char;
    let mut start_found = false;
    for (_, text) in rendered.iter().filter(|(c, ..)| *c != accent) {
        if text.starts_with("<<") {
            if counter == 0 && text.contains("<<set bg ") {
                if start_found {
                    return;
                };
                start_found = true;
                counter = end_char;
            }
            continue;
        }
        let current_count = text.chars().count();
        if counter < current_count {
            match start_found {
                true => panic!("Unable to find select end at {end_char} {text}!\ntext {text}\n{rendered:?}"),
                false => panic!("Unable to find select start at {start_char}\ntext {text}\n{rendered:?}!"),
            }
        }
        counter -= text.chars().count();
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
    expect_select(0, 39, ctx.accent_style, &rendered);
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
    expect_select(0, 39, ctx.accent_style, &rendered);
    assert_eq!(parse_complex_line(&mut rendered), (Some(1), vec![first_line.into()],));
    assert_eq!(
        parse_complex_line(&mut rendered),
        (None, vec!["pme".into(), "nt - so if you want to try it do i".into()])
    );
    assert_eq!(parse_complex_line(&mut rendered), (None, vec!["t with caution.**".into()]));
    assert!(rendered.is_empty())
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

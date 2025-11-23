use super::{
    super::tests::{expect_cursor, expect_select, parse_complex_line, parse_simple_line},
    cursor as rend_cursor, line_render,
};
use crate::cursor::{Cursor, CursorPosition};
use crate::editor::renderer::tests::count_to_cursor;
use crate::editor::tests::mock_editor;
use crate::editor_line::LineContext;
use crate::syntax::tests::{
    create_token_pairs_utf16, create_token_pairs_utf32, create_token_pairs_utf8, longline_token_pair_utf16,
    longline_token_pair_utf32, longline_token_pair_utf8, mock_utf16_lexer, mock_utf32_lexer, mock_utf8_lexer,
    zip_text_tokens,
};
use crate::{configs::FileType, editor_line::EditorLine};
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{
    layout::{Line, Rect},
    Backend,
};
use lsp_types::SemanticToken;

fn consolidate_backend_drain(drain: Vec<(ContentStyle, String)>) -> Vec<(ContentStyle, String)> {
    let mut buf = vec![];
    let mut text_buf = String::new();
    let mut style_buf = ContentStyle::default();
    for (mut style, text) in drain.into_iter() {
        if style.foreground_color == Some(Color::Reset) {
            style.foreground_color.take();
        }
        if style.background_color == Some(Color::Reset) {
            style.background_color.take();
        }
        if style.underline_color == Some(Color::Reset) {
            style.underline_color.take();
        }
        if style == style_buf {
            text_buf.push_str(text.as_str());
        } else {
            if !text_buf.is_empty() {
                buf.push((style_buf, std::mem::take(&mut text_buf)));
            }
            text_buf = text;
            style_buf = style;
        }
    }
    buf
}

fn test_line_wrap(mut render_data: Vec<(ContentStyle, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["fn", " ", "get_long_line", "() ", "->", " ", "String", " {"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(
        line,
        vec![
            "    ",
            "let",
            " ",
            "b",
            " ",
            "=",
            " ",
            "\"text🚀text🚀text🚀text🚀text🚀text",
            ">"
        ]
    );
    assert!(render_data.is_empty());
}

fn test_content(mut render_data: Vec<(ContentStyle, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["use", " ", "super", "::", "code", "::", "CodeLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(line, vec!["use", " ", "super", "::", "EditorLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    let expect: Vec<&str> = vec![];
    assert_eq!(line, expect);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(4));
    assert_eq!(line, vec!["#", "[", "test", "]"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(5));
    assert_eq!(line, vec!["fn", " ", "test_insert", "() {"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(6));
    assert_eq!(
        line,
        vec![
            "    ", "let", " ", "mut", " ", "line", " ", "=", " ", "CodeLine", "::", "new", "(", "\"text\"", ".",
            "to_owned", "());"
        ]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(7));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "4", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(8));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'e'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(9));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(10));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'🚀'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(11));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "6", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(12));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "!", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(13));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "3", ", ", "'x'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(14));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "7", ");"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(15));
    assert_eq!(
        line,
        vec![
            "    ",
            "assert",
            "!",
            "(",
            "&",
            "line",
            ".",
            "to_string",
            "() ",
            "=",
            "=",
            " ",
            "\"te🚀xext\"",
            ");",
        ]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(16));
    assert_eq!(line, vec!["}"]);
}

fn test_content_select(mut render_data: Vec<(ContentStyle, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["use", " ", "super", "::", "code", "::", "CodeLine", ";"]);
    // select start char 10 split token
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(line, vec!["use", " ", "super", ":", ":", "EditorLine", ";", "~"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    assert_eq!(line, vec!["~"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(4));
    assert_eq!(line, vec!["#", "[", "test", "]", "~"]);
    // select end char 6 split token
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(5));
    assert_eq!(line, vec!["fn", " ", "tes", "t_insert", "() {", " "]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(6));
    assert_eq!(
        line,
        vec![
            "    ", "let", " ", "mut", " ", "line", " ", "=", " ", "CodeLine", "::", "new", "(", "\"text\"", ".",
            "to_owned", "());"
        ]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(7));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "4", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(8));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'e'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(9));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(10));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'🚀'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(11));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "6", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(12));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "!", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(13));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "3", ", ", "'x'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(14));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "7", ");"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(15));
    #[rustfmt::skip]
    assert_eq!(
        line,
        vec![ "    ", "assert", "!", "(", "&", "line", ".", "to_string", "() ", "=", "=", " ", "\"te🚀xext\"", ");",]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(16));
    assert_eq!(line, vec!["}"]);
}

#[inline]
fn test_content_shrunk(mut render_data: Vec<(ContentStyle, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["use", " ", "super", "::", "code", "::", "CodeLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(line, vec!["use", " ", "super", "::", "EditorLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    let expect: Vec<&str> = vec![];
    assert_eq!(line, expect);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(4));
    assert_eq!(line, vec!["#", "[", "test", "]"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(5));
    assert_eq!(line, vec!["fn", " ", "test_insert", "() {"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(6));
    assert_eq!(
        line,
        vec!["    ", "let", " ", "mut", " ", "line", " ", "=", " ", "CodeLine", "::", "new", "(", "\"text", ">",]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(7));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "4", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(8));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'e'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(9));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(10));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "2", ", ", "'🚀'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(11));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "6", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(12));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "!", "line", ".", "is_ascii", "());"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(13));
    assert_eq!(line, vec!["    ", "line", ".", "insert", "(", "3", ", ", "'x'", ");"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(14));
    assert_eq!(line, vec!["    ", "assert", "!", "(", "line", ".", "char_len", "() ", "=", "=", " ", "7", ");"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(15));
    #[rustfmt::skip]
    assert_eq!(
        line,
        vec![ "    ", "assert", "!", "(", "&", "line", ".", "to_string", "() ", "=", "=", " ", "\"te🚀", ">"]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(16));
    assert_eq!(line, vec!["}"]);
}

/// BASIC CURSOR TEST

#[test]
fn test_cursor() {
    let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position((0, 12).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gs = GlobalState::new(Backend::init()).unwrap();");

    let line = Line { row: 0, col: 0, width: 40 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);
    let mut rendered = gs.backend().drain();
    expect_cursor(cursor.char, "<<reset style>>", &rendered);
    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["let mut gs = GlobalState::new(Backen", ">"].into_iter().map(String::from).collect())
    );
}

#[test]
fn test_cursor_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.set_position((0, 12).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gs🧛 = GlobalState::new(Backend::init()).unwrap();");

    let line = Line { row: 0, col: 0, width: 40 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);
    let mut rendered = gs.backend().drain();
    expect_cursor(cursor.char, "<<reset style>>", &rendered);
    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["let mut gs🧛 = GlobalState::new(Backe", ">"].into_iter().map(String::from).collect())
    );
}

#[test]
fn test_cursor_select() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((0, 4).into(), (0, 15).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gs = GlobalState::new(Backend::init()).unwrap();");
    let line = Line { row: 0, col: 0, width: 40 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);

    let mut rendered = gs.backend().drain();
    assert_eq!(count_to_cursor(ctx.accent_style, &rendered), cursor.char);
    let style_select = gs.theme.selected;
    // panic!("{:?}", rendered);
    expect_select(4, 15, style_select, ctx.accent_style, &rendered);

    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["let ", "mut gs = Gl", "obalState::new(Backen", ">"].into_iter().map(String::from).collect())
    );
}

#[test]
fn test_cursor_select_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((0, 4).into(), (0, 15).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gs🧛 = GlobalState::new(Backend::init()).unwrap();");
    let line = Line { row: 0, col: 0, width: 40 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);

    let mut rendered = gs.backend().drain();
    assert_eq!(count_to_cursor(ctx.accent_style, &rendered), cursor.char);
    let style_select = gs.theme.selected;
    expect_select(4, 15, style_select, ctx.accent_style, &rendered);

    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["let ", "mut gs🧛 = G", "lobalState::new(Backe", ">"].into_iter().map(String::from).collect())
    );
}

#[test]
fn wrap_cursor() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((0, 20).into(), (0, 35).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gs = GlobalState::new(Backend::init()).unwrap();");
    let line = Line { row: 0, col: 0, width: 20 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);

    let mut rendered = gs.backend().drain();
    assert_eq!(count_to_cursor(ctx.accent_style, &rendered), cursor.char - 20);
    let style_select = gs.theme.selected;
    expect_select(1, 15, style_select, ctx.accent_style, &rendered);

    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["<", "", "ate::new(Backe", "n", ">"].into_iter().map(String::from).collect())
    );
}

#[test]
fn wrap_cursor_complex() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);
    let mut cursor = Cursor::default();
    cursor.select_set((0, 20).into(), (0, 35).into());

    let mut ctx =
        LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::fg(Color::DarkGrey));
    let mut code = EditorLine::from("let mut gsormandaaseaseaeas🧛fda🧛 = GlobalState::new(Backend::init()).unwrap();");
    let line = Line { row: 0, col: 0, width: 20 };
    rend_cursor(&mut code, &mut ctx, line, &mut gs);

    let mut rendered = gs.backend().drain();
    assert_eq!(count_to_cursor(ctx.accent_style, &rendered), cursor.char - 22); // 21 (20 + 2 due to width of emojieS)
    let style_select = gs.theme.selected;
    expect_select(1, 13, style_select, ctx.accent_style, &rendered);

    assert_eq!(
        parse_complex_line(&mut rendered),
        (Some(1), ["<", "aeas🧛fda🧛 = ", "Gl", ">"].into_iter().map(String::from).collect())
    );
}

/// LINE RENDER

#[test]
fn test_line_render_utf8() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content(gs.backend.drain());
}

#[test]
fn test_line_render_utf16() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf16_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content(gs.backend.drain());
}

#[test]
fn test_line_render_utf32() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf32_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content(gs.backend.drain());
}

#[test]
fn test_line_render_shrunk_utf8() {
    let limit = 42;

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_shrunk(gs.backend.drain());
}

#[test]
fn test_line_render_shrunk_utf16() {
    let limit = 42;

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf16_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_shrunk(gs.backend.drain());
}

#[test]
fn test_line_render_shrunk_utf32() {
    let limit = 42;

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf32_lexer(FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_shrunk(gs.backend.drain());
}

#[test]
fn test_line_render_select_utf8() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_select(gs.backend.drain());
}

#[test]
fn test_line_render_select_utf16() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf16_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_select(gs.backend.drain());
}

#[test]
fn test_line_render_select_utf32() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf32_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.select_get();
        line_render(code_line, &mut ctx, line, select, &mut gs);
    }

    test_content_select(gs.backend.drain());
}

#[test]
fn test_line_wrapping_utf8() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf8_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.select_get();
    line_render(&mut content[0], &mut ctx, line, select, &mut gs);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs);

    test_line_wrap(gs.backend.drain());
}

#[test]
fn test_line_wrapping_utf16() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf16_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.select_get();
    line_render(&mut content[0], &mut ctx, line, select, &mut gs);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs);

    test_line_wrap(gs.backend.drain());
}

#[test]
fn test_line_wrapping_utf32() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 60), CrossTerm::init());
    let lexer = mock_utf32_lexer(FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&cursor, lexer.encoding().char_len, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.select_get();
    line_render(&mut content[0], &mut ctx, line, select, &mut gs);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs);

    test_line_wrap(gs.backend.drain());
}

#[test]
fn test_select_padding() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 7), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    let base_text = vec![
        String::from("from os import environ"),
        String::from("variable_data = environ.get(\"crab\", \"crab\")"),
        String::new(),
        String::from(' '),
        String::from("print(f\"{varialbe_data} .. rocket\")"),
    ];

    let content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 4, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 5, length: 2, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 3, length: 6, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 7, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_line: 1, length: 13, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 16, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 8, length: 3, token_type: 8, ..Default::default() },
            SemanticToken { delta_start: 4, length: 6, token_type: 13, ..Default::default() },
            SemanticToken { delta_start: 8, length: 6, token_type: 13, ..Default::default() },
            SemanticToken { delta_line: 3, length: 5, token_type: 6, ..Default::default() },
            SemanticToken { delta_start: 7, length: 27, token_type: 14, ..Default::default() },
        ],
    );
    editor.content = content;
    editor.file_type = FileType::Python;
    editor.lexer = mock_utf32_lexer(FileType::Python);
    editor.cursor.select_set((2, 10).into(), (0, 4).into());
    editor.render(&mut gs);
    _ = gs.backend.drain(); // ensuore all lines are rendered
    editor.render(&mut gs);

    let select_style = ContentStyle::bg(gs.theme.selected);

    let expected = vec![
        (ContentStyle::default(), "<<go to row: 1 col: 19>>1 <<clear EOL>><<set style>>from".into()),
        (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>><<set style>>".into()),
        (ContentStyle::reversed(), " ".into()),
        (
            select_style,
            "<<updated style>>os<<set style>> <<updated style>>import<<set style>> <<updated style>>environ".into(),
        ),
        (ContentStyle::default(), "<<reset style>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<reset style>><<go to row: 2 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "2 ".into()),
        (ContentStyle::default(), "<<clear EOL>><<set style>>".into()),
        (
            select_style,
            "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>variable_data<<set style>> = <<updated style>>environ\
            <<set style>>.<<updated style>>get<<set style>>(<<updated style>>\"crab\"\
            <<set style>>, <<updated style>>\"crab\"<<set style>>)"
                .into(),
        ),
        (ContentStyle::default(), "<<reset style>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 3 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "3 ".into()),
        (ContentStyle::default(), "<<clear EOL>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 4 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "4 ".into()),
        (ContentStyle::default(), "<<clear EOL>> <<go to row: 5 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "5 ".into()),
        (ContentStyle::default(), "<<clear EOL>>print(f\"{varialbe_data} .. rocket\")".into()),
        (gs.ui_theme.accent_style(), "<<set style>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 102>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 106>>".into()),
        (gs.ui_theme.accent_style(), "(73 selected) ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 81>>".into()),
        (gs.ui_theme.accent_style(), "  Doc Len 5, Ln 0, Col 4 ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 63>>".into()),
    ];
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expected);

    editor.lexer = mock_utf16_lexer(FileType::Python);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&expected[20..], &result[20..]);

    editor.lexer = mock_utf8_lexer(FileType::Python);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&expected, &result);
}

#[test]
fn test_select_padding_complex() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 7), CrossTerm::init());
    gs.force_area_calc();
    let select_style = ContentStyle::bg(gs.theme.selected);

    let base_text = vec![
        String::from("from os import environ;"),
        String::from("variable_data = environ.get(\"🦀\", \"🦀\")"),
        String::new(),
        String::from(' '),
        String::from("print(f\"{varialbe_data} .. 🚀\")"),
    ];

    let expected = vec![
        (ContentStyle::default(), "<<go to row: 1 col: 19>>1 <<clear EOL>><<set style>>from".into()),
        (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>><<set style>>".into()),
        (ContentStyle::reversed(), " ".into()),
        (
            select_style,
            "<<updated style>>os<<set style>> <<updated style>>import\
            <<set style>> <<updated style>>environ<<set style>>;"
                .into(),
        ),
        (ContentStyle::default(), "<<reset style>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<reset style>><<go to row: 2 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "2 ".into()),
        (ContentStyle::default(), "<<clear EOL>><<set style>>".into()),
        (
            select_style,
            "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>variable_data<<set style>> = \
            <<updated style>>environ<<set style>>.<<updated style>>get<<set style>>(\
            <<updated style>>\"🦀\"<<set style>>, <<updated style>>\"🦀\"<<set style>>)"
                .into(),
        ),
        (ContentStyle::default(), "<<reset style>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 3 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "3 ".into()),
        (ContentStyle::default(), "<<clear EOL>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 4 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "4 ".into()),
        (ContentStyle::default(), "<<clear EOL>> <<go to row: 5 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "5 ".into()),
        (
            ContentStyle::default(),
            "<<clear EOL>><<set style>>print<<reset style>>(f<<set style>>\"{varialbe_data} .. 🚀\"\
            <<reset style>>)<<reset style>>"
                .into(),
        ),
        (gs.ui_theme.accent_style(), "<<set style>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 102>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 106>>".into()),
        (gs.ui_theme.accent_style(), "(68 selected) ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 81>>".into()),
        (gs.ui_theme.accent_style(), "  Doc Len 5, Ln 0, Col 4 ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 63>>".into()),
    ];

    let content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 4, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 5, length: 2, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 3, length: 6, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 7, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_line: 1, length: 13, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 16, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 8, length: 3, token_type: 8, ..Default::default() },
            SemanticToken { delta_start: 4, length: 3, token_type: 13, ..Default::default() },
            SemanticToken { delta_start: 5, length: 3, token_type: 13, ..Default::default() },
            SemanticToken { delta_line: 3, length: 5, token_type: 6, ..Default::default() },
            SemanticToken { delta_start: 7, length: 22, token_type: 14, ..Default::default() },
        ],
    );
    editor.content = content;
    editor.file_type = FileType::Python;
    editor.lexer = mock_utf32_lexer(FileType::Python);
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.cursor.select_set((2, 10).into(), (0, 4).into());
    editor.render(&mut gs);
    gs.backend.drain(); // ensure all rect are calculated
    editor.render(&mut gs);

    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expected);

    let content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 4, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 5, length: 2, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 3, length: 6, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 7, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_line: 1, length: 13, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 16, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 8, length: 3, token_type: 8, ..Default::default() },
            SemanticToken { delta_start: 4, length: 4, token_type: 13, ..Default::default() },
            SemanticToken { delta_start: 6, length: 4, token_type: 13, ..Default::default() },
            SemanticToken { delta_line: 3, length: 5, token_type: 6, ..Default::default() },
            SemanticToken { delta_start: 7, length: 23, token_type: 14, ..Default::default() },
        ],
    );
    editor.content = content;
    editor.lexer = mock_utf16_lexer(FileType::Python);
    editor.render(&mut gs);

    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expected);

    let content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 4, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 5, length: 2, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 3, length: 6, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 7, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_line: 1, length: 13, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 16, length: 7, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 8, length: 3, token_type: 8, ..Default::default() },
            SemanticToken { delta_start: 4, length: 6, token_type: 13, ..Default::default() },
            SemanticToken { delta_start: 8, length: 6, token_type: 13, ..Default::default() },
            SemanticToken { delta_line: 3, length: 5, token_type: 6, ..Default::default() },
            SemanticToken { delta_start: 7, length: 25, token_type: 14, ..Default::default() },
        ],
    );
    editor.content = content;
    editor.lexer = mock_utf8_lexer(FileType::Python);
    editor.render(&mut gs);

    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expected);
}

#[test]
fn test_select_end_line_end() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 7), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.cursor.select_set((0, 24).into(), (1, 40).into());
    let select_style = ContentStyle::bg(gs.theme.selected);

    let base_text = vec![
        String::from("use std::time::Duration;"),
        String::from("const DUR: Duration = Duration::from_secs(69)"),
    ];
    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 3, token_type: 2, ..Default::default() },
            SemanticToken { delta_start: 4, length: 3, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 5, length: 4, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 6, length: 8, token_type: 3, ..Default::default() },
            SemanticToken { delta_line: 1, length: 5, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 6, length: 3, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 5, length: 8, token_type: 1, ..Default::default() },
            SemanticToken { delta_start: 11, length: 8, token_type: 1, ..Default::default() },
            SemanticToken { delta_start: 10, length: 9, token_type: 10, ..Default::default() },
            SemanticToken { delta_start: 10, length: 2, token_type: 22, ..Default::default() },
        ],
    );

    let expect = vec![
        (ContentStyle::default(), "<<go to row: 1 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "1 ".into()),
        (ContentStyle::default(), "<<clear EOL>>use std::time::Duration;".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 2 col: 19>>2 <<clear EOL>><<set style>>".into()),
        (
            select_style,
            "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>const<<set style>> \
            <<updated style>>DUR<<set style>>: <<updated style>>Duration\
            <<set style>> = <<updated style>>Duration<<set style>>::<<updated style>>from_sec"
                .into(),
        ),
        (ContentStyle::default(), "<<set bg None>>".into()),
        (ContentStyle::reversed(), "s".into()),
        (
            ContentStyle::default(),
            "<<set style>>(<<updated style>>69<<set style>>)<<reset style>> \
            <<reset style>><<go to row: 3 col: 19>><<padding: 101>>\
            <<go to row: 4 col: 19>><<padding: 101>><<go to row: 5 col: 19>><<padding: 101>>"
                .into(),
        ),
        (gs.ui_theme.accent_style(), "<<set style>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 102>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 106>>".into()),
        (gs.ui_theme.accent_style(), "(41 selected) ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 80>>".into()),
        (gs.ui_theme.accent_style(), "  Doc Len 2, Ln 1, Col 40 ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 62>>".into()),
    ];

    editor.lexer = mock_utf32_lexer(FileType::Rust);
    editor.render(&mut gs);
    gs.backend.drain();

    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);

    editor.lexer = mock_utf16_lexer(FileType::Rust);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);

    editor.lexer = mock_utf8_lexer(FileType::Rust);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);
}

#[test]
fn test_select_end_line_end_complex() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 120, 7), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.cursor.select_set((0, 15).into(), (1, 23).into());
    let select_style = ContentStyle::bg(gs.theme.selected);

    let base_text = vec![
        String::from("/// some docs 🦀"),
        String::from("const ROCKET: &str = \"🚀\";"),
    ];
    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 15, token_type: 9, ..Default::default() },
            SemanticToken { delta_line: 1, length: 5, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 6, length: 6, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 9, length: 3, token_type: 1, ..Default::default() },
            SemanticToken { delta_start: 6, length: 3, token_type: 13, ..Default::default() },
        ],
    );

    let expect = vec![
        (ContentStyle::default(), "<<go to row: 1 col: 19>>".into()),
        (gs.ui_theme.accent_fg(), "1 ".into()),
        (ContentStyle::default(), "<<clear EOL>><<set style>>/// some docs 🦀<<reset style>>".into()),
        (gs.get_accented_select(), "~".into()),
        (ContentStyle::default(), "<<go to row: 2 col: 19>>2 <<clear EOL>><<set style>>".into()),
        (
            select_style,
            "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>const<<set style>> <<updated style>>ROCKET\
            <<set style>>: &<<updated style>>str<<set style>> = <<updated style>>\"🚀"
                .into(),
        ),
        (ContentStyle::default(), "<<set bg None>>".into()),
        (ContentStyle::reversed(), "\"".into()),
        (
            ContentStyle::default(),
            "<<set style>>;<<reset style>> <<reset style>><<go to row: 3 col: 19>><<padding: 101>>\
            <<go to row: 4 col: 19>><<padding: 101>><<go to row: 5 col: 19>><<padding: 101>>"
                .into(),
        ),
        (gs.ui_theme.accent_style(), "<<set style>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 102>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 106>>".into()),
        (gs.ui_theme.accent_style(), "(24 selected) ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 80>>".into()),
        (gs.ui_theme.accent_style(), "  Doc Len 2, Ln 1, Col 23 ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".into()),
        (gs.ui_theme.accent_style(), "<<padding: 62>>".into()),
    ];

    editor.lexer = mock_utf32_lexer(FileType::Rust);
    editor.render(&mut gs);
    gs.backend.drain();

    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);

    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 15, ..Default::default() },
            SemanticToken { delta_line: 1, length: 5, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 6, length: 6, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 9, length: 3, token_type: 1, ..Default::default() },
            SemanticToken { delta_start: 6, length: 4, token_type: 13, ..Default::default() },
        ],
    );
    editor.lexer = mock_utf16_lexer(FileType::Rust);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);

    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 15, ..Default::default() },
            SemanticToken { delta_line: 1, length: 5, token_type: 3, ..Default::default() },
            SemanticToken { delta_start: 6, length: 6, token_type: 4, ..Default::default() },
            SemanticToken { delta_start: 9, length: 3, token_type: 1, ..Default::default() },
            SemanticToken { delta_start: 6, length: 6, token_type: 13, ..Default::default() },
        ],
    );
    editor.lexer = mock_utf8_lexer(FileType::Rust);
    editor.render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);
}

#[test]
fn test_wrap_select() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 7), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor.cursor.select_set((1, 5).into(), (1, 0).into());
    let select_style = ContentStyle::bg(gs.theme.selected);

    let base_text = vec![
        String::from("/// text to get wrapping docs crab"),
        String::from("/// rocket "),
    ];
    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 15, token_type: 9, ..Default::default() },
            SemanticToken { delta_line: 1, length: 7, token_type: 9, ..Default::default() },
        ],
    );

    editor.render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    gs.backend.drain();

    let expect = vec![
        (ContentStyle::default(), "<<go to row: 1 col: 15>>1 <<clear EOL>>".into()),
        (ContentStyle::reversed().with_fg(gs.ui_theme.accent()), "<".into()),
        (ContentStyle::default(), "<<updated style>>to get<<set style>> wrapping docs cr".into()),
        (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>".into()),
        (ContentStyle::reversed(), "a".into()),
        (select_style, "b".into()),
        (
            ContentStyle::default(),
            "<<reset style>><<reset style>>\
            <<go to row: 3 col: 15>><<padding: 30>>\
            <<go to row: 4 col: 15>><<padding: 30>>\
            <<go to row: 5 col: 15>><<padding: 30>>"
                .into(),
        ),
        (ContentStyle::default().with_bg(gs.ui_theme.accent()), "<<set style>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
        (ContentStyle::default().with_bg(gs.ui_theme.accent()), "<<padding: 31>>".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 32>>".into()),
        (ContentStyle::default().with_bg(gs.ui_theme.accent()), "(8 selected) ".into()),
        (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
        (ContentStyle::default().with_bg(gs.ui_theme.accent()), "n 2, Ln 0, Col 32 ".into()),
    ];
    editor.fast_render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(&result, &expect);
}

#[test]
fn test_wrap_select_complex() {
    let mut editor = mock_editor(vec![]);
    let mut gs = GlobalState::new(Rect::new(0, 0, 45, 7), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    let select_style = ContentStyle::bg(gs.theme.selected);

    let base_text = vec![
        String::from("/// text to get wrapping docs 🦀"),
        String::from("/// 🚀  "),
    ];
    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 15, token_type: 9, ..Default::default() },
            SemanticToken { delta_line: 1, length: 7, token_type: 9, ..Default::default() },
        ],
    );

    editor.lexer = mock_utf32_lexer(FileType::Rust);

    editor.cursor.select_set((1, 5).into(), (1, 0).into());
    editor.render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);

    gs.backend.drain();
    editor.fast_render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(
        result,
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 15>>1 <<clear EOL>><<updated style>>".into()),
            (ContentStyle::reversed().with_fg(gs.ui_theme.accent()), "<".into()),
            (ContentStyle::default(), "t to get<<set style>> wrapping docs".into()),
            (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>".into()),
            (ContentStyle::reversed(), " ".into()),
            (select_style, "🦀".into()),
            (ContentStyle::default(), "<<reset style>><<reset style>><<go to row: 3 col: 15>><<padding: 30>><<go to row: 4 col: 15>><<padding: 30>><<go to row: 5 col: 15>><<padding: 30>>".into()),
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 31>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 32>>".into()),
            (gs.ui_theme.accent_style(), "(8 selected) ".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "n 2, Ln 0, Col 29 ".into())
        ]
    );

    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 16, token_type: 9, ..Default::default() },
            SemanticToken { delta_line: 1, length: 8, token_type: 9, ..Default::default() },
        ],
    );

    editor.cursor.select_set((1, 5).into(), (1, 0).into());
    editor.render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);

    editor.lexer = mock_utf16_lexer(FileType::Rust);
    gs.backend.drain();
    editor.fast_render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(
        result,
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 15>>1 <<clear EOL>><<updated style>>".into()),
            (ContentStyle::reversed().with_fg(gs.ui_theme.accent()), "<".into()),
            (ContentStyle::default(), "t to get <<set style>>wrapping docs".into()),
            (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>".into()),
            (ContentStyle::reversed(), " ".into()),
            (select_style, "🦀".into()),
            (
                ContentStyle::default(),
                "<<reset style>><<reset style>><<go to row: 3 col: 15>><<padding: 30>>\
                <<go to row: 4 col: 15>><<padding: 30>><<go to row: 5 col: 15>><<padding: 30>>"
                    .into()
            ),
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 31>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 32>>".into()),
            (gs.ui_theme.accent_style(), "(8 selected) ".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "n 2, Ln 0, Col 29 ".into())
        ]
    );

    editor.content = zip_text_tokens(
        base_text.clone(),
        vec![
            SemanticToken { length: 18, token_type: 9, ..Default::default() },
            SemanticToken { delta_line: 1, length: 10, token_type: 9, ..Default::default() },
        ],
    );

    editor.cursor.select_set((1, 5).into(), (1, 0).into());
    editor.render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);
    editor.fast_render(&mut gs);
    editor.map(crate::configs::EditorAction::SelectLeft, &mut gs);

    editor.lexer = mock_utf8_lexer(FileType::Rust);
    gs.backend.drain();
    editor.fast_render(&mut gs);
    let result = consolidate_backend_drain(gs.backend.drain());
    assert_eq!(
        result,
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 15>>1 <<clear EOL>><<updated style>>".into()),
            (ContentStyle::reversed().with_fg(gs.ui_theme.accent()), "<".into()),
            (ContentStyle::default(), "t to get wr<<set style>>apping docs".into()),
            (select_style, "<<set bg Some(Rgb { r: 72, g: 72, b: 72 })>>".into()),
            (ContentStyle::reversed(), " ".into()),
            (select_style, "🦀".into()),
            (
                ContentStyle::default(),
                "<<reset style>><<reset style>><<go to row: 3 col: 15>><<padding: 30>>\
                <<go to row: 4 col: 15>><<padding: 30>><<go to row: 5 col: 15>><<padding: 30>>"
                    .into()
            ),
            (gs.ui_theme.accent_style(), "<<set style>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "<<padding: 31>>".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 32>>".into()),
            (gs.ui_theme.accent_style(), "(8 selected) ".into()),
            (ContentStyle::default(), "<<go to row: 6 col: 14>>".into()),
            (gs.ui_theme.accent_style(), "n 2, Ln 0, Col 29 ".into()),
        ]
    );
}

use super::{
    super::tests::{parse_complex_line, parse_simple_line},
    cursor as rend_cursor, inner_render,
};
use crate::configs::FileType;
use crate::global_state::GlobalState;
use crate::render::backend::{Backend, BackendProtocol};
use crate::render::layout::{Line, Rect};
use crate::syntax::tests::{
    create_token_pairs_utf16, create_token_pairs_utf32, create_token_pairs_utf8, longline_token_pair_utf16,
    longline_token_pair_utf32, longline_token_pair_utf8, mock_utf16_lexer, mock_utf32_lexer, mock_utf8_lexer,
    zip_text_tokens,
};
use crate::workspace::cursor::Cursor;
use crate::workspace::line::LineContext;
use crate::workspace::CursorPosition;
use crossterm::style::ContentStyle;

/// LINE RENDER

#[test]
fn test_line_render_utf8() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content(gs.writer.drain());
}

#[test]
fn test_line_render_utf16() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf16_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content(gs.writer.drain());
}

#[test]
fn test_line_render_utf32() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf32_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content(gs.writer.drain());
}

#[test]
fn test_line_render_shrunk_utf8() {
    let limit = 42;

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_shrunk(gs.writer.drain());
}

#[test]
fn test_line_render_shrunk_utf16() {
    let limit = 42;

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf16_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_shrunk(gs.writer.drain());
}

#[test]
fn test_line_render_shrunk_utf32() {
    let limit = 42;

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf32_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: limit };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_shrunk(gs.writer.drain());
}

#[test]
fn test_line_render_select_utf8() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_select(gs.writer.drain());
}

#[test]
fn test_line_render_select_utf16() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf16_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_select(gs.writer.drain());
}

#[test]
fn test_line_render_select_utf32() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf32_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.select_set(CursorPosition { line: 1, char: 10 }, CursorPosition { line: 4, char: 6 });

    let (tokens, text) = create_token_pairs_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 2, ContentStyle::default());

    for (idx, code_line) in content.iter_mut().enumerate() {
        let line = Line { row: idx as u16, col: 0, width: 100 };
        let select = ctx.get_select(line.width);
        inner_render(code_line, &mut ctx, line, select, &mut gs.writer);
    }

    test_content_select(gs.writer.drain());
}

#[test]
fn test_line_wrapping_utf8() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.get_select(line.width);
    inner_render(&mut content[0], &mut ctx, line, select, &mut gs.writer);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs.writer);

    test_line_wrap(gs.writer.drain());
}

#[test]
fn test_line_wrapping_utf16() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf16_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf16();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.get_select(line.width);
    inner_render(&mut content[0], &mut ctx, line, select, &mut gs.writer);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs.writer);

    test_line_wrap(gs.writer.drain());
}

#[test]
fn test_line_wrapping_utf32() {
    let rect = Rect::new(0, 0, 50, 5);
    let mut lines = rect.into_iter();

    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf32_lexer(&mut gs, FileType::Rust);

    let mut cursor = Cursor::default();
    cursor.set_position(CursorPosition { line: 1, char: 0 });

    let (tokens, text) = longline_token_pair_utf32();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = LineContext::collect_context(&mut lexer, &cursor, 1, ContentStyle::default());
    let line = lines.next().unwrap();
    let select = ctx.get_select(line.width);
    inner_render(&mut content[0], &mut ctx, line, select, &mut gs.writer);
    let line = lines.next().unwrap();
    let text = &mut content[1];
    rend_cursor(text, &mut ctx, line, &mut gs.writer);

    test_line_wrap(gs.writer.drain());
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
    assert_eq!(line, vec!["use", " ", "super", ":", ":", "EditorLine", ";"]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    assert_eq!(line, vec![" "]);
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(4));
    assert_eq!(line, vec!["#", "[", "test", "]"]);
    // select end char 6 split token
    let (line_num, line) = parse_complex_line(&mut render_data);
    assert_eq!(line_num, Some(5));
    assert_eq!(line, vec!["fn", " ", "tes", "t_insert", "() {"]);
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
            "\"te🚀",
            ">"
        ]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(16));
    assert_eq!(line, vec!["}"]);
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

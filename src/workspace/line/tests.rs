use super::code::CodeLine;
use super::EditorLine;
use crate::configs::FileType;
use crate::global_state::GlobalState;
use crate::render::backend::{Backend, BackendProtocol, Style};
use crate::render::layout::{Line, Rect};
use crate::syntax::tests::{
    create_token_pairs_utf16, create_token_pairs_utf32, create_token_pairs_utf8, longline_token_pair_utf16,
    longline_token_pair_utf32, longline_token_pair_utf8, mock_utf16_lexer, mock_utf32_lexer, mock_utf8_lexer,
    zip_text_tokens,
};
use crate::workspace::cursor::Cursor;
use crate::workspace::line::CodeLineContext;
use crate::workspace::CursorPosition;

#[test]
fn test_insert() {
    let mut line = CodeLine::new("text".to_owned());
    assert!(line.char_len() == 4);
    line.insert(2, 'e');
    assert!(line.is_simple());
    line.insert(2, '🚀');
    assert!(line.char_len() == 6);
    assert!(!line.is_simple());
    line.insert(3, 'x');
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "te🚀xext");
}

#[test]
fn test_insert_str() {
    let mut line = CodeLine::new("text".to_owned());
    line.insert_str(0, "text");
    assert!(line.is_simple());
    assert!(line.char_len() == 8);
    line.insert_str(1, "rocket🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "trocket🚀exttext");
    assert!(line.char_len() < line.to_string().len());
}

#[test]
fn test_push() {
    let mut line = CodeLine::new("text".to_owned());
    line.push('1');
    assert!(line.is_simple());
    assert!(line.char_len() == 5);
    line.push('🚀');
    assert!(!line.is_simple());
    assert!(line.to_string().len() == 9);
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text1🚀");
}

#[test]
fn test_push_str() {
    let mut line = CodeLine::new(String::new());
    assert!(line.is_simple());
    assert!(line.char_len() == 0);
    line.push_str("text");
    assert!(line.is_simple());
    assert!(line.char_len() == 4);
    line.push_str("text🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "texttext🚀");
    assert!(line.char_len() == 9);
    assert!(line.to_string().len() == 12);
}

#[test]
fn test_replace_range() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_range(0..2, "text");
    assert!(line.is_simple());
    assert!(&line.to_string() == "text23");
    assert!(line.char_len() == 6);
    line.replace_range(3..6, "🚀🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "tex🚀🚀");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_till() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_till(3, "text");
    assert!(line.is_simple());
    assert!(&line.to_string() == "text3");
    assert!(line.char_len() == 5);
    line.replace_till(2, "🚀🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "🚀🚀xt3");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_from() {
    let mut line = CodeLine::new(String::from("123🚀"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_from(3, "text");
    assert!(line.is_simple());
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "123text");
    line.replace_from(3, "🚀🚀");
    assert!(!line.is_simple());
    assert!(line.char_len() == 5);
    assert!(&line.to_string() == "123🚀🚀");
}

#[test]
fn test_remove() {
    let mut line = CodeLine::new("text🚀123".to_owned());
    assert!(!line.is_simple());
    assert!(line.char_len() == 8);
    assert!('1' == line.remove(5));
    assert!(line.char_len() == 7);
    assert!(!line.is_simple());
    assert!('🚀' == line.remove(4));
    assert!(line.is_simple());
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text23");
}

#[test]
fn test_utf8_idx_at() {
    let line = CodeLine::new("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf8_idx_at(4));
    assert_eq!(2, line.unsafe_utf8_idx_at(2));
    assert_eq!(8, line.unsafe_utf8_idx_at(5));
    assert_eq!(10, line.unsafe_utf8_idx_at(7));
    assert_eq!(15, line.unsafe_utf8_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf8_idx_at_panic() {
    let line = CodeLine::new("text🚀123🚀".to_owned());
    line.unsafe_utf8_idx_at(10);
}

#[test]
fn test_utf16_idx_at() {
    let line = CodeLine::new("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf16_idx_at(4));
    assert_eq!(2, line.unsafe_utf16_idx_at(2));
    assert_eq!(6, line.unsafe_utf16_idx_at(5));
    assert_eq!(8, line.unsafe_utf16_idx_at(7));
    assert_eq!(11, line.unsafe_utf16_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf16_idx_at_panic() {
    let line = CodeLine::new("text🚀123🚀".to_owned());
    line.unsafe_utf16_idx_at(10);
}

#[test]
fn test_split_off() {
    let mut line = CodeLine::new("text🚀123🚀".to_owned());
    line = line.split_off(2);
    assert_eq!(line.to_string(), "xt🚀123🚀");
    assert_eq!(line.char_len(), 7);
    assert_eq!(line.len(), 13);
    let new = line.split_off(4);
    assert_eq!(new.char_len(), 3);
    assert_eq!(new.len(), 6);
    assert_eq!(new.unwrap(), "23🚀");
}

#[test]
fn test_split_off_ascii() {
    let mut line = CodeLine::new("texttext".to_owned());
    let remaining = line.split_off(4);
    assert_eq!(remaining.char_len(), 4);
    assert_eq!(remaining.len(), 4);
    assert_eq!(remaining.to_string(), "text");
    assert_eq!(line.char_len(), 4);
    assert_eq!(line.len(), 4);
    assert_eq!(line.to_string(), "text");
    assert_eq!(line.to_string(), "text");
}

/// LINE RENDER

#[test]
fn test_line_render_utf8() {
    let mut gs = GlobalState::new(Backend::init()).unwrap();
    let mut lexer = mock_utf8_lexer(&mut gs, FileType::Rust);

    let cursor = Cursor::default();

    let (tokens, text) = create_token_pairs_utf8();
    let mut content = zip_text_tokens(text, tokens);

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: limit }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: limit }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: limit }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 2);

    for (idx, code_line) in content.iter_mut().enumerate() {
        code_line.render(&mut ctx, Line { row: idx as u16, col: 0, width: 100 }, &mut gs.writer);
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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 1);
    content[0].render(&mut ctx, lines.next().unwrap(), &mut gs.writer);
    content[1].cursor(&mut ctx, &mut lines, &mut gs.writer);

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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 1);
    content[0].render(&mut ctx, lines.next().unwrap(), &mut gs.writer);
    content[1].cursor(&mut ctx, &mut lines, &mut gs.writer);

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

    let mut ctx = CodeLineContext::collect_context(&mut lexer, &cursor, 1);
    content[0].render(&mut ctx, lines.next().unwrap(), &mut gs.writer);
    content[1].cursor(&mut ctx, &mut lines, &mut gs.writer);

    test_line_wrap(gs.writer.drain());
}

fn parse_simple_line(rendered: &mut Vec<(Style, String)>) -> (Option<usize>, Vec<String>) {
    let mut line_idx = None;
    for (idx, (_, txt)) in rendered.iter().enumerate() {
        if !txt.starts_with("<<go to row") {
            line_idx = txt.trim().parse().ok();
            rendered.drain(..idx + 2);
            break;
        }
    }
    for (idx, (_, t)) in rendered.iter().enumerate() {
        if t.starts_with("<<go to row") {
            return (line_idx, rendered.drain(..idx).map(|(_, t)| t).collect());
        }
    }
    (line_idx, rendered.drain(..).map(|(_, t)| t).collect())
}

fn parse_complex_line(rendered: &mut Vec<(Style, String)>) -> (Option<usize>, Vec<String>) {
    let (line_idx, raw_data) = parse_simple_line(rendered);
    let mut parsed = vec![];
    let mut current = String::new();
    let mut first = true;
    for part in raw_data {
        if part.starts_with("<<") {
            if first {
                continue;
            }
            parsed.push(std::mem::take(&mut current));
        } else {
            current.push_str(&part);
        }
        first = false;
    }
    if !current.is_empty() {
        parsed.push(current);
    }
    (line_idx, parsed)
}

#[inline]
fn test_content(mut render_data: Vec<(Style, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["use", " ", "super", "::", "code", "::", "CodeLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(line, vec!["use", " ", "super", "::", "EditorLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    assert_eq!(line, vec![""]);
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
    assert_eq!(line, vec!["}", ""]);
}

#[inline]
fn test_content_select(mut render_data: Vec<(Style, String)>) {
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
    assert_eq!(line, vec!["}", ""]);
}

#[inline]
fn test_content_shrunk(mut render_data: Vec<(Style, String)>) {
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(1));
    assert_eq!(line, vec!["use", " ", "super", "::", "code", "::", "CodeLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(2));
    assert_eq!(line, vec!["use", " ", "super", "::", "EditorLine", ";"]);
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(3));
    assert_eq!(line, vec![""]);
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
        vec!["    ", "let", " ", "mut", " ", "line", " ", "=", " ", "CodeLine", "::", "new", "(", "\"tex", ">>",]
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
            "\"te",
            ">>"
        ]
    );
    let (line_num, line) = parse_simple_line(&mut render_data);
    assert_eq!(line_num, Some(16));
    assert_eq!(line, vec!["}", ""]);
}

fn test_line_wrap(mut render_data: Vec<(Style, String)>) {
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
            "\"text🚀text🚀text🚀text🚀text🚀tex",
            ">>"
        ]
    );
    assert!(render_data.is_empty());
}

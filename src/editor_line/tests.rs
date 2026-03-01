pub use crate::editor_line::parser::{LineParser, CARRIAGE_NLINE, MSDOS_NLINE, POSIX_NLINE, RISCOS_NLINE};
use crate::{editor::syntax::Encoding, editor_line::EditorLine};
use logos::Logos;

#[test]
fn test_get() {
    let line = EditorLine::from("bytb 🚀 asd 🚀");
    assert_eq!(line.get(1, 2), Some("y"));
    assert_eq!(line.get(2, 3), Some("t"));
    assert_eq!(line.get(5, 6), Some("🚀"));
    assert_eq!(line.get(10, 11), Some(" "));
    assert_eq!(line.get(11, 12), Some("🚀"));
    assert_eq!(line.get(11, 11), Some(""));
    assert_eq!(line.get(100, 200), None);
}

#[test]
fn trim_counted() {
    let line = EditorLine::from("   // coment");
    assert_eq!(line.trim_start_counted(), (3, "// coment"));
    let line = EditorLine::from("  🚀 //coment");
    assert_eq!(line.trim_start_counted(), (2, "🚀 //coment"));
}

#[test]
fn test_insert() {
    let mut line = EditorLine::new_posix("text".to_owned());
    assert!(line.char_len() == 4);
    let encoding = Encoding::utf32();
    line.insert_simple(2, 'e', &encoding);
    assert!(line.is_simple());
    line.insert_simple(2, '🚀', &encoding);
    assert!(line.char_len() == 6);
    assert!(!line.is_simple());
    line.insert_simple(3, 'x', &encoding);
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "te🚀xext");
}

#[test]
fn test_insert_str() {
    let mut line = EditorLine::new_posix("text".to_owned());
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
    let encoding = Encoding::utf32();
    let mut line = EditorLine::new_posix("text".to_owned());
    line.push_simple('1', &encoding);
    assert!(line.is_simple());
    assert!(line.char_len() == 5);
    line.push_simple('🚀', &encoding);
    assert!(!line.is_simple());
    assert!(line.to_string().len() == 9);
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text1🚀");
}

#[test]
fn test_push_str() {
    let mut line = EditorLine::new_posix(String::new());
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
    let mut line = EditorLine::new_posix(String::from("🚀123"));
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
    let mut line = EditorLine::new_posix(String::from("🚀123"));
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
    let mut line = EditorLine::new_posix(String::from("123🚀"));
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
    let mut line = EditorLine::new_posix("text🚀123".to_owned());
    let encoding = Encoding::utf32();
    assert!(!line.is_simple());
    assert!(line.char_len() == 8);
    assert!('1' == line.remove(5, &encoding));
    assert!(line.char_len() == 7);
    assert!(!line.is_simple());
    assert!('🚀' == line.remove(4, &encoding));
    assert!(line.is_simple());
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text23");
}

#[test]
fn test_utf8_idx_at() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf8_idx_at(4));
    assert_eq!(2, line.unsafe_utf8_idx_at(2));
    assert_eq!(8, line.unsafe_utf8_idx_at(5));
    assert_eq!(10, line.unsafe_utf8_idx_at(7));
    assert_eq!(15, line.unsafe_utf8_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf8_idx_at_panic() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    line.unsafe_utf8_idx_at(10);
}

#[test]
fn test_utf16_idx_at() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf16_idx_at(4));
    assert_eq!(2, line.unsafe_utf16_idx_at(2));
    assert_eq!(6, line.unsafe_utf16_idx_at(5));
    assert_eq!(8, line.unsafe_utf16_idx_at(7));
    assert_eq!(11, line.unsafe_utf16_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf16_idx_at_panic() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    line.unsafe_utf16_idx_at(10);
}

#[test]
fn test_split_off() {
    let mut line = EditorLine::new_posix("text🚀123🚀".to_owned());
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
    let mut line = EditorLine::new_posix("texttext".to_owned());
    let remaining = line.split_off(4);
    assert_eq!(remaining.char_len(), 4);
    assert_eq!(remaining.len(), 4);
    assert_eq!(remaining.to_string(), "text");
    assert_eq!(line.char_len(), 4);
    assert_eq!(line.len(), 4);
    assert_eq!(line.to_string(), "text");
    assert_eq!(line.to_string(), "text");
}

#[test]
fn test_parse() {
    let text = "a💀w\ndawda\radaw\r\nwas\n_💀_\n\r+++";
    let content = LineParser::split_lines(text).into_editor_lines();
    let expect = [
        EditorLine::new("a💀w".into(), POSIX_NLINE),
        EditorLine::new("dawda".into(), CARRIAGE_NLINE),
        EditorLine::new("adaw".into(), MSDOS_NLINE),
        EditorLine::new("was".into(), POSIX_NLINE),
        EditorLine::new("_💀_".into(), RISCOS_NLINE),
        EditorLine::new("+++".into(), POSIX_NLINE),
    ];
    assert_eq!(content.len(), expect.len());
    for (real, expect) in content.iter().zip(expect.iter()) {
        assert_eq!(real.as_str(), expect.as_str());
        assert_eq!(real.end(), expect.end());
    }
}

#[test]
fn parser_stable() {
    let data = "asd\n\tasdawdaw\n\radwadawdawda\r\nadawdadwd\radwa";
    let tokens = LineParser::lexer(data);
    assert_eq!(
        tokens.collect::<Vec<_>>(),
        [
            Ok(LineParser::POSIX_NEWLINE),
            Ok(LineParser::TAB),
            Ok(LineParser::RISCOS_NEWLINE),
            Ok(LineParser::MSDOS_NEWLINE),
            Ok(LineParser::CARRIAGE_NEWLINE)
        ],
    );
}

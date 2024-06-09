use super::code::CodeLine;
use super::EditorLine;

#[test]
fn test_insert() {
    let mut line = CodeLine::new("text".to_owned());
    assert!(line.char_len() == 4);
    line.insert(2, 'e');
    assert!(line.is_ascii());
    line.insert(2, '🚀');
    assert!(line.char_len() == 6);
    assert!(!line.is_ascii());
    line.insert(3, 'x');
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "te🚀xext");
}

#[test]
fn test_insert_str() {
    let mut line = CodeLine::new("text".to_owned());
    line.insert_str(0, "text");
    assert!(line.is_ascii());
    assert!(line.char_len() == 8);
    line.insert_str(1, "rocket🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "trocket🚀exttext");
    assert!(line.char_len() < line.to_string().len());
}

#[test]
fn test_push() {
    let mut line = CodeLine::new("text".to_owned());
    line.push('1');
    assert!(line.is_ascii());
    assert!(line.char_len() == 5);
    line.push('🚀');
    assert!(!line.is_ascii());
    assert!(line.to_string().len() == 9);
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text1🚀");
}

#[test]
fn test_push_str() {
    let mut line = CodeLine::new(String::new());
    assert!(line.is_ascii());
    assert!(line.char_len() == 0);
    line.push_str("text");
    assert!(line.is_ascii());
    assert!(line.char_len() == 4);
    line.push_str("text🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "texttext🚀");
    assert!(line.char_len() == 9);
    assert!(line.to_string().len() == 12);
}

#[test]
fn test_replace_range() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_ascii());
    assert!(line.char_len() == 4);
    line.replace_range(0..2, "text");
    assert!(line.is_ascii());
    assert!(&line.to_string() == "text23");
    assert!(line.char_len() == 6);
    line.replace_range(3..6, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "tex🚀🚀");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_till() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_ascii());
    assert!(line.char_len() == 4);
    line.replace_till(3, "text");
    assert!(line.is_ascii());
    assert!(&line.to_string() == "text3");
    assert!(line.char_len() == 5);
    line.replace_till(2, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "🚀🚀xt3");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_from() {
    let mut line = CodeLine::new(String::from("123🚀"));
    assert!(!line.is_ascii());
    assert!(line.char_len() == 4);
    line.replace_from(3, "text");
    assert!(line.is_ascii());
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "123text");
    line.replace_from(3, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(line.char_len() == 5);
    assert!(&line.to_string() == "123🚀🚀");
}

#[test]
fn test_remove() {
    let mut line = CodeLine::new("text🚀123".to_owned());
    assert!(!line.is_ascii());
    assert!(line.char_len() == 8);
    assert!('1' == line.remove(5));
    assert!(line.char_len() == 7);
    assert!(!line.is_ascii());
    assert!('🚀' == line.remove(4));
    assert!(line.is_ascii());
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

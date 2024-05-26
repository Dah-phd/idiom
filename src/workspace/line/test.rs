use super::code::CodeLine;
use super::EditorLine;

#[test]
fn test_insert() {
    let mut line = CodeLine::new("text".to_owned());
    assert!(line.len() == 4);
    line.insert(2, 'e');
    assert!(line.is_ascii());
    line.insert(2, '🚀');
    assert!(line.len() == 6);
    assert!(!line.is_ascii());
    line.insert(3, 'x');
    assert!(line.len() == 7);
    assert!(&line.to_string() == "te🚀xext");
}

#[test]
fn test_insert_str() {
    let mut line = CodeLine::new("text".to_owned());
    line.insert_str(0, "text");
    assert!(line.is_ascii());
    assert!(line.len() == 8);
    line.insert_str(1, "rocket🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "trocket🚀exttext");
    assert!(line.len() < line.to_string().len());
}

#[test]
fn test_push() {
    let mut line = CodeLine::new("text".to_owned());
    line.push('1');
    assert!(line.is_ascii());
    assert!(line.len() == 5);
    line.push('🚀');
    assert!(!line.is_ascii());
    assert!(line.to_string().len() == 9);
    assert!(line.len() == 6);
    assert!(&line.to_string() == "text1🚀");
}

#[test]
fn test_push_str() {
    let mut line = CodeLine::new(String::new());
    assert!(line.is_ascii());
    assert!(line.len() == 0);
    line.push_str("text");
    assert!(line.is_ascii());
    assert!(line.len() == 4);
    line.push_str("text🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "texttext🚀");
    assert!(line.len() == 9);
    assert!(line.to_string().len() == 12);
}

#[test]
fn test_replace_range() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_ascii());
    assert!(line.len() == 4);
    line.replace_range(0..2, "text");
    assert!(line.is_ascii());
    assert!(&line.to_string() == "text23");
    assert!(line.len() == 6);
    line.replace_range(3..6, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "tex🚀🚀");
    assert!(line.len() == 5);
}

#[test]
fn test_replace_till() {
    let mut line = CodeLine::new(String::from("🚀123"));
    assert!(!line.is_ascii());
    assert!(line.len() == 4);
    line.replace_till(3, "text");
    assert!(line.is_ascii());
    assert!(&line.to_string() == "text3");
    assert!(line.len() == 5);
    line.replace_till(2, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(&line.to_string() == "🚀🚀xt3");
    assert!(line.len() == 5);
}

#[test]
fn test_replace_from() {
    let mut line = CodeLine::new(String::from("123🚀"));
    assert!(!line.is_ascii());
    assert!(line.len() == 4);
    line.replace_from(3, "text");
    assert!(line.is_ascii());
    assert!(line.len() == 7);
    assert!(&line.to_string() == "123text");
    line.replace_from(3, "🚀🚀");
    assert!(!line.is_ascii());
    assert!(line.len() == 5);
    assert!(&line.to_string() == "123🚀🚀");
}

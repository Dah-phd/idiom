use super::code::CodeLine;
use super::EditorLine;

#[test]
fn test_insert() {
    let mut line = CodeLine::new("text".to_owned());
    assert_eq!(line.len(), 4);
    line.insert(2, 'e');
    assert!(line.is_ascii());
    line.insert(2, '🚀');
    assert_eq!(6, line.len());
    assert!(!line.is_ascii());
    line.insert(3, 'x');
    assert_eq!(line.len(), 7);
    assert_eq!(&line.to_string(), "te🚀xext");
}

#[test]
fn test_insert_str() {
    let mut line = CodeLine::new("text".to_owned());
    line.insert_str(0, "text");
    assert!(line.is_ascii());
    assert_eq!(line.len(), 8);
    line.insert_str(1, "rocket🚀");
    assert!(!line.is_ascii());
    assert_eq!(&line.to_string(), "trocket🚀exttext");
    assert!(line.len() < line.to_string().len());
}

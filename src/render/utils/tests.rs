use super::{UTF8Safe, UTF8SafeStringExt, WriteChunks};
const TEXT: &str = "123🚀13";

#[test]
fn test_utf8_insert_str() {
    let mut s = String::new();
    s.utf8_insert_str(0, TEXT);
    assert_eq!(s, TEXT);
    s.utf8_insert_str(4, TEXT);
    assert!(&s == "123🚀123🚀1313");
}

#[test]
fn test_utf8_insert() {
    let mut s = String::new();
    s.utf8_insert(0, '🚀');
    assert!(&s == "🚀");
    s.utf8_insert(1, '🚀');
    s.utf8_insert(2, 'r');
    assert!(&s == "🚀🚀r");
}

#[test]
#[should_panic]
fn test_truncate() {
    let mut s = String::from(TEXT);
    s.truncate(4);
}

#[test]
fn test_truncate_utf8() {
    assert_eq!((4, "123"), "123".truncate_width(7));
    assert_eq!((1, "123"), TEXT.truncate_width(4));
    assert_eq!(3, TEXT.truncate_width(4).1.len());
    assert_eq!((0, "123🚀"), TEXT.truncate_width(5));
    assert_eq!(7, TEXT.truncate_width(5).1.len());
    assert_eq!(4, TEXT.truncate_width(5).1.chars().count());
    assert_eq!((0, "🚀13"), TEXT.truncate_width_start(4));
    assert_eq!((1, "13"), TEXT.truncate_width_start(3));
}

#[test]
#[should_panic]
fn test_split_std() {
    let _ = TEXT.split_at(4);
}

#[test]
fn test_split_utf8() {
    assert_eq!(TEXT.split_at(3), TEXT.utf8_split_at(3));
    assert_eq!(("123🚀", "13"), TEXT.utf8_split_at(4));
}

/// example issue
#[test]
#[should_panic]
fn test_utf8_split_off_panic() {
    let mut s = String::from(TEXT);
    let _ = s.split_off(4);
}

#[test]
#[should_panic]
fn test_utf8_split_off_out_of_bounds() {
    let mut s = String::from(TEXT);
    s.utf8_split_off(30);
}

#[test]
fn test_utf8_split_off() {
    let mut s = String::from(TEXT);
    assert_eq!(s.utf8_split_off(4), String::from("13"));
    assert_eq!(s, "123🚀");
}

/// example issue
#[test]
#[should_panic]
fn test_replace_range() {
    let mut s = String::from(TEXT);
    s.replace_range(4.., ""); // in char boundry
}

#[test]
fn test_utf8_replace_range() {
    let mut s = String::new();
    s.replace_range(0..0, "asd");
    assert!(&s == "asd");
    s.clear();
    s.utf8_replace_range(0..0, "🚀🚀");
    assert_eq!(&s, "🚀🚀");
    s.utf8_replace_range(1..2, "asd");
    assert_eq!(&s, "🚀asd");
}

#[test]
#[should_panic]
fn test_utf8_replace_range_panic() {
    let mut s = String::new();
    s.utf8_replace_range(0..1, "panic");
}

#[test]
fn test_replace_from() {
    let mut s = String::from("text");
    s.utf8_replace_from(0, "123");
    assert!(&s == "123");
    s.clear();
    s.utf8_replace_from(0, "123");
    assert!(&s == "123");
}

#[test]
fn test_replace_till() {
    let mut s = String::from("🚀🚀");
    s.utf8_replace_till(1, "asd");
    assert!(&s == "asd🚀");
    s.clear();
    s.utf8_replace_till(0, "🚀");
    assert_eq!(&s, "🚀");
}

#[test]
fn test_utf8_replaces() {
    let mut s = String::from(TEXT);
    let mut std_s = s.clone();
    s.utf8_replace_from(4, "replace_with");
    std_s.replace_range(7.., "replace_with");
    assert_eq!(s, std_s);
}

#[test]
fn test_utf8_str() {
    assert_eq!(TEXT.len(), 9);
    assert_eq!(TEXT.char_len(), 6);
    assert_eq!(TEXT.width(), 7);
}

/// represent issue solved by UTF8 traits
#[test]
#[should_panic]
fn test_std_remove() {
    let mut s = String::from(TEXT);
    s.remove(4); // in char boundry
}

#[test]
fn test_utf8_remove() {
    let mut s = String::from(TEXT);
    assert_eq!(s.len(), 9);
    assert_eq!(s.char_len(), 6);
    assert_eq!(s.width(), 7);
    assert_eq!(s.utf8_remove(4), '1');
    assert_eq!(s.utf8_remove(3), '🚀');
    assert_eq!(&s, "1233");
}

#[test]
fn test_utf8_get() {
    assert_eq!(TEXT.utf8_get(0, 10), None);
    assert_eq!(TEXT.utf8_get(0, 3), Some("123"));
    assert_eq!(TEXT.utf8_get(3, 4), Some("🚀"));
}

#[test]
fn test_utf8_get_from() {
    assert_eq!(TEXT.utf8_get_from(10), None);
    assert_eq!(TEXT.utf8_get_from(0), Some(TEXT));
    assert_eq!(TEXT.utf8_get_from(3), Some("🚀13"));
    assert_eq!(TEXT.utf8_get_from(4), Some("13"));
}

#[test]
fn test_utf8_get_till() {
    assert_eq!(TEXT.utf8_get_to(10), None);
    assert_eq!(TEXT.utf8_get_to(3), Some("123"));
    assert_eq!(TEXT.utf8_get_to(4), Some("123🚀"));
}

#[test]
#[should_panic]
fn test_utf8_remove_panic() {
    let mut s = String::new();
    s.utf8_remove(0);
}

#[test]
fn test_chunks() {
    let text = "123🚀asdas123123123afsadasras";
    let mut chunks = WriteChunks::new(text, 4);
    assert_eq!(chunks.next(), Some((3, "123")));
    assert_eq!(chunks.next(), Some((4, "🚀as")));
    assert_eq!(chunks.next(), Some((4, "das1")));
    assert_eq!(chunks.next(), Some((4, "2312")));
    assert_eq!(chunks.next(), Some((4, "3123")));
    assert_eq!(chunks.next(), Some((4, "afsa")));
    assert_eq!(chunks.next(), Some((4, "dasr")));
    assert_eq!(chunks.next(), Some((2, "as")));
    assert_eq!(chunks.next(), None);
}

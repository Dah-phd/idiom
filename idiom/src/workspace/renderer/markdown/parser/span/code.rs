use super::super::Span;
use super::super::Span::Code;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_code(text: &str) -> Option<(Span, usize)> {
    lazy_static! {
        static ref CODE_SINGLE: Regex = Regex::new(r"^`(?P<text>.+?)`").expect("Pattern tested!");
        static ref CODE_DOUBLE: Regex = Regex::new(r"^``(?P<text>.+?)``").expect("Pattern tested!");
    }

    if let Some(caps) = CODE_DOUBLE.captures(text) {
        let t = caps.name("text")?.as_str();
        return Some((Code(t.to_owned()), t.len() + 4));
    }
    if let Some(caps) = CODE_SINGLE.captures(text) {
        let t = caps.name("text")?.as_str();
        return Some((Code(t.to_owned()), t.len() + 2));
    }
    None
}

#[test]
fn finds_code() {
    assert_eq!(parse_code("`testing things` test"), Some((Code("testing things".to_owned()), 16)));

    assert_eq!(parse_code("``testing things`` test"), Some((Code("testing things".to_owned()), 18)));

    assert_eq!(parse_code("``testing things`` things`` test"), Some((Code("testing things".to_owned()), 18)));

    assert_eq!(parse_code("`w` testing things test"), Some((Code("w".to_owned()), 3)));

    assert_eq!(parse_code("`w`` testing things test"), Some((Code("w".to_owned()), 3)));

    assert_eq!(parse_code("``w`` testing things test"), Some((Code("w".to_owned()), 5)));

    assert_eq!(parse_code("``w``` testing things test"), Some((Code("w".to_owned()), 5)));
}

#[test]
fn no_false_positives() {
    assert_eq!(parse_code("`` testing things test"), None);
    assert_eq!(parse_code("` test"), None);
}

#[test]
fn no_early_matching() {
    assert_eq!(parse_code("were ``testing things`` test"), None);
    assert_eq!(parse_code("were `testing things` test"), None);
}

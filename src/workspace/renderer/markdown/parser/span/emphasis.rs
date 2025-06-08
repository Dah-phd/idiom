use super::super::span::parse_spans;
use super::super::Span;
use super::super::Span::Emphasis;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_emphasis(text: &str) -> Option<(Span, usize)> {
    lazy_static! {
        static ref EMPHASIS_UNDERSCORE: Regex = Regex::new(r"^_(?P<text>.+?)_").expect("Pattern tested!");
        static ref EMPHASIS_STAR: Regex = Regex::new(r"^\*(?P<text>.+?)\*").expect("Pattern tested!");
    }
    let caps = EMPHASIS_UNDERSCORE.captures(text).or(EMPHASIS_STAR.captures(text))?;

    let t = caps.name("text")?.as_str();
    Some((Emphasis(parse_spans(t)), t.len() + 2))
}

#[cfg(test)]
mod test {
    use super::super::Span::{Emphasis, Text};
    use super::parse_emphasis;

    #[test]
    fn finds_emphasis() {
        assert_eq!(
            parse_emphasis("_testing things_ test"),
            Some((Emphasis(vec![Text("testing things".to_owned())]), 16))
        );

        assert_eq!(
            parse_emphasis("*testing things* test"),
            Some((Emphasis(vec![Text("testing things".to_owned())]), 16))
        );

        assert_eq!(
            parse_emphasis("_testing things_ things_ test"),
            Some((Emphasis(vec![Text("testing things".to_owned())]), 16))
        );

        assert_eq!(parse_emphasis("_w_ things_ test"), Some((Emphasis(vec![Text("w".to_owned())]), 3)));

        assert_eq!(parse_emphasis("*w* things* test"), Some((Emphasis(vec![Text("w".to_owned())]), 3)));

        assert_eq!(parse_emphasis("_w__ testing things test"), Some((Emphasis(vec![Text("w".to_owned())]), 3)));
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_emphasis("__ testing things test"), None);
        assert_eq!(parse_emphasis("_ test"), None);
    }

    #[test]
    fn no_early_matching() {
        assert_eq!(parse_emphasis("were _testing things_ test"), None);
        assert_eq!(parse_emphasis("were *testing things* test"), None);
    }
}

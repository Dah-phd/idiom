use super::super::span::parse_spans;
use super::super::Span;
use super::super::Span::Strong;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_strong(text: &str) -> Option<(Span, usize)> {
    lazy_static! {
        static ref STRONG_UNDERSCORE: Regex = Regex::new(r"^__(?P<text>.+?)__").expect("Pattern tested!");
        static ref STRONG_STAR: Regex = Regex::new(r"^\*\*(?P<text>.+?)\*\*").expect("Pattern tested!");
    }

    let caps = STRONG_UNDERSCORE.captures(text).or(STRONG_STAR.captures(text))?;

    let text = caps.name("text")?.as_str();
    Some((Strong(parse_spans(text)), text.len() + 4))
}

#[cfg(test)]
mod test {
    use super::super::Span::{Strong, Text};
    use super::parse_strong;

    #[test]
    fn finds_strong() {
        assert_eq!(
            parse_strong("__testing things__ test"),
            Some((Strong(vec![Text("testing things".to_owned())]), 18))
        );

        assert_eq!(
            parse_strong("**testing things** test"),
            Some((Strong(vec![Text("testing things".to_owned())]), 18))
        );

        assert_eq!(
            parse_strong("__testing things__ things__ test"),
            Some((Strong(vec![Text("testing things".to_owned())]), 18))
        );

        assert_eq!(parse_strong("__w__ things_ test"), Some((Strong(vec![Text("w".to_owned())]), 5)));

        assert_eq!(parse_strong("**w** things** test"), Some((Strong(vec![Text("w".to_owned())]), 5)));

        assert_eq!(parse_strong("__w___ testing things test"), Some((Strong(vec![Text("w".to_owned())]), 5)));
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_strong("__ testing things test"), None);
        assert_eq!(parse_strong("__testing things** test"), None);
        assert_eq!(parse_strong("____ testing things test"), None);
        assert_eq!(parse_strong("** test"), None);
        assert_eq!(parse_strong("**** test"), None);
    }

    #[test]
    fn no_early_matching() {
        assert_eq!(parse_strong("were __testing things__ test"), None);
        assert_eq!(parse_strong("were **testing things** test"), None);
    }
}

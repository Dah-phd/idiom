use super::super::Span;
use super::super::Span::Text;

mod emphasis;
mod image;
mod link;
mod strong;
use self::emphasis::parse_emphasis;
use self::image::parse_image;
use self::link::parse_link;
use self::strong::parse_strong;
use pipeline::{pipe_fun, pipe_opt};

pub fn parse_spans<'a>(text: &'a str) -> Vec<Span<'a>> {
    let mut tokens = vec![];
    let mut text_span_len = 0;
    let mut char_idx = 0;
    while char_idx < text.len() {
        match parse_span(&text[char_idx..]) {
            Some((span, consumed_chars)) => {
                if text_span_len != 0 {
                    tokens.push(Text(&text[(char_idx - text_span_len)..char_idx]));
                }
                tokens.push(span);
                text_span_len = 0;
                char_idx += consumed_chars;
            }
            None => {
                char_idx += 1;
                text_span_len += 1;
                while !text.is_char_boundary(char_idx) {
                    char_idx += 1;
                    text_span_len += 1;
                }
            }
        }
    }
    if text_span_len != 0 {
        tokens.push(Text(&text[(text.len() - text_span_len)..char_idx]));
    }
    tokens
}

fn parse_span<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    pipe_opt!(
        text
        => parse_strong
        => parse_emphasis
        => parse_image
        => parse_link
    )
}

#[cfg(test)]
mod test {
    use super::super::span::parse_spans;
    use super::super::Span::{Emphasis, Image, Link, Strong, Text};
    use std::str;

    #[test]
    fn converts_into_text() {
        assert_eq!(parse_spans("this is a test"), vec![Text("this is a test")]);
    }

    #[test]
    fn finds_code() {
        assert_eq!(parse_spans("this `is a` test"), vec![Text("this `is a` test")]);
        assert_eq!(parse_spans("this ``is a`` test"), vec![Text("this ``is a`` test")]);
    }

    #[test]
    fn finds_emphasis() {
        assert_eq!(parse_spans("this _is a_ test"), vec![Text("this "), Emphasis(vec![Text("is a")]), Text(" test")]);
        assert_eq!(parse_spans("this *is a* test"), vec![Text("this "), Emphasis(vec![Text("is a")]), Text(" test")]);
    }

    #[test]
    fn finds_strong() {
        assert_eq!(parse_spans("this __is a__ test"), vec![Text("this "), Strong(vec![Text("is a")]), Text(" test")]);
        assert_eq!(parse_spans("this **is a** test"), vec![Text("this "), Strong(vec![Text("is a")]), Text(" test")]);
    }

    #[test]
    fn finds_link() {
        assert_eq!(
            parse_spans("this is [an example](example.com) test"),
            vec![
                Text("this is "),
                Link("an example".to_owned(), "example.com".to_owned(), None),
                Text(" test")
            ]
        );
    }

    #[test]
    fn finds_image() {
        assert_eq!(
            parse_spans("this is ![an example](example.com) test"),
            vec![
                Text("this is "),
                Image("an example".to_owned(), "example.com".to_owned(), None),
                Text(" test")
            ]
        );
    }

    #[test]
    fn finds_everything() {
        assert_eq!(
            parse_spans("some text ![an image](image.com) _emphasis_ __strong__ `teh codez` [a link](example.com)  "),
            vec![
                Text("some text "),
                Image("an image".to_owned(), "image.com".to_owned(), None),
                Text(" "),
                Emphasis(vec![Text("emphasis")]),
                Text(" "),
                Strong(vec![Text("strong")]),
                Text(" "),
                Text("teh codez"),
                Text(" "),
                Link("a link".to_owned(), "example.com".to_owned(), None),
                Text("  ")
            ]
        );
    }

    #[test]
    fn properly_consumes_multibyte_utf8() {
        let test_phrase = str::from_utf8(b"This shouldn\xE2\x80\x99t panic").expect("Should not fail!");
        let _ = parse_spans(test_phrase);
    }
}

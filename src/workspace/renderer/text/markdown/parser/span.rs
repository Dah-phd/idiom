use super::super::Span;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_spans<'a>(text: &'a str) -> Vec<Span<'a>> {
    let mut tokens = vec![];
    let mut text_span_len = 0;
    let mut char_idx = 0;
    while char_idx < text.len() {
        match parse_span(&text[char_idx..]) {
            Some((span, consumed_chars)) => {
                if text_span_len != 0 {
                    tokens.push(Span::Text(&text[(char_idx - text_span_len)..char_idx]));
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
        tokens.push(Span::Text(&text[(text.len() - text_span_len)..char_idx]));
    }
    tokens
}

fn parse_span<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    parse_code(text).or(parse_strong(text)).or(parse_emphasis(text)).or(parse_image(text)).or(parse_link(text))
}

pub fn parse_image<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    lazy_static! {
        static ref IMAGE: Regex =
            Regex::new("^!\\[(?P<text>.*?)\\]\\((?P<url>.*?)(?:\\s\"(?P<title>.*?)\")?\\)").expect("Pattern tested");
    }

    let caps = IMAGE.captures(text)?;
    let text = if let Some(mat) = caps.name("text") { mat.as_str().to_owned() } else { "".to_owned() };
    let url = if let Some(mat) = caps.name("url") { mat.as_str().to_owned() } else { "".to_owned() };
    let title = caps.name("title").map(|mat| mat.as_str().to_owned());
    // TODO correctly get whitespace length between url and title
    let len = text.len() + url.len() + 5 + title.clone().map_or(0, |t| t.len() + 3);
    Some((Span::Image(text, url, title), len))
}

pub fn parse_strong<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    lazy_static! {
        static ref STRONG_UNDERSCORE: Regex = Regex::new(r"^__(?P<text>.+?)__").expect("Pattern tested!");
        static ref STRONG_STAR: Regex = Regex::new(r"^\*\*(?P<text>.+?)\*\*").expect("Pattern tested!");
    }

    let caps = STRONG_UNDERSCORE.captures(text).or(STRONG_STAR.captures(text))?;

    let text = caps.name("text")?.as_str();
    Some((Span::Strong(parse_spans(text)), text.len() + 4))
}

pub fn parse_emphasis<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    lazy_static! {
        static ref EMPHASIS_UNDERSCORE: Regex = Regex::new(r"^_(?P<text>.+?)_").expect("Pattern tested!");
        static ref EMPHASIS_STAR: Regex = Regex::new(r"^\*(?P<text>.+?)\*").expect("Pattern tested!");
    }
    let caps = EMPHASIS_UNDERSCORE.captures(text).or(EMPHASIS_STAR.captures(text))?;

    let t = caps.name("text")?.as_str();
    Some((Span::Emphasis(parse_spans(t)), t.len() + 2))
}

pub fn parse_link<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    lazy_static! {
        static ref LINK: Regex =
            Regex::new("^\\[(?P<text>.*?)\\]\\((?P<url>.*?)(?:\\s\"(?P<title>.*?)\")?\\)").expect("Pattern tested");
    }

    let caps = LINK.captures(text)?;
    let text = if let Some(mat) = caps.name("text") { mat.as_str().to_owned() } else { "".to_owned() };
    let url = if let Some(mat) = caps.name("url") { mat.as_str().to_owned() } else { "".to_owned() };
    let title = caps.name("title").map(|mat| mat.as_str().to_owned());
    // let title = caps.name("title").map(|t| t.to_owned());
    // TODO correctly get whitespace length between url and title
    let len = text.len() + url.len() + 4 + title.clone().map_or(0, |t| t.len() + 3);
    Some((Span::Link(text, url, title), len))
}

pub fn parse_code<'a>(text: &'a str) -> Option<(Span<'a>, usize)> {
    lazy_static! {
        static ref CODE_SINGLE: Regex = Regex::new(r"^`(?P<text>.+?)`").expect("Pattern tested!");
        static ref CODE_DOUBLE: Regex = Regex::new(r"^``(?P<text>.+?)``").expect("Pattern tested!");
    }

    if let Some(caps) = CODE_DOUBLE.captures(text) {
        let t = caps.name("text")?.as_str();
        return Some((Span::Code(t), t.len() + 4));
    }
    if let Some(caps) = CODE_SINGLE.captures(text) {
        let t = caps.name("text")?.as_str();
        return Some((Span::Code(t), t.len() + 2));
    }
    None
}

#[cfg(test)]
mod test {
    use super::super::Span::{Code, Emphasis, Image, Link, Strong, Text};
    use super::{parse_code, parse_emphasis, parse_image, parse_link, parse_spans, parse_strong};

    #[test]
    fn converts_into_text() {
        assert_eq!(parse_spans("this is a test"), vec![Text("this is a test")]);
    }

    #[test]
    fn finds_breaks() {
        assert_eq!(parse_spans("this is a test  "), vec![Text("this is a test  ")]);
    }

    #[test]
    fn finds_code() {
        assert_eq!(parse_spans("this `is a` test"), vec![Text("this "), Code("is a"), Text(" test")]);
        assert_eq!(parse_spans("this ``is a`` test"), vec![Text("this "), Code("is a"), Text(" test")]);
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
                Code("teh codez"),
                Text(" "),
                Link("a link".to_owned(), "example.com".to_owned(), None),
                Text("  "),
            ]
        );
    }

    #[test]
    fn properly_consumes_multibyte_utf8() {
        let test_phrase = str::from_utf8(b"This shouldn\xE2\x80\x99t panic").expect("Should not fail!");
        let _ = parse_spans(test_phrase);
    }

    #[test]
    fn finds_emphasis_full() {
        assert_eq!(parse_emphasis("_testing things_ test"), Some((Emphasis(vec![Text("testing things")]), 16)));
        assert_eq!(parse_emphasis("*testing things* test"), Some((Emphasis(vec![Text("testing things")]), 16)));
        assert_eq!(parse_emphasis("_testing things_ things_ test"), Some((Emphasis(vec![Text("testing things")]), 16)));
        assert_eq!(parse_emphasis("_w_ things_ test"), Some((Emphasis(vec![Text("w")]), 3)));
        assert_eq!(parse_emphasis("*w* things* test"), Some((Emphasis(vec![Text("w")]), 3)));
        assert_eq!(parse_emphasis("_w__ testing things test"), Some((Emphasis(vec![Text("w")]), 3)));
    }

    #[test]
    fn no_false_positives_emp() {
        assert_eq!(parse_emphasis("__ testing things test"), None);
        assert_eq!(parse_emphasis("_ test"), None);
    }

    #[test]
    fn no_early_matching_emp() {
        assert_eq!(parse_emphasis("were _testing things_ test"), None);
        assert_eq!(parse_emphasis("were *testing things* test"), None);
    }

    #[test]
    fn finds_image_unit() {
        assert_eq!(
            parse_image("![an example](example.com) test"),
            Some((Image("an example".to_owned(), "example.com".to_owned(), None), 26))
        );

        assert_eq!(
            parse_image("![](example.com) test"),
            Some((Image("".to_owned(), "example.com".to_owned(), None), 16))
        );

        assert_eq!(
            parse_image("![an example]() test"),
            Some((Image("an example".to_owned(), "".to_owned(), None), 15))
        );

        assert_eq!(parse_image("![]() test"), Some((Image("".to_owned(), "".to_owned(), None), 5)));

        assert_eq!(
            parse_image("![an example](example.com \"Title\") test"),
            Some((Image("an example".to_owned(), "example.com".to_owned(), Some("Title".to_owned())), 34))
        );

        assert_eq!(
            parse_image("![an example](example.com) test [a link](example.com)"),
            Some((Image("an example".to_owned(), "example.com".to_owned(), None), 26))
        );
    }

    #[test]
    fn no_false_positives_img() {
        assert_eq!(parse_image("![()] testing things test"), None);
        assert_eq!(parse_image("!()[] testing things test"), None);
    }

    #[test]
    fn no_early_matching_img() {
        assert_eq!(parse_image("were ![an example](example.com) test"), None);
    }

    #[test]
    fn finds_link_unit() {
        assert_eq!(
            parse_link("[an example](example.com) test"),
            Some((Link("an example".to_owned(), "example.com".to_owned(), None), 25))
        );

        assert_eq!(parse_link("[](example.com) test"), Some((Link("".to_owned(), "example.com".to_owned(), None), 15)));

        assert_eq!(parse_link("[an example]() test"), Some((Link("an example".to_owned(), "".to_owned(), None), 14)));

        assert_eq!(parse_link("[]() test"), Some((Link("".to_owned(), "".to_owned(), None), 4)));

        assert_eq!(
            parse_link("[an example](example.com \"Title\") test"),
            Some((Link("an example".to_owned(), "example.com".to_owned(), Some("Title".to_owned())), 33))
        );

        assert_eq!(
            parse_link("[an example](example.com) test [a link](example.com)"),
            Some((Link("an example".to_owned(), "example.com".to_owned(), None), 25))
        );
    }

    #[test]
    fn no_false_positives_link() {
        assert_eq!(parse_link("[()] testing things test"), None);
        assert_eq!(parse_link("()[] testing things test"), None);
    }

    #[test]
    fn no_early_matching_link() {
        assert_eq!(parse_link("were [an example](example.com) test"), None);
    }

    #[test]
    fn finds_strong_unit() {
        assert_eq!(parse_strong("__testing things__ test"), Some((Strong(vec![Text("testing things")]), 18)));

        assert_eq!(parse_strong("**testing things** test"), Some((Strong(vec![Text("testing things")]), 18)));

        assert_eq!(parse_strong("__testing things__ things__ test"), Some((Strong(vec![Text("testing things")]), 18)));

        assert_eq!(parse_strong("__w__ things_ test"), Some((Strong(vec![Text("w")]), 5)));

        assert_eq!(parse_strong("**w** things** test"), Some((Strong(vec![Text("w")]), 5)));

        assert_eq!(parse_strong("__w___ testing things test"), Some((Strong(vec![Text("w")]), 5)));
    }

    #[test]
    fn no_false_positives_str() {
        assert_eq!(parse_strong("__ testing things test"), None);
        assert_eq!(parse_strong("__testing things** test"), None);
        assert_eq!(parse_strong("____ testing things test"), None);
        assert_eq!(parse_strong("** test"), None);
        assert_eq!(parse_strong("**** test"), None);
    }

    #[test]
    fn no_early_matching_str() {
        assert_eq!(parse_strong("were __testing things__ test"), None);
        assert_eq!(parse_strong("were **testing things** test"), None);
    }

    #[test]
    fn finds_code_unit() {
        assert_eq!(parse_code("`testing things` test"), Some((Code("testing things"), 16)));
        assert_eq!(parse_code("``testing things`` test"), Some((Code("testing things"), 18)));
        assert_eq!(parse_code("``testing things`` things`` test"), Some((Code("testing things"), 18)));
        assert_eq!(parse_code("`w` testing things test"), Some((Code("w"), 3)));
        assert_eq!(parse_code("`w`` testing things test"), Some((Code("w"), 3)));
        assert_eq!(parse_code("``w`` testing things test"), Some((Code("w"), 5)));
        assert_eq!(parse_code("``w``` testing things test"), Some((Code("w"), 5)));
    }

    #[test]
    fn no_false_positives_cc() {
        assert_eq!(parse_code("`` testing things test"), None);
        assert_eq!(parse_code("` test"), None);
    }

    #[test]
    fn no_early_matching_cc() {
        assert_eq!(parse_code("were ``testing things`` test"), None);
        assert_eq!(parse_code("were `testing things` test"), None);
    }
}

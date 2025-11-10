use super::super::span::parse_spans;
use super::super::Block;
use super::super::Block::Header;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_atx_header(line: &str) -> Option<Block> {
    lazy_static! {
        static ref ATX_HEADER_RE: Regex =
            Regex::new(r"^(?P<level>#{1,6})\s(?P<text>.*?)(?:\s#*)?$").expect("Pattern already testsed!");
    }

    let caps = ATX_HEADER_RE.captures(line)?;
    Some(Header(parse_spans(caps.name("text")?.as_str()), caps.name("level")?.as_str().len()))
}

#[cfg(test)]
mod test {
    use super::super::super::Span::Text;
    use super::super::Block::Header;
    use super::parse_atx_header;

    #[test]
    fn finds_atx_header() {
        assert_eq!(parse_atx_header("### Test"), Some(Header(vec![Text("Test".to_owned())], 3)));

        assert_eq!(parse_atx_header("# Test"), Some(Header(vec![Text("Test".to_owned())], 1)));

        assert_eq!(parse_atx_header("###### Test"), Some(Header(vec![Text("Test".to_owned())], 6)));

        assert_eq!(
            parse_atx_header("### Test and a pretty long sentence"),
            Some(Header(vec![Text("Test and a pretty long sentence".to_owned())], 3))
        );
    }

    #[test]
    fn ignores_closing_hashes() {
        assert_eq!(parse_atx_header("### Test ###"), Some(Header(vec![Text("Test".to_owned())], 3)));

        assert_eq!(parse_atx_header("# Test #"), Some(Header(vec![Text("Test".to_owned())], 1)));

        assert_eq!(parse_atx_header("###### Test ##"), Some(Header(vec![Text("Test".to_owned())], 6)));

        assert_eq!(
            parse_atx_header("### Test and a pretty long sentence #########"),
            Some(Header(vec![Text("Test and a pretty long sentence".to_owned())], 3))
        );
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_atx_header("####### Test"), None);
        assert_eq!(parse_atx_header("Test #"), None);
        assert_eq!(parse_atx_header("T ### est #"), None);
    }
}

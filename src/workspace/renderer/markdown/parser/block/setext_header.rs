use super::super::span::parse_spans;
use super::super::Block;
use super::super::Block::Header;
use lazy_static::lazy_static;
use regex::Regex;

pub fn parse_setext_header(lines: &[&str]) -> Option<(Block, usize)> {
    lazy_static! {
        static ref HORIZONTAL_RULE_1: Regex = Regex::new(r"^===+$").unwrap();
        static ref HORIZONTAL_RULE_2: Regex = Regex::new(r"^---+$").unwrap();
    }

    if lines.len() > 1 {
        if HORIZONTAL_RULE_1.is_match(lines[1]) {
            return Some((Header(parse_spans(lines[0]), 1), 2));
        } else if HORIZONTAL_RULE_2.is_match(lines[1]) {
            return Some((Header(parse_spans(lines[0]), 2), 2));
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::super::super::Span::Text;
    use super::super::Block::Header;
    use super::parse_setext_header;

    #[test]
    fn finds_atx_header() {
        assert_eq!(
            parse_setext_header(&vec!["Test", "=========="]).unwrap(),
            (Header(vec![Text("Test".to_owned())], 1), 2)
        );

        assert_eq!(
            parse_setext_header(&vec!["Test", "----------"]).unwrap(),
            (Header(vec![Text("Test".to_owned())], 2), 2)
        );

        assert_eq!(
            parse_setext_header(&vec!["This is a test", "==="]).unwrap(),
            (Header(vec![Text("This is a test".to_owned())], 1), 2)
        );

        assert_eq!(
            parse_setext_header(&vec!["This is a test", "---"]).unwrap(),
            (Header(vec![Text("This is a test".to_owned())], 2), 2)
        );
    }
}

use super::super::span::parse_spans;
use super::super::Block;

pub fn parse_atx_header<'a>(line: &'a str) -> Option<Block<'a>> {
    let mut level = 0;
    for ch in line.chars() {
        if level > 6 {
            return None;
        }
        if ch != '#' {
            break;
        }
        level += 1;
    }

    if level == 0 {
        return None;
    }
    Some(Block::Header(parse_spans(&line[level..].trim_start()), level))
}

#[cfg(test)]
mod test {
    use super::super::super::Span::Text;
    use super::super::Block::Header;
    use super::parse_atx_header;

    #[test]
    fn finds_atx_header() {
        assert_eq!(parse_atx_header("### Test"), Some(Header(vec![Text("Test")], 3)));

        assert_eq!(parse_atx_header("# Test"), Some(Header(vec![Text("Test")], 1)));

        assert_eq!(parse_atx_header("###### Test"), Some(Header(vec![Text("Test")], 6)));

        assert_eq!(
            parse_atx_header("### Test and a pretty long sentence"),
            Some(Header(vec![Text("Test and a pretty long sentence")], 3))
        );
    }

    #[test]
    fn ignores_closing_hashes() {
        assert_eq!(parse_atx_header("### Test ###"), Some(Header(vec![Text("Test")], 3)));

        assert_eq!(parse_atx_header("# Test #"), Some(Header(vec![Text("Test")], 1)));

        assert_eq!(parse_atx_header("###### Test ##"), Some(Header(vec![Text("Test")], 6)));

        assert_eq!(
            parse_atx_header("### Test and a pretty long sentence #########"),
            Some(Header(vec![Text("Test and a pretty long sentence")], 3))
        );
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_atx_header("####### Test"), None);
        assert_eq!(parse_atx_header("Test #"), None);
        assert_eq!(parse_atx_header("T ### est #"), None);
    }
}

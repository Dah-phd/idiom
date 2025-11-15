use pipeline::{pipe_fun, pipe_opt};

pub fn parse_blockquote<'a>(line: &'a str) -> Option<Block<'a>> {
    let mut nesting = 0;

    for ch in line.chars() {
        if ch != '>' {
            break;
        }
        nesting += 1;
    }

    if nesting == 0 {
        return None;
    }
    Some(Block::Blockquote(String::from(&line[nesting..]), nesting))
}

pub fn parse_blocks<'a>(line: &'a str) -> Option<Block<'a>> {
    pipe_opt!(
        line
        => parse_hr
        => parse_atx_header
        => parse_code_block
        => parse_blockquote
    )
}

pub fn parse_code_block<'a>(line: &'a str) -> Option<Block<'a>> {
    if !line.starts_with("```") {
        return None;
    }
    if line.len() == 3 {
        Some(Block::CodeBlock(None))
    } else {
        Some(Block::CodeBlock(Some(line[3..].to_owned())))
    }
}

pub fn parse_hr<'a>(line: &'a str) -> Option<Block<'a>> {
    if line.len() < 3 {
        return None;
    }
    if !line.chars().all(|c| c == '-') && !line.chars().all(|c| c == '=') {
        return None;
    }
    Some(Block::Hr)
}

use super::super::Block;
use super::span::parse_spans;

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
    use super::super::Block::Hr;
    use super::parse_atx_header;
    use super::parse_hr;

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

    #[test]
    fn finds_hr() {
        assert_eq!(parse_hr("-------"), Some(Hr));
        assert_eq!(parse_hr("---"), Some(Hr));
        assert_eq!(parse_hr("----------------------------"), Some(Hr));
        assert_eq!(parse_hr("-------"), Some(Hr));

        assert_eq!(parse_hr("======="), Some(Hr));
        assert_eq!(parse_hr("==="), Some(Hr));
        assert_eq!(parse_hr("============================"), Some(Hr));
    }

    #[test]
    fn no_false_positives_hr() {
        assert_eq!(parse_hr("a-------"), None);
        assert_eq!(parse_hr("--- a"), None);
        assert_eq!(parse_hr("--a-"), None);
        assert_eq!(parse_hr("-------====--------------"), None);

        assert_eq!(parse_hr("a======"), None);
        assert_eq!(parse_hr("=== a"), None);
        assert_eq!(parse_hr("==a="), None);
        assert_eq!(parse_hr("=======---================="), None);
    }
}

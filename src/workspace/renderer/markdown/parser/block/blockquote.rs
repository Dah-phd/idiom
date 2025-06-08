use super::super::block::parse_blocks;
use super::super::Block;
use super::super::Block::Blockquote;

pub fn parse_blockquote(lines: &[&str]) -> Option<(Block, usize)> {
    // if the first char isnt a blockquote don't even bother
    if lines[0].is_empty() || !lines[0].starts_with(">") {
        return None;
    }

    // the content of the blockquote
    let mut content = String::new();

    // counts the number of parsed lines to return
    let mut i = 0;

    // captures if the previous item was a newline
    // meaning the blockquote ends next if it's not
    // explicitly continued with a >
    let mut prev_newline = false;

    for line in lines {
        // stop parsing on two newlines or if the paragraph after
        // a newline isn't started with a >
        // we continue to parse if it's just another empty line
        if prev_newline && !line.is_empty() && !line.starts_with(">") {
            break;
        }
        prev_newline = line.is_empty();
        let mut chars = line.chars();
        let begin = match chars.next() {
            Some('>') => match chars.next() {
                Some(' ') => 2,
                _ => 1,
            },
            _ => 0,
        };
        if i > 0 {
            content.push('\n');
        }
        content.push_str(&line[begin..line.len()]);
        i += 1;
    }

    if i > 0 {
        return Some((Blockquote(parse_blocks(&content)), i));
    }

    None
}

#[cfg(test)]
mod test {
    use super::super::Block::Blockquote;
    use super::parse_blockquote;

    #[test]
    fn finds_blockquote() {
        match parse_blockquote(&["> A citation", "> is good"]) {
            Some((Blockquote(_), 2)) => (),
            _ => panic!(),
        }

        match parse_blockquote(&["> A citation", "> is good,", "very good"]) {
            Some((Blockquote(_), 3)) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn knows_when_to_stop() {
        match parse_blockquote(&["> A citation", "> is good", "", "whatever"]) {
            Some((Blockquote(_), 3)) => (),
            _ => panic!(),
        }
    }

    #[test]
    fn no_false_positives() {
        assert_eq!(parse_blockquote(&["wat > this"]), None);
    }

    #[test]
    fn no_early_matching() {
        assert_eq!(parse_blockquote(&["Hello", "> A citation", "> is good", "", "whatever"]), None);
    }
}

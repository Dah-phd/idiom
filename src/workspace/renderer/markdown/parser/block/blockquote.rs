use super::super::Block;

pub fn parse_blockquote(line: &str) -> Option<Block> {
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

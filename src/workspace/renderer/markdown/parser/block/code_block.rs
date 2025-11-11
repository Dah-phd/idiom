use super::super::Block;

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

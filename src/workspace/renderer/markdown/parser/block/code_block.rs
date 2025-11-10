use super::super::Block;

pub fn parse_code_block(line: &str) -> Option<Block> {
    if !line.starts_with("```") {
        return None
    }
    if line.len() == 3 {
        Some(Block::CodeBlock(None))
    } else {
        Some(Block::CodeBlock(Some(line[3..].to_owned())))
    }
            
}

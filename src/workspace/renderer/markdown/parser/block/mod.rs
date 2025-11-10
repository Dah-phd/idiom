use super::super::Block;
use super::super::Block::Paragraph;
use super::super::Span::Text;
use super::span::parse_spans;
use pipeline::{pipe_fun, pipe_opt};

mod atx_header;
mod blockquote;
mod code_block;
mod hr;
use self::atx_header::parse_atx_header;
use self::blockquote::parse_blockquote;
use self::code_block::parse_code_block;
use self::hr::parse_hr;

pub fn parse_blocks(line: &str) -> Option<Block> {
    pipe_opt!(
        line
        => parse_hr
        => parse_atx_header
        => parse_code_block
        => parse_blockquote
    )
}

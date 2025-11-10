/// markdown v0.3.0 (pre-release)
/// the functionallity provided is enough for the purpose
/// keeping the code as is, ref [MIT]:
/// https://crates.io/crates/markdown
mod block;
mod span;

#[allow(missing_docs)]
#[derive(Debug, PartialEq, Clone)]
pub enum Block {
    Header(Vec<Span>, usize),
    Paragraph(Vec<Span>),
    Blockquote(String, usize),
    CodeBlock(Option<String>),
    Hr,
}

#[allow(missing_docs)]
#[derive(Debug, PartialEq, Clone)]
pub enum Span {
    Text(String),
    Link(String, String, Option<String>),
    Image(String, String, Option<String>),
    Emphasis(Vec<Span>),
    Strong(Vec<Span>),
}

pub fn parse(md: &str) -> Block {
    block::parse_blocks(md).unwrap_or(Block::Paragraph(span::parse_spans(md)))
}

/// markdown v0.3.0 (pre-release)
/// the functionallity provided is enough for the purpose
/// keeping the code as is, ref [MIT]:
/// https://crates.io/crates/markdown
mod block;
mod span;

pub fn parse<'a>(md: &'a str) -> Block<'a> {
    block::parse_blocks(md).unwrap_or(Block::Paragraph(span::parse_spans(md)))
}

#[derive(Debug, PartialEq, Clone)]
pub enum Block<'a> {
    Header(Vec<Span<'a>>, usize),
    Paragraph(Vec<Span<'a>>),
    Blockquote(String, usize),
    CodeBlock(Option<String>),
    Hr,
}

impl<'a> Block<'a> {
    pub fn render(&'a self) {
        todo!()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Span<'a> {
    Text(&'a str),
    Link(String, String, Option<String>),
    Image(String, String, Option<String>),
    Emphasis(Vec<Span<'a>>),
    Strong(Vec<Span<'a>>),
    Code(&'a str),
}

impl<'a> Span<'a> {
    pub fn render(&'a self) {
        todo!()
    }
}

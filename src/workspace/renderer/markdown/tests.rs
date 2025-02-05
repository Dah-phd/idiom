use markdown::{tokenize, Block, Span};

#[test]
fn bumba() {
    let txt = "```";
    assert_eq!(tokenize(txt), vec![Block::Paragraph(vec![Span::Code(String::from('`'))])]);
}

use logos::Logos;

use super::LangStream;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
pub enum GenericToken {
    #[token("def ")]
    DeclareFn,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[token("\"\"\"")]
    MultiString,

    #[regex("#[a-zA-Z_]+")]
    #[regex("@[a-zA-Z_]+")]
    Decorator,

    #[token("while ")]
    #[token("for ")]
    #[token("async ")]
    #[token("break")]
    #[token("return")]
    #[token(" in ")]
    #[token("continue")]
    #[token("if ")]
    #[token("elif ")]
    #[token("else:")]
    FlowControl,

    #[token("    ")]
    Scope,

    #[token("with")]
    Context,

    #[token("!=")]
    #[token("not ")]
    Negate,

    #[regex("class")]
    DeclareStruct,

    #[token("self")]
    SelfRef,

    #[token("=")]
    Assign,

    #[token(".")]
    InstanceInvoked,

    #[token("(")]
    LBrack,

    #[token(")")]
    RBrack,

    #[token("{")]
    DOpen,

    #[token("}")]
    DClose,

    #[token("[")]
    LOpen,

    #[token("]")]
    LClose,

    #[regex(r#": ?[a-zA-Z]+"#, |lex| lex.slice().to_owned())]
    TypeHint(String),

    Type(String),

    #[token("->")]
    ReturnHint,

    #[token("<=")]
    GreatEq,

    #[token(">=")]
    LesssEq,

    #[token("<")]
    Lesser,

    #[token(">")]
    Greater,

    #[regex("[0-9]+.?[0-9]+")]
    Float,

    #[regex("[0-9]+")]
    Int,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),
}

impl LangStream for GenericToken {
    fn init_definitions() -> super::Definitions {
        super::Definitions { structs: vec![], function: vec![], variables: vec![] }
    }
    fn parse(defs: &mut super::Definitions, text: &Vec<String>, tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        todo!()
    }

    fn type_id(&self) -> u32 {
        0
    }

    fn modifier(&self) -> u32 {
        0
    }

    fn parse_semantics(text: &Vec<String>, tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        todo!()
    }
}

use logos::Logos;

use super::LangStream;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
pub enum GenericToken {
    #[token("def")]
    #[token("function")]
    #[token("fn")]
    DeclareFn,

    #[token("const")]
    #[token("let")]
    #[token("var")]
    DeclareVar,

    #[token("class")]
    #[token("struct")]
    DeclareStruct,

    #[token("enum")]
    DeclareEnum,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[regex("#[a-zA-Z_]+")]
    #[regex("@[a-zA-Z_]+")]
    Decorator,

    #[token("while")]
    #[token("for")]
    #[token("async ")]
    #[token("break")]
    #[token("return")]
    #[token("in")]
    #[token("continue")]
    #[token("if")]
    #[token("elif")]
    #[token("else")]
    #[token("loop")]
    FlowControl,

    #[token("    ")]
    Scope,

    #[token("with")]
    Context,

    #[token("!=")]
    #[token("not ")]
    Negate,

    #[token("Self")]
    #[token("cls")]
    ClassRef,

    #[token("self")]
    SelfRef,

    #[token("=")]
    #[token(":=")]
    Assign,

    #[token(".")]
    InstanceInvoked,

    #[token("(")]
    LBrack,

    #[token(")")]
    RBrack,

    #[token("{")]
    ScopeOpen,

    #[token("}")]
    ScopeClose,

    #[token("[")]
    LOpen,

    #[token("]")]
    LClose,

    #[regex(r#": ?[a-zA-Z_]+"#, |lex| lex.slice().to_owned())]
    TypeHint(String),

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

    #[regex("-?[0-9]+\\.[0-9]+")]
    Float,

    #[regex("-?[0-9]+")]
    Int,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),

    // convertible types
    Type(String),
    Function(String),
    Enum(String),
    Struct(String),
    NameSpace(String),
}

impl LangStream for GenericToken {
    fn init_definitions() -> super::Definitions {
        super::Definitions { types: vec![], function: vec![], variables: vec![], keywords: vec![] }
    }
    fn parse(text: &[String], tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        todo!()
    }

    fn type_id(&self) -> u32 {
        0
    }

    fn modifier(&self) -> u32 {
        0
    }
}

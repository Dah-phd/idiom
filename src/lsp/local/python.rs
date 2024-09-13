use std::ops::Range;

use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
enum PythonStream {
    #[token("def ")]
    DeclareFn,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[token("\"\"\"")]
    MultiString,

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

    #[token("with ")]
    Context,

    #[token(" not ")]
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

fn parase_assignment(var_name: String, type_name: &PythonStream) {
    let var_type = match type_name {
        PythonStream::LBrack => Some(1),
        PythonStream::DOpen => Some(2),
        PythonStream::LOpen => Some(3),
        _ => None,
    };
}

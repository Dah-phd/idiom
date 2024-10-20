use super::{Definitions, Func, LangStream, ObjType, PositionedToken, Struct, Var, NON_TOKEN_ID};
use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
pub enum TSToken {
    #[token("function")]
    DeclareFn,

    #[token("let")]
    #[token("var")]
    #[token("const")]
    DeclareVar,

    #[token("class")]
    #[token("interface")]
    #[token("type")]
    DeclareStruct,

    #[token("enum")]
    DeclareEnum,

    #[token("->")]
    ReturnHint,

    #[token("new")]
    Constructor,

    #[token("this")]
    SelfRef,

    #[token("extends")]
    Inherit,

    #[token(".")]
    InstanceInvoked,

    #[token("=")]
    Assign,

    #[token(":")]
    OpenScope,

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

    #[token("==")]
    #[token("===")]
    Equals,
    #[token("<=")]
    GreatEq,
    #[token(">=")]
    LesssEq,
    #[token("<")]
    Lesser,
    #[token(">")]
    Greater,

    #[regex("//(.)*")]
    Comment,

    #[token("public")]
    #[token("private")]
    #[token("protected")]
    Visibility,

    #[token("null")]
    #[token("undefined")]
    Null,

    #[token("any")]
    #[token("unknown")]
    #[token("never")]
    #[token("void")]
    Opeque,

    #[token("async")]
    #[token("await")]
    #[token("for")]
    #[token("while")]
    #[token("break")]
    #[token("continue")]
    #[token("return")]
    #[token("if")]
    #[token("in")]
    #[token("try")]
    #[token("catch")]
    #[token("throw")]
    FlowControl,

    #[token("instanceof")]
    #[token("typeof")]
    Keyword,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[token("true")]
    #[token("false")]
    Bool,
    #[regex("-?[0-9]+\\.[0-9]+")]
    Float,
    #[regex("-?[0-9]+")]
    Int,

    #[token("import")]
    #[token("export")]
    #[token("require")]
    NameSpaceKeyWord,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),

    #[regex(r#": ?[a-zA-Z]+"#, |lex| lex.slice().to_owned())]
    TypeHint(String),

    Type(String),
    Function(String),
    NameSpace(String),
}

impl LangStream for TSToken {
    fn parse<'a>(text: impl Iterator<Item = &'a str>, tokens: &mut Vec<Vec<PositionedToken<Self>>>) {
        tokens.clear();
        for line in text {
            let mut token_line = Vec::new();
            let mut logos = Self::lexer(line);
            while let Some(token_result) = logos.next() {
                let tstoken = match token_result {
                    Ok(tstoken) => tstoken,
                    Err(_) => continue,
                };
                match tstoken {
                    Self::DeclareFn => {
                        token_line.push(tstoken.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_tstoken)) = logos.next() {
                            next_tstoken.name_to_func();
                            token_line.push(next_tstoken.to_postioned(logos.span(), line));
                        }
                    }
                    Self::DeclareStruct => {
                        token_line.push(tstoken.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_tstoken)) = logos.next() {
                            next_tstoken.name_to_class();
                            token_line.push(next_tstoken.to_postioned(logos.span(), line));
                        }
                    }
                    Self::LBrack => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.derive_from_name();
                            pos_token.refresh_type();
                        }
                    }
                    Self::NameSpaceKeyWord => {
                        drain_import(line, &mut logos, &mut token_line);
                    }
                    _ => {
                        token_line.push(tstoken.to_postioned(logos.span(), line));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::NameSpace(..) => 0,
            Self::Type(..) => 1,
            Self::TypeHint(..) => 6,
            Self::Name(..) => 8,
            Self::Function(..) => 10,
            Self::DeclareFn
            | Self::DeclareStruct
            | Self::DeclareVar
            | Self::FlowControl
            | Self::Constructor
            | Self::SelfRef
            | Self::Keyword
            | Self::Opeque
            | Self::Null
            | Self::Bool
            | Self::NameSpaceKeyWord => 11,
            Self::Comment => 12,
            Self::String => 13,
            Self::Int | Self::Float => 14,
            _ => NON_TOKEN_ID,
        }
    }

    fn modifier(&self) -> u32 {
        match self {
            Self::FlowControl => 1,
            _ => 0,
        }
    }

    fn objectify(&self) -> super::ObjType {
        match self {
            Self::Name(name) => ObjType::Var(name),
            Self::Type(name) | Self::TypeHint(name) => ObjType::Struct(name),
            Self::Function(name) => ObjType::Fn(name),
            _ => ObjType::None,
        }
    }

    fn init_definitions() -> Definitions {
        Definitions {
            types: vec![Struct::new("Date")],
            function: vec![Func { name: "fetch".to_owned() }],
            variables: vec![Var { name: "console".to_owned() }],
            keywords: vec![
                "new",
                "class",
                "for",
                "while",
                "try",
                "catch",
                "extends",
                "true",
                "false",
                "null",
                "void",
                "undefined",
                "async",
                "await",
            ],
        }
    }
}

impl TSToken {
    fn derive_from_name(&mut self) {
        if let Self::Name(name) = self {
            match name.chars().find(|ch| *ch != '_').map(|ch| ch.is_uppercase()).unwrap_or_default() {
                true => self.name_to_class(),
                false => self.name_to_func(),
            }
        }
    }

    fn name_to_func(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Function(std::mem::take(name));
        }
    }

    fn name_to_class(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Type(std::mem::take(name));
        }
    }

    fn name_to_namespace(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::NameSpace(std::mem::take(name));
        }
    }
}

fn drain_import(line: &str, logos: &mut Lexer<'_, TSToken>, token_line: &mut Vec<PositionedToken<TSToken>>) {
    token_line.push(TSToken::NameSpaceKeyWord.to_postioned(logos.span(), line));
    while let Some(token_result) = logos.next() {
        let mut pytoken = match token_result {
            Ok(pytoken) => pytoken,
            Err(_) => continue,
        };
        pytoken.name_to_namespace();
        token_line.push(pytoken.to_postioned(logos.span(), line));
    }
}

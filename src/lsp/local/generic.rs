use logos::{Lexer, Logos};

use super::{utils::NON_TOKEN_ID, LangStream, PositionedToken};

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

    #[token("use")]
    #[token("import")]
    #[token("from")]
    #[token("require")]
    NameSpaceKeyWord,

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
    #[token("True")]
    #[token("False")]
    #[token("true")]
    #[token("false")]
    Bool,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),

    // convertible types
    Type(String),
    Function(String),
    Enum(String),
    NameSpace(String),
}

impl LangStream for GenericToken {
    fn init_definitions() -> super::Definitions {
        super::Definitions { types: vec![], function: vec![], variables: vec![], keywords: vec![] }
    }
    fn parse(text: &[String], tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        tokens.clear();
        for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = GenericToken::lexer(line);
            while let Some(token_result) = logos.next() {
                let gen_token = match token_result {
                    Ok(pytoken) => pytoken,
                    Err(_) => continue,
                };
                match gen_token {
                    Self::DeclareFn => {
                        token_line.push(gen_token.to_postioned(logos.span(), line.as_str()));
                        if let Some(Ok(mut next_gentoken)) = logos.next() {
                            next_gentoken.name_to_func();
                            token_line.push(next_gentoken.to_postioned(logos.span(), line.as_str()));
                        }
                    }
                    Self::DeclareStruct => {
                        token_line.push(gen_token.to_postioned(logos.span(), line.as_str()));
                        if let Some(Ok(mut next_gentoken)) = logos.next() {
                            next_gentoken.name_to_class();
                            token_line.push(next_gentoken.to_postioned(logos.span(), line.as_str()));
                        }
                    }
                    Self::DeclareEnum => {
                        token_line.push(gen_token.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_gentoken)) = logos.next() {
                            next_gentoken.name_to_enum();
                            token_line.push(next_gentoken.to_postioned(logos.span(), line.as_str()));
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
                        token_line.push(gen_token.to_postioned(logos.span(), line.as_str()));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::NameSpace(..) => 0,
            Self::Type(..) | Self::Enum(..) => 1,
            Self::Name(..) => 8,
            Self::Function(..) => 10,
            Self::DeclareFn
            | Self::DeclareStruct
            | Self::DeclareEnum
            | Self::DeclareVar
            | Self::FlowControl
            | Self::SelfRef
            | Self::ClassRef
            | Self::Bool
            | Self::NameSpaceKeyWord => 11,
            Self::String => 13,
            Self::Int | Self::Float => 14,
            Self::Decorator => 15,
            _ => NON_TOKEN_ID,
        }
    }

    fn modifier(&self) -> u32 {
        match self {
            Self::FlowControl => 1,
            _ => 0,
        }
    }
}

impl GenericToken {
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

    fn name_to_enum(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Enum(std::mem::take(name));
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

fn drain_import(line: &str, logos: &mut Lexer<'_, GenericToken>, token_line: &mut Vec<PositionedToken<GenericToken>>) {
    token_line.push(GenericToken::NameSpaceKeyWord.to_postioned(logos.span(), line));
    while let Some(token_result) = logos.next() {
        let mut pytoken = match token_result {
            Ok(gen_token) => gen_token,
            Err(_) => continue,
        };
        pytoken.name_to_namespace();
        token_line.push(pytoken.to_postioned(logos.span(), line));
    }
}

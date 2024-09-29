use super::{Definitions, Func, LangStream, ObjType, PositionedToken, Struct, Var, NON_TOKEN_ID};
use logos::{Lexer, Logos};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")] // Ignore this regex pattern between tokens
pub enum Rustacean {
    // Tokens can be literal strings, of any length.
    #[token("fn")]
    DeclareFn,
    #[token("let")]
    DeclareVar,
    #[token("struct")]
    DeclareStruct,
    #[token("enum")]
    DeclareEnum,
    #[token("union")]
    DeclareUnion,
    #[token("type")]
    DeclareType,

    #[token("pub")]
    Public,

    #[token("mut")]
    Mutable,

    #[token("impl")]
    ImplementInterface,

    #[token("use")]
    NameSpaceKeyWord,

    #[token("crate")]
    Crate,

    #[token("match")]
    #[token("while")]
    #[token("for")]
    #[token("async")]
    #[token("await")]
    #[token("break")]
    #[token("return")]
    #[token("in")]
    #[token("continue")]
    #[token("if")]
    #[token("else")]
    FlowControl,

    #[token(r#"'\\'"#)]
    #[regex("'.'")]
    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    String,

    #[regex(r#"#\[[^\]]*\]"#)]
    Decorator,

    #[regex("//(.)*")]
    Comment,

    #[token("Self")]
    ClassRef,
    #[token("self")]
    SelfRef,
    #[token("::")]
    ParantInvoked,
    #[token(".")]
    InstanceInvoked,

    #[token("(")]
    LBrack,
    #[token(")")]
    RBrack,

    #[token("{")]
    OpenScope,
    #[token("}")]
    CloseScope,
    #[token(";")]
    EndLine,
    #[token("=")]
    Assign,
    #[token(",")]
    Comma,

    #[token("|")]
    ClosureArgs,

    #[token(": impl")]
    #[token(":impl")]
    StaticDispatch,

    #[regex(": ?[a-zA-Z]+")]
    TypeAssign,

    #[regex("<[a-zA-Z_]+>")]
    #[regex("<[a-zA-Z_]+, ?[a-zA-Z_]+>")]
    TypeInner,

    #[regex("<'[a-z_]+>")]
    LifeTime,

    #[regex("&'[a-z_]+")]
    LifeTimeAnnotation,

    #[token("->")]
    Return,

    #[regex("[a-zA-Z_']+!")]
    Macros,

    #[token("=>")]
    PatternAction,

    #[token("!")]
    Negate,
    #[token("==")]
    Equals,
    #[token("<=")]
    GreatEq,
    #[token(">=")]
    LesssEq,
    #[token("<")]
    Lesser,
    #[token(">")]
    Greater,

    #[regex(r#"-?[0-9]+\.[0-9]+"#)]
    Float,
    #[regex("-?[0-9]+")]
    Int,
    #[token("true")]
    #[token("false")]
    Bool,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),

    Type(String),
    Struct(String),
    NameSpace(String),
    Function(String),
    Trait(String),
    Enum(String),
    EnumMember(String),
    Union(String),
}

impl LangStream for Rustacean {
    fn parse(text: &[String], tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        tokens.clear();
        let mut lines = text.iter();
        while let Some(mut line) = lines.next() {
            // for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = Rustacean::lexer(line);
            while let Some(token_result) = logos.next() {
                let rustacean = match token_result {
                    Ok(rustacean) => rustacean,
                    Err(_) => continue,
                };
                match rustacean {
                    Self::DeclareFn => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_rustacean)) = logos.next() {
                            next_rustacean.name_to_func();
                            token_line.push(next_rustacean.to_postioned(logos.span(), line));
                        }
                    }
                    Self::DeclareEnum => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        if let Some(Ok(next_rustacean)) = logos.next() {
                            match next_rustacean {
                                Rustacean::Name(name) => {
                                    token_line.push(Rustacean::Enum(name).to_postioned(logos.span(), line));
                                    line = parse_enum(&mut logos, line, &mut lines, &mut token_line, tokens);
                                }
                                _ => token_line.push(next_rustacean.to_postioned(logos.span(), line)),
                            }
                        }
                    }
                    Self::DeclareUnion => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_rustacean)) = logos.next() {
                            next_rustacean.name_to_union();
                            token_line.push(next_rustacean.to_postioned(logos.span(), line));
                        }
                    }
                    Self::DeclareType => {
                        drain_type_declare(line, &mut logos, &mut token_line);
                    }
                    Self::DeclareStruct => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        if let Some(Ok(next_rustacean)) = logos.next() {
                            match next_rustacean {
                                Rustacean::Name(name) => {
                                    token_line.push(Rustacean::Struct(name).to_postioned(logos.span(), line));
                                    line = parse_struct(&mut logos, line, &mut lines, &mut token_line, tokens);
                                }
                                _ => token_line.push(next_rustacean.to_postioned(logos.span(), line)),
                            }
                        }
                    }
                    Self::StaticDispatch => {
                        let mut span = logos.span();
                        span.start += 1;
                        token_line.push(rustacean.to_postioned(span, line));
                        if let Some(Ok(Self::Name(name))) = logos.next() {
                            token_line.push(Self::Type(name).to_postioned(logos.span(), line));
                        }
                    }
                    Self::Return => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        match logos.next() {
                            Some(Ok(Rustacean::Name(name))) => {
                                token_line.push(Rustacean::Type(name).to_postioned(logos.span(), line));
                            }
                            Some(Ok(next_rustation)) => {
                                token_line.push(next_rustation.to_postioned(logos.span(), line));
                            }
                            _ => (),
                        }
                    }
                    Self::TypeAssign => {
                        let mut span = logos.span();
                        span.start += 1;
                        token_line.push(rustacean.to_postioned(span, line));
                    }
                    Self::LifeTime => {
                        let mut span = logos.span();
                        span.start += 2;
                        span.end -= 1;
                        token_line.push(rustacean.to_postioned(span, line));
                    }
                    Self::LifeTimeAnnotation => {
                        let mut span = logos.span();
                        span.start += 2;
                        token_line.push(rustacean.to_postioned(span, line));
                    }
                    Self::Macros => {
                        let mut span = logos.span();
                        span.end -= 1;
                        token_line.push(rustacean.to_postioned(span, line));
                    }
                    Self::TypeInner => {
                        if let Some(prev_token) = token_line.last_mut() {
                            prev_token.lang_token.name_to_type();
                            prev_token.refresh_type();
                        }
                        let mut span = logos.span();
                        span.start += 1;
                        span.end -= 1;
                        token_line.push(rustacean.to_postioned(span, line));
                    }
                    Self::LBrack => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.name_to_func();
                            pos_token.refresh_type();
                        }
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                    }
                    Self::ImplementInterface => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        drain_impl(line, &mut logos, &mut token_line);
                    }
                    Self::ParantInvoked => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.name_to_struct();
                            pos_token.refresh_type();
                        }
                    }
                    Self::NameSpaceKeyWord => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                        line = parse_import(&mut logos, line, &mut lines, &mut token_line, tokens);
                    }
                    _ => {
                        token_line.push(rustacean.to_postioned(logos.span(), line));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::NameSpace(..) => 0,
            Self::Type(..) | Self::Trait(..) | Self::Union(..) | Self::Enum(..) | Self::Struct(..) => 1,
            Self::TypeAssign | Self::TypeInner => 6,
            Self::Name(..) => 8,
            Self::Function(..) => 10,
            Self::DeclareFn
            | Self::DeclareStruct
            | Self::DeclareEnum
            | Self::DeclareUnion
            | Self::DeclareVar
            | Self::FlowControl
            | Self::DeclareType
            | Self::SelfRef
            | Self::ClassRef
            | Self::Bool
            | Self::Crate
            | Self::Mutable
            | Self::Macros
            | Self::StaticDispatch
            | Self::ImplementInterface
            | Self::Public
            | Self::LifeTime
            | Self::LifeTimeAnnotation
            | Self::NameSpaceKeyWord => 11,
            Self::Comment => 12,
            Self::String => 13,
            Self::Int | Self::Float => 14,
            Self::Decorator => 15,
            Self::EnumMember(..) => 16,
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
            Self::Type(name) | Self::Trait(name) | Self::Union(name) | Self::Enum(name) | Self::Struct(name) => {
                ObjType::Struct(name)
            }
            Self::Function(name) => ObjType::Fn(name),
            _ => ObjType::None,
        }
    }

    fn init_definitions() -> Definitions {
        Definitions {
            types: vec![
                Struct::new("None"),  // 0
                Struct::new("tuple"), // 1
                Struct::new("bool"),  // 2
                Struct::new("Vec"),   // 3
                Struct::new("usize"),
                Struct::new("fsize"),  // 4
                Struct::new("isize"),  // 5
                Struct::new("Option"), // 6
                Struct::new("Result"), // 7
            ],
            function: vec![
                Func { name: "vec!".to_owned() },
                Func { name: "aiter".to_owned() },
                Func { name: "all".to_owned() },
                Func { name: "any".to_owned() },
                Func { name: "anext".to_owned() },
                Func { name: "ascii".to_owned() },
                Func { name: "open".to_owned() },
                Func { name: "println!".to_owned() },
            ],
            variables: vec![Var { name: "true".to_owned() }, Var { name: "false".to_owned() }],
            keywords: vec![
                "let", "mut", "for", "while", "pub", "crate", "enum", "struct", "union", "fn",
            ],
        }
    }
}

impl Rustacean {
    fn name_to_func(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Function(std::mem::take(name));
        }
    }

    fn type_to_trait(&mut self) {
        if let Self::Type(name) = self {
            *self = Self::Trait(std::mem::take(name));
        }
    }

    fn name_to_struct(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Struct(std::mem::take(name));
        }
    }

    fn name_to_type(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Type(std::mem::take(name));
        }
    }

    fn name_to_union(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Union(std::mem::take(name));
        }
    }
}

fn parse_enum<'a>(
    logos: &mut Lexer<'a, Rustacean>,
    mut line: &'a String,
    lines: &mut impl Iterator<Item = &'a String>,
    token_line: &mut Vec<PositionedToken<Rustacean>>,
    tokens: &mut Vec<Vec<PositionedToken<Rustacean>>>,
) -> &'a String {
    let mut scope: usize = 0;
    let mut tuple_member: usize = 0;
    loop {
        while let Some(token_result) = logos.next() {
            if let Ok(rustacean) = token_result {
                match rustacean {
                    Rustacean::Name(name) if tuple_member != 0 => {
                        token_line.push(Rustacean::Type(name).to_postioned(logos.span(), line));
                    }
                    Rustacean::Name(name) => {
                        token_line.push(Rustacean::EnumMember(name).to_postioned(logos.span(), line));
                    }
                    Rustacean::LBrack => {
                        tuple_member += 1;
                    }
                    Rustacean::RBrack => {
                        tuple_member = tuple_member.saturating_sub(1);
                    }
                    Rustacean::OpenScope => {
                        scope += 1;
                        if scope > 1 {
                            line = parse_struct(logos, line, lines, token_line, tokens);
                        }
                    }
                    Rustacean::CloseScope => {
                        scope = scope.saturating_sub(1);
                        if scope == 0 {
                            return line;
                        }
                    }
                    _ => token_line.push(rustacean.to_postioned(logos.span(), line)),
                }
            }
        }
        if scope == 0 {
            break;
        }
        match lines.next() {
            None => break,
            Some(next_line) => line = next_line,
        }
        tokens.push(std::mem::take(token_line));
        *logos = Rustacean::lexer(line);
    }
    line
}

fn parse_struct<'a>(
    logos: &mut Lexer<'a, Rustacean>,
    mut line: &'a String,
    lines: &mut impl Iterator<Item = &'a String>,
    token_line: &mut Vec<PositionedToken<Rustacean>>,
    tokens: &mut Vec<Vec<PositionedToken<Rustacean>>>,
) -> &'a String {
    let mut scope: usize = 0;
    loop {
        while let Some(token_result) = logos.next() {
            if let Ok(rustacean) = token_result {
                match rustacean {
                    Rustacean::OpenScope => {
                        scope += 1;
                    }
                    Rustacean::CloseScope => {
                        scope = scope.saturating_sub(1);
                        if scope == 0 {
                            return line;
                        }
                    }
                    _ => token_line.push(rustacean.to_postioned(logos.span(), line)),
                }
            }
        }
        if scope == 0 {
            break;
        }
        match lines.next() {
            None => break,
            Some(next_line) => line = next_line,
        }
        tokens.push(std::mem::take(token_line));
        *logos = Rustacean::lexer(line);
    }
    line
}

fn parse_import<'a>(
    logos: &mut Lexer<'a, Rustacean>,
    mut line: &'a String,
    lines: &mut impl Iterator<Item = &'a String>,
    token_line: &mut Vec<PositionedToken<Rustacean>>,
    tokens: &mut Vec<Vec<PositionedToken<Rustacean>>>,
) -> &'a String {
    let mut scope: usize = 0;
    loop {
        while let Some(token_result) = logos.next() {
            if let Ok(rustacean) = token_result {
                match rustacean {
                    Rustacean::Name(name) => {
                        token_line.push(Rustacean::NameSpace(name).to_postioned(logos.span(), line));
                    }
                    Rustacean::OpenScope => {
                        scope += 1;
                    }
                    Rustacean::CloseScope => {
                        scope = scope.saturating_sub(1);
                        if scope == 0 {
                            return line;
                        }
                    }
                    _ => token_line.push(rustacean.to_postioned(logos.span(), line)),
                }
            }
        }
        if scope == 0 {
            break;
        }
        match lines.next() {
            None => break,
            Some(next_line) => line = next_line,
        }
        tokens.push(std::mem::take(token_line));
        *logos = Rustacean::lexer(line);
    }
    line
}

fn drain_impl(line: &str, logos: &mut Lexer<'_, Rustacean>, token_line: &mut Vec<PositionedToken<Rustacean>>) {
    match logos.next() {
        Some(Ok(Rustacean::Name(name))) => token_line.push(Rustacean::Type(name).to_postioned(logos.span(), line)),
        _ => return,
    }
    while let Some(rustacean) = logos.next() {
        match rustacean {
            Ok(Rustacean::FlowControl) => {
                let prev = token_line.last_mut().expect("pushed above");
                prev.lang_token.type_to_trait();
                prev.refresh_type();
                token_line.push(Rustacean::ImplementInterface.to_postioned(logos.span(), line))
            }
            Ok(Rustacean::Name(name)) => {
                token_line.push(Rustacean::Type(name).to_postioned(logos.span(), line));
            }
            Ok(Rustacean::TypeInner) => {
                let mut span = logos.span();
                span.start += 1;
                span.end -= 1;
                token_line.push(Rustacean::TypeInner.to_postioned(span, line));
            }
            _ => return,
        }
    }
}

fn drain_type_declare(line: &str, logos: &mut Lexer<'_, Rustacean>, token_line: &mut Vec<PositionedToken<Rustacean>>) {
    token_line.push(Rustacean::DeclareType.to_postioned(logos.span(), line));
    while let Some(token_result) = logos.next() {
        let rustacean = match token_result {
            Ok(rustacean) => rustacean,
            Err(_) => continue,
        };
        match rustacean {
            Rustacean::Name(name) => token_line.push(Rustacean::Type(name).to_postioned(logos.span(), line)),
            Rustacean::TypeInner => {
                let mut span = logos.span();
                if span.len() > 2 {
                    span.start += 1;
                    span.end -= 1;
                };
                token_line.push(rustacean.to_postioned(span, line))
            }
            _ => token_line.push(rustacean.to_postioned(logos.span(), line)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::lsp::local::{LangStream, PositionedToken};

    use super::Rustacean;
    use logos::{Logos, Span};

    #[test]
    fn test_chars() {
        let txt = "let m = 'c';";
        let mut logos = Rustacean::lexer(txt);
        assert_eq!(logos.next(), Some(Ok(Rustacean::DeclareVar)));
        assert_eq!(logos.span(), Span { start: 0, end: 3 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::Name("m".to_owned()))));
        assert_eq!(logos.span(), Span { start: 4, end: 5 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::Assign)));
        assert_eq!(logos.span(), Span { start: 6, end: 7 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::String)));
        assert_eq!(logos.span(), Span { start: 8, end: 11 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::EndLine)));
        assert_eq!(logos.span(), Span { start: 11, end: 12 });
        assert_eq!(logos.next(), None);
        let txt = "'\\\\'";
        let mut logos = Rustacean::lexer(txt);
        assert_eq!(logos.next(), Some(Ok(Rustacean::String)));
        assert_eq!(logos.next(), None);
    }

    #[test]
    fn test_declare_type() {
        let mut tokens = vec![];
        let txt = "pub type IdiomResult<T> = Result<T, IdiomError>;";
        Rustacean::parse(&[txt.to_owned()], &mut tokens);
        assert_eq!(
            tokens,
            vec![[
                PositionedToken { from: 0, len: 3, token_type: 11, modifier: 0, lang_token: Rustacean::Public },
                PositionedToken { from: 4, len: 4, token_type: 11, modifier: 0, lang_token: Rustacean::DeclareType },
                PositionedToken {
                    from: 9,
                    len: 11,
                    token_type: 1,
                    modifier: 0,
                    lang_token: Rustacean::Type("IdiomResult".to_owned())
                },
                PositionedToken { from: 21, len: 1, token_type: 6, modifier: 0, lang_token: Rustacean::TypeInner },
                PositionedToken { from: 24, len: 1, token_type: 17, modifier: 0, lang_token: Rustacean::Assign },
                PositionedToken {
                    from: 26,
                    len: 6,
                    token_type: 1,
                    modifier: 0,
                    lang_token: Rustacean::Type("Result".to_owned())
                },
                PositionedToken { from: 33, len: 13, token_type: 6, modifier: 0, lang_token: Rustacean::TypeInner },
                PositionedToken { from: 47, len: 1, token_type: 17, modifier: 0, lang_token: Rustacean::EndLine }
            ]]
        );
    }

    #[test]
    fn test_macros() {
        let txt = "println!(\"kjlahfksljahjf __ 🔥\");";
        let mut logos = Rustacean::lexer(txt);
        assert_eq!(logos.next(), Some(Ok(Rustacean::Macros)));
        assert_eq!(logos.span(), Span { start: 0, end: 8 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::LBrack)));
        assert_eq!(logos.span(), Span { start: 8, end: 9 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::String)));
        assert_eq!(logos.span(), Span { start: 9, end: 33 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::RBrack)));
        assert_eq!(logos.span(), Span { start: 33, end: 34 });
        assert_eq!(logos.next(), Some(Ok(Rustacean::EndLine)));
        assert_eq!(logos.span(), Span { start: 34, end: 35 });
    }

    #[test]
    fn test_decorator() {
        let mut tokens = vec![];
        let txt = "#[derive(Logos, Debug, PartialEq)]";
        Rustacean::parse(&[txt.to_owned()], &mut tokens);
        assert_eq!(
            tokens,
            vec![[PositionedToken { from: 0, len: 34, token_type: 15, modifier: 0, lang_token: Rustacean::Decorator }]]
        );

        let mut tokens = vec![];
        let txt = "LSP(#[from] LSPError)";
        Rustacean::parse(&[txt.to_owned()], &mut tokens);
        assert_eq!(
            tokens,
            vec![[
                PositionedToken {
                    from: 0,
                    len: 3,
                    token_type: 10,
                    modifier: 0,
                    lang_token: Rustacean::Function("LSP".to_owned())
                },
                PositionedToken { from: 3, len: 1, token_type: 17, modifier: 0, lang_token: Rustacean::LBrack },
                PositionedToken { from: 4, len: 7, token_type: 15, modifier: 0, lang_token: Rustacean::Decorator },
                PositionedToken {
                    from: 12,
                    len: 8,
                    token_type: 8,
                    modifier: 0,
                    lang_token: Rustacean::Name("LSPError".to_owned())
                },
                PositionedToken { from: 20, len: 1, token_type: 17, modifier: 0, lang_token: Rustacean::RBrack }
            ]]
        );
    }

    #[test]
    fn test_commas() {
        let txt = "let b = (1, 2, 3, 4);";
        let mut logos = Rustacean::lexer(txt);
        assert_eq!(Some(Ok(Rustacean::DeclareVar)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Name("b".to_owned()))), logos.next());
        assert_eq!(Some(Ok(Rustacean::Assign)), logos.next());
        assert_eq!(Some(Ok(Rustacean::LBrack)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Int)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Comma)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Int)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Comma)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Int)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Comma)), logos.next());
        assert_eq!(Some(Ok(Rustacean::Int)), logos.next());
        assert_eq!(Some(Ok(Rustacean::RBrack)), logos.next());
        assert_eq!(Some(Ok(Rustacean::EndLine)), logos.next());
    }
}

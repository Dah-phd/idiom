use logos::{Lexer, Logos};

use super::{
    super::utils::NON_TOKEN_ID, Definitions, Func, LangStream, ObjType, PositionedToken, PositionedTokenParser, Struct,
    Var,
};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
pub enum Pincer {
    #[regex("class")]
    DeclareStruct,

    #[token("def")]
    DeclareFn,

    #[token("let")]
    #[token("var")]
    DeclareVar,

    #[token("import")]
    #[token("from")]
    NameSpaceKeyWord,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[token("\"\"\"")]
    MultiString,

    #[regex("@[a-zA-Z_]+")]
    Decorator,

    #[regex("//(.)*")]
    Comment,

    #[token("while")]
    #[token("for")]
    #[token("async ")]
    #[token("break")]
    #[token("return")]
    #[token("in")]
    #[token("continue")]
    #[token("if")]
    #[token("elif")]
    #[token("else:")]
    #[token("case")]
    #[token("raise")]
    FlowControl,

    #[token("    ")]
    Scope,

    #[token("with")]
    Context,

    #[token("cls")]
    ClassRef,

    #[token("self")]
    SelfRef,

    #[token("=")]
    Assign,

    #[token(".")]
    InstanceInvoked,

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

    #[token("->")]
    ReturnHint,

    #[token("not ")]
    Negate,

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

    #[regex(r#": ?[a-zA-Z]+"#, |lex| lex.slice().to_owned())]
    TypeHint(String),

    Type(String),
    Function(String),
    NameSpace(String),
}

impl LangStream for Pincer {
    fn parse<'a>(
        text: impl Iterator<Item = &'a str>,
        tokens: &mut Vec<Vec<super::PositionedToken<Self>>>,
        parser: PositionedTokenParser<Self>,
    ) {
        tokens.clear();
        let mut is_multistring = false;
        for line in text {
            let mut token_line = Vec::new();
            let mut logos = Pincer::lexer(line);
            while let Some(token_result) = logos.next() {
                if is_multistring {
                    token_line.push(parser(Self::MultiString, logos.span(), line));
                    if matches!(token_result, Ok(Self::MultiString)) {
                        is_multistring = false;
                    }
                    continue;
                }
                let pincer = match token_result {
                    Ok(pytoken) => pytoken,
                    Err(_) => continue,
                };
                match pincer {
                    Self::DeclareFn => {
                        token_line.push(parser(pincer, logos.span(), line));
                        if let Some(Ok(mut next_pincer)) = logos.next() {
                            next_pincer.name_to_func();
                            token_line.push(parser(next_pincer, logos.span(), line));
                        }
                    }
                    Self::DeclareStruct => {
                        token_line.push(parser(pincer, logos.span(), line));
                        if let Some(Ok(mut next_pincer)) = logos.next() {
                            next_pincer.name_to_class();
                            token_line.push(parser(next_pincer, logos.span(), line));
                        }
                    }
                    Self::DeclareVar => {
                        token_line.push(parser(pincer, logos.span(), line));
                        if let Some(Ok(next_pincer)) = logos.next() {
                            token_line.push(parser(next_pincer, logos.span(), line));
                        }
                    }
                    Self::LBrack => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.derive_from_name();
                            pos_token.refresh_type();
                        }
                    }
                    Self::MultiString => {
                        token_line.push(parser(pincer, logos.span(), line));
                        is_multistring = true;
                    }
                    Self::NameSpaceKeyWord => {
                        drain_import(line, &mut logos, &mut token_line, parser);
                    }
                    _ => {
                        token_line.push(parser(pincer, logos.span(), line));
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
            | Self::DeclareVar
            | Self::DeclareStruct
            | Self::Negate
            | Self::FlowControl
            | Self::Context
            | Self::SelfRef
            | Self::ClassRef
            | Self::NameSpaceKeyWord => 11,
            Self::Comment => 12,
            Self::String | Self::MultiString => 13,
            Self::Int | Self::Float => 14,
            Self::Decorator => 15,
            _ => NON_TOKEN_ID,
        }
    }

    fn modifier(&self) -> u32 {
        match self {
            Self::FlowControl | Self::Context => 1,
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
            types: vec![
                Struct::new("None"),  // 0
                Struct::new("tuple"), // 1
                Struct::new("dict"),  // 2
                Struct::new("list"),  // 3
                Struct::new("str"),   // 4
                Struct::new("int"),   // 5
                Struct::new("float"), // 6
                Struct::new("bool"),  // 7
            ],
            function: vec![
                Func { name: "abs".to_owned() },
                Func { name: "aiter".to_owned() },
                Func { name: "all".to_owned() },
                Func { name: "any".to_owned() },
                Func { name: "anext".to_owned() },
                Func { name: "ascii".to_owned() },
                Func { name: "open".to_owned() },
                Func { name: "print".to_owned() },
            ],
            variables: vec![Var { name: "true".to_owned() }, Var { name: "false".to_owned() }],
            keywords: vec!["def", "class", "with", "for", "while", "not", "except", "raise", "try"],
        }
    }
}

impl Pincer {
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

fn drain_import(
    line: &str,
    logos: &mut Lexer<'_, Pincer>,
    token_line: &mut Vec<PositionedToken<Pincer>>,
    parser: PositionedTokenParser<Pincer>,
) {
    token_line.push(parser(Pincer::NameSpaceKeyWord, logos.span(), line));
    while let Some(token_result) = logos.next() {
        let mut pytoken = match token_result {
            Ok(pytoken) => pytoken,
            Err(_) => continue,
        };
        pytoken.name_to_namespace();
        token_line.push(parser(pytoken, logos.span(), line));
    }
}

#[cfg(test)]
mod test {
    use super::LangStream;
    use super::PositionedToken;

    use super::Pincer;
    use logos::Logos;
    use logos::Span;

    #[test]
    fn test_decor() {
        let txt = "@staticmethod";
        let mut lex = Pincer::lexer(txt);
        assert_eq!(Some(Ok(Pincer::Decorator)), lex.next());
        assert_eq!(lex.span(), Span { start: 0, end: 13 });
        assert!(lex.next().is_none());
    }

    #[test]
    fn test_comment() {
        let txt = "// hello world 'asd' and \"text\"";
        let mut lex = Pincer::lexer(txt);
        assert_eq!(Some(Ok(Pincer::Comment)), lex.next());
        assert_eq!(Span { start: 0, end: 31 }, lex.span());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_class() {
        let txt = "class WorkingDirectory:";
        let mut lex = Pincer::lexer(txt);
        assert_eq!(Some(Ok(Pincer::DeclareStruct)), lex.next());
        assert_eq!(lex.span(), Span { start: 0, end: 5 });
        assert_eq!(Some(Ok(Pincer::Name("WorkingDirectory".to_owned()))), lex.next());
        assert_eq!(lex.span(), Span { start: 6, end: 22 });
        assert_eq!(Some(Ok(Pincer::OpenScope)), lex.next());
        assert_eq!(lex.span(), Span { start: 22, end: 23 });
        assert!(lex.next().is_none());
    }

    #[test]
    fn test_scope() {
        let text = vec!["class Test:", "    value = 3"];
        let mut tokens = vec![];
        Pincer::parse(text.into_iter(), &mut tokens, PositionedToken::<Pincer>::utf32);
        assert_eq!(
            tokens,
            vec![
                vec![
                    PositionedToken { from: 0, len: 5, token_type: 11, modifier: 0, lang_token: Pincer::DeclareStruct },
                    PositionedToken {
                        from: 6,
                        len: 4,
                        token_type: 1,
                        modifier: 0,
                        lang_token: Pincer::Type("Test".to_owned())
                    },
                    PositionedToken { from: 10, len: 1, token_type: 17, modifier: 0, lang_token: Pincer::OpenScope }
                ],
                vec![
                    PositionedToken { from: 0, len: 4, token_type: 17, modifier: 0, lang_token: Pincer::Scope },
                    PositionedToken {
                        from: 4,
                        len: 5,
                        token_type: 8,
                        modifier: 0,
                        lang_token: Pincer::Name("value".to_owned())
                    },
                    PositionedToken { from: 10, len: 1, token_type: 17, modifier: 0, lang_token: Pincer::Assign },
                    PositionedToken { from: 12, len: 1, token_type: 14, modifier: 0, lang_token: Pincer::Int }
                ]
            ]
        );
    }
}

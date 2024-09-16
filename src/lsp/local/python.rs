use logos::Logos;

use super::{Definitions, Func, LangStream, PositionedToken, Struct, Var};

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")]
pub enum PyToken {
    #[regex("class")]
    DeclareStruct,

    #[token("def ")]
    DeclareFn,

    #[token("import")]
    #[token("from")]
    NameSpace,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    #[regex(r#"'([^'\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*'"#)]
    String,

    #[token("\"\"\"")]
    MultiString,

    #[regex("@[a-zA-Z_]+")]
    Decorator,

    #[regex("#*")]
    Comment,

    #[token("while ")]
    #[token("for ")]
    #[token("async ")]
    #[token("break")]
    #[token("return")]
    #[token("in ")]
    #[token("continue")]
    #[token("if ")]
    #[token("elif ")]
    #[token("else:")]
    #[token("case ")]
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
}

// SemanticTokenType::NAMESPACE,      // 0
// SemanticTokenType::TYPE,           // 1
// SemanticTokenType::CLASS,          // 2
// SemanticTokenType::ENUM,           // 3
// SemanticTokenType::INTERFACE,      // 4
// SemanticTokenType::STRUCT,         // 5
// SemanticTokenType::TYPE_PARAMETER, // 6
// SemanticTokenType::PARAMETER,      // 7
// SemanticTokenType::VARIABLE,       // 8
// SemanticTokenType::PROPERTY,       // 9
// SemanticTokenType::FUNCTION,       // 10
// SemanticTokenType::KEYWORD,        // 11
// SemanticTokenType::COMMENT,        // 12
// SemanticTokenType::STRING,         // 13
// SemanticTokenType::NUMBER,         // 14
// SemanticTokenType::DECORATOR,      // 15

impl LangStream for PyToken {
    fn parse(_defs: &mut Definitions, text: &Vec<String>, tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        tokens.clear();
        for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = PyToken::lexer(line);
            while let Some(token_result) = logos.next() {
                let pytoken = match token_result {
                    Ok(pytoken) => pytoken,
                    Err(_) => continue,
                };
                match pytoken {
                    Self::DeclareFn => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                        if let Some(Ok(next_pytoken)) = logos.next() {
                            match next_pytoken {
                                Self::Name(name) => {
                                    token_line.push(Self::Function(name).to_postioned(logos.span()));
                                }
                                _ => token_line.push(next_pytoken.to_postioned(logos.span())),
                            }
                        }
                    }
                    Self::DeclareStruct => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                        if let Some(Ok(next_pytoken)) = logos.next() {
                            match next_pytoken {
                                Self::Name(name) => {
                                    token_line.push(Self::Type(name).to_postioned(logos.span()));
                                }
                                _ => token_line.push(next_pytoken.to_postioned(logos.span())),
                            }
                        }
                    }
                    Self::LBrack => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.name_to_func();
                            pos_token.refresh_type();
                        }
                    }
                    _ => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn parse_semantics(text: &Vec<String>, tokens: &mut Vec<Vec<PositionedToken<Self>>>) {
        tokens.clear();
        for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = PyToken::lexer(line);
            while let Some(token_result) = logos.next() {
                let pytoken = match token_result {
                    Ok(pytoken) => pytoken,
                    Err(_) => continue,
                };
                match pytoken {
                    Self::DeclareFn => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                        if let Some(Ok(next_pytoken)) = logos.next() {
                            match next_pytoken {
                                Self::Name(name) => {
                                    token_line.push(Self::Function(name).to_postioned(logos.span()));
                                }
                                _ => token_line.push(next_pytoken.to_postioned(logos.span())),
                            }
                        }
                    }
                    Self::DeclareStruct => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                        if let Some(Ok(next_pytoken)) = logos.next() {
                            match next_pytoken {
                                Self::Name(name) => {
                                    token_line.push(Self::Type(name).to_postioned(logos.span()));
                                }
                                _ => token_line.push(next_pytoken.to_postioned(logos.span())),
                            }
                        }
                    }
                    Self::LBrack => {
                        if let Some(pos_token) = token_line.last_mut() {
                            pos_token.lang_token.name_to_func();
                            pos_token.refresh_type();
                        }
                    }
                    _ => {
                        token_line.push(pytoken.to_postioned(logos.span()));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::NameSpace => 0,
            Self::Type(..) => 1,
            Self::TypeHint(..) => 6,
            Self::Name(..) => 8,
            Self::Function(..) => 10,
            Self::DeclareFn
            | Self::DeclareStruct
            | Self::Negate
            | Self::FlowControl
            | Self::Context
            | Self::SelfRef
            | Self::ClassRef => 11,
            Self::String | Self::MultiString => 13,
            Self::Int | Self::Float => 14,
            Self::Decorator => 15,
            _ => 20,
        }
    }

    fn modifier(&self) -> u32 {
        match self {
            Self::FlowControl | Self::Context => 1,
            _ => 0,
        }
    }

    fn init_definitions() -> Definitions {
        Definitions {
            structs: vec![
                Struct::new("None"),                                                                      // 0
                Struct::new("tuple"),                                                                     // 1
                Struct::new("dict").meth("get").meth("remove").meth("keys").meth("items").meth("values"), // 2
                Struct::new("list").meth("pop").meth("remove").meth("insert"),                            // 3
                Struct::new("str"),                                                                       // 4
                Struct::new("int"),                                                                       // 5
                Struct::new("float"),                                                                     // 6
                Struct::new("bool"),                                                                      // 7
            ],
            function: vec![
                Func { name: "abs".to_owned(), args: vec![5], returns: Some(5) },
                Func { name: "aiter".to_owned(), ..Default::default() },
                Func { name: "all".to_owned(), args: vec![], returns: Some(7) },
                Func { name: "any".to_owned(), args: vec![], returns: Some(7) },
                Func { name: "anext".to_owned(), ..Default::default() },
                Func { name: "ascii".to_owned(), ..Default::default() },
            ],
            variables: vec![
                Var { name: "True".to_owned(), var_type: 0 },
                Var { name: "False".to_owned(), var_type: 0 },
            ],
        }
    }
}

impl PyToken {
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
}

#[cfg(test)]
mod test {
    use super::PyToken;
    use logos::Logos;
    use logos::Span;

    #[test]
    fn test_decor() {
        let txt = "@staticmethod";
        let mut lex = PyToken::lexer(txt);
        assert_eq!(Some(Ok(PyToken::Decorator)), lex.next());
        assert_eq!(lex.span(), Span { start: 0, end: 13 });
        assert!(lex.next().is_none());
    }

    #[test]
    fn test_comment() {
        let txt = "# hello world 'asd' and \"text\"";
        let mut lex = PyToken::lexer(txt);
        assert_eq!(Some(Ok(PyToken::Comment)), lex.next());
        // assert_eq!(Some(Ok(PyToken::Comment)), lex.next());
    }

    #[test]
    fn test_class() {
        let txt = "class WorkingDirectory:";
        let mut lex = PyToken::lexer(txt);
        assert_eq!(Some(Ok(PyToken::DeclareStruct)), lex.next());
        assert_eq!(lex.span(), Span { start: 0, end: 5 });
        assert_eq!(Some(Ok(PyToken::Name("WorkingDirectory".to_owned()))), lex.next());
        assert_eq!(lex.span(), Span { start: 6, end: 22 });
        assert_eq!(Some(Ok(PyToken::OpenScope)), lex.next());
        assert_eq!(lex.span(), Span { start: 22, end: 23 });
        assert!(lex.next().is_none());
    }
}

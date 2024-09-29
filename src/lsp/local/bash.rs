use logos::Logos;

use super::{utils::NON_TOKEN_ID, Definitions, LangStream};

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r" ")]
pub enum BashToken {
    #[token("alias")]
    #[token("eval")]
    #[token("echo")]
    #[token("printf")]
    #[token("printenv")]
    #[token("grep")]
    #[token("sed")]
    #[token("time")]
    #[token("source")]
    Bins,

    #[token("function")]
    DeclareFn,

    #[regex(r#"\\."#)]
    Escaped,

    #[token("\"")]
    StringMarker,

    #[token("if")]
    #[token("fi")]
    #[token("else")]
    #[token("elif")]
    #[token("then")]
    #[token("for")]
    #[token("in")]
    #[token("do")]
    #[token("while")]
    #[token("done")]
    #[token("case")]
    #[token("esac")]
    #[token("coproc")]
    #[token("select")]
    #[token("until")]
    #[token("exit")]
    FlowControl,

    #[token("(")]
    LBrack,
    #[token(")")]
    RBrack,
    #[token("[")]
    LSqrBrack,
    #[token("]")]
    RSqrBrack,
    #[token("{")]
    LCBrack,
    #[token("}")]
    RCBrack,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]+="#, |lex| lex.slice().to_owned())]
    Variable(String),

    #[regex(r#"\$[a-zA-Z_][a-zA-Z_0-9]+"#, |lex| lex.slice().to_owned())]
    #[token("$?", |_| "$?".to_owned())]
    VariableRef(String),

    #[token(";")]
    Sep,

    #[token("#!")]
    Env,

    #[token("#")]
    Comment,

    #[regex(r#"[a-zA-Z_][a-zA-Z_0-9]*"#, |lex| lex.slice().to_owned())]
    Name(String),

    Function(String),
    String,
}

impl LangStream for BashToken {
    fn parse(text: &[String], tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        tokens.clear();
        let mut is_multistring = false;
        for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = BashToken::lexer(line);
            while let Some(token_result) = logos.next() {
                if is_multistring {
                    match token_result {
                        Ok(Self::VariableRef(name)) => {
                            token_line.push(Self::VariableRef(name).to_postioned(logos.span(), line));
                        }
                        Ok(Self::StringMarker) => {
                            token_line.push(Self::String.to_postioned(logos.span(), line));
                            is_multistring = false;
                        }
                        _ => {
                            token_line.push(Self::String.to_postioned(logos.span(), line));
                        }
                    }
                    continue;
                }
                let shtoken = match token_result {
                    Ok(shtoken) => shtoken,
                    Err(_) => continue,
                };
                match shtoken {
                    Self::DeclareFn => {
                        token_line.push(shtoken.to_postioned(logos.span(), line));
                        if let Some(Ok(mut next_shtoken)) = logos.next() {
                            next_shtoken.name_to_func();
                            token_line.push(next_shtoken.to_postioned(logos.span(), line));
                        }
                    }
                    Self::StringMarker => {
                        token_line.push(Self::String.to_postioned(logos.span(), line));
                        is_multistring = true;
                    }
                    Self::Variable(mut name) => {
                        name.pop();
                        let mut span = logos.span();
                        span.end -= 1;
                        token_line.push(Self::Variable(name).to_postioned(span, line))
                    }
                    Self::Comment => {
                        token_line.push(Self::Comment.to_postioned(logos.span(), line));
                        while let Some(_) = logos.next() {
                            token_line.push(Self::Comment.to_postioned(logos.span(), line));
                        }
                    }
                    Self::Env => {
                        token_line.push(Self::Env.to_postioned(logos.span(), line));
                        while let Some(_) = logos.next() {
                            token_line.push(Self::Env.to_postioned(logos.span(), line));
                        }
                    }
                    _ => {
                        token_line.push(shtoken.to_postioned(logos.span(), line));
                    }
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::Bins => 2,
            Self::VariableRef(..) => 8,
            Self::Function(..) => 10,
            Self::Variable(..) => 11,
            Self::Comment => 12,
            Self::String => 13,
            Self::DeclareFn | Self::FlowControl | Self::Env => 11,
            _ => NON_TOKEN_ID,
        }
    }

    fn modifier(&self) -> u32 {
        match self {
            Self::FlowControl | Self::Env => 1,
            _ => 0,
        }
    }

    fn init_definitions() -> Definitions {
        Definitions { types: vec![], function: vec![], keywords: vec![], variables: vec![] }
    }
}

impl BashToken {
    fn name_to_func(&mut self) {
        if let Self::Name(name) = self {
            *self = Self::Function(std::mem::take(name));
        }
    }
}

#[cfg(test)]
mod test {
    use logos::Logos;

    use crate::lsp::local::{LangStream, PositionedToken};

    use super::BashToken;

    #[test]
    fn test_multistring() {
        let txt = "\"some text \\\" more text \"";
        let mut logos = BashToken::lexer(txt);
        assert_eq!(logos.next(), Some(Ok(BashToken::StringMarker)));
        assert_eq!(logos.next(), Some(Ok(BashToken::Name("some".to_owned()))));
        assert_eq!(logos.next(), Some(Ok(BashToken::Name("text".to_owned()))));
        assert_eq!(logos.next(), Some(Ok(BashToken::Escaped)));
        assert_eq!(logos.next(), Some(Ok(BashToken::Name("more".to_owned()))));
        assert_eq!(logos.next(), Some(Ok(BashToken::Name("text".to_owned()))));
        assert_eq!(logos.next(), Some(Ok(BashToken::StringMarker)));
        assert_eq!(logos.next(), None);
    }

    #[test]
    fn test_comment_and_env() {
        let ctxt = "# this is comment";
        let mut tokens = vec![];
        BashToken::parse(&[ctxt.to_owned()], &mut tokens);
        assert_eq!(
            tokens,
            [[
                PositionedToken { from: 0, len: 1, token_type: 12, modifier: 0, lang_token: BashToken::Comment },
                PositionedToken { from: 2, len: 4, token_type: 12, modifier: 0, lang_token: BashToken::Comment },
                PositionedToken { from: 7, len: 2, token_type: 12, modifier: 0, lang_token: BashToken::Comment },
                PositionedToken { from: 10, len: 7, token_type: 12, modifier: 0, lang_token: BashToken::Comment }
            ]]
        );
        tokens.clear();
        let etxt = "#! usr";
        BashToken::parse(&[etxt.to_owned()], &mut tokens);
        assert_eq!(
            tokens,
            [[
                PositionedToken { from: 0, len: 2, token_type: 11, modifier: 1, lang_token: BashToken::Env },
                PositionedToken { from: 3, len: 3, token_type: 11, modifier: 1, lang_token: BashToken::Env }
            ]]
        );
    }
}

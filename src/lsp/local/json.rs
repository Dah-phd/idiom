use logos::Logos;

use super::{utils::NON_TOKEN_ID, LangStream};

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum JsonValue {
    #[token("false")]
    #[token("true")]
    Bool,

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("null")]
    Null,

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?")]
    Number,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    String,
}

impl LangStream for JsonValue {
    fn parse(text: &[String], tokens: &mut Vec<Vec<super::PositionedToken<Self>>>) {
        tokens.clear();
        for line in text.iter() {
            let mut token_line = Vec::new();
            let mut logos = Self::lexer(line);
            while let Some(json_result) = logos.next() {
                if let Ok(json_value) = json_result {
                    token_line.push(json_value.to_postioned(logos.span(), line));
                }
            }
            tokens.push(token_line);
        }
    }

    fn type_id(&self) -> u32 {
        match self {
            Self::Bool | Self::Null => 11,
            Self::String => 13,
            Self::Number => 14,
            _ => NON_TOKEN_ID,
        }
    }

    fn modifier(&self) -> u32 {
        0
    }
}

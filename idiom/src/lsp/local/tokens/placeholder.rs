use logos::Logos;

use super::{LangStream, PositionedToken, PositionedTokenParser};
use std::fmt::Debug;

/// Placeholder token, where type is needed but logic is actually executed based on logic
#[derive(Logos, Debug, PartialEq)]
pub enum PlaceholderToken {}

impl LangStream for PlaceholderToken {
    fn type_id(&self) -> u32 {
        0
    }

    fn modifier(&self) -> u32 {
        0
    }

    fn parse<'a>(
        _text: impl Iterator<Item = &'a str>,
        _tokens: &mut Vec<Vec<PositionedToken<Self>>>,
        _parser: PositionedTokenParser<Self>,
    ) {
    }
}

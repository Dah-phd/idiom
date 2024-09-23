use crate::lsp::local::LangStream;
use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum BashToken {}

use anyhow::Result;
use lsp_types::{CompletionItem, CompletionResponse};
use serde_json::{from_value, Value};

#[derive(Debug)]
pub enum LSPResponseType {
    Completion(i64),
    Hover(i64),
    SignitureHelp(i64),
}

impl LSPResponseType {
    pub fn id(&self) -> &i64 {
        match self {
            Self::Completion(id) => id,
            Self::Hover(id) => id,
            Self::SignitureHelp(id) => id,
        }
    }

    pub fn parse(&self, value: Value) -> Result<Vec<CompletionItem>> {
        match from_value::<CompletionResponse>(value)? {
            CompletionResponse::Array(arr) => Ok(arr),
            CompletionResponse::List(ls) => Ok(ls.items),
        }
    }
}

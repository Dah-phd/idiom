use lsp_types::{
    request::GotoDeclarationResponse, CompletionItem, CompletionResponse, GotoDefinitionResponse, Hover, Location,
    SemanticTokensRangeResult, SemanticTokensResult, SignatureHelp, WorkspaceEdit,
};
use serde_json::{from_value, Value};

#[derive(Debug)]
pub enum LSPResponseType {
    Completion(i64, String, usize),
    Hover(i64),
    SignatureHelp(i64),
    References(i64),
    Renames(i64),
    Tokens(i64),
    TokensPartial(i64),
    Definition(i64),
    Declaration(i64),
}

impl LSPResponseType {
    pub fn id(&self) -> &i64 {
        match self {
            Self::Completion(id, ..) => id,
            Self::Hover(id) => id,
            Self::SignatureHelp(id) => id,
            Self::References(id) => id,
            Self::Renames(id) => id,
            Self::Tokens(id) => id,
            Self::TokensPartial(id) => id,
            Self::Definition(id) => id,
            Self::Declaration(id) => id,
        }
    }

    pub fn parse(&self, value: Option<Value>) -> Option<LSPResult> {
        Some(match self {
            Self::Completion(.., line, idx) => match from_value::<CompletionResponse>(value?).ok()? {
                CompletionResponse::Array(arr) => LSPResult::Completion(arr, line.to_owned(), *idx),
                CompletionResponse::List(ls) => LSPResult::Completion(ls.items, line.to_owned(), *idx),
            },
            Self::Hover(..) => LSPResult::Hover(from_value(value?).ok()?),
            Self::SignatureHelp(..) => LSPResult::SignatureHelp(from_value(value?).ok()?),
            Self::References(..) => LSPResult::References(from_value(value?).ok()?),
            Self::Renames(..) => LSPResult::Renames(from_value(value?).ok()?),
            Self::Tokens(..) => LSPResult::Tokens(from_value(value?).ok()?),
            Self::TokensPartial(..) => LSPResult::TokensPartial(from_value(value?).ok()?),
            Self::Definition(..) => LSPResult::Definition(from_value(value?).ok()?),
            Self::Declaration(..) => LSPResult::Declaration(from_value(value?).ok()?),
        })
    }
}

pub enum LSPResult {
    Completion(Vec<CompletionItem>, String, usize),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    References(Option<Vec<Location>>),
    Renames(WorkspaceEdit),
    Tokens(SemanticTokensResult),
    TokensPartial(SemanticTokensRangeResult),
    Definition(GotoDefinitionResponse),
    Declaration(GotoDeclarationResponse),
}

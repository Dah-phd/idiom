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
    TokensPartial {
        id: i64,
        max_lines: usize,
    },
    #[allow(dead_code)]
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
            Self::TokensPartial { id, .. } => id,
            Self::Definition(id) => id,
            Self::Declaration(id) => id,
        }
    }

    pub fn parse(&self, value: Option<Value>) -> Option<LSPResponse> {
        Some(match self {
            Self::Completion(.., line, idx) => match from_value::<CompletionResponse>(value?).ok()? {
                CompletionResponse::Array(arr) => LSPResponse::Completion(arr, line.to_owned(), *idx),
                CompletionResponse::List(ls) => LSPResponse::Completion(ls.items, line.to_owned(), *idx),
            },
            Self::Hover(..) => LSPResponse::Hover(from_value(value?).ok()?),
            Self::SignatureHelp(..) => LSPResponse::SignatureHelp(from_value(value?).ok()?),
            Self::References(..) => LSPResponse::References(from_value(value?).ok()?),
            Self::Renames(..) => LSPResponse::Renames(from_value(value?).ok()?),
            Self::Tokens(..) => LSPResponse::Tokens(from_value(value?).ok()?),
            Self::TokensPartial { max_lines, .. } => {
                LSPResponse::TokensPartial { result: from_value(value?).ok()?, max_lines: *max_lines }
            }
            Self::Definition(..) => LSPResponse::Definition(from_value(value?).ok()?),
            Self::Declaration(..) => LSPResponse::Declaration(from_value(value?).ok()?),
        })
    }
}

pub enum LSPResponse {
    Completion(Vec<CompletionItem>, String, usize),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    References(Option<Vec<Location>>),
    Renames(WorkspaceEdit),
    Tokens(SemanticTokensResult),
    TokensPartial { result: SemanticTokensRangeResult, max_lines: usize },
    Definition(GotoDefinitionResponse),
    Declaration(GotoDeclarationResponse),
}

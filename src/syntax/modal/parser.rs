use lsp_types::{
    request::GotoDeclarationResponse, CompletionItem, CompletionResponse, GotoDefinitionResponse, Hover,
    SemanticTokensResult, SignatureHelp, WorkspaceEdit,
};
use serde_json::{from_value, Value};

#[derive(Debug)]
pub enum LSPResponseType {
    Completion(i64, String, usize),
    Hover(i64),
    SignatureHelp(i64),
    Renames(i64),
    TokensFull(i64),
    Definition(i64),
    Declaration(i64),
}

impl LSPResponseType {
    pub fn id(&self) -> &i64 {
        match self {
            Self::Completion(id, ..) => id,
            Self::Hover(id) => id,
            Self::SignatureHelp(id) => id,
            Self::Renames(id) => id,
            Self::TokensFull(id) => id,
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
            Self::Hover(..) => LSPResult::Hover(from_value::<Hover>(value?).ok()?),
            Self::SignatureHelp(..) => LSPResult::SignatureHelp(from_value::<SignatureHelp>(value?).ok()?),
            Self::Renames(..) => LSPResult::Renames(from_value::<WorkspaceEdit>(value?).ok()?),
            Self::TokensFull(..) => LSPResult::Tokens(from_value::<SemanticTokensResult>(value?).ok()?),
            Self::Definition(..) => LSPResult::Definition(from_value::<GotoDefinitionResponse>(value?).ok()?),
            Self::Declaration(..) => LSPResult::Declaration(from_value::<GotoDeclarationResponse>(value?).ok()?),
        })
    }
}

pub enum LSPResult {
    Completion(Vec<CompletionItem>, String, usize),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    Renames(WorkspaceEdit),
    Tokens(SemanticTokensResult),
    Definition(GotoDefinitionResponse),
    Declaration(GotoDeclarationResponse),
}

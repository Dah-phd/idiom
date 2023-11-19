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

    pub fn parse(&self, value: Value) -> LSPResult {
        match self {
            Self::Completion(.., line, idx) => {
                if let Ok(response) = from_value::<CompletionResponse>(value) {
                    return match response {
                        CompletionResponse::Array(arr) => LSPResult::Completion(arr, line.to_owned(), *idx),
                        CompletionResponse::List(ls) => LSPResult::Completion(ls.items, line.to_owned(), *idx),
                    };
                }
            }
            Self::Hover(..) => {
                if let Ok(response) = from_value::<Hover>(value) {
                    return LSPResult::Hover(response);
                }
            }
            Self::SignatureHelp(..) => {
                if let Ok(response) = from_value::<SignatureHelp>(value) {
                    return LSPResult::SignatureHelp(response);
                }
            }
            Self::Renames(..) => {
                if let Ok(response) = from_value::<WorkspaceEdit>(value) {
                    return LSPResult::Renames(response);
                }
            }
            Self::TokensFull(..) => {
                if let Ok(response) = from_value::<SemanticTokensResult>(value) {
                    return LSPResult::Tokens(response);
                }
            }
            Self::Definition(..) => {
                if let Ok(response) = from_value::<GotoDefinitionResponse>(value) {
                    return LSPResult::Definition(response);
                }
            }
            Self::Declaration(..) => {
                if let Ok(response) = from_value::<GotoDeclarationResponse>(value) {
                    return LSPResult::Declaration(response);
                }
            }
        }
        LSPResult::None
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
    None,
}

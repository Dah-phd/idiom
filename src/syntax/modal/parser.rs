use lsp_types::{CompletionItem, CompletionResponse, Hover, SignatureHelp, WorkspaceEdit};
use serde_json::{from_value, Value};

#[derive(Debug)]
pub enum LSPResponseType {
    Completion(i64),
    Hover(i64),
    SignatureHelp(i64),
    Renames(i64),
}

impl LSPResponseType {
    pub fn id(&self) -> &i64 {
        match self {
            Self::Completion(id) => id,
            Self::Hover(id) => id,
            Self::SignatureHelp(id) => id,
            Self::Renames(id) => id,
        }
    }

    pub fn parse(&self, value: Value) -> LSPResult {
        match self {
            Self::Completion(..) => {
                if let Ok(response) = from_value::<CompletionResponse>(value) {
                    return match response {
                        CompletionResponse::Array(arr) => LSPResult::Completion(arr),
                        CompletionResponse::List(ls) => LSPResult::Completion(ls.items),
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
        }
        LSPResult::None
    }
}

pub enum LSPResult {
    Completion(Vec<CompletionItem>),
    Hover(Hover),
    SignatureHelp(SignatureHelp),
    Renames(WorkspaceEdit),
    None,
}

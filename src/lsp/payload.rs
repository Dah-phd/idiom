use crate::{
    cursor::CursorPosition,
    lsp::{LSPError, LSPNotification, LSPRequest, LSPResponseType},
};
use lsp_types::{
    notification::DidChangeTextDocument,
    request::{
        Completion, GotoDeclaration, GotoDefinition, HoverRequest, References, Rename, SemanticTokensFullRequest,
        SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    Range, TextDocumentContentChangeEvent, Uri,
};

pub enum Payload {
    /// Notifications
    Sync(Uri, i32, Vec<TextDocumentContentChangeEvent>),
    FullSync(Uri, i32, String),
    /// Requests
    Tokens(Uri, i64),
    PartialTokens(Uri, Range, i64, usize),
    Completion(Uri, CursorPosition, i64, String),
    Rename(Uri, CursorPosition, String, i64),
    References(Uri, CursorPosition, i64),
    Definition(Uri, CursorPosition, i64),
    Declaration(Uri, CursorPosition, i64),
    Hover(Uri, CursorPosition, i64),
    SignatureHelp(Uri, CursorPosition, i64),
    /// Send serialized
    Direct(String),
}

impl Payload {
    pub fn try_stringify(self) -> Result<(String, Option<(i64, LSPResponseType)>), LSPError> {
        match self {
            // Direct sending of serialized message
            Payload::Direct(msg) => Ok((msg, None)),
            // Create and stringify notification
            Payload::Sync(uri, version, events) => {
                LSPNotification::<DidChangeTextDocument>::file_did_change(uri, version, events)
                    .stringify()
                    .map(|text| (text, None))
            }
            Payload::FullSync(uri, version, text) => {
                let full_changes = vec![TextDocumentContentChangeEvent { range: None, range_length: None, text }];
                LSPNotification::<DidChangeTextDocument>::file_did_change(uri, version, full_changes)
                    .stringify()
                    .map(|text| (text, None))
            }
            // Create and send request
            Payload::References(uri, c, id) => LSPRequest::<References>::references(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::References)))),
            Payload::Definition(uri, c, id) => LSPRequest::<GotoDefinition>::definition(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Definition)))),
            Payload::Declaration(uri, c, id) => LSPRequest::<GotoDeclaration>::declaration(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Declaration)))),
            Payload::Completion(uri, c, id, line) => LSPRequest::<Completion>::completion(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Completion(line, c))))),
            Payload::Tokens(uri, id) => LSPRequest::<SemanticTokensFullRequest>::semantics_full(uri, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Tokens)))),
            Payload::PartialTokens(uri, range, id, max_lines) => {
                LSPRequest::<SemanticTokensRangeRequest>::semantics_range(uri, range, id)
                    .stringify()
                    .map(|text| (text, Some((id, LSPResponseType::TokensPartial { max_lines }))))
            }
            Payload::Rename(uri, c, new_name, id) => LSPRequest::<Rename>::rename(uri, c, new_name, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Renames)))),
            Payload::Hover(uri, c, id) => LSPRequest::<HoverRequest>::hover(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::Hover)))),
            Payload::SignatureHelp(uri, c, id) => LSPRequest::<SignatureHelpRequest>::signature_help(uri, c, id)
                .stringify()
                .map(|text| (text, Some((id, LSPResponseType::SignatureHelp)))),
        }
    }
}

impl From<String> for Payload {
    #[inline]
    fn from(value: String) -> Self {
        Self::Direct(value)
    }
}

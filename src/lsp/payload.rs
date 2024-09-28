use crate::lsp::{LSPError, LSPNotification, LSPRequest};
use crate::workspace::CursorPosition;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::request::Completion;
use lsp_types::request::GotoDeclaration;
use lsp_types::request::GotoDefinition;
use lsp_types::request::HoverRequest;
use lsp_types::request::References;
use lsp_types::request::Rename;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::request::SemanticTokensRangeRequest;
use lsp_types::request::SignatureHelpRequest;
use lsp_types::{Range, TextDocumentContentChangeEvent, Uri};

pub enum Payload {
    /// Notifications
    Sync(Uri, i32, Vec<TextDocumentContentChangeEvent>),
    FullSync(Uri, i32, String),
    /// Requests
    Tokens(Uri, i64),
    PartialTokens(Uri, Range, i64),
    Completion(Uri, CursorPosition, i64),
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
    pub fn try_stringify(self) -> Result<String, LSPError> {
        match self {
            // Direct sending of serialized message
            Payload::Direct(msg) => Ok(msg),
            // Create and stringify notification
            Payload::Sync(uri, version, events) => {
                LSPNotification::<DidChangeTextDocument>::file_did_change(uri, version, events).stringify()
            }
            Payload::FullSync(uri, version, text) => {
                let full_changes = vec![TextDocumentContentChangeEvent { range: None, range_length: None, text }];
                LSPNotification::<DidChangeTextDocument>::file_did_change(uri, version, full_changes).stringify()
            }
            // Create and send request
            Payload::References(uri, c, id) => LSPRequest::<References>::references(uri, c, id).stringify(),
            Payload::Definition(uri, c, id) => LSPRequest::<GotoDefinition>::definition(uri, c, id).stringify(),
            Payload::Declaration(uri, c, id) => LSPRequest::<GotoDeclaration>::declaration(uri, c, id).stringify(),
            Payload::Completion(uri, c, id) => LSPRequest::<Completion>::completion(uri, c, id).stringify(),
            Payload::Tokens(uri, id) => LSPRequest::<SemanticTokensFullRequest>::semantics_full(uri, id).stringify(),
            Payload::PartialTokens(uri, range, id) => {
                LSPRequest::<SemanticTokensRangeRequest>::semantics_range(uri, range, id).stringify()
            }
            Payload::Rename(uri, c, new_name, id) => LSPRequest::<Rename>::rename(uri, c, new_name, id).stringify(),
            Payload::Hover(uri, c, id) => LSPRequest::<HoverRequest>::hover(uri, c, id).stringify(),
            Payload::SignatureHelp(uri, c, id) => {
                LSPRequest::<SignatureHelpRequest>::signature_help(uri, c, id).stringify()
            }
        }
    }
}

impl From<String> for Payload {
    #[inline]
    fn from(value: String) -> Self {
        Self::Direct(value)
    }
}

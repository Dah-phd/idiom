use super::{Diagnostic, LSPNotification, LSPRequest, Response};
use crate::{configs::FileType, lsp::LSPError, syntax::DiagnosticLine, workspace::CursorPosition};

use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument},
    request::{
        Completion, GotoDeclaration, GotoDefinition, HoverRequest, References, Rename, SemanticTokensFullRequest,
        SemanticTokensRangeRequest, SignatureHelpRequest,
    },
    Range, ServerCapabilities, TextDocumentContentChangeEvent,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;

/// LSPClient
/// Receives and sends messages to the LSP server running.
/// Sending is done by channel.
/// Received messages are stored in Mutex dicts.
/// Responses are received by ID - so every editor can receive its answere only to send Requests.
/// Failure on broken LSP server.
/// Diagnostics are received from Diagnostic objec stored in hashmap based on path.
#[derive(Clone)]
pub struct LSPClient {
    diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
    responses: Arc<Mutex<HashMap<i64, Response>>>,
    channel: UnboundedSender<String>,
    request_counter: Rc<RefCell<i64>>,
    pub capabilities: ServerCapabilities,
}

impl LSPClient {
    pub fn new(
        diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
        responses: Arc<Mutex<HashMap<i64, Response>>>,
        channel: UnboundedSender<String>,
        capabilities: ServerCapabilities,
    ) -> Self {
        Self { diagnostics, responses, channel, request_counter: Rc::default(), capabilities }
    }

    pub fn request<T>(&mut self, mut request: LSPRequest<T>) -> Result<i64, LSPError>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_id();
        request.id = id;
        self.channel.send(request.stringify()?)?;
        Ok(id)
    }

    pub fn notify<T>(&mut self, notification: LSPNotification<T>) -> Result<(), LSPError>
    where
        T: lsp_types::notification::Notification,
        T::Params: serde::Serialize,
    {
        self.channel.send(notification.stringify()?)?;
        Ok(())
    }

    pub fn get(&self, id: &i64) -> Option<Response> {
        let mut que = self.responses.try_lock().ok()?;
        que.remove(id)
    }

    pub fn get_lsp_registration(&self) -> Arc<Mutex<HashMap<PathBuf, Diagnostic>>> {
        Arc::clone(&self.diagnostics)
    }

    pub fn get_diagnostics(&self, path: &Path) -> Option<Vec<(usize, DiagnosticLine)>> {
        self.diagnostics.try_lock().ok()?.get_mut(path)?.lines.take()
    }

    pub fn is_closed(&self) -> bool {
        self.channel.is_closed()
    }

    pub fn request_partial_tokens(&mut self, path: &Path, range: Range) -> Option<i64> {
        self.capabilities.semantic_tokens_provider.as_ref()?;
        self.request(LSPRequest::<SemanticTokensRangeRequest>::semantics_range(path, range)?).ok()
    }

    pub fn request_full_tokens(&mut self, path: &Path) -> Option<i64> {
        self.capabilities.semantic_tokens_provider.as_ref()?;
        self.request(LSPRequest::<SemanticTokensFullRequest>::semantics_full(path)?).ok()
    }

    pub fn request_completions(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.request(LSPRequest::<Completion>::completion(path, c)?).ok()
    }

    pub fn request_rename(&mut self, path: &Path, c: CursorPosition, new_name: String) -> Option<i64> {
        self.request(LSPRequest::<Rename>::rename(path, c, new_name)?).ok()
    }

    pub fn request_signitures(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.capabilities.signature_help_provider.as_ref()?;
        self.request(LSPRequest::<SignatureHelpRequest>::signature_help(path, c)?).ok()
    }

    pub fn request_hover(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.capabilities.hover_provider.as_ref()?;
        self.request(LSPRequest::<HoverRequest>::hover(path, c)?).ok()
    }

    pub fn request_references(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.capabilities.references_provider.as_ref()?;
        self.request(LSPRequest::<References>::references(path, c)?).ok()
    }

    pub fn request_declarations(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.capabilities.declaration_provider.as_ref()?;
        self.request(LSPRequest::<GotoDeclaration>::declaration(path, c)?).ok()
    }

    #[allow(dead_code)]
    pub fn request_definitions(&mut self, path: &Path, c: CursorPosition) -> Option<i64> {
        self.capabilities.definition_provider.as_ref()?;
        self.request(LSPRequest::<GotoDefinition>::definition(path, c)?).ok()
    }

    pub fn file_did_open(&mut self, path: &Path, file_type: &FileType, content: String) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidOpenTextDocument>::file_did_open(path, file_type, content)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_change(
        &mut self,
        path: &Path,
        version: i32,
        change_events: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidChangeTextDocument>::file_did_change(path, version, change_events)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_save(&mut self, path: &Path) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidSaveTextDocument>::file_did_save(path)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_close(&mut self, path: &Path) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidCloseTextDocument>::file_did_close(path)?;
        self.notify(notification)?;
        Ok(())
    }

    fn next_id(&mut self) -> i64 {
        let mut id = self.request_counter.borrow_mut();
        *id += 1;
        *id
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use lsp_types::ServerCapabilities;

    use super::LSPClient;

    #[test]
    fn test_counter() {
        let (rx, _tx) = tokio::sync::mpsc::unbounded_channel();
        let mut mock = LSPClient {
            diagnostics: Arc::new(Mutex::new(HashMap::new())),
            responses: Arc::new(Mutex::new(HashMap::new())),
            channel: rx,
            request_counter: Rc::new(RefCell::new(0)),
            capabilities: ServerCapabilities::default(),
        };
        assert_eq!(1, mock.next_id());
        assert_eq!(2, mock.next_id());
    }
}

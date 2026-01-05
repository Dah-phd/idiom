use super::{
    local::{build_with_enrichment, create_semantic_capabilities, start_lsp_handler},
    messages::DiagnosticHandle,
    payload::Payload,
    EditorDiagnostics, LSPError, LSPNotification, LSPRequest, LSPResponse, LSPResult, Requests, Responses,
    TreeDiagnostics,
};
use crate::{configs::FileType, cursor::CursorPosition, utils::split_arc};
use lsp_types::{
    notification::{DidCloseTextDocument, DidOpenTextDocument, DidRenameFiles, DidSaveTextDocument, Exit, Initialized},
    request::Shutdown,
    CompletionOptions, InitializedParams, PositionEncodingKind, Range, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentSyncKind, Uri,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::{
    process::ChildStdin,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

/// LSPClient
/// Receives and sends messages to the LSP server running.
/// Sending is done by channel.
/// Received messages are stored in Mutex dicts.
/// Responses are received by ID - so every editor can receive its answere only to send Requests.
/// Failure on broken LSP server.
/// Diagnostics are received from Diagnostic objec stored in hashmap based on path.
pub struct LSPClient {
    diagnostics: Arc<Mutex<DiagnosticHandle>>,
    responses: Arc<Responses>,
    channel: UnboundedSender<Payload>,
    id_gen: MonoID,
    // can handle some requests, syntax and autocomplete
    local_lsp: Option<JoinHandle<LSPResult<()>>>,
    pub capabilities: ServerCapabilities,
}

impl Clone for LSPClient {
    fn clone(&self) -> Self {
        Self {
            diagnostics: Arc::clone(&self.diagnostics),
            responses: Arc::clone(&self.responses),
            channel: self.channel.clone(),
            id_gen: self.id_gen.clone(),
            local_lsp: None,
            capabilities: self.capabilities.clone(),
        }
    }
}

impl LSPClient {
    pub fn new(
        stdin: ChildStdin,
        file_type: FileType,
        diagnostics: Arc<Mutex<DiagnosticHandle>>,
        requests: Arc<Requests>,
        responses: Arc<Responses>,
        mut capabilities: ServerCapabilities,
    ) -> LSPResult<(JoinHandle<LSPResult<()>>, Self)> {
        let (channel, rx) = unbounded_channel::<Payload>();

        let lsp_send_handler =
            build_with_enrichment(rx, stdin, file_type, requests, Arc::clone(&responses), &mut capabilities);

        let notification: LSPNotification<Initialized> = LSPNotification::with(InitializedParams {});
        channel.send(notification.stringify()?.into())?;
        Ok((
            lsp_send_handler,
            Self { diagnostics, responses, channel, id_gen: MonoID::default(), capabilities, local_lsp: None },
        ))
    }

    pub fn local_lsp(file_type: FileType) -> Self {
        let (channel, rx) = unbounded_channel::<Payload>();

        let (responses, response_handler) = split_arc::<Responses>();
        let capabilities = ServerCapabilities {
            semantic_tokens_provider: Some(create_semantic_capabilities()),
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
            completion_provider: Some(CompletionOptions::default()),
            position_encoding: Some(PositionEncodingKind::UTF32),
            ..Default::default()
        };

        // starting local lsp /parsing + generating tokens/
        let lsp_send_handler = start_lsp_handler(rx, file_type, response_handler);
        Self {
            diagnostics: Arc::default(),
            responses,
            channel,
            id_gen: MonoID::default(),
            capabilities,
            local_lsp: Some(lsp_send_handler),
        }
    }

    /// instead of haveing checks all over the place this will simply do nothing with LSP request
    pub fn placeholder() -> Self {
        let (channel, _) = tokio::sync::mpsc::unbounded_channel::<Payload>();
        Self {
            diagnostics: Arc::default(),
            responses: Arc::default(),
            channel,
            id_gen: MonoID::default(),
            local_lsp: None,
            capabilities: ServerCapabilities::default(),
        }
    }

    #[inline]
    pub fn get_responses(&self) -> Option<MutexGuard<'_, HashMap<i64, LSPResponse>>> {
        self.responses.try_lock().ok()
    }

    /// ensures old requests that may not have been handled due to tab change are cleared
    pub fn clear_requests(&self) {
        self.responses.lock().unwrap().clear();
    }

    #[inline]
    pub fn get_diagnostics(&self, uri: &Uri) -> (Option<EditorDiagnostics>, Option<TreeDiagnostics>) {
        match self.diagnostics.try_lock() {
            Ok(mut guard) => guard.collect(uri),
            _ => (None, None),
        }
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        self.channel.is_closed()
    }

    #[inline]
    pub fn request_partial_tokens(&mut self, uri: Uri, range: Range, max_lines: usize) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::PartialTokens(uri, range, id, max_lines))?;
        Ok(id)
    }

    #[inline]
    pub fn request_full_tokens(&mut self, uri: Uri) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Tokens(uri, id))?;
        Ok(id)
    }

    #[inline]
    pub fn request_completions(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Completion(uri, c, id))?;
        Ok(id)
    }

    pub fn request_rename(&mut self, uri: Uri, c: CursorPosition, new_name: String) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Rename(uri, c, new_name, id))?;
        Ok(id)
    }

    pub fn formatting(&mut self, uri: Uri, indent: usize, save: bool) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Formatting { uri, id, indent, save })?;
        Ok(id)
    }

    pub fn request_signitures(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::SignatureHelp(uri, c, id))?;
        Ok(id)
    }

    pub fn request_hover(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Hover(uri, c, id))?;
        Ok(id)
    }

    pub fn request_references(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::References(uri, c, id))?;
        Ok(id)
    }

    pub fn request_declarations(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Declaration(uri, c, id))?;
        Ok(id)
    }

    #[allow(dead_code)]
    pub fn request_definitions(&mut self, uri: Uri, c: CursorPosition) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::Definition(uri, c, id))?;
        Ok(id)
    }

    pub fn update_path(&mut self, old_uri: Uri, new_uri: Uri) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidRenameFiles>::rename_file(old_uri, new_uri)?;
        self.channel.send(notification.stringify()?.into()).map_err(LSPError::from)
    }

    pub fn file_did_open(&mut self, uri: Uri, file_type: FileType, content: String) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidOpenTextDocument>::file_did_open(uri, file_type, content);
        self.channel.send(notification.stringify()?.into()).map_err(LSPError::from)
    }

    pub fn full_sync(&mut self, uri: Uri, version: i32, text: String) -> Result<(), LSPError> {
        self.channel.send(Payload::FullSync(uri, version, text)).map_err(LSPError::from)
    }

    pub fn sync(
        &mut self,
        uri: Uri,
        version: i32,
        events: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), LSPError> {
        self.channel.send(Payload::Sync(uri, version, events)).map_err(LSPError::from)
    }

    pub fn file_did_save(&mut self, uri: Uri, content: String) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidSaveTextDocument>::file_did_save(uri, content);
        self.channel.send(notification.stringify()?.into()).map_err(LSPError::from)
    }

    pub fn file_did_close(&mut self, uri: Uri) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidCloseTextDocument>::file_did_close(uri);
        self.channel.send(notification.stringify()?.into()).map_err(LSPError::from)
    }

    pub fn init(&mut self) -> Result<(), LSPError> {
        let notification: LSPNotification<Initialized> = LSPNotification::with(InitializedParams {});
        self.channel.send(notification.stringify()?.into()).map_err(LSPError::from)
    }

    pub fn stop(&mut self) {
        let id = self.id_gen.next_id();
        if let Ok(text) = LSPRequest::<Shutdown>::with(id, ()).stringify() {
            let _ = self.channel.send(Payload::Direct(text));
        }
        if let Ok(text) = LSPNotification::<Exit>::with(()).stringify() {
            let _ = self.channel.send(Payload::Direct(text));
        }
        *self = Self::placeholder();
    }
}

impl Drop for LSPClient {
    fn drop(&mut self) {
        // if pseudo lsp is running ensure it is dropped on editor destruction
        if let Some(pseudo_lsp) = self.local_lsp.take() {
            pseudo_lsp.abort();
        }
    }
}

#[derive(Clone, Default)]
pub struct MonoID {
    inner: Rc<RefCell<i64>>,
}

impl MonoID {
    fn next_id(&mut self) -> i64 {
        let mut id = self.inner.borrow_mut();
        *id += 1;
        *id
    }
}

#[cfg(test)]
mod test {
    use super::{LSPClient, MonoID};

    #[test]
    fn test_gen_id() {
        let mut id_gen = MonoID::default();
        assert_eq!(1, id_gen.next_id());
        assert_eq!(2, id_gen.next_id());
    }

    #[test]
    fn test_holder() {
        let holder = LSPClient::placeholder();
        assert!(holder.channel.is_closed());
    }
}

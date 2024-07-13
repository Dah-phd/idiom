use super::{Diagnostic, LSPNotification, LSPRequest, LSPResult, Response};
use crate::{
    configs::FileType,
    lsp::LSPError,
    syntax::DiagnosticLine,
    workspace::{actions::LSPEvent, CursorPosition},
};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidRenameFiles, DidSaveTextDocument, Exit,
        Initialized,
    },
    request::{
        Completion, GotoDeclaration, GotoDefinition, HoverRequest, References, Rename, SemanticTokensFullRequest,
        SemanticTokensRangeRequest, Shutdown, SignatureHelpRequest,
    },
    InitializedParams, Range, ServerCapabilities, TextDocumentContentChangeEvent, Uri,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    process::ChildStdin,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

pub enum Payload {
    /// Notifications
    Sync(Uri, i32, Vec<LSPEvent>),
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
    fn try_stringify(self, position_map: fn(LSPEvent) -> TextDocumentContentChangeEvent) -> Result<String, LSPError> {
        match self {
            // Direct sending of serialized message
            Payload::Direct(msg) => Ok(msg),
            // Create and stringify notification
            Payload::Sync(uri, version, events) => {
                let changes = events.into_iter().map(position_map).collect();
                LSPNotification::<DidChangeTextDocument>::file_did_change(uri, version, changes).stringify()
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
    channel: UnboundedSender<Payload>,
    id_gen: MonoID,
    pub capabilities: ServerCapabilities,
}

impl LSPClient {
    pub fn new(
        mut stdin: ChildStdin,
        diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
        responses: Arc<Mutex<HashMap<i64, Response>>>,
        capabilities: ServerCapabilities,
    ) -> LSPResult<(JoinHandle<LSPResult<()>>, Self)> {
        let (channel, mut rx) = unbounded_channel::<Payload>();

        let position_map = match capabilities.position_encoding.as_ref().map(|inner| inner.as_str()) {
            Some("utf-8") => pos_utf8,
            Some("utf-32") => pos_utf32,
            _ => pos_utf16,
        };

        // starting send handler
        let lsp_send_handler = tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(lsp_msg_text) = msg.try_stringify(position_map) {
                    stdin.write_all(lsp_msg_text.as_bytes()).await?;
                    stdin.flush().await?;
                }
            }
            Ok(())
        });

        let notification: LSPNotification<Initialized> = LSPNotification::with(InitializedParams {});
        channel.send(notification.stringify()?.into())?;
        Ok((lsp_send_handler, Self { diagnostics, responses, channel, id_gen: MonoID::default(), capabilities }))
    }

    pub fn placeholder() -> Self {
        let (channel, _) = tokio::sync::mpsc::unbounded_channel::<Payload>();
        Self {
            diagnostics: Arc::default(),
            responses: Arc::default(),
            channel,
            id_gen: MonoID::default(),
            capabilities: ServerCapabilities::default(),
        }
    }

    pub fn get(&self, id: &i64) -> Option<Response> {
        let mut que = self.responses.try_lock().ok()?;
        que.remove(id)
    }

    pub fn get_lsp_registration(&self) -> Arc<Mutex<HashMap<PathBuf, Diagnostic>>> {
        Arc::clone(&self.diagnostics)
    }

    #[inline]
    pub fn get_diagnostics(&self, path: &Path) -> Option<Vec<(usize, DiagnosticLine)>> {
        self.diagnostics.try_lock().ok()?.get_mut(path)?.lines.take()
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        self.channel.is_closed()
    }

    #[inline]
    pub fn request_partial_tokens(&mut self, uri: Uri, range: Range) -> LSPResult<i64> {
        let id = self.id_gen.next_id();
        self.channel.send(Payload::PartialTokens(uri, range, id))?;
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

    pub fn sync(&mut self, uri: Uri, version: i32, events: Vec<LSPEvent>) -> Result<(), LSPError> {
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

fn pos_utf8(event: LSPEvent) -> TextDocumentContentChangeEvent {
    event.utf8_text_change()
}

fn pos_utf16(event: LSPEvent) -> TextDocumentContentChangeEvent {
    event.utf16_text_change()
}

fn pos_utf32(event: LSPEvent) -> TextDocumentContentChangeEvent {
    event.utf32_text_change()
}

#[cfg(test)]
mod test {
    use super::MonoID;

    #[test]
    fn test_gen_id() {
        let mut gen = MonoID::default();
        assert_eq!(1, gen.next_id());
        assert_eq!(2, gen.next_id());
    }
}

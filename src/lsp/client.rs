use super::{Diagnostic, LSPNotification, LSPRequest, LSPResult, Response};
use crate::{
    configs::FileType,
    lsp::LSPError,
    syntax::DiagnosticLine,
    workspace::{actions::LSPEvent, CursorPosition},
};
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
use tokio::{
    io::AsyncWriteExt,
    process::ChildStdin,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

pub enum Payload {
    Direct(String),
    Sync(PathBuf, i32, Vec<LSPEvent>),
}

impl Payload {}

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
    request_counter: Rc<RefCell<i64>>,
    pub capabilities: ServerCapabilities,
}

impl LSPClient {
    pub fn new(
        mut stdin: ChildStdin,
        diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
        responses: Arc<Mutex<HashMap<i64, Response>>>,
        capabilities: ServerCapabilities,
    ) -> (JoinHandle<LSPResult<()>>, Self) {
        let (channel, mut rx) = unbounded_channel::<Payload>();

        let position_map = match capabilities.position_encoding.as_ref().map(|inner| inner.as_str()) {
            Some("utf-8") => pos_utf8,
            Some("utf-32") => pos_utf32,
            _ => pos_utf16,
        };

        // starting send handler
        let lsp_send_handler = tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Payload::Direct(msg) => stdin.write_all(msg.as_bytes()).await?,
                    Payload::Sync(path, version, events) => {
                        let msg = LSPNotification::<DidSaveTextDocument>::file_did_change(
                            &path,
                            version,
                            events.into_iter().map(position_map).collect(),
                        )
                        .stringify()?;
                        stdin.write_all(msg.as_bytes()).await?;
                    }
                }
                stdin.flush().await?;
            }
            Ok(())
        });
        (lsp_send_handler, Self { diagnostics, responses, channel, request_counter: Rc::default(), capabilities })
    }

    pub fn placeholder() -> Self {
        let (channel, _) = tokio::sync::mpsc::unbounded_channel::<Payload>();
        Self {
            diagnostics: Arc::default(),
            responses: Arc::default(),
            channel,
            request_counter: Rc::default(),
            capabilities: ServerCapabilities::default(),
        }
    }

    #[inline]
    pub fn request<T>(&mut self, mut request: LSPRequest<T>) -> Result<i64, LSPError>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_id();
        request.id = id;
        self.channel.send(request.stringify()?.into())?;
        Ok(id)
    }

    #[inline]
    pub fn notify<T>(&mut self, notification: LSPNotification<T>) -> Result<(), LSPError>
    where
        T: lsp_types::notification::Notification,
        T::Params: serde::Serialize,
    {
        self.channel.send(notification.stringify()?.into())?;
        Ok(())
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
    pub fn request_partial_tokens(&mut self, path: &Path, range: Range) -> LSPResult<i64> {
        self.request(LSPRequest::<SemanticTokensRangeRequest>::semantics_range(path, range)?)
    }

    #[inline]
    pub fn request_full_tokens(&mut self, path: &Path) -> LSPResult<i64> {
        self.request(LSPRequest::<SemanticTokensFullRequest>::semantics_full(path)?)
    }

    #[inline]
    pub fn request_completions(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<Completion>::completion(path, c)?)
    }

    pub fn request_rename(&mut self, path: &Path, c: CursorPosition, new_name: String) -> LSPResult<i64> {
        self.request(LSPRequest::<Rename>::rename(path, c, new_name)?)
    }

    pub fn request_signitures(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<SignatureHelpRequest>::signature_help(path, c)?)
    }

    pub fn request_hover(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<HoverRequest>::hover(path, c)?)
    }

    pub fn request_references(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<References>::references(path, c)?)
    }

    pub fn request_declarations(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<GotoDeclaration>::declaration(path, c)?)
    }

    #[allow(dead_code)]
    pub fn request_definitions(&mut self, path: &Path, c: CursorPosition) -> LSPResult<i64> {
        self.request(LSPRequest::<GotoDefinition>::definition(path, c)?)
    }

    pub fn file_did_open(&mut self, path: &Path, file_type: FileType, content: String) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidOpenTextDocument>::file_did_open(path, file_type, content);
        self.notify(notification)?;
        Ok(())
    }

    pub fn sync(&mut self, path: PathBuf, version: i32, events: Vec<LSPEvent>) -> Result<(), LSPError> {
        self.channel.send(Payload::Sync(path, version, events))?;
        Ok(())
    }

    #[inline]
    pub fn file_did_change(
        &mut self,
        path: &Path,
        version: i32,
        change_events: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidChangeTextDocument>::file_did_change(path, version, change_events);
        self.notify(notification)
    }

    pub fn file_did_save(&mut self, path: &Path) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidSaveTextDocument>::file_did_save(path)?;
        self.notify(notification)
    }

    pub fn file_did_close(&mut self, path: &Path) -> Result<(), LSPError> {
        let notification = LSPNotification::<DidCloseTextDocument>::file_did_close(path);
        self.notify(notification)
    }

    #[inline]
    fn next_id(&mut self) -> i64 {
        let mut id = self.request_counter.borrow_mut();
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

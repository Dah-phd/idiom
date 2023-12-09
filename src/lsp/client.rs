use super::{Diagnostic, LSPNotification, LSPRequest, Response};
use crate::configs::FileType;

use anyhow::Result;
use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument},
    request::{SemanticTokensFullRequest, SemanticTokensRangeRequest},
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentContentChangeEvent,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Debug)]
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
        Self { diagnostics, responses, channel, request_counter: Rc::new(RefCell::new(0)), capabilities }
    }

    pub fn request<T>(&mut self, mut request: LSPRequest<T>) -> Option<i64>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_id();
        request.id = id;
        if self.channel.send(request.stringify().ok()?).is_ok() {
            return Some(id);
        }
        None
    }

    pub fn notify<T>(&mut self, notification: LSPNotification<T>) -> Result<()>
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

    pub fn partial_tokens(&mut self, path: &Path, range: Range) -> Option<i64> {
        self.capabilities.semantic_tokens_provider.as_ref()?;
        self.request(LSPRequest::<SemanticTokensRangeRequest>::semantics_range(path, range)?)
    }

    pub fn full_tokens(&mut self, path: &Path) -> Option<i64> {
        self.capabilities.semantic_tokens_provider.as_ref()?;
        self.request(LSPRequest::<SemanticTokensFullRequest>::semantics_full(path)?)
    }

    pub fn file_did_open(&mut self, path: &Path, file_type: &FileType, content: String) -> Result<()> {
        let notification = LSPNotification::<DidOpenTextDocument>::file_did_open(path, file_type, content)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_change(
        &mut self,
        path: &Path,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let notification = LSPNotification::<DidChangeTextDocument>::file_did_change(path, version, content_changes)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_save(&mut self, path: &Path) -> Result<()> {
        let notification = LSPNotification::<DidSaveTextDocument>::file_did_save(path)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn file_did_close(&mut self, path: &Path) -> Result<()> {
        let notification = LSPNotification::<DidCloseTextDocument>::file_did_close(path)?;
        self.notify(notification)?;
        Ok(())
    }

    pub fn get_diagnostics(&self, doctument: &Path) -> Option<PublishDiagnosticsParams> {
        self.diagnostics.try_lock().ok()?.get_mut(doctument)?.take()
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

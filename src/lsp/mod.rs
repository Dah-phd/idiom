mod builder;
mod client;
mod error;
mod local;
mod lsp_stream;
mod messages;
mod notification;
mod payload;
mod request;
pub mod servers;
pub use client::LSPClient;
pub use error::{LSPError, LSPResult};
pub use local::{Highlighter, init_local_tokens};
pub use messages::{Diagnostic, DiagnosticType, EditorDiagnostics, LSPResponse, LSPResponseType, TreeDiagnostics};
pub use notification::LSPNotification;
pub use request::LSPRequest;

use lsp_types::Uri;
use std::{collections::HashMap, path::Path, str::FromStr, sync::Mutex};
use tokio::{process::Child, task::JoinHandle};

pub type Responses = Mutex<HashMap<i64, LSPResponse>>;
pub type Requests = Mutex<HashMap<i64, LSPResponseType>>;

#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    _inner: Child,
    client: LSPClient,
    lsp_json_handler: JoinHandle<LSPResult<()>>,
    lsp_send_handler: JoinHandle<LSPResult<()>>,
    attempts: u8,
}

impl LSP {
    pub fn aquire_client(&self) -> LSPClient {
        self.client.clone()
    }

    #[allow(dead_code)]
    pub fn borrow_client(&self) -> &LSPClient {
        &self.client
    }

    pub fn is_running(&self) -> bool {
        !self.lsp_json_handler.is_finished() && !self.lsp_send_handler.is_finished()
    }
}

impl Drop for LSP {
    fn drop(&mut self) {
        self.lsp_send_handler.abort();
        self.lsp_json_handler.abort();
    }
}

#[inline(always)]
pub fn as_url(path: &Path) -> Uri {
    Uri::from_str(format!("file://{}", path.display()).as_str()).expect("Path should always be parsable!")
}

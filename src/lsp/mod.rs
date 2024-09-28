mod client;
mod error;
mod local;
mod lsp_stream;
mod messages;
mod notification;
mod payload;
mod request;
mod servers;
use crate::configs::FileType;
use crate::utils::split_arc;
pub use client::LSPClient;
pub use error::{LSPError, LSPResult};
pub use local::init_local_tokens;
use lsp_stream::JsonRCP;
pub use messages::{
    Diagnostic, DiagnosticHandle, DiagnosticType, EditorDiagnostics, LSPMessage, LSPResponse, LSPResponseType,
    Response, TreeDiagnostics,
};
pub use notification::LSPNotification;
pub use request::LSPRequest;

use lsp_types::{request::Initialize, InitializeResult, Uri};
use serde_json::from_value;
use std::{collections::HashMap, path::Path, process::Stdio, str::FromStr, sync::Mutex};
use tokio::{io::AsyncWriteExt, process::Child, task::JoinHandle};

pub type Responses = Mutex<HashMap<i64, Response>>;

#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    lsp_cmd: String,
    inner: Child,
    client: LSPClient,
    lsp_json_handler: JoinHandle<LSPResult<()>>,
    lsp_send_handler: JoinHandle<LSPResult<()>>,
    attempts: usize,
}

impl LSP {
    pub async fn new(lsp_cmd: String, file_type: FileType) -> LSPResult<Self> {
        let mut server = servers::server_cmd(&lsp_cmd)?;
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = JsonRCP::new(&mut inner)?;
        let mut stdin =
            inner.stdin.take().ok_or(LSPError::InternalError("Failed to take stdin of JsonRCP (LSP)".to_owned()))?;

        // setting up storage
        let (responses, responses_handler) = split_arc::<Responses>();
        let (diagnostics, diagnostics_handler) = split_arc::<Mutex<DiagnosticHandle>>();

        // sending init requests
        stdin.write_all(LSPRequest::<Initialize>::init_request()?.stringify()?.as_bytes()).await?;
        stdin.flush().await?;
        let mut init_response = json_rpc.next::<LSPMessage>().await?;
        while !matches!(init_response, LSPMessage::Response(..)) {
            init_response = json_rpc.next().await?;
        }
        let capabilities = from_value::<InitializeResult>(init_response.unwrap()?)?.capabilities;

        // starting response handler
        let lsp_json_handler = tokio::task::spawn(async move {
            loop {
                match json_rpc.next().await? {
                    LSPMessage::Response(inner) => {
                        responses_handler.lock().unwrap().insert(inner.id, inner);
                    }
                    LSPMessage::Diagnostic(uri, params) => {
                        diagnostics_handler.lock().unwrap().insert(uri, params);
                    }
                    LSPMessage::Request(_inner) => {
                        // TODO: investigate handle
                        // requests_handler.lock().await.push(inner)
                    }
                    LSPMessage::Error(_err) => {
                        // TODO: investigate handle
                    }
                    LSPMessage::Unknown(_obj) => {
                        // TODO: investigate handle
                    }
                }
            }
        });

        let (lsp_send_handler, client) = LSPClient::new(stdin, file_type, diagnostics, responses, capabilities)?;

        Ok(Self { client, lsp_cmd, inner, lsp_json_handler, lsp_send_handler, attempts: 5 })
    }

    pub async fn check_status(&mut self, file_type: FileType) -> LSPResult<Option<LSPError>> {
        if self.lsp_json_handler.is_finished() || self.lsp_send_handler.is_finished() {
            if self.attempts == 0 {
                return Err(LSPError::internal("Json RCP unable to recover after 5 attempts!"));
            }
            match Self::new(self.lsp_cmd.to_owned(), file_type).await {
                Ok(lsp) => {
                    let mut broken = std::mem::replace(self, lsp);
                    let _ = broken.dash_nine().await; // ensure old lsp is dead!
                    return Ok(Some(match broken.lsp_json_handler.await {
                        Ok(Err(err)) => err,
                        Ok(Ok(..)) => LSPError::internal("Json RCP handler returned unexpectedly!"),
                        Err(join_err) => LSPError::internal(format!("Json RCP handler join failed: {join_err}")),
                    }));
                }
                Err(err) => {
                    self.attempts -= 1;
                    return Err(err);
                }
            };
        }
        Ok(None)
    }

    pub fn aquire_client(&self) -> LSPClient {
        self.client.clone()
    }

    #[allow(dead_code)]
    pub fn borrow_client(&self) -> &LSPClient {
        &self.client
    }

    pub async fn graceful_exit(&mut self) -> LSPResult<()> {
        self.client.stop();
        self.dash_nine().await?;
        Ok(())
    }

    async fn dash_nine(&mut self) -> LSPResult<()> {
        self.lsp_json_handler.abort();
        self.lsp_send_handler.abort();
        self.inner.kill().await?;
        Ok(())
    }
}

#[inline(always)]
pub fn as_url(path: &Path) -> Uri {
    Uri::from_str(format!("file://{}", path.display()).as_str()).expect("Path should always be parsable!")
}

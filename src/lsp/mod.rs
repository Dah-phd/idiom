mod client;
mod error;
mod lsp_stream;
mod messages;
mod notification;
mod request;
mod servers;
use crate::utils::{into_guard, split_arc_mutex, split_arc_mutex_async};
pub use client::LSPClient;
pub use error::{LSPError, LSPResult};
use lsp_stream::JsonRCP;
pub use messages::{Diagnostic, GeneralNotification, LSPMessage, LSPResponse, LSPResponseType, Request, Response};
pub use notification::LSPNotification;
pub use request::LSPRequest;
use url::Url;

use lsp_types::{
    notification::{Exit, Initialized},
    request::{Initialize, Shutdown},
    InitializeResult, InitializedParams,
};
use serde_json::from_value;
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::{io::AsyncWriteExt, process::Child, sync::mpsc, task::JoinHandle};

#[cfg(build = "debug")]
use crate::utils::debug_to_file;

#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    pub notifications: Arc<Mutex<Vec<GeneralNotification>>>,
    pub requests: Arc<tokio::sync::Mutex<Vec<Request>>>,
    lsp_cmd: String,
    inner: Child,
    client: LSPClient,
    lsp_json_handler: JoinHandle<LSPResult<()>>,
    lsp_send_handler: JoinHandle<LSPResult<()>>,
    attempts: usize,
}

impl LSP {
    pub async fn new(lsp_cmd: String) -> LSPResult<Self> {
        let mut server = servers::server_cmd(&lsp_cmd)?;
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = JsonRCP::new(&mut inner)?;
        let mut stdin =
            inner.stdin.take().ok_or(LSPError::InternalError("Failed to take stdin of JsonRCP (LSP)".to_owned()))?;

        // setting up storage
        let (responses, responses_handler) = split_arc_mutex(HashMap::new());
        let (notifications, notifications_handler) = split_arc_mutex(Vec::new());
        let (requests, requests_handler) = split_arc_mutex_async(Vec::new());
        let (diagnostics, diagnostics_handler) = split_arc_mutex(HashMap::new());

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
                let msg = json_rpc.next().await?;
                match msg {
                    LSPMessage::Response(inner) => {
                        into_guard(&responses_handler).insert(inner.id, inner);
                    }
                    LSPMessage::Notification(inner) => into_guard(&notifications_handler).push(inner),
                    LSPMessage::Diagnostic(uri, params) => {
                        into_guard(&diagnostics_handler).insert(uri, params);
                    }
                    LSPMessage::Request(inner) => {
                        #[cfg(build = "debug")]
                        debug_to_file("test_data.lsp_request", inner.to_string());
                        requests_handler.lock().await.push(inner)
                    }
                    LSPMessage::Error(_err) => {
                        #[cfg(build = "debug")]
                        debug_to_file("test_data.lsp_err", _err.to_string());
                        // TODO: investigate handle
                    }
                    LSPMessage::Unknown(_obj) => {
                        #[cfg(build = "debug")]
                        debug_to_file("test_data.lsp_unknown", _obj.to_string());
                        // TODO: investigate handle
                    }
                }
            }
        });

        // starting sending channel
        let (channel, mut rx) = mpsc::unbounded_channel::<String>();

        // starting send handler
        let lsp_send_handler = tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                stdin.write_all(msg.as_bytes()).await?;
                stdin.flush().await?;
            }
            Ok(())
        });

        let mut lsp = Self {
            notifications,
            requests,
            client: LSPClient::new(diagnostics, responses, channel, capabilities),
            lsp_cmd,
            inner,
            lsp_json_handler,
            lsp_send_handler,
            attempts: 5,
        };

        //initialized
        lsp.initialized().await?;
        Ok(lsp)
    }

    pub async fn check_status(&mut self) -> LSPResult<Option<LSPError>> {
        if self.lsp_json_handler.is_finished() || self.lsp_send_handler.is_finished() {
            if self.attempts == 0 {
                return Err(LSPError::internal("Json RCP unable to recover after 5 attempts!"));
            }
            match Self::new(self.lsp_cmd.to_owned()).await {
                Ok(lsp) => {
                    #[cfg(build = "debug")]
                    debug_to_file("test_data.restart", self.attempts);
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

    async fn initialized(&mut self) -> LSPResult<()> {
        let notification: LSPNotification<Initialized> = LSPNotification::with(InitializedParams {});
        self.client.notify(notification)?;
        Ok(())
    }

    pub fn aquire_client(&self) -> LSPClient {
        self.client.clone()
    }

    pub fn borrow_client(&self) -> &LSPClient {
        &self.client
    }

    pub async fn graceful_exit(&mut self) -> LSPResult<()> {
        let shoutdown_request: LSPRequest<Shutdown> = LSPRequest::with(0, ());
        let _ = self.client.request(shoutdown_request);
        let notification: LSPNotification<Exit> = LSPNotification::with(());
        let _ = self.client.notify(notification);
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
fn as_url(path: &Path) -> Result<Url, url::ParseError> {
    Url::parse(&format!("file:///{}", path.display()))
}

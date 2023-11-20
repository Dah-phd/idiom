mod lsp_stream;
mod messages;
mod notification;
mod python;
mod request;
mod rust;
use crate::configs::FileType;
use crate::utils::{into_guard, split_arc_mutex, split_arc_mutex_async};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit, Initialized,
};
use lsp_types::request::{Initialize, Shutdown};
use serde_json::from_value;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

use anyhow::{anyhow, Error, Result};

use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeResult, InitializedParams, PublishDiagnosticsParams, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, Url, VersionedTextDocumentIdentifier,
};

use lsp_stream::LSPMessageStream;
use messages::done_auto_response;
pub use messages::{Diagnostic, GeneralNotification, LSPMessage, Request, Response};
use notification::LSPNotification;
pub use request::LSPRequest;

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct LSP {
    pub responses: Arc<Mutex<HashMap<i64, Response>>>,
    pub notifications: Arc<Mutex<Vec<GeneralNotification>>>,
    pub requests: Arc<tokio::sync::Mutex<Vec<Request>>>,
    pub diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
    pub initialized: InitializeResult,
    lsp_err_msg: Arc<Mutex<Vec<String>>>,
    file_type: FileType,
    counter: i64,
    inner: Child,
    lsp_json_handler: JoinHandle<Error>,
    lsp_send_handler: JoinHandle<Result<()>>,
    lsp_send_channel: Sender<String>,
    attempts: usize,
}

impl LSP {
    pub async fn from(file_type: &FileType) -> Result<Self> {
        match file_type {
            FileType::Rust => Self::new(rust::start_lsp(), *file_type).await,
            FileType::Python => Self::new(python::start_lsp(), *file_type).await,
            _ => Err(anyhow!("Not supported LSP!")),
        }
    }

    async fn new(mut server: Command, language: FileType) -> Result<Self> {
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = LSPMessageStream::new(&mut inner)?;
        let mut stdin = inner.stdin.take().ok_or(anyhow!("LSP stdin"))?;

        // setting up storage
        let (responses, responses_handler) = split_arc_mutex(HashMap::new());
        let (notifications, notifications_handler) = split_arc_mutex(Vec::new());
        let (requests, requests_handler) = split_arc_mutex_async(Vec::new());
        let (diagnostics, diagnostics_handler) = split_arc_mutex(HashMap::new());

        // sending init requests
        stdin.write_all(LSPRequest::<Initialize>::init_request()?.stringify()?.as_bytes()).await?;
        stdin.flush().await?;
        let msg = json_rpc.next().await?.unwrap()?;
        let initialized: InitializeResult = from_value(msg)?;
        let lsp_err_msg = json_rpc.get_errors();

        // starting response handler
        let lsp_json_handler = tokio::task::spawn(async move {
            loop {
                match json_rpc.next().await {
                    Ok(msg) => {
                        match msg {
                            LSPMessage::Response(inner) => {
                                into_guard(&responses_handler).insert(inner.id, inner);
                            }
                            LSPMessage::Notification(inner) => into_guard(&notifications_handler).push(inner),
                            LSPMessage::Diagnostic(uri, params) => {
                                into_guard(&diagnostics_handler).insert(uri, params);
                            }
                            LSPMessage::Request(inner) => requests_handler.lock().await.push(inner),
                            _ => (), //devnull
                        }
                    }
                    Err(err) => {
                        return err;
                    }
                }
            }
        });

        // starting sending channel
        let (lsp_send_channel, mut rx) = mpsc::channel::<String>(100);

        // starting send handler
        let lsp_send_handler = tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                stdin.write_all(msg.as_bytes()).await?;
                stdin.flush().await?;
            }
            Ok(())
        });

        let mut lsp = Self {
            responses,
            lsp_err_msg,
            notifications,
            requests,
            diagnostics,
            counter: 0,
            file_type: language,
            inner,
            lsp_json_handler,
            lsp_send_handler,
            lsp_send_channel,
            initialized,
            attempts: 5,
        };

        //initialized
        lsp.initialized().await?;
        Ok(lsp)
    }

    pub async fn check_status(&mut self) -> Result<Option<Error>> {
        if self.lsp_json_handler.is_finished() {
            if self.attempts == 0 {
                return Err(anyhow!("Unable to recover!"));
            }
            match Self::from(&self.file_type).await {
                Ok(lsp) => {
                    let broken = std::mem::replace(self, lsp);
                    return Ok(Some(match broken.lsp_json_handler.await {
                        Ok(err) => err,
                        Err(join_err) => anyhow!("Failed to collect crash report! Join err {}", join_err.to_string()),
                    }));
                }
                Err(err) => {
                    self.attempts -= 1;
                    return Err(anyhow!("LSP creashed! Failed to rebuild LSP! {}", err.to_string()));
                }
            };
        }
        Ok(None)
    }

    pub fn get_diagnostics(&self, doctument: &Path) -> Option<PublishDiagnosticsParams> {
        self.diagnostics.try_lock().ok()?.get_mut(doctument)?.take()
    }

    pub async fn auto_responde(&mut self) {
        let mut requests = self.requests.lock().await;
        if requests.is_empty() {
            return;
        }
        let mut keep = Vec::new();
        for request in requests.iter_mut() {
            keep.push(!done_auto_response(request, &mut self.lsp_send_channel).await);
        }
        requests.retain(|_| keep.remove(0));
    }

    pub fn get(&self, id: &i64) -> Option<Response> {
        let mut que = self.responses.try_lock().ok()?;
        que.remove(id)
    }

    async fn initialized(&mut self) -> Result<()> {
        self.notify::<Initialized>(LSPNotification::with(InitializedParams {})).await
    }

    pub fn aquire_channel(&mut self) -> Sender<String> {
        self.lsp_send_channel.clone()
    }

    pub async fn request<T>(&mut self, mut request: LSPRequest<T>) -> Result<i64>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        self.counter += 1;
        request.id = self.counter;
        self.lsp_send_channel.send(request.stringify()?).await?;
        Ok(self.counter)
    }

    pub async fn response<T>(&mut self, response: LSPRequest<T>) -> Result<()>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        self.lsp_send_channel.send(response.stringify()?).await?;
        Ok(())
    }

    async fn notify<T>(&mut self, notification: LSPNotification<T>) -> Result<()>
    where
        T: lsp_types::notification::Notification,
        T::Params: serde::Serialize,
    {
        self.lsp_send_channel.send(notification.stringify()?).await?;
        Ok(())
    }

    pub async fn graceful_exit(&mut self) -> Result<()> {
        self.counter += 1;
        let shoutdown_request: LSPRequest<Shutdown> = LSPRequest::with(self.counter, ());
        self.request(shoutdown_request).await?;
        self.notify::<Exit>(LSPNotification::with(())).await?;
        self.dash_nine().await?;
        Ok(())
    }

    async fn dash_nine(&mut self) -> Result<()> {
        self.lsp_json_handler.abort();
        self.lsp_send_handler.abort();
        self.inner.kill().await?;
        Ok(())
    }

    pub async fn file_did_change(
        channel: &mut Sender<String>,
        path: &Path,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let notification: LSPNotification<DidChangeTextDocument> = LSPNotification::with(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(as_url(path)?, version),
            content_changes,
        });
        channel.send(notification.stringify()?).await?;
        Ok(())
    }

    pub async fn file_did_open(channel: &mut Sender<String>, path: &PathBuf, file_type: &FileType) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let notification: LSPNotification<DidOpenTextDocument> = LSPNotification::with(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: as_url(path)?,
                language_id: String::from(file_type),
                version: 0,
                text: content,
            },
        });
        channel.send(notification.stringify()?).await?;
        Ok(())
    }

    pub async fn file_did_save(channel: &mut Sender<String>, path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let notification: LSPNotification<DidSaveTextDocument> = LSPNotification::with(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
            text: Some(content),
        });
        channel.send(notification.stringify()?).await?;
        Ok(())
    }

    pub async fn file_did_close(channel: &mut Sender<String>, path: &Path) -> Result<()> {
        let notification: LSPNotification<DidCloseTextDocument> = LSPNotification::with(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
        });
        channel.send(notification.stringify()?).await?;
        Ok(())
    }
}

fn as_url(path: &Path) -> Result<Url> {
    Ok(Url::parse(&format!("file:///{}", path.display()))?)
}

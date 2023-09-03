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
use lsp_types::request::{HoverRequest, Initialize, References, Shutdown, SignatureHelpRequest};
pub use messages::LSPMessage;
use request::LSPRequest;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use anyhow::{anyhow, Result};

use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    HoverParams, InitializedParams, PartialResultParams, Position, PublishDiagnosticsParams, ReferenceContext,
    ReferenceParams, SignatureHelpParams, TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier, WorkDoneProgressParams,
};

use self::messages::done_auto_response;
pub use self::messages::{Diagnostic, GeneralNotification, Request, Response};
use self::notification::LSPNotification;

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
pub struct LSP {
    pub responses: Arc<Mutex<HashMap<i64, Response>>>,
    pub notifications: Arc<Mutex<Vec<GeneralNotification>>>,
    pub requests: Arc<tokio::sync::Mutex<Vec<Request>>>,
    pub diagnostics: Arc<Mutex<HashMap<PathBuf, Diagnostic>>>,
    pub errs: Arc<Mutex<Vec<String>>>,
    language: FileType,
    counter: usize,
    inner: Child,
    handler: JoinHandle<()>,
    stdin: ChildStdin,
}

#[allow(dead_code)]
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

        let mut stdin = inner.stdin.take().unwrap();
        let request = LSPRequest::<Initialize>::init_request()?;
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let (responses, responses_handler) = split_arc_mutex(HashMap::new());
        let (notifications, notifications_handler) = split_arc_mutex(Vec::new());
        let (errs, errs_handler) = split_arc_mutex(Vec::new());
        let (requests, requests_handler) = split_arc_mutex_async(Vec::new());
        let (diagnostics, diagnostics_handler) = split_arc_mutex(HashMap::new());
        let handler = tokio::task::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let lsp_message = String::from_utf8_lossy(&msg);
                for msg in lsp_message.split("Content-Length") {
                    if let Some(msg) = LSPMessage::parse(msg) {
                        match msg {
                            LSPMessage::Response(inner) => {
                                into_guard(&responses_handler).insert(inner.id, inner);
                            }
                            LSPMessage::Notification(inner) => into_guard(&notifications_handler).push(inner),
                            LSPMessage::Diagnostic(uri, params) => {
                                into_guard(&diagnostics_handler).insert(uri, params);
                            }
                            LSPMessage::Request(inner) => requests_handler.lock().await.push(inner),
                        }
                    } else if !msg.is_empty() {
                        into_guard(&errs_handler).push(msg.to_owned());
                    }
                }
            }
        });
        let ser_req = request.stringify()?;
        let _ = stdin.write(ser_req.as_bytes()).await?;
        stdin.flush().await?;
        Ok(Self { responses, notifications, requests, diagnostics, errs, counter: 0, language, inner, handler, stdin })
    }

    pub fn is_live(&self) -> bool {
        !self.handler.is_finished()
    }

    pub fn get_diagnostics(&self, doctument: &Path) -> Option<PublishDiagnosticsParams> {
        self.diagnostics.try_lock().ok()?.get_mut(&doctument.canonicalize().ok()?)?.take()
    }

    pub async fn auto_responde(&mut self) {
        let mut requests = self.requests.lock().await;
        if requests.is_empty() {
            return;
        }
        let mut keep = Vec::new();
        for request in requests.iter_mut() {
            keep.push(!done_auto_response(request, &mut self.stdin).await);
        }
        requests.retain(|_| keep.remove(0));
    }

    pub fn get(&self, id: i64) -> Option<Response> {
        let mut que = self.responses.try_lock().ok()?;
        que.remove(&id)
    }

    pub async fn initialized(&mut self) -> Option<()> {
        self.notify::<Initialized>(LSPNotification::with(InitializedParams {})).await.ok()
    }

    pub async fn file_did_open(&mut self, path: &PathBuf) -> Option<()> {
        let content = std::fs::read_to_string(path).ok()?;
        let notification: LSPNotification<DidOpenTextDocument> = LSPNotification::with(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: as_url(path)?,
                language_id: String::from(&self.language),
                version: 0,
                text: content,
            },
        });
        self.notify(notification).await.ok()
    }

    pub async fn file_did_save(&mut self, path: &PathBuf) -> Option<()> {
        let content = std::fs::read_to_string(path).ok()?;
        let notification: LSPNotification<DidSaveTextDocument> = LSPNotification::with(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
            text: Some(content),
        });
        self.notify(notification).await.ok()
    }

    pub async fn file_did_change(
        &mut self,
        path: &Path,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Option<()> {
        let notification: LSPNotification<DidChangeTextDocument> = LSPNotification::with(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(as_url(path)?, version),
            content_changes,
        });
        self.notify(notification).await.ok()
    }

    pub async fn file_did_close(&mut self, path: &Path) -> Option<()> {
        let notification: LSPNotification<DidCloseTextDocument> = LSPNotification::with(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: as_url(path)? },
        });
        self.notify(notification).await.ok()
    }

    pub async fn request_references(&mut self, path: &Path, line: u32, char: u32) -> Option<usize> {
        let request: LSPRequest<References> = LSPRequest::with(
            0,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                context: ReferenceContext { include_declaration: true },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        );
        self.request(request).await
    }

    pub async fn request_hover(&mut self, path: &Path, line: u32, char: u32) -> Option<usize> {
        let request: LSPRequest<HoverRequest> = LSPRequest::with(
            self.counter,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        );
        self.request(request).await
    }

    pub async fn request_signiture_help(&mut self, path: &Path, line: u32, char: u32) -> Option<usize> {
        let request: LSPRequest<SignatureHelpRequest> = LSPRequest::with(
            self.counter,
            SignatureHelpParams {
                context: None,
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new(as_url(path)?),
                    position: Position::new(line, char),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        );
        self.request(request).await
    }

    async fn request<T>(&mut self, mut request: LSPRequest<T>) -> Option<usize>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        self.counter += 1;
        request.id = self.counter;
        if let Ok(message) = request.stringify() {
            if self.stdin.write(message.as_bytes()).await.is_ok() && self.stdin.flush().await.is_ok() {
                return Some(self.counter);
            }
        }
        self.counter -= 1;
        None
    }

    pub async fn response<T>(&mut self, response: LSPRequest<T>) -> Result<()>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        let message = response.stringify()?;
        let _ = self.stdin.write(message.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn notify<T>(&mut self, notification: LSPNotification<T>) -> Result<()>
    where
        T: lsp_types::notification::Notification,
        T::Params: serde::Serialize,
    {
        let message = notification.stringify()?;
        let _ = self.stdin.write(message.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn graceful_exit(&mut self) -> Result<()> {
        self.counter += 1;
        let shoutdown_request: LSPRequest<Shutdown> = LSPRequest::with(self.counter, ());
        self.request(shoutdown_request).await.ok_or(anyhow!("Failed to notify shoutdown"))?;
        self.notify::<Exit>(LSPNotification::with(())).await?;
        self.dash_nine().await?;
        Ok(())
    }

    async fn dash_nine(&mut self) -> Result<()> {
        self.handler.abort();
        self.inner.kill().await?;
        Ok(())
    }
}

fn as_url(path: &Path) -> Option<Url> {
    Url::parse(&format!("file:///{}", path.as_os_str().to_str()?)).ok()
}

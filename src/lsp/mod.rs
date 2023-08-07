mod messages;
mod notification;
mod python;
mod request;
mod rust;
use crate::messages::FileType;
use lsp_types::notification::{DidOpenTextDocument, Exit, Initialized};
use lsp_types::request::{HoverRequest, Initialize, References, Shutdown, SignatureHelpRequest};
use messages::LSPMessage;
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

use lsp_types::Url;
use lsp_types::{
    DidOpenTextDocumentParams, HoverParams, InitializedParams, NumberOrString, PartialResultParams, Position,
    ReferenceContext, ReferenceParams, SignatureHelpParams, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, WorkDoneProgressParams,
};

use self::messages::done_auto_response;
use self::notification::LSPNotification;

#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
pub struct LSP {
    pub responses: Arc<Mutex<HashMap<i64, LSPMessage>>>,
    pub notifications: Arc<Mutex<Vec<LSPMessage>>>,
    pub requests: Arc<tokio::sync::Mutex<Vec<LSPMessage>>>,
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
        let mut inner = server
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = inner.stdin.take().unwrap();
        let request = LSPRequest::<Initialize>::init_request()?;
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let (responses, responses_handler) = to_split_arc_mutex(HashMap::new());
        let (notifications, notifications_handler) = to_split_arc_mutex(Vec::new());
        let requests = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let requests_handler = Arc::clone(&requests);
        let (errs, errs_handler) = to_split_arc_mutex(Vec::new());
        let handler = tokio::task::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let lsp_message = String::from_utf8_lossy(&msg);
                for msg in lsp_message.split("Content-Length") {
                    if let Some(msg) = LSPMessage::parse(msg) {
                        match msg {
                            LSPMessage::Response { id, .. } => match responses_handler.lock() {
                                Ok(mut guard) => {
                                    guard.insert(id, msg);
                                }
                                Err(poisoned) => {
                                    poisoned.into_inner().insert(id, msg);
                                }
                            },
                            LSPMessage::ResponseErr { id, .. } => match responses_handler.lock() {
                                Ok(mut guard) => {
                                    guard.insert(id, msg);
                                }
                                Err(poisoned) => {
                                    poisoned.into_inner().insert(id, msg);
                                }
                            },
                            LSPMessage::Notification { .. } => match notifications_handler.lock() {
                                Ok(mut guard) => guard.push(msg),
                                Err(poisoned) => poisoned.into_inner().push(msg),
                            },
                            LSPMessage::Request { .. } => requests_handler.lock().await.push(msg),
                        }
                    } else if !msg.is_empty() {
                        match errs_handler.lock() {
                            Ok(mut guard) => guard.push(msg.to_owned()),
                            Err(poisoned) => poisoned.into_inner().push(msg.to_owned()),
                        }
                    }
                }
            }
        });
        let ser_req = request.stringify()?;
        let _ = stdin.write(ser_req.as_bytes()).await?;
        stdin.flush().await?;
        Ok(Self {
            responses,
            notifications,
            requests,
            errs,
            counter: 0,
            language,
            inner,
            handler,
            stdin,
        })
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

    pub fn get(&self, id: i64) -> Option<LSPMessage> {
        let mut que = self.responses.try_lock().ok()?;
        que.remove(&id)
    }

    pub async fn initialized(&mut self) -> Option<()> {
        let notification: LSPNotification<Initialized> = LSPNotification::with(InitializedParams {});
        self.notify(notification.stringify().ok()?).await.ok()
    }

    pub async fn file_did_open(&mut self, path: &PathBuf) -> Option<()> {
        let content = std::fs::read_to_string(path).ok()?;
        let notification: LSPNotification<DidOpenTextDocument> = LSPNotification::with(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&format!("file:///{}", path.as_os_str().to_str()?)).ok()?,
                language_id: String::from(&self.language),
                version: 0,
                text: content,
            },
        });
        let lsp_message = notification.stringify().ok()?;
        self.notify(lsp_message).await.ok()
    }

    pub async fn request_references(&mut self, path: &Path, line: u32, char: u32) -> Option<usize> {
        let request: LSPRequest<References> = LSPRequest::with(
            0,
            ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!("file:///{}", path.as_os_str().to_str()?)).ok()?,
                    },
                    position: Position::new(line, char),
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams {
                    work_done_token: Some(NumberOrString::String(format!("ref{}", self.counter))),
                },
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
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!("file:///{}", path.as_os_str().to_str()?)).ok()?,
                    },
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
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!("file:///{}", path.as_os_str().to_str()?)).ok()?,
                    },
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
        self.notify(message).await
    }

    async fn notify(&mut self, lsp_notification: String) -> Result<()> {
        let _ = self.stdin.write(lsp_notification.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn graceful_exit(&mut self) -> Result<()> {
        self.counter += 1;
        let shoutdown_request: LSPRequest<Shutdown> = LSPRequest::with(self.counter, ());
        self.request(shoutdown_request)
            .await
            .ok_or(anyhow!("Failed to notify shoutdown"))?;
        let exit: LSPNotification<Exit> = LSPNotification::with(());
        self.notify(exit.stringify()?).await?;
        self.dash_nine().await?;
        Ok(())
    }

    async fn dash_nine(&mut self) -> Result<()> {
        self.handler.abort();
        self.inner.kill().await?;
        Ok(())
    }
}

fn to_split_arc_mutex<T>(inner: T) -> (Arc<Mutex<T>>, Arc<Mutex<T>>) {
    let arc = Arc::new(Mutex::new(inner));
    let clone = Arc::clone(&arc);
    (arc, clone)
}

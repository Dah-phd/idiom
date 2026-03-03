mod client;
mod error;
mod local;
mod lsp_stream;
mod messages;
mod notification;
mod payload;
mod request;
pub mod servers;
use crate::{
    configs::{get_config_dir, FileType},
    utils::{split_arc, SHELL},
};
pub use client::LSPClient;
pub use error::{LSPError, LSPResult};
pub use local::{init_local_tokens, Highlighter};
use lsp_stream::JsonRPC;
pub use messages::{
    Diagnostic, DiagnosticHandle, DiagnosticType, EditorDiagnostics, LSPMessage, LSPResponse, LSPResponseType,
    TreeDiagnostics,
};
pub use notification::LSPNotification;
pub use request::LSPRequest;

use lsp_types::{request::Initialize, InitializeResult, Uri};
use serde_json::{from_value, Value};
use std::{collections::HashMap, path::Path, process::Stdio, str::FromStr, sync::Mutex};
use tokio::{io::AsyncWriteExt, process::Child, process::Command, task::JoinHandle};

pub type Responses = Mutex<HashMap<i64, LSPResponse>>;
pub type Requests = Mutex<HashMap<i64, LSPResponseType>>;

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
        let mut server = server_cmd(&lsp_cmd)?;
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = JsonRPC::new(&mut inner)?;
        let mut stdin =
            inner.stdin.take().ok_or(LSPError::InternalError(String::from("Failed to take stdin of JsonRCP (LSP)")))?;

        // setting up storage
        let (responses, responses_handler) = split_arc::<Responses>();
        let (sent_request, sent_handler) = split_arc::<Requests>();
        let (diagnostics, diagnostics_handler) = split_arc::<Mutex<DiagnosticHandle>>();

        // sending init requests
        stdin.write_all(LSPRequest::<Initialize>::init_request()?.stringify()?.as_bytes()).await?;
        stdin.flush().await?;

        let init_response = skip_to_response(&mut json_rpc).await?;
        let capabilities = from_value::<InitializeResult>(init_response)?.capabilities;

        // starting response handler
        let lsp_json_handler = tokio::task::spawn(async move {
            loop {
                match json_rpc.next().await? {
                    LSPMessage::Response(inner) => {
                        let Some(resp_type) = sent_handler.lock().unwrap().remove(&inner.id) else {
                            continue;
                        };
                        if let Some(response) = inner.result {
                            let response = match resp_type.parse(response) {
                                Ok(response) => response,
                                Err(error) => LSPResponse::Error(format!("LSP PARSE({resp_type:?}): {error}")),
                            };
                            responses_handler.lock().unwrap().insert(inner.id, response);
                        } else if let Some(error) = inner.error {
                            let response = match resp_type {
                                LSPResponseType::Tokens => LSPResponse::Tokens(Err(error.to_string())),
                                // value was modified before returning range
                                // could cause artefacts - F5 refreshes all
                                LSPResponseType::TokensPartial { .. }
                                    if LSPResponse::err_msg_contains(&error, "content modified") =>
                                {
                                    LSPResponse::Empty
                                }
                                _ => LSPResponse::Error(format!("{resp_type:?}: {error}")),
                            };
                            responses_handler.lock().unwrap().insert(inner.id, response);
                        }
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

        let (lsp_send_handler, client) =
            LSPClient::new(stdin, file_type, diagnostics, sent_request, responses, capabilities)?;

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

    pub fn is_running(&self) -> bool {
        !self.lsp_json_handler.is_finished() && !self.lsp_send_handler.is_finished()
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

pub fn server_cmd(lsp: &str) -> LSPResult<Command> {
    if lsp.contains("${cfg_dir}") {
        let cfg_dir = get_config_dir().ok_or(LSPError::internal("Failed to find config dir!"))?.display().to_string();
        let mut cmd = Command::new(SHELL);
        cmd.arg("-c").arg(lsp.replace("${cfg_dir}", cfg_dir.as_str()));
        return Ok(cmd);
    }
    let mut cmd = Command::new(SHELL);
    cmd.arg("-c").arg(lsp);
    Ok(cmd)
}

/// get the Value representation of the response or error
#[inline]
async fn skip_to_response(rpc: &mut JsonRPC) -> LSPResult<Value> {
    loop {
        let LSPMessage::Response(resp) = rpc.next::<LSPMessage>().await? else {
            continue;
        };
        return match resp.result {
            Some(result) => Ok(result),
            None => Err(LSPError::ResponseError(format!("{:?}", resp.error))),
        };
    }
}

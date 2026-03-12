use super::error::LSPError;
use super::lsp_stream::JsonRPC;
pub use super::messages::{DiagnosticHandle, LSPMessage, LSPResponse, LSPResponseType};
pub use super::request::LSPRequest;
use super::{LSPClient, LSPResult, Requests, Responses, LSP};
use crate::{
    configs::{get_config_dir, FileType},
    utils::{split_arc, SHELL},
};
use lsp_types::{request::Initialize, InitializeResult, ServerCapabilities};
use serde_json::from_value;
use serde_json::Value;
use std::{process::Stdio, sync::Mutex};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
};

/// holds all components needed to build LSP before creating of the client and main loop
/// namely the server command has started successfully (some servers can take their time)
pub struct LSPBuilder {
    json_rpc: JsonRPC,
    lsp_cmd: String,
    file_type: FileType,
    capabilities: ServerCapabilities,
    inner: Child,
    stdin: ChildStdin,
    attempt: Option<u8>,
}

impl LSPBuilder {
    pub async fn new(lsp_cmd: String, file_type: FileType) -> LSPResult<LSPBuilder> {
        let mut server = server_cmd_kill_on_drop(&lsp_cmd)?;
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = JsonRPC::new(&mut inner)?;
        let mut stdin =
            inner.stdin.take().ok_or(LSPError::InternalError(String::from("Failed to take stdin of JsonRCP (LSP)")))?;

        // sending init requests
        stdin.write_all(LSPRequest::<Initialize>::init_request()?.stringify()?.as_bytes()).await?;
        stdin.flush().await?;

        let init_response = skip_to_response(&mut json_rpc).await?;
        let capabilities = from_value::<InitializeResult>(init_response)?.capabilities;
        Ok(LSPBuilder { json_rpc, lsp_cmd, file_type, capabilities, inner, stdin, attempt: None })
    }

    pub async fn new_attempt(lsp_cmd: String, file_type: FileType, attempt: Option<u8>) -> LSPResult<LSPBuilder> {
        let mut builder = Self::new(lsp_cmd, file_type).await?;
        if let Some(attempt) = attempt {
            builder.attempt(attempt);
        }
        Ok(builder)
    }

    pub fn attempt(&mut self, attempt: u8) -> &mut Self {
        self.attempt = Some(attempt);
        self
    }

    pub fn spawn(self) -> LSPResult<LSP> {
        let LSPBuilder { mut json_rpc, lsp_cmd, file_type, capabilities, inner, stdin, attempt } = self;

        // setting up storage
        let (responses, responses_handler) = split_arc::<Responses>();
        let (sent_request, sent_handler) = split_arc::<Requests>();
        let (diagnostics, diagnostics_handler) = split_arc::<Mutex<DiagnosticHandle>>();

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

        Ok(LSP { client, lsp_cmd, _inner: inner, lsp_json_handler, lsp_send_handler, attempts: attempt.unwrap_or(5) })
    }
}

pub fn server_cmd_kill_on_drop(lsp: &str) -> LSPResult<Command> {
    let mut cmd = Command::new(SHELL);
    cmd.kill_on_drop(true).arg("-c");
    if lsp.contains("${cfg_dir}") {
        let cfg_dir = get_config_dir().ok_or(LSPError::internal("Failed to find config dir!"))?.display().to_string();
        cmd.arg(lsp.replace("${cfg_dir}", cfg_dir.as_str()));
    } else {
        cmd.arg(lsp);
    }
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

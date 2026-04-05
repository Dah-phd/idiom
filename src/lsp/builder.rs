use super::error::LSPError;
use super::lsp_stream::JsonRPC;
pub use super::messages::{DiagnosticHandle, LSPMessage, LSPResponse, LSPResponseType};
pub use super::request::LSPRequest;
use super::{LSP, LSPClient, LSPResult, Requests, Responses};
use crate::{
    app::ASYNC_RT,
    configs::{FileType, get_config_dir},
    utils::{SHELL, split_arc},
};
use lsp_types::{InitializeResult, ServerCapabilities};
use serde_json::from_value;
use serde_json::{Map, Value};
use std::{
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
    task::JoinHandle,
};

pub type InitCfg = Option<Map<String, Value>>;

/// holds all components needed to build LSP before creating of the client and main loop
/// namely the server command has started successfully (some servers can take their time)
pub struct LSPBuilder {
    inner: Child,
    stdin: ChildStdin,
    attempt: Option<u8>,
    file_type: FileType,
    capabilities: ServerCapabilities,
    sent_request: Arc<Requests>,
    responses: Arc<Responses>,
    diagnostics: Arc<Mutex<DiagnosticHandle>>,
    lsp_json_handler: JoinHandle<LSPResult<()>>,
}

impl LSPBuilder {
    pub async fn init_lsp(lsp_cmd: String, init_cfg: InitCfg, file_type: FileType) -> LSPResult<LSPBuilder> {
        let mut server = server_cmd_kill_on_drop(&lsp_cmd)?;
        let mut inner = server.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::piped()).spawn()?;

        // splitting subprocess
        let mut json_rpc = JsonRPC::tokio_rt_new(&mut inner)?;
        let mut stdin =
            inner.stdin.take().ok_or(LSPError::InternalError(String::from("Failed to take stdin of JsonRCP (LSP)")))?;

        // init request
        let disable_tokens = are_semantics_disabled(&init_cfg);
        let init = match init_cfg {
            Some(init_cfg) => LSPRequest::init_request_with_mods(init_cfg)?,
            None => LSPRequest::init_request()?,
        };

        // sending init requests
        stdin.write_all(init.stringify()?.as_bytes()).await?;
        stdin.flush().await?;

        let init_response = skip_to_response(&mut json_rpc).await?;
        let mut capabilities = from_value::<InitializeResult>(init_response)?.capabilities;

        // disable tokens if init_cfg holds logic to remove capability
        if disable_tokens {
            capabilities.semantic_tokens_provider.take();
        }

        // setting up storage
        let (responses, responses_handler) = split_arc::<Responses>();
        let (sent_request, sent_handler) = split_arc::<Requests>();
        let (diagnostics, diagnostics_handler) = split_arc::<Mutex<DiagnosticHandle>>();

        Ok(LSPBuilder {
            file_type,
            capabilities,
            inner,
            stdin,
            attempt: None,
            sent_request,
            responses,
            diagnostics,
            lsp_json_handler: ASYNC_RT.spawn(json_rpc_loop(
                json_rpc,
                sent_handler,
                responses_handler,
                diagnostics_handler,
            )),
        })
    }

    pub async fn new_attempt(
        lsp_cmd: String,
        init_cfg: InitCfg,
        file_type: FileType,
        attempt: Option<u8>,
    ) -> LSPResult<LSPBuilder> {
        let mut builder = Self::init_lsp(lsp_cmd, init_cfg, file_type).await?;
        if let Some(attempt) = attempt {
            builder.attempt(attempt);
        }
        Ok(builder)
    }

    pub fn attempt(&mut self, attempt: u8) -> &mut Self {
        self.attempt = Some(attempt);
        self
    }

    pub fn finish(self) -> LSPResult<LSP> {
        let LSPBuilder {
            file_type,
            capabilities,
            inner,
            stdin,
            attempt,
            sent_request,
            responses,
            diagnostics,
            lsp_json_handler,
        } = self;

        let (lsp_send_handler, client) =
            LSPClient::new(stdin, file_type, diagnostics, sent_request, responses, capabilities)?;

        Ok(LSP { client, _inner: inner, lsp_json_handler, lsp_send_handler, attempts: attempt.unwrap_or(5) })
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

#[inline]
async fn json_rpc_loop(
    mut json_rpc: JsonRPC,
    sent_handler: Arc<Requests>,
    responses_handler: Arc<Responses>,
    diagnostics_handler: Arc<Mutex<DiagnosticHandle>>,
) -> LSPResult<()> {
    loop {
        match json_rpc.next().await? {
            LSPMessage::Response(resp) => {
                let Some(resp_type) = sent_handler.lock().unwrap().remove(&resp.id) else {
                    continue;
                };
                if let Some(response) = resp.result {
                    let response = match resp_type.parse(response) {
                        Ok(response) => response,
                        Err(error) => LSPResponse::Error(format!("LSP PARSE({resp_type:?}): {error}")),
                    };
                    responses_handler.lock().unwrap().insert(resp.id, response);
                } else if let Some(error) = resp.error {
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
                    responses_handler.lock().unwrap().insert(resp.id, response);
                }
            }
            LSPMessage::Diagnostic(uri, params) => {
                diagnostics_handler.lock().unwrap().insert(uri, params);
            }
            LSPMessage::Request(_inner) => {
                // TODO: investigate handle
                // requests_handler.lock().await.push(resp)
            }
            LSPMessage::Error(_err) => {
                // TODO: investigate handle
            }
            LSPMessage::Unknown(_obj) => {
                // TODO: investigate handle
            }
        }
    }
}

/// ensures tokens are disabled - so tokens will be generated locally
fn are_semantics_disabled(init_cfg: &InitCfg) -> bool {
    match init_cfg {
        Some(mods) => mods
            .get("capabilities")
            .and_then(|cap| cap.as_object()?.get("textDocument"))
            .and_then(|td| td.as_object()?.get("semanticTokens"))
            .map(Value::is_null)
            .unwrap_or_default(),
        None => false,
    }
}

#[cfg(test)]
mod test {
    use super::are_semantics_disabled;
    use crate::configs::{EditorConfigs, FileType};

    #[test]
    fn test_semantic_check() {
        let default_cfg = EditorConfigs::default();
        assert!(!are_semantics_disabled(&None));
        let py_cfg = default_cfg.derive_lsp(&FileType::Python).unwrap().1;
        assert!(are_semantics_disabled(&py_cfg));
        let rust_cfg = default_cfg.derive_lsp(&FileType::Rust).unwrap().1;
        assert!(!are_semantics_disabled(&rust_cfg));
    }
}

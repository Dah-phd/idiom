pub use super::error::LSPError;
use super::lsp_stream::JsonRPC;
pub use super::messages::{DiagnosticHandle, LSPMessage, LSPResponse, LSPResponseType};
pub use super::request::LSPRequest;
use super::{server_cmd, skip_to_response, LSPClient, LSPResult, Requests, Responses, LSP};
use crate::configs::{EditorConfigs, FileType};
use crate::utils::split_arc;

use lsp_types::{request::Initialize, InitializeResult, ServerCapabilities};
use serde_json::from_value;
use std::{
    collections::{
        hash_map::{Entry, HashMap},
        HashSet,
    },
    process::Stdio,
    sync::Mutex,
};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin},
    task::JoinHandle,
};

pub enum LSPServerStatus {
    None,
    Pending,
    ReadyClient(LSPClient),
}

pub enum LSPRunningStatus {
    Running,
    Dead(LSPError),
    Recoverd(LSPError),
    Failing(LSPError),
}

/// holds all components needed to build LSP before creating of the client and main loop
/// namely the server command has started successfully (some servers can take their time)
struct LSPPreload {
    json_rpc: JsonRPC,
    lsp_cmd: String,
    file_type: FileType,
    capabilities: ServerCapabilities,
    inner: Child,
    stdin: ChildStdin,
}

#[derive(Default)]
pub struct LSPServers {
    ready_servers: HashMap<FileType, LSP>,
    in_waiting: HashMap<FileType, JoinHandle<LSPResult<LSPPreload>>>,
}

impl LSPServers {
    pub fn new(preloads: HashSet<(FileType, String)>) -> Self {
        Self {
            in_waiting: preloads
                .into_iter()
                .map(|(file_type, lsp_cmd)| (file_type, tokio::task::spawn(LSP::pre_load(lsp_cmd, file_type))))
                .collect(),
            ready_servers: HashMap::default(),
        }
    }

    pub fn get_running(&mut self, ft: &FileType) -> Option<&mut LSP> {
        match self.ready_servers.get_mut(ft) {
            // return stable or recoverable servers
            Some(lsp) if lsp.is_running() || lsp.attempts != 0 => Some(lsp),
            _ => None,
        }
    }

    pub fn get_or_init_server(&mut self, ft: &FileType, cfg: &EditorConfigs) -> LSPServerStatus {
        match self.ready_servers.get(ft) {
            Some(lsp) if lsp.is_running() || lsp.attempts != 0 => {
                return LSPServerStatus::ReadyClient(lsp.aquire_client())
            }
            // dead lsp
            Some(..) => return LSPServerStatus::None,
            _ => (),
        }

        if let Entry::Vacant(lsp_entry) = self.in_waiting.entry(*ft) {
            let Some(lsp_cmd) = cfg.derive_lsp(ft) else {
                return LSPServerStatus::None;
            };
            let file_type = *ft;
            let init_task = tokio::task::spawn(LSP::pre_load(lsp_cmd, file_type));
            lsp_entry.insert(init_task);
        }

        LSPServerStatus::Pending
    }

    pub async fn check_lsp(&mut self, file_type: FileType) -> Option<LSPRunningStatus> {
        let lsp = self.ready_servers.get_mut(&file_type)?;
        Some(match lsp.check_status(file_type).await {
            Ok(data) => match data {
                None => LSPRunningStatus::Running,
                Some(err) => LSPRunningStatus::Recoverd(err),
            },
            Err(err) if lsp.attempts == 0 => LSPRunningStatus::Dead(err),
            Err(err) => LSPRunningStatus::Failing(err),
        })
    }

    pub fn are_all_servers_ready(&self) -> bool {
        self.in_waiting.is_empty()
    }

    pub async fn apply_started_servers(&mut self, mut apply_cb: impl FnMut(FileType, LSPResult<&mut LSP>)) {
        let Self { ready_servers, in_waiting } = self;
        let mut finished = in_waiting.extract_if(|_, v| v.is_finished());
        // explicit handles due to async logic
        while let Some((file_type, init_task)) = finished.next() {
            match init_task.await {
                Ok(preload_result) => match preload_result.and_then(LSP::from_preload) {
                    Ok(mut lsp) => match ready_servers.entry(file_type) {
                        Entry::Vacant(entry) => apply_cb(file_type, Ok(entry.insert(lsp))),
                        Entry::Occupied(mut entry) => {
                            apply_cb(file_type, Ok(&mut lsp));
                            let mut old = entry.insert(lsp);
                            _ = old.dash_nine().await;
                        }
                    },
                    Err(error) => apply_cb(file_type, Err(error)),
                },
                Err(join_error) => {
                    (apply_cb)(file_type, Err(LSPError::InternalError(format!("Failed to await LSP: {join_error}"))));
                }
            };
        }
    }
}

impl LSP {
    async fn pre_load(lsp_cmd: String, file_type: FileType) -> LSPResult<LSPPreload> {
        let mut server = server_cmd(&lsp_cmd)?;
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
        Ok(LSPPreload { json_rpc, lsp_cmd, file_type, capabilities, inner, stdin })
    }

    fn from_preload(preload: LSPPreload) -> LSPResult<LSP> {
        let LSPPreload { mut json_rpc, lsp_cmd, file_type, capabilities, inner, stdin } = preload;

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

        Ok(Self { client, lsp_cmd, inner, lsp_json_handler, lsp_send_handler, attempts: 5 })
    }
}

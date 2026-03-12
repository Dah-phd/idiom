pub use super::{builder::LSPBuilder, error::LSPError};
use super::{LSPClient, LSPResult, LSP};
use crate::configs::{EditorConfigs, FileType};

use std::collections::{
    hash_map::{Entry, HashMap},
    HashSet,
};
use tokio::task::JoinHandle;

pub enum LSPServerStatus {
    None,
    Pending,
    ReadyClient(LSPClient),
}

pub enum LSPRunningStatus {
    Running,
    Dead,
    Failing,
}

#[derive(Default)]
pub struct LSPServers {
    ready_servers: HashMap<FileType, LSP>,
    in_waiting: HashMap<FileType, JoinHandle<LSPResult<LSPBuilder>>>,
}

impl LSPServers {
    pub fn new(preloads: HashSet<(FileType, String)>) -> Self {
        Self {
            in_waiting: preloads
                .into_iter()
                .map(|(file_type, lsp_cmd)| (file_type, tokio::task::spawn(LSPBuilder::new(lsp_cmd, file_type))))
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
        let attempt = match self.ready_servers.get(ft) {
            Some(lsp) if lsp.is_running() || lsp.attempts != 0 => {
                return LSPServerStatus::ReadyClient(lsp.aquire_client())
            }
            Some(lsp) if lsp.attempts == 0 => {
                return LSPServerStatus::None;
            }
            Some(..) => self.ready_servers.remove(ft).map(|lsp| lsp.attempts),
            _ => None,
        };

        if let Entry::Vacant(lsp_entry) = self.in_waiting.entry(*ft) {
            let Some(lsp_cmd) = cfg.derive_lsp(ft) else {
                return LSPServerStatus::None;
            };
            let file_type = *ft;
            let init_task = tokio::task::spawn(LSPBuilder::new_attempt(lsp_cmd, file_type, attempt));
            lsp_entry.insert(init_task);
        }

        LSPServerStatus::Pending
    }

    /// performs check on the status of running LSP
    /// even if returned None the LSP could be in pending status
    pub fn check_running_lsp(&mut self, file_type: FileType) -> Option<LSPRunningStatus> {
        let lsp = self.ready_servers.get_mut(&file_type)?;
        if lsp.is_running() {
            return Some(LSPRunningStatus::Running);
        }
        let Some((lsp_cmd, attempt)) = self.ready_servers.remove(&file_type)?.decompose() else {
            return Some(LSPRunningStatus::Dead);
        };
        let task = tokio::task::spawn(LSPBuilder::new_attempt(lsp_cmd, file_type, Some(attempt)));
        if let Some(old) = self.in_waiting.insert(file_type, task) {
            old.abort();
        }
        Some(LSPRunningStatus::Failing)
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
                Ok(preload_result) => match preload_result.and_then(LSPBuilder::spawn) {
                    Ok(mut lsp) => match ready_servers.entry(file_type) {
                        Entry::Vacant(entry) => apply_cb(file_type, Ok(entry.insert(lsp))),
                        Entry::Occupied(mut entry) => {
                            apply_cb(file_type, Ok(&mut lsp));
                            _ = entry.insert(lsp);
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
    fn decompose(mut self) -> Option<(String, u8)> {
        let attempts = self.attempts.checked_sub(1)?;
        let lsp_cmd = std::mem::take(&mut self.lsp_cmd);
        Some((lsp_cmd, attempts))
    }
}

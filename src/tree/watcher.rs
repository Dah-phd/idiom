use std::{path::PathBuf, time::Duration};

use crate::lsp::Diagnostic;
use crate::{global_state::GlobalState, tree::TreePath};
use notify::{
    event::{AccessKind, AccessMode, ModifyKind},
    Config, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

pub type DianosticHandle = Arc<Mutex<HashMap<PathBuf, Diagnostic>>>;

pub struct TreeWatcher {
    _inner: RecommendedWatcher,
    receiver: std::sync::mpsc::Receiver<Result<Event, Error>>,
    sync_handler: Option<JoinHandle<TreePath>>,
    lsp_register: Vec<DianosticHandle>,
}

impl TreeWatcher {
    pub fn root() -> Result<Self, Error> {
        let (tx, receiver) = std::sync::mpsc::channel();
        RecommendedWatcher::new(tx, Config::default().with_poll_interval(Duration::from_secs(1)))
            .and_then(|mut inner| inner.watch(&PathBuf::from("./src"), RecursiveMode::Recursive).map(|_| inner))
            .map(|_inner| Self { _inner, receiver, sync_handler: None, lsp_register: Vec::new() })
    }

    pub async fn poll(&mut self, tree: &mut TreePath, gs: &mut GlobalState) -> bool {
        let mut status = false;

        if matches!(&self.sync_handler, Some(handle) if handle.is_finished()) {
            match self.sync_handler.take().unwrap().await {
                Ok(new_tree) => {
                    *tree = new_tree;
                    status = true;
                }
                Err(err) => gs.error(format!("File tree sync failure! ERR: {err}")),
            };
        }

        let mut should_sync = false;
        let mut map_errors = false;

        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(Event { kind, paths, .. }) = result {
                match kind {
                    EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                        map_errors = !status;
                        for path in paths {
                            gs.workspace.push(crate::global_state::WorkspaceEvent::FileUpdated(path));
                        }
                    }
                    EventKind::Modify(ModifyKind::Name(..)) | EventKind::Create(..) | EventKind::Remove(..) => {
                        should_sync = true;
                    }
                    _ => {}
                }
            }
        }

        if map_errors {
            lsp_sync_diagnosic(tree, &self.lsp_register);
        }

        if should_sync {
            self.start_sync(tree.clone());
        }

        status || map_errors
    }

    pub fn map_errors(&self, tree: &mut TreePath) {
        lsp_sync_diagnosic(tree, &self.lsp_register);
    }

    fn start_sync(&mut self, mut tree: TreePath) {
        let lsp_register = self.lsp_register.clone();
        if let Some(handle) = self.sync_handler.replace(tokio::spawn(async move {
            tree.sync_base();
            lsp_sync_diagnosic(&mut tree, &lsp_register);
            tree
        })) {
            handle.abort();
        };
    }

    pub fn register_lsp(&mut self, tree: &mut TreePath, lsp: DianosticHandle) {
        self.lsp_register.push(lsp);
        lsp_sync_diagnosic(tree, &self.lsp_register);
    }
}

fn lsp_sync_diagnosic(tree: &mut TreePath, lsp_register: &[DianosticHandle]) {
    for lsp in lsp_register.iter() {
        if let Ok(lock) = lsp.try_lock() {
            for (path, diagnostic) in lock.iter() {
                tree.map_diagnostics_base(path, diagnostic.errors, diagnostic.warnings);
            }
        }
    }
}

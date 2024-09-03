use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{error::IdiomResult, lsp::Diagnostic};
use crate::{global_state::GlobalState, tree::TreePath};
use notify::{
    event::{AccessKind, AccessMode, ModifyKind},
    Config, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
pub type DianosticHandle = Arc<Mutex<HashMap<PathBuf, Diagnostic>>>;

const TICK: Duration = Duration::from_secs(1);

pub enum TreeWatcher {
    System {
        _inner: RecommendedWatcher,
        receiver: std::sync::mpsc::Receiver<Result<Event, Error>>,
        lsp_register: Vec<DianosticHandle>,
    },
    Manual {
        clock: Instant,
        lsp_register: Vec<DianosticHandle>,
    },
}

impl TreeWatcher {
    pub fn root() -> Self {
        let (tx, receiver) = std::sync::mpsc::channel();
        RecommendedWatcher::new(tx, Config::default())
            .and_then(|mut inner| inner.watch(&PathBuf::from("."), RecursiveMode::Recursive).map(|_| inner))
            .map(|_inner| Self::System { _inner, receiver, lsp_register: Vec::new() })
            .unwrap_or(Self::Manual { clock: Instant::now(), lsp_register: Vec::new() })
    }

    pub fn poll(
        &mut self,
        tree: &mut TreePath,
        path_parser: fn(&Path) -> IdiomResult<PathBuf>,
        gs: &mut GlobalState,
    ) -> bool {
        let mut full_sync = false;
        let mut should_sync = false;
        let mut map_errors = false;
        match self {
            Self::System { receiver, lsp_register, .. } => {
                while let Ok(result) = receiver.try_recv() {
                    if let Ok(Event { kind, paths, .. }) = result {
                        match kind {
                            EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                                map_errors = true;
                                for path in paths {
                                    gs.workspace.push(crate::global_state::WorkspaceEvent::FileUpdated(path));
                                }
                            }
                            EventKind::Modify(ModifyKind::Name(..)) | EventKind::Create(..) | EventKind::Remove(..)
                                if !full_sync =>
                            {
                                should_sync = true;
                                for path in paths {
                                    match path.parent() {
                                        Some(path) => match path_parser(path) {
                                            Ok(formatted_path) => match tree.find_by_path_skip_root(&formatted_path) {
                                                Some(inner_tree) => {
                                                    inner_tree.sync();
                                                    panic!("bumba")
                                                }
                                                None => full_sync = true,
                                            },
                                            Err(..) => match tree.find_by_path_skip_root(path) {
                                                Some(inner_tree) => {
                                                    inner_tree.sync();
                                                    panic!("bumba")
                                                }
                                                None => full_sync = true,
                                            },
                                        },
                                        _ => full_sync = true,
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if map_errors {
                    lsp_sync_diagnosic(tree, lsp_register);
                }
            }
            Self::Manual { clock, lsp_register } => {
                full_sync = clock.elapsed() > TICK;
                if full_sync {
                    *clock = Instant::now();
                    lsp_sync_diagnosic(tree, lsp_register);
                    should_sync = true;
                }
            }
        }

        if full_sync {
            tree.sync_base();
        }

        should_sync || map_errors
    }

    pub fn map_errors(&self, tree: &mut TreePath) {
        match self {
            Self::Manual { lsp_register, .. } => {
                lsp_sync_diagnosic(tree, lsp_register);
            }
            Self::System { lsp_register, .. } => {
                lsp_sync_diagnosic(tree, lsp_register);
            }
        }
    }

    pub fn register_lsp(&mut self, tree: &mut TreePath, lsp: DianosticHandle) {
        match self {
            Self::Manual { lsp_register, .. } => {
                lsp_register.push(lsp);
                lsp_sync_diagnosic(tree, lsp_register);
            }
            Self::System { lsp_register, .. } => {
                lsp_register.push(lsp);
                lsp_sync_diagnosic(tree, lsp_register);
            }
        }
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

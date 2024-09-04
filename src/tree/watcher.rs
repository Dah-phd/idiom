use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{error::IdiomResult, lsp::Diagnostic};
use crate::{
    global_state::{GlobalState, WorkspaceEvent},
    tree::TreePath,
};
use bitflags::bitflags;
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
        match self {
            Self::System { receiver, lsp_register, .. } => {
                let mut handler = EventHandles::default();
                while let Ok(event) = receiver.try_recv() {
                    handler.handle(event, tree, gs, path_parser, lsp_register);
                }
                !handler.is_all()
            }
            Self::Manual { clock, lsp_register } => {
                if clock.elapsed() > TICK {
                    tree.sync_base();
                    *clock = Instant::now();
                    lsp_sync_diagnosic(tree, lsp_register);
                    true
                } else {
                    false
                }
            }
        }
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

bitflags! {
    /// Workspace and Footer are always drawn
    #[derive(PartialEq, Eq)]
    pub struct EventHandles: u8 {
        const CONTENT = 0b0000_0100;
        const TREE_PARTIAL = 0b0000_0010;
        const TREE  = 0b0000_0001;
    }
}

impl Default for EventHandles {
    fn default() -> Self {
        Self::CONTENT | Self::TREE | Self::TREE_PARTIAL
    }
}

impl EventHandles {
    fn handle(
        &mut self,
        event: Result<Event, Error>,
        tree: &mut TreePath,
        gs: &mut GlobalState,
        path_parser: fn(&Path) -> IdiomResult<PathBuf>,
        lsp_register: &[DianosticHandle],
    ) {
        if let Ok(Event { kind, paths, .. }) = event {
            match kind {
                EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                    for path in paths {
                        gs.workspace.push(WorkspaceEvent::FileUpdated(path));
                    }
                    if self.contains(Self::CONTENT) {
                        self.remove(Self::CONTENT);
                        lsp_sync_diagnosic(tree, lsp_register);
                    }
                }
                EventKind::Modify(ModifyKind::Name(..)) | EventKind::Create(..) | EventKind::Remove(..)
                    if self.contains(Self::TREE) =>
                {
                    for path in paths {
                        match path.parent() {
                            Some(path) => match path_parser(path) {
                                Ok(formatted_path) => match tree.find_by_path_skip_root(&formatted_path) {
                                    Some(inner_tree) => {
                                        self.remove(Self::TREE_PARTIAL);
                                        inner_tree.sync();
                                    }
                                    None => {
                                        tree.sync_base();
                                        self.remove(Self::TREE)
                                    }
                                },
                                Err(..) => match tree.find_by_path_skip_root(path) {
                                    Some(inner_tree) => {
                                        self.remove(Self::TREE_PARTIAL);
                                        inner_tree.sync();
                                    }
                                    None => {
                                        tree.sync_base();
                                        self.remove(Self::TREE)
                                    }
                                },
                            },
                            _ => {
                                tree.sync_base();
                                self.remove(Self::TREE)
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

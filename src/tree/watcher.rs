use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::PathParser;
use crate::error::IdiomError;
use crate::{
    error::IdiomResult,
    global_state::{GlobalState, IdiomEvent},
    tree::TreePath,
};
use bitflags::bitflags;
use notify::{
    event::{AccessKind, AccessMode, ModifyKind},
    Config, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

const TICK: Duration = Duration::from_secs(1);

pub enum TreeWatcher {
    System { inner: RecommendedWatcher, receiver: std::sync::mpsc::Receiver<Result<Event, Error>> },
    Manual { clock: Instant },
}

impl TreeWatcher {
    pub fn root(path: &Path) -> Self {
        let (tx, receiver) = std::sync::mpsc::channel();
        RecommendedWatcher::new(tx, Config::default())
            .and_then(|mut inner| inner.watch(path, RecursiveMode::NonRecursive).map(|_| inner))
            .map(|inner| Self::System { inner, receiver })
            .unwrap_or(Self::Manual { clock: Instant::now() })
    }

    pub fn watch(&mut self, path: &Path) -> IdiomResult<()> {
        if let Self::System { inner: _inner, .. } = self {
            _inner.watch(path, RecursiveMode::NonRecursive).map_err(|err| IdiomError::any(err.to_string()))?;
        }
        Ok(())
    }

    pub fn stop_watch(&mut self, path: &Path) -> IdiomResult<()> {
        if let Self::System { inner: _inner, .. } = self {
            _inner.unwatch(path).map_err(|err| IdiomError::any(err.to_string()))?;
        }
        Ok(())
    }

    pub fn poll(&mut self, tree: &mut TreePath, path_parser: PathParser, gs: &mut GlobalState) -> bool {
        match self {
            Self::System { receiver, .. } => {
                let mut handler = EventHandles::default();
                while let Ok(event) = receiver.try_recv() {
                    handler.handle(event, tree, gs, path_parser);
                }
                !handler.is_all()
            }
            Self::Manual { clock, .. } => {
                if clock.elapsed() > TICK {
                    tree.sync_base();
                    *clock = Instant::now();
                    true
                } else {
                    false
                }
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
    ) {
        if let Ok(Event { kind, paths, .. }) = event {
            use EventKind::*;
            match kind {
                Access(AccessKind::Close(AccessMode::Write)) => {
                    for path in paths {
                        gs.event.push(IdiomEvent::FileUpdated(path));
                    }
                    if self.contains(Self::CONTENT) {
                        self.remove(Self::CONTENT);
                    }
                }
                Create(..) | Remove(..) | Modify(ModifyKind::Name(..)) if self.contains(Self::TREE) => {
                    for path in paths.into_iter() {
                        match path.parent().and_then(|path| tree.find_by_path_skip_root(path, path_parser)) {
                            Some(inner_tree) => {
                                self.remove(Self::TREE_PARTIAL);
                                inner_tree.sync();
                            }
                            None => {
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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{configs::KeyMap, error::IdiomResult, lsp::DiagnosticType, render::state::State};

use super::{file_clipboard::FileClipboard, watcher::TreeWatcher, Tree, TreePath};

pub fn mock_tree() -> Tree {
    let key_map = KeyMap::default().tree_key_map();
    Tree {
        key_map,
        watcher: TreeWatcher::Manual { clock: Instant::now() },
        tree_clipboard: FileClipboard::default(),
        state: State::default(),
        diagnostics_state: HashMap::new(),
        selected_path: PathBuf::new(),
        tree: TreePath::Folder {
            path: PathBuf::new(),
            tree: None,
            display: String::new(),
            diagnostic: DiagnosticType::None,
        },
        display_offset: 2,
        path_parser: basic_parser,
        rebuild: false,
    }
}

fn basic_parser(path: &Path) -> IdiomResult<PathBuf> {
    Ok(path.to_owned())
}

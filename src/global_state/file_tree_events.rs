use std::path::PathBuf;

use crate::{
    configs::Mode,
    popups::{
        message,
        popups_tree::{file_selector, search_tree_files},
    },
    tree::Tree,
};

use super::WorkspaceEvent;

#[derive(Debug, Clone)]
pub enum TreeEvent {
    PopupAccess,
    Open(PathBuf),
    OpenAtLine(PathBuf, usize),
    RenameFile(String),
    SearchFiles(String),
    SelectPathFull(String),
}

impl TreeEvent {
    pub fn map(self, tree: &mut Tree, mode: &mut Mode) -> Option<WorkspaceEvent> {
        match self {
            Self::PopupAccess => mode.update_tree(tree),
            Self::SearchFiles(pattern) => {
                mode.clear_popup();
                mode.popup(search_tree_files(pattern));
            }
            Self::Open(path) => {
                tree.select_by_path(&path);
                return Some(WorkspaceEvent::Open(path, 0));
            }
            Self::OpenAtLine(path, line) => {
                tree.select_by_path(&path);
                return Some(WorkspaceEvent::Open(path, line));
            }
            Self::RenameFile(name) => {
                mode.clear_popup();
                if let Err(error) = tree.rename_file(name) {
                    mode.popup(Box::new(message(error.to_string())));
                }
            }
            Self::SelectPathFull(pattern) => {
                mode.popup_select(file_selector(tree.search_paths(&pattern)));
            }
        }
        None
    }
}

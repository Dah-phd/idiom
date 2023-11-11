use crate::{
    components::{
        popups::{message, tree_popups::file_selector},
        Tree,
    },
    configs::Mode,
};

#[derive(Debug, Clone)]
pub enum TreeEvent {
    SelectPath(String),
    RenameFile(String),
    SelectPathFull(String),
}

impl TreeEvent {
    pub fn map(self, tree: &mut Tree, mode: &mut Mode) {
        match self {
            Self::SelectPath(pattern) => {
                mode.popup_select(file_selector(tree.search_select_paths(pattern)));
            }
            Self::RenameFile(name) => {
                mode.clear_popup();
                if let Err(error) = tree.rename_file(name) {
                    mode.popup(Box::new(message(error.to_string())));
                }
            }
            Self::SelectPathFull(pattern) => {
                mode.popup_select(file_selector(tree.search_paths(pattern)));
            }
        }
    }
}

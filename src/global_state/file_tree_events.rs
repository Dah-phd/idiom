use std::path::PathBuf;

use super::PopupMessage;

#[derive(Debug, Clone)]
pub enum TreeEvent {
    PopupAccess,
    Open(PathBuf),
    OpenAtLine(PathBuf, usize),
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    RenameFile(String),
    SearchFiles(String),
    SelectTreeFiles(String),
    SelectTreeFilesFull(String),
}

impl From<TreeEvent> for PopupMessage {
    fn from(event: TreeEvent) -> Self {
        PopupMessage::Tree(event)
    }
}

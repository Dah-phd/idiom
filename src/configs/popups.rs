use std::path::PathBuf;

use crate::components::workspace::Select;

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    Open(PathBuf, usize),
    ActivateEditor(usize),
    SelectPath(String),
    SelectPathFull(String),
    SelectTreeFiles(String),
    SelectTreeFilesFull(String),
    SelectOpenedFile(String),
    Rename(String),
    RenameFile(String),
    GoToLine(usize),
    GoToSelect(Select),
    ReplaceSelect(String, Select),
    UpdateWorkspace,
    UpdateFooter,
    UpdateTree,
    Exit,
    SaveAndExit,
    None,
    Done,
}

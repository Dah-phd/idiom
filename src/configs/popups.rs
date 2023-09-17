use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    Open((PathBuf, usize)),
    ActivateEditor(usize),
    SelectPath(String),
    SelectPathFull(String),
    SelectTreeFiles(String),
    SelectTreeFilesFull(String),
    SelectOpenedFile(String),
    RenameFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

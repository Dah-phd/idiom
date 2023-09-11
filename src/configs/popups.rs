use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    Open((PathBuf, usize)),
    SelectPath(String),
    SelectTreeFiles(String),
    SelectOpenedFile(String),
    RenameFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    Open((PathBuf, usize)),
    SelectPath(String),
    SelectFileLine(String),
    RenameFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

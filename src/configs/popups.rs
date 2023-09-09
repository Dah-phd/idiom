use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    OpenFile((PathBuf, usize)),
    RenameFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

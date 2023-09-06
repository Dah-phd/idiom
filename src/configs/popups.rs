#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    RenameFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

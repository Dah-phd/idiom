#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreatFile(String),
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

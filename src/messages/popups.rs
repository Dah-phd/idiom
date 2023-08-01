#[derive(Debug, Clone)]
pub enum PopupMessage {
    GoToLine(usize),
    Exit,
    SaveAndExit,
    None,
    Done,
}

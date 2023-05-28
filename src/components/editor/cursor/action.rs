#[derive(Debug, Default)]
pub struct ActionLogger {
    buffer: String,
    done: Vec<Action>,
    undone: Vec<Action>,
}

#[derive(Debug)]
enum Action {
    Text(String, CursorPosition),
    Cut(String, CursorPosition),
    Paste(String, CursorPosition),
}

#[derive(Debug)]
struct CursorPosition {
    line: usize,
    char: usize,
}

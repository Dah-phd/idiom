use crate::components::editor::CursorPosition;
use std::collections::VecDeque;

pub fn trim_start_inplace(line: &mut String) {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace()) {
        line.replace_range(..idx, "");
    };
}

pub fn get_closing_char(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

pub fn derive_end_cursor_position(from: &CursorPosition, clip: &str) -> CursorPosition {
    let mut to = *from;
    let mut lines = clip.split('\n').peekable();
    while let Some(line) = lines.next() {
        to.line += 1;
        if lines.peek().is_none() {
            to.char = line.len();
        }
    }
    to
}

#[derive(Debug, Clone)]
pub struct LimitedQue<T> {
    inner: VecDeque<T>,
    pub capacity: usize,
}

impl<T> Default for LimitedQue<T> {
    fn default() -> Self {
        Self {
            inner: VecDeque::new(),
            capacity: 1000,
        }
    }
}

impl<T> LimitedQue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            capacity,
        }
    }

    pub fn take(&self) -> &VecDeque<T> {
        &self.inner
    }

    pub fn push(&mut self, value: T) {
        if self.capacity == self.inner.len() {
            self.inner.pop_front();
        };
        self.inner.push_back(value)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop_front()
    }
}

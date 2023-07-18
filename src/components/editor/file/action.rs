use crate::utils::{derive_end_cursor_position, LimitedQue};

use super::select::CursorPosition;
use super::Editor;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct ActionLogger {
    buffer: Option<Action>,
    pub done: LimitedQue<Action>,
    pub undone: LimitedQue<Action>,
    clock: Instant,
}

impl Default for ActionLogger {
    fn default() -> Self {
        Self {
            buffer: Option::default(),
            done: LimitedQue::default(),
            undone: LimitedQue::default(),
            clock: Instant::now(),
        }
    }
}

impl ActionLogger {
    pub fn tick(&mut self) {
        if self.clock.elapsed() >= TICK {
            self.push_buffer();
            self.clock = Instant::now();
        }
    }

    pub fn replace(&mut self, at: CursorPosition, new: impl Into<String>, old: impl Into<String>) {}

    pub fn remove(&mut self, position: CursorPosition, content: String) {}

    pub fn push_char(&mut self, new_position: CursorPosition, ch: char) {}

    fn push_buffer(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.done.push(buffer)
        }
    }

    pub fn undo(&mut self, editor: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.done.pop() {}
    }

    pub fn redo(&mut self, content: &mut Vec<String>) {
        self.push_buffer();
        if let Some(action) = self.undone.pop() {}
    }
}

#[derive(Debug)]
pub struct Action {
    cursor: CursorPosition,
}

impl Action {}

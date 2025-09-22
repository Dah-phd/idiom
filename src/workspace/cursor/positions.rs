use serde::{Deserialize, Serialize};

pub type Select = (CursorPosition, CursorPosition);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub char: usize, // this is char position not byte index
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CharRange {
    pub from: usize,
    pub to: usize,
}

impl CharRange {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.from == self.to
    }

    pub fn into_select(self, line: usize) -> Select {
        (CursorPosition { line, char: self.from }, CursorPosition { line, char: self.to })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectPosition {
    pub from: CursorPosition,
    pub to: CursorPosition,
    swaped: bool,
}

impl SelectPosition {
    pub fn cursor_pos(&self) -> CursorPosition {
        match self.swaped {
            true => self.from,
            false => self.to,
        }
    }

    pub fn init_pos(&self) -> CursorPosition {
        match self.swaped {
            true => self.to,
            false => self.from,
        }
    }

    pub fn init_to_cursor(&self) -> (CursorPosition, CursorPosition) {
        match self.swaped {
            true => (self.to, self.from),
            false => (self.from, self.to),
        }
    }
}

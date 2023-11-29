use std::ops::RangeInclusive;

use lsp_types::Position;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    pub line: usize,
    pub char: usize,
}

impl From<&CursorPosition> for Position {
    fn from(value: &CursorPosition) -> Self {
        Position { line: value.line as u32, character: value.char as u32 }
    }
}

impl From<CursorPosition> for Position {
    fn from(value: CursorPosition) -> Self {
        Position { line: value.line as u32, character: value.char as u32 }
    }
}

impl From<(usize, usize)> for CursorPosition {
    fn from(value: (usize, usize)) -> Self {
        Self { line: value.0, char: value.1 }
    }
}

impl From<Position> for CursorPosition {
    fn from(value: Position) -> Self {
        Self { line: value.line as usize, char: value.character as usize }
    }
}

impl From<&Position> for CursorPosition {
    fn from(value: &Position) -> Self {
        Self { line: value.line as usize, char: value.character as usize }
    }
}

impl CursorPosition {
    pub fn line_range(&self, sub: usize, add: usize) -> std::ops::Range<usize> {
        self.line.checked_sub(sub).unwrap_or_default()..self.line + add
    }

    pub fn as_range(&self) -> RangeInclusive<usize> {
        self.line..=self.line
    }

    pub fn diff_char(&mut self, offset: usize) {
        self.char = self.char.checked_sub(offset).unwrap_or_default()
    }

    pub fn diff_line(&mut self, offset: usize) {
        self.line = self.line.checked_sub(offset).unwrap_or_default()
    }
}

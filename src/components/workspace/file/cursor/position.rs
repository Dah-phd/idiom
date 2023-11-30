use std::ops::{Add, Sub};

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

pub enum Offset {
    Pos(usize),
    Neg(usize),
}

impl Offset {
    fn offset(self, val: usize) -> usize {
        match self {
            Self::Pos(numba) => val + numba,
            Self::Neg(numba) => val.checked_sub(numba).unwrap_or_default(),
        }
    }
}

impl Add<usize> for Offset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Self::Pos(numba) => Self::Pos(numba + rhs),
            Self::Neg(numba) => {
                if numba > rhs {
                    Self::Neg(numba - rhs)
                } else {
                    Self::Pos(rhs - numba)
                }
            }
        }
    }
}

impl Sub<usize> for Offset {
    type Output = Offset;
    fn sub(self, rhs: usize) -> Self::Output {
        match self {
            Self::Neg(numba) => Self::Neg(numba + rhs),
            Self::Pos(numba) => {
                if numba > rhs {
                    Self::Pos(numba - rhs)
                } else {
                    Self::Neg(rhs - numba)
                }
            }
        }
    }
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Self::Pos(value)
    }
}

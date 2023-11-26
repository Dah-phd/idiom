use std::ops::{Add, RangeInclusive, Sub};

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

    pub fn offset_char(&mut self, offset: Offset) {
        match offset {
            Offset::Neg(val) => self.char = self.char.checked_sub(val).unwrap_or_default(),
            Offset::Pos(val) => self.char += val,
        }
    }

    pub fn offset_line(&mut self, offset: Offset) {
        match offset {
            Offset::Neg(val) => self.line = self.line.checked_sub(val).unwrap_or_default(),
            Offset::Pos(val) => self.line += val,
        }
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

#[derive(Debug, Clone, Copy)]
pub enum Offset {
    Pos(usize),
    Neg(usize),
}

impl Offset {
    pub fn unwrap(self) -> usize {
        match self {
            Self::Neg(inner) => inner,
            Self::Pos(inner) => inner,
        }
    }
}

impl From<Offset> for usize {
    fn from(value: Offset) -> Self {
        match value {
            Offset::Neg(val) => val,
            Offset::Pos(val) => val,
        }
    }
}

impl Add for Offset {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Self::Pos(val) => match rhs {
                Self::Pos(rhs_val) => Self::Pos(val + rhs_val),
                Self::Neg(rhs_val) => {
                    if val < rhs_val {
                        Self::Neg(rhs_val - val)
                    } else {
                        Self::Pos(val - rhs_val)
                    }
                }
            },
            Self::Neg(val) => match rhs {
                Self::Neg(rhs_val) => Self::Neg(val + rhs_val),
                Self::Pos(rhs_val) => {
                    if val > rhs_val {
                        Self::Neg(val - rhs_val)
                    } else {
                        Self::Pos(rhs_val - val)
                    }
                }
            },
        }
    }
}

impl Add<usize> for Offset {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Self::Pos(val) => Self::Pos(val + rhs),
            Self::Neg(val) => {
                if val > rhs {
                    Self::Neg(val - rhs)
                } else {
                    Self::Pos(rhs - val)
                }
            }
        }
    }
}

impl Sub<usize> for Offset {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        match self {
            Self::Neg(val) => Self::Neg(val + rhs),
            Self::Pos(val) => {
                if rhs > val {
                    Self::Neg(rhs - val)
                } else {
                    Self::Pos(val - rhs)
                }
            }
        }
    }
}

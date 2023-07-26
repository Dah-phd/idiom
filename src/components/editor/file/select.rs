use std::ops::{Add, Sub};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    pub line: usize,
    pub char: usize,
}

impl From<(usize, usize)> for CursorPosition {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0,
            char: value.1,
        }
    }
}

#[allow(dead_code)]
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

    pub fn diff_char(&mut self, offset: usize) {
        self.char = self.char.checked_sub(offset).unwrap_or_default()
    }

    pub fn diff_line(&mut self, offset: usize) {
        self.line = self.line.checked_sub(offset).unwrap_or_default()
    }
}

pub enum Offset {
    Pos(usize),
    Neg(usize),
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

#[derive(Debug, Clone, Copy)]
pub enum Select {
    None,
    Range(CursorPosition, CursorPosition),
}

impl Default for Select {
    fn default() -> Self {
        Self::None
    }
}

impl Select {
    pub fn is_empty(&self) -> bool {
        matches!(self, Select::None)
    }

    pub fn drop(&mut self) {
        (*self) = Self::None;
    }

    pub fn init(&mut self, line: usize, char: usize) {
        if matches!(self, Select::None) {
            (*self) = Self::Range((line, char).into(), (line, char).into())
        }
    }

    pub fn push(&mut self, position: &CursorPosition) {
        if let Self::Range(_, to) = self {
            (*to) = *position
        }
    }

    pub fn get(&self) -> Option<(&CursorPosition, &CursorPosition)> {
        match self {
            Self::None => None,
            Self::Range(from, to) => {
                if from.line > to.line || from.line == to.line && from.char > to.char {
                    Some((to, from))
                } else {
                    Some((from, to))
                }
            }
        }
    }
}

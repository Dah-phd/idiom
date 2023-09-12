use std::ops::{Add, RangeInclusive, Sub};

use super::action::ActionLogger;
type CutContent = Option<(CursorPosition, CursorPosition, String)>;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CursorPosition {
    pub line: usize,
    pub char: usize,
}

impl From<(usize, usize)> for CursorPosition {
    fn from(value: (usize, usize)) -> Self {
        Self { line: value.0, char: value.1 }
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
    pub fn drop(&mut self) {
        (*self) = Self::None;
    }

    pub fn extract_logged(&mut self, content: &mut Vec<String>, action_logger: &mut ActionLogger) -> CutContent {
        if let Self::Range(mut from, mut to) = std::mem::replace(self, Self::None) {
            if to.line < from.line || to.line == from.line && to.char < from.char {
                (from, to) = (to, from);
            };
            action_logger.init_replace_from_select(&from, &to, content);
            return Some((from, to, clip_content(&from, &to, content)));
        }
        None
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

    pub fn get_mut(&mut self) -> Option<(&mut CursorPosition, &mut CursorPosition)> {
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

fn clip_content(from: &CursorPosition, to: &CursorPosition, content: &mut Vec<String>) -> String {
    if from.line == to.line {
        let line = &mut content[from.line];
        let clip = line[from.char..to.char].to_owned();
        line.replace_range(from.char..to.char, "");
        clip
    } else {
        let mut clip_vec = vec![content[from.line].split_off(from.char)];
        let mut last_line = to.line;
        while from.line < last_line {
            last_line -= 1;
            if from.line == last_line {
                let final_clip = content.remove(from.line + 1);
                let (clipped, remaining) = final_clip.split_at(to.char);
                content[from.line].push_str(remaining);
                clip_vec.push(clipped.to_owned())
            } else {
                clip_vec.push(content.remove(from.line + 1))
            }
        }
        clip_vec.join("\n")
    }
}

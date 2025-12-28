use crate::{editor_line::EditorLine, utils::Direction};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub type Select = (CursorPosition, CursorPosition);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub char: usize, // this is char position not byte index
}

impl CursorPosition {
    pub fn next(self, content: &[EditorLine]) -> Option<Self> {
        let Self { mut line, mut char } = self;
        char += 1;
        let next_chars = content.get(line)?.char_len();
        if next_chars > char {
            return Some(Self { line, char });
        }
        line += 1;
        while content.get(line)?.char_len() == 0 {
            line += 1;
        }
        Some(Self { line, char: 0 })
    }

    pub fn prev(self, content: &[EditorLine]) -> Option<Self> {
        let Self { mut line, char } = self;
        if self.char != 0 {
            if content.get(line)?.char_len() < char {
                return None;
            }
            return Some(Self { line, char: char - 1 });
        }
        line = line.checked_sub(1)?;
        let mut prev_chars = content.get(line)?.char_len();
        while prev_chars == 0 {
            line = line.checked_sub(1)?;
            prev_chars = content[line].char_len();
        }
        Some(Self { line, char: prev_chars - 1 })
    }
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

    pub fn len(&self) -> usize {
        self.to - self.from
    }

    pub fn into_select(self, line: usize) -> Select {
        (CursorPosition { line, char: self.from }, CursorPosition { line, char: self.to })
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CharRangeUnbound {
    pub from: Option<usize>,
    pub to: Option<usize>,
}

#[allow(dead_code)]
impl CharRangeUnbound {
    #[inline]
    pub fn is_empty(&self) -> bool {
        let start = self.from.unwrap_or_default();
        self.to == Some(start)
    }

    #[inline]
    pub fn is_all(&self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    #[inline]
    pub fn has_bound_start(&self) -> bool {
        self.from.is_some()
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.from.unwrap_or_default()
    }

    #[inline]
    pub fn end(&self) -> Option<usize> {
        self.to
    }

    #[inline]
    pub fn bound(self, max_range: usize) -> CharRange {
        CharRange { from: self.from.unwrap_or_default(), to: self.to.unwrap_or(max_range) }
    }
}

#[inline]
pub fn checked_select(from: CursorPosition, to: CursorPosition) -> Option<Select> {
    match from.cmp(&to) {
        Ordering::Greater => Some((to, from)),
        Ordering::Equal => None,
        Ordering::Less => Some((from, to)),
    }
}

#[inline]
pub fn checked_select_with_direction(from: CursorPosition, to: CursorPosition) -> Option<(Select, Direction)> {
    match from.cmp(&to) {
        Ordering::Greater => Some(((to, from), Direction::Reversed)),
        Ordering::Equal => None,
        Ordering::Less => Some(((from, to), Direction::Normal)),
    }
}

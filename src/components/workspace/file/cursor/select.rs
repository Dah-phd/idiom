use lsp_types::Range;

use super::CursorPosition;

#[derive(Debug, Clone, Copy, Default)]
pub enum Select {
    #[default]
    None,
    Range(CursorPosition, CursorPosition),
}

impl From<Range> for Select {
    fn from(value: Range) -> Self {
        Self::Range(value.start.into(), value.end.into())
    }
}

impl Select {
    pub fn take(&mut self) -> Self {
        let unordered = std::mem::take(self);
        if let Self::Range(from, to) = unordered {
            if from.line > to.line || from.line == to.line && from.char > to.char {
                return Self::Range(to, from);
            }
        }
        unordered
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

    pub fn reduce_at(&mut self, line: usize, reduction: usize) {
        if let Self::Range(from, to) = self {
            if from.line == line {
                from.char = from.char.checked_sub(reduction).unwrap_or_default();
            }
            if to.line == line {
                to.char = to.char.checked_sub(reduction).unwrap_or_default();
            }
        }
    }

    pub fn increase_at(&mut self, line: usize, increase: usize) {
        if let Self::Range(from, to) = self {
            if from.line == line {
                from.char += increase;
            }
            if to.line == line {
                to.char += increase;
            }
        }
    }

    pub fn len(&self, content: &[String]) -> usize {
        if let Some((from, to)) = self.get() {
            if from.line == to.line {
                return content[from.line][from.char..to.char].len();
            };
        }
        0
    }

    pub fn try_unwrap(self) -> Option<(CursorPosition, CursorPosition)> {
        if let Self::Range(from, to) = self {
            if from.line > to.line || from.line == to.line && from.char > to.char {
                return Some((to, from));
            }
            return Some((from, to));
        }
        None
    }

    pub fn take_option(&mut self) -> Option<(CursorPosition, CursorPosition)> {
        let unordered = std::mem::take(self);
        if let Self::Range(from, to) = unordered {
            if from.line > to.line || from.line == to.line && from.char > to.char {
                return Some((to, from));
            }
            return Some((from, to));
        }
        None
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy)]
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

    pub fn push(&mut self, line: usize, char: usize) {
        if let Self::Range(_, to) = self {
            (*to) = (line, char).into()
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

use lsp_types::Range;

use super::action::ActionLogger;
use super::utils::clip_content;
use super::CursorPosition;
type CutContent = Option<(CursorPosition, CursorPosition, String)>;

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

impl From<Range> for Select {
    fn from(value: Range) -> Self {
        Self::Range(value.start.into(), value.end.into())
    }
}

impl Select {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

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

    pub fn len(&self, content: &[String]) -> usize {
        if let Some((from, to)) = self.get() {
            if from.line == to.line {
                return content[from.line][from.char..to.char].len();
            };
        }
        0
    }
}

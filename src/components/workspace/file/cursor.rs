use std::ops::{Add, RangeInclusive, Sub};

use super::action::ActionLogger;
use super::select::Select;
use super::utils::{backspace_indent_handler, derive_indent_from, get_closing_char, unindent_if_before_base_pattern};
use crate::configs::EditorConfigs;
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

impl CursorPosition {
    pub fn backspace(
        &mut self,
        mut select: Select,
        content: &mut Vec<String>,
        action_logger: &mut ActionLogger,
        configs: &EditorConfigs,
    ) {
        if content.is_empty() || self.line == 0 && self.char == 0 {
            return;
        }
        if let Some((from, ..)) = select.extract_logged(content, action_logger) {
            self.line = from.line;
            self.char = from.char;
            action_logger.finish_replace(*self, &content[self.line..=self.line]);
        } else if self.char == 0 {
            let prev_line_idx = self.line - 1;
            action_logger.init_replace(*self, &content[prev_line_idx..=self.line]);
            let current_line = content.remove(self.line);
            self.line -= 1;
            let prev_line = &mut content[self.line];
            self.char = prev_line.len();
            prev_line.push_str(&current_line);
            action_logger.finish_replace(*self, &content[self.line..=self.line]);
        } else {
            let line = &mut content[self.line];
            action_logger.prep_buffer(self, line);
            let offset = backspace_indent_handler(configs, line, self.char);
            self.offset_char(offset);
            action_logger.backspace(self);
        }
    }

    pub fn del(&mut self, content: &mut Vec<String>, mut select: Select, action_logger: &mut ActionLogger) {
        if content.is_empty() {
            return;
        }
        if let Some((from, ..)) = select.extract_logged(content, action_logger) {
            self.line = from.line;
            self.char = from.char;
            action_logger.finish_replace(*self, &content[self.line..=self.line]);
        } else if content[self.line].len() == self.char {
            if content.len() > self.line + 1 {
                action_logger.init_replace(*self, &content[self.line..=self.line + 1]);
                let next_line = content.remove(self.line + 1);
                content[self.line].push_str(&next_line);
                action_logger.finish_replace(*self, &content[self.line..=self.line])
            }
        } else {
            let line = &mut content[self.line];
            action_logger.del(self, line);
            line.remove(self.char);
        }
    }

    pub fn new_line(&mut self, content: &mut Vec<String>, action_logger: &mut ActionLogger, configs: &EditorConfigs) {
        if content.is_empty() {
            action_logger.init_replace(*self, &content[self.as_range()]);
            content.push(String::new());
            action_logger.finish_replace(*self, &content[self.line_range(0, 1)]);
            self.line += 1;
            return;
        }
        let prev_line = &mut content[self.line];
        action_logger.init_replace(*self, &[prev_line.to_owned()]);
        let mut line = if prev_line.len() >= self.char { prev_line.split_off(self.char) } else { String::new() };
        let indent = derive_indent_from(configs, prev_line);
        line.insert_str(0, &indent);
        self.line += 1;
        self.char = indent.len();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(last, first)) {
                    unindent_if_before_base_pattern(configs, &mut line);
                    content.insert(self.line, line);
                    content.insert(self.line, indent);
                    action_logger.finish_replace(*self, &content[self.line_range(1, 2)]);
                    return;
                }
            }
        }
        content.insert(self.line, line);
        action_logger.finish_replace(*self, &content[self.line_range(1, 1)]);
    }

    pub fn push(&mut self, ch: char, mut select: Select, content: &mut Vec<String>, action_logger: &mut ActionLogger) {
        if let Some((from, to)) = select.get_mut() {
            let replace = if let Some(closing) = get_closing_char(ch) {
                action_logger.init_replace_from_select(from, to, content);
                content[from.line].insert(from.char, ch);
                from.char += 1;
                if from.line == to.line {
                    to.char += 1;
                }
                content[to.line].insert(to.char, closing);
                from.line..to.line + 1
            } else {
                self.line = from.line;
                self.char = from.char;
                let (from, ..) = select.extract_logged(content, action_logger).unwrap();
                content[from.line].insert(from.char, ch);
                from.line..from.line + 1
            };
            self.char += 1;
            action_logger.finish_replace(*self, &content[replace]);
        } else if let Some(line) = content.get_mut(self.line) {
            action_logger.push_char(self, line, ch);
            line.insert(self.char, ch);
            self.char += 1;
            if let Some(closing) = get_closing_char(ch) {
                line.insert(self.char, closing);
                action_logger.inser_char(self, line, closing);
            }
        } else {
            action_logger.push_char(self, "", ch);
            content.insert(self.line, ch.to_string());
            self.char = 1;
        }
    }

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

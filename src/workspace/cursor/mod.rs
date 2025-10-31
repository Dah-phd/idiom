mod positions;
mod word;

use crate::{utils::Direction, workspace::line::EditorLine};
use idiom_tui::layout::Rect;
use lsp_types::Position;
use serde::{Deserialize, Serialize};

pub use positions::{CharRange, CursorPosition, Select, SelectPosition};
pub use word::{EncodedWordRange, PositionedWord, WordRange};

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,    // this is a char position not byte index
    phantm_char: usize, // keeps record for up/down movement
    pub at_line: usize,
    pub max_rows: usize,
    pub text_width: usize,
    select: Option<CursorPosition>,
}

impl Cursor {
    pub fn sized(editor_screen: Rect, offset: usize) -> Self {
        let text_width = editor_screen.width.saturating_sub(offset + 1);
        let max_rows = editor_screen.height as usize;
        Self { text_width, max_rows, ..Default::default() }
    }

    pub fn matches_content(&self, content: &[EditorLine]) -> bool {
        if content.len() <= self.line {
            return false;
        }
        if content[self.line].char_len() < self.char {
            return false;
        }
        if let Some(from) = self.select {
            if content.len() <= from.line || content.len() <= self.line {
                return false;
            }
            if content[from.line].char_len() < from.char || content[self.line].char_len() < self.char {
                return false;
            }
        }
        true
    }

    pub fn set_cursor_checked_with_select(&mut self, position: CursorPosition, content: &[EditorLine]) {
        self.set_cursor_checked(position, content);
        self.init_select();
    }

    pub fn set_cursor_checked(&mut self, mut position: CursorPosition, content: &[EditorLine]) {
        if self.line < position.line {
            let mut current_line_len = content[self.line].char_len();
            let mut offset = 0;
            while current_line_len > self.text_width && self.line < position.line.saturating_sub(offset) {
                current_line_len = current_line_len.saturating_sub(self.text_width);
                offset += 1;
            }
            position.line = position.line.saturating_sub(offset);
            if position.line == self.line && offset != 0 {
                position.char += offset * self.text_width;
            };
        };
        match content.get(position.line) {
            Some(line) => {
                if line.char_len() > position.char {
                    self.set_char(position.char);
                } else {
                    self.set_char(line.char_len());
                }
                self.line = position.line;
            }
            None => {
                self.line = content.len().saturating_sub(1);
                self.set_char(content[self.line].char_len())
            }
        }
    }

    pub fn match_content(&mut self, content: &[EditorLine]) {
        let Some(line) = content.get(self.line) else {
            self.select = None;
            self.end_of_file(content);
            return;
        };
        self.adjust_char(line);
        let Some(from) = self.select else { return };
        if (self.line < from.line) || (self.line == from.line && self.char <= from.char) {
            self.select = None;
        }
    }

    pub fn get_position(&self) -> CursorPosition {
        self.into()
    }

    pub fn set_position(&mut self, position: CursorPosition) {
        self.line = position.line;
        self.char = position.char;
        self.phantm_char = position.char;
    }

    pub fn add_to_char(&mut self, offset: usize) {
        self.char += offset;
        self.phantm_char = self.char;
    }

    pub fn sub_char(&mut self, offset: usize) {
        self.char -= offset;
        self.phantm_char = self.char;
    }

    #[inline(always)]
    pub fn set_char(&mut self, char: usize) {
        self.char = char;
        self.phantm_char = char;
    }

    pub fn end_of_line(&mut self, content: &[EditorLine]) {
        self.char = content[self.line].char_len();
        self.phantm_char = self.char;
    }

    pub fn end_of_file(&mut self, content: &[EditorLine]) {
        if !content.is_empty() {
            self.line = content.len() - 1;
            self.char = content[self.line].char_len();
        }
    }

    pub fn start_of_file(&mut self) {
        self.char = 0;
        self.at_line = 0;
        self.line = 0;
    }

    pub fn start_of_line(&mut self, content: &[EditorLine]) {
        self.char = 0;
        for ch in content[self.line].chars() {
            if !ch.is_whitespace() {
                self.phantm_char = self.char;
                return;
            }
            self.char += 1;
        }
    }

    // MOVEMENT

    pub fn up(&mut self, content: &[EditorLine]) {
        self.select = None;
        self.move_up(content)
    }

    fn move_up(&mut self, content: &[EditorLine]) {
        if self.text_width <= self.char {
            self.char -= self.text_width;
            return;
        }
        if self.line == 0 {
            self.set_char(0);
            return;
        }
        self.line -= 1;
        self.adjust_char(&content[self.line]);
    }

    pub fn screen_up(&mut self, content: &[EditorLine]) {
        self.select = None;
        self.line = self.line.saturating_sub(self.max_rows);
        self.at_line = self.at_line.saturating_sub(self.max_rows);
        self.adjust_char(&content[self.line]);
    }

    pub fn scroll_up(&mut self, content: &[EditorLine]) {
        if self.at_line != 0 {
            self.at_line -= 1;
        };
        self.up(content);
    }

    pub fn select_up(&mut self, content: &[EditorLine]) {
        self.init_select();
        self.move_up(content);
    }

    pub fn select_scroll_up(&mut self, content: &[EditorLine]) {
        self.init_select();
        if self.at_line != 0 {
            self.at_line -= 1;
        };
        self.move_up(content);
    }

    pub fn down(&mut self, content: &[EditorLine]) {
        self.select = None;
        self.move_down(content);
    }

    fn move_down(&mut self, content: &[EditorLine]) {
        if content.is_empty() {
            return;
        }
        let current_line_len = content[self.line].char_len();
        if current_line_len > self.char + self.text_width {
            self.char += self.text_width;
            return;
        }
        if content.len() <= self.line + 1 {
            self.char = current_line_len;
            return;
        }
        self.line += 1;
        self.adjust_char(&content[self.line]);
    }

    pub fn screen_down(&mut self, content: &[EditorLine]) {
        self.select = None;
        if content.is_empty() {
            return;
        };
        self.line = std::cmp::min(content.len() - 1, self.line + self.max_rows);
        self.at_line = std::cmp::min(content.len() - 1, self.at_line + self.max_rows);
        self.adjust_char(&content[self.line]);
    }

    pub fn scroll_down(&mut self, content: &[EditorLine]) {
        if self.at_line + 2 < content.len() {
            self.at_line += 1;
        };
        self.down(content);
    }

    pub fn select_down(&mut self, content: &[EditorLine]) {
        self.init_select();
        self.move_down(content);
    }

    pub fn select_scroll_down(&mut self, content: &[EditorLine]) {
        self.init_select();
        if self.at_line + 2 < content.len() {
            self.at_line += 1;
        };
        self.move_down(content);
    }

    pub fn left(&mut self, content: &[EditorLine]) {
        self.select = None;
        self.move_left(content);
    }

    fn move_left(&mut self, content: &[EditorLine]) {
        if self.char > 0 {
            self.char -= 1
        } else if self.line > 0 {
            self.line -= 1;
            if let Some(line) = content.get(self.line) {
                self.char = line.char_len();
            }
            if self.line < self.at_line {
                self.at_line -= 1;
            }
        }
        self.phantm_char = self.char;
    }

    pub fn jump_left(&mut self, content: &[EditorLine]) {
        self.select = None;
        self._jump_left(content);
    }

    pub fn jump_left_select(&mut self, content: &[EditorLine]) {
        self.init_select();
        self._jump_left(content);
    }

    fn _jump_left(&mut self, content: &[EditorLine]) {
        let mut line = &content[self.line][..self.char];
        let mut last_was_char = false;
        if line.is_empty() && self.line > 0 {
            self.move_left(content);
            line = &content[self.line][..self.char];
        }
        for ch in line.chars().rev() {
            if last_was_char && !ch.is_alphabetic() || self.char == 0 {
                self.phantm_char = self.char;
                return;
            }
            self.char -= 1;
            last_was_char = ch.is_alphabetic();
        }
    }

    pub fn select_left(&mut self, content: &[EditorLine]) {
        self.init_select();
        self.move_left(content);
    }

    pub fn right(&mut self, content: &[EditorLine]) {
        self.select = None;
        self.move_right(content);
    }

    fn move_right(&mut self, content: &[EditorLine]) {
        if let Some(line) = content.get(self.line) {
            if line.char_len() > self.char {
                self.char += 1
            } else if content.len() - 1 > self.line {
                self.line += 1;
                self.char = 0;
            }
        }
        self.phantm_char = self.char;
    }

    pub fn jump_right(&mut self, content: &[EditorLine]) {
        self.select = None;
        self._jump_right(content);
    }

    pub fn jump_right_select(&mut self, content: &[EditorLine]) {
        self.init_select();
        self._jump_right(content);
    }

    fn _jump_right(&mut self, content: &[EditorLine]) {
        let mut line = &content[self.line][self.char..];
        let mut last_was_char = false;
        if line.is_empty() && content.len() - 1 > self.line {
            self.move_right(content);
            line = &content[self.line][self.char..];
        }
        for ch in line.chars() {
            if last_was_char && !ch.is_alphabetic() {
                self.phantm_char = self.char;
                return;
            }
            self.char += 1;
            last_was_char = ch.is_alphabetic();
        }
    }

    pub fn select_right(&mut self, content: &[EditorLine]) {
        self.init_select();
        self.move_right(content);
    }

    pub fn adjust_max_line(&mut self, content: &[EditorLine]) {
        if self.line >= content.len() {
            self.line = content.len().saturating_sub(1);
            self.adjust_char(&content[self.line]);
        }
    }

    #[inline(always)]
    pub fn adjust_char(&mut self, line: &EditorLine) {
        self.char = self.phantm_char;
        if line.char_len() < self.char {
            self.char = line.char_len()
        }
    }

    pub fn add_line_offset(&mut self, offset: usize) {
        self.line += offset;
        self.at_line += offset;
        if let Some(from) = self.select.as_mut() {
            from.line += offset;
        }
    }

    // SELECT

    pub fn init_select(&mut self) {
        if self.select.is_none() {
            self.select.replace(CursorPosition { line: self.line, char: self.char });
        }
    }

    pub fn select_is_none(&self) -> bool {
        self.select.is_none()
    }

    pub fn select_drop(&mut self) {
        self.select = None;
    }

    pub fn select_to(&mut self, position: CursorPosition) {
        if position.line == self.line && position.char == self.char {
            return;
        }
        self.init_select();
        self.set_position(position);
    }

    pub fn select_set(&mut self, from: CursorPosition, to: CursorPosition) {
        self.set_position(to);
        self.select.replace(from);
    }

    pub fn select_get(&self) -> Option<Select> {
        match self.select.as_ref() {
            None => None,
            Some(from) => {
                let cursor = CursorPosition::from(self);
                if from.line > self.line || from.line == self.line && from.char > self.char {
                    Some((cursor, *from))
                } else {
                    Some((*from, cursor))
                }
            }
        }
    }

    pub fn select_get_direction(&self) -> Option<(CursorPosition, CursorPosition, Direction)> {
        match self.select.as_ref() {
            None => None,
            Some(from) => {
                let cursor = CursorPosition::from(self);
                if from.line > self.line || from.line == self.line && from.char > self.char {
                    Some((cursor, *from, Direction::Reversed))
                } else {
                    Some((*from, cursor, Direction::Normal))
                }
            }
        }
    }

    pub fn select_take(&mut self) -> Option<Select> {
        match self.select.take() {
            None => None,
            Some(from) => {
                let cursor = CursorPosition { line: self.line, char: self.char };
                if from.line > self.line || from.line == self.line && from.char > self.char {
                    Some((cursor, from))
                } else {
                    Some((from, cursor))
                }
            }
        }
    }

    pub fn select_take_direction(&mut self) -> Option<(CursorPosition, CursorPosition, Direction)> {
        match self.select.take() {
            None => None,
            Some(from) => {
                let to = CursorPosition { line: self.line, char: self.char };
                if from > to {
                    Some((to, from, Direction::Reversed))
                } else {
                    Some((from, to, Direction::Normal))
                }
            }
        }
    }

    pub fn select_len(&self, content: &[EditorLine]) -> usize {
        self.select_get()
            .map(|(from, to)| {
                if from.line == to.line {
                    return to.char - from.char;
                };
                let mut iter = content.iter().skip(from.line).take(to.line - from.line);
                let mut len = iter.next().map(|line| line.chars().skip(from.char).count()).unwrap_or_default() + 1;
                for line in iter {
                    len += line.char_len() + 1;
                }
                len + to.char
            })
            .unwrap_or_default()
    }

    /// retrn the starting point of select
    /// the value can be smaller or bigger than current poistion
    pub fn select_from_raw(&self) -> Option<CursorPosition> {
        self.select
    }

    pub fn select_word(&mut self, content: &[EditorLine]) {
        let Some(range) = WordRange::find_at(content, self.get_position()) else {
            return;
        };
        let (from, to) = range.as_select();
        self.select_set(from, to);
    }

    pub fn reset(&mut self) {
        self.line = 0;
        self.char = 0;
        self.phantm_char = 0;
        self.at_line = 0;
        self.select = None;
    }

    // MULTI CURSOR UTILS

    pub fn set_cursor(&mut self, other: &Cursor) {
        self.select = other.select;
        self.set_position(other.get_position());
    }

    pub fn clone_above(&mut self, content: &[EditorLine]) -> Option<Self> {
        let line = self.line.checked_sub(1)?;
        let char = std::cmp::min(self.char, content[line].char_len());
        let mut select = self.select;
        if let Some(position) = select.as_mut() {
            if position.line == 0 {
                position.char = 0;
            } else {
                position.line -= 1;
                position.char = std::cmp::min(position.char, content[position.line].char_len());
            }
        };
        Some(Self { line, char, text_width: self.text_width, select, ..Default::default() })
    }

    pub fn clone_below(&mut self, content: &[EditorLine]) -> Option<Self> {
        let line = self.line + 1;
        let char = std::cmp::min(self.char, content.get(line)?.char_len());
        let mut select = self.select;
        if let Some(position) = select.as_mut() {
            let next_line = position.line + 1;
            if next_line < content.len() {
                position.line = next_line;
                position.char = std::cmp::min(position.char, content[position.line].char_len());
            } else {
                position.char = content[position.line].char_len();
            }
        };
        Some(Self { line, char, text_width: self.text_width, select, ..Default::default() })
    }

    pub fn merge_if_intersect(&mut self, other: &Cursor) -> bool {
        let cursor = CursorPosition { line: self.line, char: self.char };
        let mut oth_cursor = CursorPosition { line: other.line, char: other.char };
        match self.select {
            Some(from) if from > cursor => match other.select {
                Some(mut oth_from) => {
                    // likely not needed match self direction
                    if oth_from < oth_cursor {
                        std::mem::swap(&mut oth_from, &mut oth_cursor);
                    };
                    if from >= oth_from && oth_from >= cursor {
                        self.select_set(from, std::cmp::min(oth_cursor, cursor));
                        return true;
                    };
                    if oth_from >= from && from >= oth_cursor {
                        self.select_set(oth_from, std::cmp::min(oth_cursor, cursor));
                        return true;
                    };
                }
                None => {
                    let oth_pos = CursorPosition { line: other.line, char: other.char };
                    return from >= oth_pos && oth_pos >= cursor;
                }
            },
            Some(from) if from < cursor => match other.select {
                Some(mut oth_from) => {
                    // likely not needed match self direction
                    if oth_from > oth_cursor {
                        std::mem::swap(&mut oth_from, &mut oth_cursor);
                    };
                    if from <= oth_from && oth_from <= cursor {
                        self.select_set(from, std::cmp::max(oth_cursor, cursor));
                        return true;
                    };
                    if oth_from <= from && from <= oth_cursor {
                        self.select_set(oth_from, std::cmp::max(oth_cursor, cursor));
                        return true;
                    };
                }
                None => {
                    let oth_pos = CursorPosition { line: other.line, char: other.char };
                    return from <= oth_pos && oth_pos <= cursor;
                }
            },
            Some(..) => match other.select {
                Some(oth_from) => {
                    if (oth_from >= cursor && cursor >= oth_cursor) || (oth_from <= cursor && cursor <= oth_cursor) {
                        self.select_set(oth_from, oth_cursor);
                        return true;
                    }
                }
                None => return oth_cursor == cursor,
            },
            None => match other.select {
                Some(oth_from) => {
                    let pos = CursorPosition { line: self.line, char: self.char };
                    if (oth_from >= pos && pos >= oth_cursor) || (oth_cursor >= pos && pos >= oth_from) {
                        self.select_set(oth_from, oth_cursor);
                        return true;
                    };
                }
                None => return self.line == other.line && self.char == other.char,
            },
        }
        false
    }
}

impl From<SelectPosition> for (CursorPosition, CursorPosition) {
    fn from(value: SelectPosition) -> Self {
        (value.from, value.to)
    }
}

impl From<&SelectPosition> for (CursorPosition, CursorPosition) {
    fn from(value: &SelectPosition) -> Self {
        (value.from, value.to)
    }
}

impl From<&mut Cursor> for CursorPosition {
    fn from(value: &mut Cursor) -> Self {
        Self { line: value.line, char: value.char }
    }
}

impl From<&Cursor> for CursorPosition {
    fn from(value: &Cursor) -> Self {
        Self { line: value.line, char: value.char }
    }
}

impl From<&Cursor> for Position {
    fn from(value: &Cursor) -> Self {
        Self { line: value.line as u32, character: value.char as u32 }
    }
}

impl From<&mut Cursor> for Position {
    fn from(value: &mut Cursor) -> Self {
        Self { line: value.line as u32, character: value.char as u32 }
    }
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

impl PartialEq<CursorPosition> for Cursor {
    fn eq(&self, position: &CursorPosition) -> bool {
        self.line == position.line && self.char == position.char
    }
}

impl From<&mut SelectPosition> for (CursorPosition, CursorPosition) {
    fn from(value: &mut SelectPosition) -> Self {
        (value.from, value.to)
    }
}

#[cfg(test)]
mod test;

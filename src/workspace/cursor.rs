use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::{execute, queue};
use lsp_types::Position;

use super::line::EditorLine;

pub type Select = (CursorPosition, CursorPosition);

#[derive(Debug, Default)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    phantm_char: usize, // keeps record for up/down movement
    pub at_line: usize,
    pub max_rows: usize,
    pub text_width: usize,
    select: Option<Select>,
}

impl Cursor {
    pub fn hide(writer: &mut impl std::io::Write) -> std::io::Result<()> {
        queue!(writer, Hide)
    }

    #[inline]
    pub fn render(
        &self,
        writer: &mut impl std::io::Write,
        area: crate::render::layout::Rect,
        offset: usize,
    ) -> std::io::Result<()> {
        let mut line = self.line;
        let mut char = self.char;
        while char >= self.text_width {
            line += 1;
            char -= self.text_width;
        }
        let row = area.row + (line - self.at_line) as u16;
        let col = area.col + (char + offset) as u16;
        execute!(writer, MoveTo(col, row), Show)
    }

    pub fn terminal_cursor_pos(&self, area: ratatui::layout::Rect, offset: usize) -> (u16, u16) {
        (area.x + (self.char + offset) as u16, area.y + (self.line - self.at_line) as u16)
    }

    pub fn set_cursor_checked_with_select(&mut self, position: CursorPosition, content: &[impl EditorLine]) {
        self.set_cursor_checked(position, content);
        self.init_select();
        self.push_to_select();
    }

    pub fn set_cursor_checked(&mut self, mut position: CursorPosition, content: &[impl EditorLine]) {
        if self.line < position.line {
            let mut current_line_len = content[self.line].len();
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
                if line.len() > position.char {
                    self.set_char(position.char);
                } else {
                    self.set_char(line.len());
                }
                self.line = position.line;
            }
            None => {
                self.line = content.len().saturating_sub(1);
                self.set_char(content[self.line].len())
            }
        }
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

    pub fn set_char(&mut self, char: usize) {
        self.char = char;
        self.phantm_char = char;
    }

    pub fn end_of_line(&mut self, content: &[impl EditorLine]) {
        self.char = content[self.line].len();
        self.phantm_char = self.char;
    }

    pub fn end_of_file(&mut self, content: &[impl EditorLine]) {
        if !content.is_empty() {
            self.line = content.len() - 1;
            self.char = content[self.line].len();
        }
    }

    pub fn start_of_file(&mut self) {
        self.char = 0;
        self.at_line = 0;
        self.line = 0;
    }

    pub fn start_of_line(&mut self, content: &[impl EditorLine]) {
        self.char = 0;
        for ch in content[self.line].as_str().chars() {
            if !ch.is_whitespace() {
                self.phantm_char = self.char;
                return;
            }
            self.char += 1;
        }
    }

    pub fn up(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self.move_up(content)
    }

    fn move_up(&mut self, content: &[impl EditorLine]) {
        if self.line == 0 {
            if self.char >= self.text_width {
                self.char -= self.text_width;
            };
            return;
        }
        if self.char >= self.text_width {
            self.char -= self.text_width;
            return;
        }
        self.line -= 1;
        let line_len = content[self.line].len();
        if line_len >= self.text_width {
            while self.char < line_len {
                self.char += self.text_width;
            }
            self.char -= self.text_width;
            return;
        };
        self.adjust_char(&content[self.line].as_str());
    }

    pub fn scroll_up(&mut self, content: &[impl EditorLine]) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.up(content)
        }
    }

    pub fn select_up(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self.move_up(content);
        self.push_to_select();
    }

    pub fn down(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self.move_down(content);
    }

    fn move_down(&mut self, content: &[impl EditorLine]) {
        if content.is_empty() {
            return;
        }
        let line_len = content[self.line].len();
        if line_len >= self.text_width && line_len.saturating_sub(self.char) > self.text_width {
            self.char += self.text_width;
            self.correct_cursor_wrapped_line();
            return;
        };
        if content.len() <= self.line + 1 {
            return;
        }
        self.line += 1;
        self.adjust_char(&content[self.line].as_str());
    }

    pub fn select_down(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self.move_down(content);
        self.push_to_select();
    }

    pub fn scroll_down(&mut self, content: &[impl EditorLine]) {
        if self.at_line + 2 < content.len() {
            self.at_line += 1;
            self.down(content)
        }
    }

    pub fn left(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self.move_left(content);
    }

    fn move_left(&mut self, content: &[impl EditorLine]) {
        if self.char > 0 {
            self.char -= 1
        } else if self.line > 0 {
            self.line -= 1;
            if let Some(line) = content.get(self.line) {
                self.char = line.len();
            }
            if self.line < self.at_line {
                self.at_line -= 1;
            }
        }
        self.phantm_char = self.char;
    }

    pub fn jump_left(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self._jump_left(content);
    }

    pub fn jump_left_select(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self._jump_left(content);
        self.push_to_select();
    }

    fn _jump_left(&mut self, content: &[impl EditorLine]) {
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

    pub fn select_left(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self.move_left(content);
        self.push_to_select();
    }

    pub fn right(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self.move_right(content);
    }

    fn move_right(&mut self, content: &[impl EditorLine]) {
        if let Some(line) = content.get(self.line) {
            if line.len() > self.char {
                self.char += 1
            } else if content.len() - 1 > self.line {
                self.line += 1;
                self.char = 0;
            }
        }
        self.phantm_char = self.char;
    }

    pub fn jump_right(&mut self, content: &[impl EditorLine]) {
        self.select = None;
        self._jump_right(content);
    }

    pub fn jump_right_select(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self._jump_right(content);
        self.push_to_select();
    }

    pub fn _jump_right(&mut self, content: &[impl EditorLine]) {
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

    pub fn select_right(&mut self, content: &[impl EditorLine]) {
        self.init_select();
        self.move_right(content);
        self.push_to_select();
    }

    pub fn adjust_max_line(&mut self, content: &[impl EditorLine]) {
        if self.line >= content.len() {
            self.line = content.len().saturating_sub(1);
            self.adjust_char(&content[self.line].as_str());
        }
    }

    pub fn adjust_char(&mut self, line: &str) {
        self.char = self.phantm_char;
        if line.len() < self.char {
            self.char = line.len()
        }
    }

    pub fn correct_cursor_position(&mut self) {
        if self.line < self.at_line {
            self.at_line = self.line
        }
        if self.line + 1 > self.max_rows + self.at_line {
            self.at_line = self.line - self.max_rows + 1
        }
    }

    fn correct_cursor_wrapped_line(&mut self) {
        if self.at_line == self.line {
            return;
        }
        let pseudo_line = self.line + self.char / self.text_width;
        if self.at_line + self.max_rows < pseudo_line + 1 {
            self.at_line += 1;
        }
    }

    pub fn init_select(&mut self) {
        if self.select.is_none() {
            let position = self.into();
            self.select.replace((position, position));
        }
    }

    pub fn push_to_select(&mut self) {
        if let Some((_, to)) = self.select.as_mut() {
            *to = CursorPosition { line: self.line, char: self.char };
        }
    }

    pub fn select_is_none(&self) -> bool {
        self.select.is_none()
    }

    pub fn select_line_offset(&mut self, offset: usize) {
        if let Some((from, to)) = self.select.as_mut() {
            from.line += offset;
            to.line += offset;
        }
    }

    pub fn select_get(&self) -> Option<Select> {
        match self.select.as_ref() {
            None => None,
            Some((from, to)) => {
                if from.line > to.line || from.line == to.line && from.char > to.char {
                    Some((*to, *from))
                } else {
                    Some((*from, *to))
                }
            }
        }
    }

    pub fn select_drop(&mut self) {
        self.select = None;
    }

    pub fn select_set(&mut self, from: CursorPosition, to: CursorPosition) {
        self.set_position(to);
        self.select.replace((from, to));
    }

    pub fn select_replace(&mut self, select: Option<Select>) {
        self.select = select;
        if let Some((_, to)) = self.select {
            self.set_position(to);
        };
    }

    pub fn select_take(&mut self) -> Option<Select> {
        match self.select.take() {
            None => None,
            Some((from, to)) => {
                if from.line > to.line || from.line == to.line && from.char > to.char {
                    Some((to, from))
                } else {
                    Some((from, to))
                }
            }
        }
    }

    pub fn select_len(&self, content: &[impl EditorLine]) -> usize {
        self.select_get()
            .map(|(from, to)| {
                if from.line == to.line {
                    return to.char - from.char;
                };
                let mut iter = content.iter().skip(from.line).take(to.line - from.line);
                let mut len =
                    iter.next().map(|line| line.as_str().chars().skip(from.char).count()).unwrap_or_default() + 1;
                for line in iter {
                    len += line.len() + 1;
                }
                len + to.char
            })
            .unwrap_or_default()
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

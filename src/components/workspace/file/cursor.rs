use lsp_types::Position;

#[derive(Debug, Default)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    phantm_char: usize, // keeps record for up/down movement
    pub at_line: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl Cursor {
    pub fn position(&self) -> CursorPosition {
        self.into()
    }

    pub fn set_position(&mut self, position: CursorPosition) {
        self.line = position.line;
        self.char = position.char;
        self.phantm_char = position.char;
    }

    pub fn add_char(&mut self, offset: usize) {
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

    pub fn end_of_line(&mut self, content: &[String]) {
        self.char = content[self.line].len();
        self.phantm_char = self.char;
    }

    pub fn end_of_file(&mut self, content: &[String]) {
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

    pub fn start_of_line(&mut self, content: &[String]) {
        self.char = 0;
        for ch in content[self.line].chars() {
            if !ch.is_whitespace() {
                self.phantm_char = self.char;
                return;
            }
            self.char += 1;
        }
    }

    pub fn up(&mut self, content: &[String]) {
        self.select = None;
        self.move_up(content)
    }

    fn move_up(&mut self, content: &[String]) {
        if self.line > 0 {
            self.line -= 1;
            self.adjust_char(content);
        }
    }

    pub fn scroll_up(&mut self, content: &[String]) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.up(content)
        }
    }

    pub fn select_up(&mut self, content: &[String]) {
        self.init_select();
        self.move_up(content);
        self.push_to_select();
    }

    pub fn down(&mut self, content: &[String]) {
        self.select = None;
        self.move_down(content);
    }

    fn move_down(&mut self, content: &[String]) {
        if content.is_empty() || content.len() - 1 <= self.line {
            return;
        }
        self.line += 1;
        self.adjust_char(content);
    }

    pub fn select_down(&mut self, content: &[String]) {
        self.init_select();
        self.move_down(content);
        self.push_to_select();
    }

    pub fn scroll_down(&mut self, content: &[String]) {
        if self.at_line < content.len() - 2 {
            self.at_line += 1;
            self.down(content)
        }
    }

    pub fn left(&mut self, content: &[String]) {
        self.select = None;
        self.move_left(content);
    }

    fn move_left(&mut self, content: &[String]) {
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

    pub fn jump_left(&mut self, content: &[String]) {
        self.select = None;
        self._jump_left(content);
    }

    pub fn jump_left_select(&mut self, content: &[String]) {
        self.init_select();
        self._jump_left(content);
        self.push_to_select();
    }

    fn _jump_left(&mut self, content: &[String]) {
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

    pub fn select_left(&mut self, content: &[String]) {
        self.init_select();
        self.move_left(content);
        self.push_to_select();
    }

    pub fn right(&mut self, content: &[String]) {
        self.select = None;
        self.move_right(content);
    }

    fn move_right(&mut self, content: &[String]) {
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

    pub fn jump_right(&mut self, content: &[String]) {
        self.select = None;
        self._jump_right(content);
    }

    pub fn jump_right_select(&mut self, content: &[String]) {
        self.init_select();
        self._jump_right(content);
        self.push_to_select();
    }

    pub fn _jump_right(&mut self, content: &[String]) {
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

    pub fn select_right(&mut self, content: &[String]) {
        self.init_select();
        self.move_right(content);
        self.push_to_select();
    }

    pub fn adjust_max_line(&mut self, content: &[String]) {
        if self.line >= content.len() {
            self.line = content.len() - 1;
            self.adjust_char(content);
        }
    }

    pub fn adjust_char(&mut self, content: &[String]) {
        if let Some(line) = content.get(self.line) {
            self.char = self.phantm_char;
            if line.len() < self.char {
                self.char = line.len()
            }
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

    pub fn select_get(&self) -> Option<(&CursorPosition, &CursorPosition)> {
        match self.select.as_ref() {
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

    pub fn select_drop(&mut self) {
        self.select = None;
    }

    pub fn select_set(&mut self, from: CursorPosition, to: CursorPosition) {
        self.set_position(to);
        self.select.replace((from, to));
    }

    pub fn select_take(&mut self) -> Option<(CursorPosition, CursorPosition)> {
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

    pub fn select_len(&self, content: &[String]) -> usize {
        if let Some((from, to)) = self.select_get() {
            if from.line == to.line {
                return content[from.line][from.char..to.char].len();
            };
            let mut len = 0;
            for line in content[from.line..to.line].iter() {
                len += line.len();
            }
            return len;
        }
        0
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

#[derive(Default, Debug)]
pub struct Position {
    pub line: usize,
    pub char: usize,
}

#[derive(Default, Debug)]
pub struct Cursor {
    pub position: Position,
    pub max_rows: u16,
    pub at_line: usize,
}

impl Cursor {
    fn adjust_cursor_max_char(&mut self, content: &mut Vec<String>) {
        if let Some(line) = content.get(self.position.line) {
            if line.len() < self.position.char {
                self.position.char = line.len()
            }
        }
    }

    pub fn navigate_up_content(&mut self, content: &mut Vec<String>) {
        if self.at_line >= self.position.line {
            self.scroll_up_content(content)
        } else if self.position.line > 0 {
            self.position.line -= 1;
            self.adjust_cursor_max_char(content);
        }
    }

    pub fn scroll_down_content(&mut self, content: &mut Vec<String>) {
        if self.at_line < content.len() - 2 {
            self.at_line += 1;
            self.navigate_down_content(content)
        }
    }

    pub fn scroll_up_content(&mut self, content: &mut Vec<String>) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.navigate_up_content(content)
        }
    }

    pub fn navigate_down_content(&mut self, content: &mut Vec<String>) {
        if self.position.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if content.len() - 1 > self.position.line {
            self.position.line += 1;
            self.adjust_cursor_max_char(content);
        }
    }

    pub fn navigate_left_content(&mut self, content: &mut Vec<String>) {
        if self.position.char > 0 {
            self.position.char -= 1
        } else if self.position.line > 0 {
            self.position.line -= 1;
            if let Some(line) = content.get(self.position.line) {
                self.position.char = line.len();
            }
        }
    }

    pub fn navigate_right_content(&mut self, content: &mut Vec<String>) {
        if let Some(line) = content.get(self.position.line) {
            if line.len() > self.position.char {
                self.position.char += 1
            } else if content.len() - 1 > self.position.line {
                self.position.line += 1;
                self.position.char = 0;
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct Select;

#[derive(Default, Debug)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    pub max_rows: u16,
    pub at_line: usize,
    pub selected: Vec<Select>,
    pub clipboard: Vec<String>,
}

impl Cursor {
    fn adjust_cursor_max_char(&mut self, content: &mut Vec<String>) {
        if let Some(line) = content.get(self.line) {
            if line.len() < self.char {
                self.char = line.len()
            }
        }
    }

    pub fn navigate_up_content(&mut self, content: &mut Vec<String>) {
        if self.at_line >= self.line {
            self.scroll_up_content(content)
        } else if self.line > 0 {
            self.line -= 1;
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
        if self.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if content.len() - 1 > self.line {
            self.line += 1;
            self.adjust_cursor_max_char(content);
        }
    }

    pub fn navigate_left_content(&mut self, content: &mut Vec<String>) {
        if self.char > 0 {
            self.char -= 1
        } else if self.line > 0 {
            self.line -= 1;
            if let Some(line) = content.get(self.line) {
                self.char = line.len();
            }
        }
    }

    pub fn navigate_right_content(&mut self, content: &mut Vec<String>) {
        if let Some(line) = content.get(self.line) {
            if line.len() > self.char {
                self.char += 1
            } else if content.len() - 1 > self.line {
                self.line += 1;
                self.char = 0;
            }
        }
    }

    pub fn paste(&mut self, content: &mut Vec<String>) {
        if let Some(clip) = self.clipboard.pop() {
            content.insert(self.line, clip);
            self.line += 1;
        }
    }

    pub fn copy(&mut self, content: &mut Vec<String>) {
        if self.selected.is_empty() {
            if let Some(line) = content.get(self.line) {
                self.clipboard.push(line.to_owned())
            }
        }
    }

    pub fn cut(&mut self, content: &mut Vec<String>) {
        if self.selected.is_empty() {
            self.clipboard.push(content.remove(self.line));
            if self.line >= content.len() {
                self.line -= 1;
            }
        }
    }
}

mod action;
mod clipboard;
mod select;
use clipboard::Clipboard;
use select::Select;

#[derive(Default, Debug)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    pub max_rows: u16,
    pub at_line: usize,
    pub selected: Select,
    clipboard: Clipboard,
    should_paste_line: bool,
}

impl Cursor {
    fn adjust_cursor_max_char(&mut self, content: &mut [String]) {
        if let Some(line) = content.get(self.line) {
            if line.len() < self.char {
                self.char = line.len()
            }
        }
    }

    fn up(&mut self, content: &mut Vec<String>) {
        if self.at_line >= self.line {
            self.scroll_up_content(content)
        } else if self.line > 0 {
            self.line -= 1;
            self.adjust_cursor_max_char(content);
        }
    }

    pub fn scroll_up_content(&mut self, content: &mut Vec<String>) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.navigate_up_content(content)
        }
    }

    pub fn navigate_up_content(&mut self, content: &mut Vec<String>) {
        self.selected.drop();
        self.up(content)
    }

    pub fn select_up_content(&mut self, content: &mut Vec<String>) {
        self.selected.init(self.line, self.char);
        self.up(content);
        self.selected.push(self.line, self.char);
    }

    pub fn swap_up_line(&mut self, content: &mut Vec<String>) {
        if self.at_line >= self.line {
            self.scroll_up_content(content)
        } else if self.line > 0 {
            let new_line = self.line - 1;
            content.swap(self.line, new_line);
            self.line = new_line;
        }
    }

    fn down(&mut self, content: &mut Vec<String>) {
        if content.is_empty() {
            return;
        }
        if self.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if content.len() - 1 > self.line {
            self.line += 1;
            self.adjust_cursor_max_char(content);
        }
    }

    pub fn scroll_down_content(&mut self, content: &mut Vec<String>) {
        if self.at_line < content.len() - 2 {
            self.at_line += 1;
            self.navigate_down_content(content)
        }
    }

    pub fn navigate_down_content(&mut self, content: &mut Vec<String>) {
        self.selected.drop();
        self.down(content);
    }

    pub fn select_down_content(&mut self, content: &mut Vec<String>) {
        self.selected.init(self.line, self.char);
        self.down(content);
        self.selected.push(self.line, self.char);
    }

    pub fn swap_down_line(&mut self, content: &mut Vec<String>) {
        if content.is_empty() {
            return;
        }
        if self.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if content.len() - 1 > self.line {
            let new_line = self.line + 1;
            content.swap(self.line, new_line);
            self.line = new_line;
        }
    }

    fn left(&mut self, content: &[String]) {
        if self.char > 0 {
            self.char -= 1
        } else if self.line > 0 {
            self.line -= 1;
            if let Some(line) = content.get(self.line) {
                self.char = line.len();
            }
        }
    }

    pub fn left_jump(&mut self, content: &[String]) {
        let mut line = &content[self.line][..self.char];
        let mut last_was_char = false;
        loop {
            if line.is_empty() || line.chars().all(|c| !c.is_alphabetic() && c != '_') {
                if self.line > 0 {
                    self.line -= 1;
                    line = &content[self.line];
                    self.char = line.len();
                } else {
                    return;
                }
            }
            for ch in line.chars().rev() {
                if last_was_char && !ch.is_alphabetic() && ch != '_' || self.char == 0 {
                    if self.at_line >= self.line && self.at_line > 0 {
                        self.at_line -= 1;
                    }
                    return;
                }
                self.char -= 1;
                if ch.is_alphabetic() || ch == '_' {
                    last_was_char = true;
                };
            }
        }
    }

    pub fn navigate_left_content(&mut self, content: &mut [String]) {
        self.selected.drop();
        self.left(content);
    }

    pub fn select_left_content(&mut self, content: &mut [String]) {
        self.selected.init(self.line, self.char);
        self.left(content);
        self.selected.push(self.line, self.char);
    }

    fn right(&mut self, content: &mut Vec<String>) {
        if let Some(line) = content.get(self.line) {
            if line.len() > self.char {
                self.char += 1
            } else if content.len() - 1 > self.line {
                self.line += 1;
                self.char = 0;
            }
        }
    }

    pub fn right_jump(&mut self, content: &[String]) {
        let mut line = &content[self.line][self.char..];
        let mut found_word = false;
        let mut last_was_char = false;
        loop {
            if line.is_empty() || line.chars().all(|c| !c.is_alphabetic() && c != '_') {
                if content.len() - 1 > self.line {
                    self.line += 1;
                    self.char = 0;
                    line = &content[self.line];
                } else {
                    return;
                }
            }
            for ch in line.chars() {
                if last_was_char && found_word && !ch.is_alphabetic() && ch != '_' {
                    if self.line > self.max_rows as usize - 3 + self.at_line {
                        self.at_line += 1;
                    }
                    return;
                }
                self.char += 1;
                if !found_word && ch.is_alphabetic() || ch == '_' {
                    last_was_char = true;
                    found_word = true;
                };
            }
        }
    }

    pub fn navigate_right_content(&mut self, content: &mut Vec<String>) {
        self.selected.drop();
        self.right(content);
    }

    pub fn select_right_content(&mut self, content: &mut Vec<String>) {
        self.selected.init(self.line, self.char);
        self.right(content);
        self.selected.push(self.line, self.char);
    }

    fn insert_clip(&mut self, clip: String, content: &mut Vec<String>) {
        let mut lines: Vec<_> = clip.split('\n').collect();
        if lines.is_empty() {
            return;
        }
        if self.should_paste_line && lines.len() == 2 && lines[1].is_empty() {
            content.insert(self.line, lines[0].into())
        } else if lines.len() == 1 {
            let text = lines[0];
            content[self.line].insert_str(self.char, lines[0]);
            self.char += text.len();
        } else {
            let line = content.remove(self.line);
            let (prefix, suffix) = line.split_at(self.char);
            let mut first_line = prefix.to_owned();
            if lines.len() == 1 {
                first_line.push_str(lines[0]);
                content.insert(self.line, first_line);
                content.insert(self.line + 1, suffix.to_owned());
            } else {
                first_line.push_str(lines.remove(0));
                content.insert(self.line, first_line);
                let last_idx = lines.len() - 1;
                for (idx, select) in lines.iter().enumerate() {
                    let next_line = if idx == last_idx {
                        let mut last_line = select.to_string();
                        self.char = last_line.len();
                        last_line.push_str(suffix);
                        last_line
                    } else {
                        select.to_string()
                    };
                    content.insert(self.line + 1, next_line);
                    self.down(content);
                }
            }
        }
    }

    pub fn paste(&mut self, content: &mut Vec<String>) {
        if let Some(clip) = self.clipboard.get() {
            self.insert_clip(clip, content)
        }
    }

    pub fn copy(&mut self, content: &[String]) {
        self.should_paste_line = false;
        if let Some((from, to)) = self.selected.get() {
            if from.0 == to.0 {
                self.clipboard.push(content[from.0][from.1..to.1].to_owned());
            } else {
                let mut at_line = from.0;
                let mut clip_vec = Vec::new();
                clip_vec.push(content[from.0][from.1..].to_owned());
                while at_line < to.0 {
                    at_line += 1;
                    if at_line != to.0 {
                        clip_vec.push(content[at_line].to_owned())
                    } else {
                        clip_vec.push(content[at_line][..to.1].to_owned())
                    }
                }
                self.clipboard.push(clip_vec.join("\n"));
            }
        } else {
            self.should_paste_line = true;
            let mut line = content[self.line].to_owned();
            line.push('\n');
            self.clipboard.push(line);
        }
    }

    pub fn remove(&mut self, content: &mut Vec<String>) -> String {
        if let Some((from, to)) = self.selected.get() {
            let clip = if from.0 == to.0 {
                self.char = from.1;
                let data = content.remove(from.0);
                let mut payload = String::from(&data[..from.1]);
                payload.push_str(&data[to.1..]);
                content.insert(from.0, payload);
                data[from.1..to.1].to_owned()
            } else {
                let mut clip_vec = vec![content[from.0].split_off(from.1)];
                let mut last_line = to.0;
                while from.0 < last_line {
                    last_line -= 1;
                    if from.0 == last_line {
                        let final_clip = content.remove(from.0 + 1);
                        let (clipped, remaining) = final_clip.split_at(to.1);
                        content[from.0].push_str(remaining);
                        clip_vec.push(clipped.to_owned())
                    } else {
                        clip_vec.push(content.remove(from.0 + 1))
                    }
                }
                self.line = from.0;
                self.char = from.1;
                clip_vec.join("\n")
            };
            self.selected.drop();
            clip
        } else {
            let mut clip = content.remove(self.line);
            clip.push('\n');
            if self.line >= content.len() {
                self.line -= 1;
                self.char = content[self.line].len() - 1;
            } else {
                self.char = 0;
            }
            clip
        }
    }

    pub fn cut(&mut self, content: &mut Vec<String>) {
        self.should_paste_line = false;
        let cut_content = self.remove(content);
        self.clipboard.push(cut_content);
    }
}

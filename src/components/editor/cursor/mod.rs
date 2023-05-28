mod select;
use select::{Clip, Select};

#[derive(Default, Debug)]
pub struct Cursor {
    pub line: usize,
    pub char: usize,
    pub max_rows: u16,
    pub at_line: usize,
    pub selected: Select,
    clipboard: Vec<Clip>,
}

impl Cursor {
    fn adjust_cursor_max_char(&mut self, content: &mut [String]) {
        if let Some(line) = content.get(self.line) {
            if line.len() < self.char {
                self.char = line.len()
            }
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

    fn up(&mut self, content: &mut Vec<String>) {
        if self.at_line >= self.line {
            self.scroll_up_content(content)
        } else if self.line > 0 {
            self.line -= 1;
            self.adjust_cursor_max_char(content);
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

    fn down(&mut self, content: &mut Vec<String>) {
        if self.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if content.len() - 1 > self.line {
            self.line += 1;
            self.adjust_cursor_max_char(content);
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

    pub fn navigate_right_content(&mut self, content: &mut Vec<String>) {
        self.selected.drop();
        self.right(content);
    }

    pub fn select_right_content(&mut self, content: &mut Vec<String>) {
        self.selected.init(self.line, self.char);
        self.right(content);
        self.selected.push(self.line, self.char);
    }

    pub fn paste(&mut self, content: &mut Vec<String>) {
        if let Some(clip) = self.clipboard.pop() {
            match clip {
                Clip::Line(line) => content.insert(self.line, line),
                Clip::Text(text) => {
                    if let Some(current_line) = content.get_mut(self.line) {
                        current_line.insert_str(self.char, &text);
                        self.char += text.len();
                    }
                }
                Clip::Section(vec) => {
                    let len = vec.len();
                    if len == 0 {
                        return;
                    }
                    let line = content.remove(self.line);
                    let (prefix, suffix) = line.split_at(self.char);
                    let mut first_line = prefix.to_owned();
                    if len == 1 {
                        first_line.push_str(&vec[0]);
                        content.insert(self.line, first_line);
                        content.insert(self.line + 1, suffix.to_owned());
                    } else {
                        let mut iter_select = vec.iter().enumerate();
                        if let Some((_, first_select)) = iter_select.next() {
                            first_line.push_str(first_select);
                            content.insert(self.line, first_line);
                        }
                        for (idx, select) in iter_select {
                            let next_line = if idx == len - 1 {
                                let mut last_line = select.to_owned();
                                self.char = last_line.len();
                                last_line.push_str(suffix);
                                last_line
                            } else {
                                select.to_owned()
                            };
                            content.insert(self.line + 1, next_line);
                            self.down(content);
                        }
                    }
                }
            }
        }
    }

    pub fn copy(&mut self, content: &[String]) {
        match self.selected {
            Select::None => {
                if let Some(line) = content.get(self.line) {
                    self.clipboard.push(Clip::Line(line.to_owned()));
                }
            }
            Select::Range(from, to) => {
                if from.0 == to.0 {
                    if let Some(line) = content.get(from.0) {
                        self.clipboard.push(Clip::Text(line[from.1..to.1].to_owned()));
                    }
                } else {
                    let mut at_line = from.0;
                    let mut clip_vec = Vec::new();
                    if let Some(first_line) = content.get(from.0) {
                        clip_vec.push(first_line[from.1..].to_owned())
                    }
                    while at_line < to.0 {
                        at_line += 1;
                        if let Some(line) = content.get(at_line) {
                            if at_line != to.0 {
                                clip_vec.push(line.to_owned())
                            } else {
                                clip_vec.push(line[..to.1].to_owned())
                            }
                        }
                    }
                    self.clipboard.push(Clip::Section(clip_vec))
                }
            }
            _ => {}
        }
    }

    pub fn cut(&mut self, content: &mut Vec<String>) {
        match self.selected {
            Select::None => {
                self.clipboard.push(Clip::Line(content.remove(self.line)));
                if self.line >= content.len() {
                    self.line -= 1;
                    self.char = content[self.line].len() - 1;
                } else {
                    self.char = 0;
                }
            }
            Select::Range(from, to) => {
                if from.0 == to.0 {
                    self.char = from.1;
                    let data = content.remove(from.0);
                    self.clipboard.push(Clip::Text(data[from.1..to.1].to_owned()));
                    let mut payload = String::new();
                    payload.push_str(&data[..from.1]);
                    payload.push_str(&data[to.1..]);
                    content.insert(from.0, payload);
                } else {
                    let mut last_line = to.0;
                    let mut clip_vec = Vec::new();
                    clip_vec.push(content.remove(from.0)[from.1..].to_owned());
                    while from.0 < last_line {
                        last_line -= 1;
                        if from.0 == last_line {
                            if let Some(last_line) = content.get_mut(from.0) {
                                clip_vec.push(last_line[..to.1].to_owned());
                                (*last_line) = String::from(&last_line[to.1..]);
                            }
                        } else {
                            clip_vec.push(content.remove(from.0))
                        }
                    }
                    self.clipboard.push(Clip::Section(clip_vec));
                    self.line = from.0;
                    self.char = from.1;
                    self.selected.drop();
                }
            }
            _ => {}
        }
    }
}

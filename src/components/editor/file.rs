use std::path::PathBuf;
use crate::messages::FileType;

const INDENT_ENDINGS: &str = ":({";
const UNIDENT_ENDINGS: &str = ")}";
const INDENT_TYPE: &str = "    ";
#[derive(Debug)]
pub struct Editor {
    pub content: Vec<String>,
    pub cursor: (usize, usize), // line, char
    pub at_line: usize,
    pub path: PathBuf,
    pub max_rows: u16,
    pub file_type: FileType,
}

impl Editor {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            content: content.lines().map(String::from).collect(),
            cursor: (0, 0),
            at_line: 0,
            file_type: FileType::derive_type(&path),
            path,
            max_rows: 0,
        })
    }

    pub fn scroll_down(&mut self) {
        if self.at_line < self.content.len() - 2 {
            self.at_line += 1;
            self.navigate_down()
        }
    }

    pub fn scroll_up(&mut self) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.navigate_up()
        }
    }

    pub fn navigate_up(&mut self) {
        if self.at_line >= self.cursor.0 {
            self.scroll_up()
        } else if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn navigate_down(&mut self) {
        if self.cursor.0 > self.max_rows as usize - 3 + self.at_line  {
            self.at_line += 1;
        }
        if self.content.len() - 1 > self.cursor.0 {
            self.cursor.0 += 1;
            self.adjust_cursor_max_char();
        }
    }

    fn adjust_cursor_max_char(&mut self) {
        if let Some(line) = self.content.get(self.cursor.0) {
            if line.len() < self.cursor.1 {
                self.cursor.1 = line.len()
            }
        }
    }

    pub fn navigate_left(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1
        } else if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            if let Some(line) = self.content.get(self.cursor.0) {
                self.cursor.1 = line.len();
            }
        }
    }
    pub fn navigate_right(&mut self) {
        if let Some(line) = self.content.get(self.cursor.0) {
            if line.len() > self.cursor.1 {
                self.cursor.1 += 1
            } else if self.content.len() - 1 > self.cursor.0 {
                self.cursor.0 += 1;
                self.cursor.1 = 0;
            }
        }
    }

    pub fn push_str(&mut self, c: &str) {
        if let Some(line) = self.content.get_mut(self.cursor.0) {
            line.insert_str(self.cursor.1, c);
            self.cursor.1 += c.len();
        }
    }

    pub fn backspace(&mut self) {
        // TODO needs work
        if self.cursor.0 != 0 {
            let previous = self.content.get(self.cursor.0 - 1).cloned();
            let current = self.content.get_mut(self.cursor.0);
            if let Some(line) = current {
                let (frist, second) = line.split_at(self.cursor.1);
                if frist.is_empty() {
                    if let Some(previous) = previous {
                        let prev_len = previous.len();
                        (*line) = format!("{}{}", previous, second);
                        self.cursor.0 -= 1;
                        self.content.remove(self.cursor.0);
                        self.cursor.1 = prev_len;
                        return;
                    }
                } else {
                    (*line) = format!("{}{}", &frist[..frist.len() - 1], second)
                }
            }
        } else if let Some(line) = self.content.get_mut(self.cursor.0) {
            let (first, second) = line.split_at(self.cursor.1);
            if !first.is_empty() {
                (*line) = format!("{}{}", &first[..first.len() - 1], second);
            };
        }
        if self.cursor.1 != 0 {
            self.cursor.1 -= 1;
        }
    }

    pub fn del(&mut self) {
        // TODO needs work
        let next_line = self.content.get(self.cursor.0 + 1).cloned();
        let current_line = self.content.get_mut(self.cursor.0);
        if let Some(line) = current_line {
            let (first, second) = line.split_at(self.cursor.1);
            if second.is_empty() {
                if let Some(new_content) = next_line {
                    line.push_str(new_content.as_str());
                    self.content.remove(self.cursor.0 + 1);
                }
            } else {
                (*line) = format!("{}{}", first, &second[1..]);
            }
        }
    }

    pub fn indent(&mut self) {
        self.push_str(INDENT_TYPE)
    }

    pub fn unindent(&mut self) {
        if let Some(line) = self.content.get_mut(self.cursor.0) {
            if line.starts_with(INDENT_TYPE) {
                line.strip_prefix(INDENT_TYPE);
            }
        }
    }

    pub fn new_line(&mut self) {
        if let Some(line) = self.content.get(self.cursor.0) {
            let new_line = if line.len() - 1 > self.cursor.1 {
                let (replace_line, new_line) = line.split_at(self.cursor.1);
                let new_line = String::from(new_line);
                self.content[self.cursor.0] = String::from(replace_line);
                new_line
            } else {
                String::new()
            };
            self.content.insert(self.cursor.0 + 1, new_line);
            self.cursor.0 += 1;
            self.cursor.1 = 0;
            self.get_indent();
        }
    }

    fn get_indent(&mut self) {
        if self.cursor.0 != 0 {
            if let Some(mut prev_line) = self.content.get(self.cursor.0).cloned() {
                if let Some(last) = prev_line.trim_end().chars().last() {
                    if INDENT_ENDINGS.contains(last) {
                        self.indent()
                    }
                    if UNIDENT_ENDINGS.contains(last) {
                        self.unindent()
                    }
                }
                if prev_line.starts_with(INDENT_TYPE) {
                    self.indent();
                    prev_line = prev_line[INDENT_TYPE.len()..].to_owned()
                }
            }
        }
    }

    pub fn save(&self) {
        std::fs::write(&self.path, self.content.join("\n")).unwrap();
    }
}

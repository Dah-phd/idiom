use std::path::PathBuf;
#[derive(Debug)]
pub struct Editor {
    pub content: Vec<String>,
    pub cursor: (usize, usize), // line, char
    pub at_line: usize,
    pub path: PathBuf,
}

impl Editor {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            content: content.split('\n').map(String::from).collect(),
            cursor: (0, 0),
            at_line: 0,
            path,
        })
    }

    pub fn scroll_down(&mut self) {
        if self.at_line < self.content.len() - 2 {
            self.at_line += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.at_line != 0 {
            self.at_line -= 1;
        }
    }

    pub fn navigate_up(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn navigate_down(&mut self) {
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
        self.push_str("    ")
    }

    pub fn new_line(&mut self) {
        if let Some(line) = self.content.get(self.cursor.0) {
            let indent = String::from("    ");
            if line.len() - 1 > self.cursor.1 {
                let (replace_line, new_line) = line.split_at(self.cursor.1);
                let new_line = String::from(new_line);
                self.content[self.cursor.0] = String::from(replace_line);
                self.content.insert(self.cursor.0 + 1, new_line);
            } else {
                self.content.insert(self.cursor.0 + 1, String::new())
            }
            self.cursor.0 += 1;
            self.cursor.1 = 0;
        }
    }

    pub fn save(&self) {
        std::fs::write(&self.path, self.content.join("\n")).unwrap();
    }

    pub fn compare(&self) -> Option<Vec<(usize, String)>> {
        let mut deltas = vec![];
        let new_content = std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .split('\n')
            .map(String::from)
            .collect::<Vec<_>>();
        let max = if self.content.len() > new_content.len() {
            self.content.len()
        } else {
            new_content.len()
        };

        let empty_str = String::from("");
        for idx in 0..max {
            let line = self.content.get(idx).unwrap_or(&empty_str);
            let new_line = new_content.get(idx).unwrap_or(&empty_str);
            if line != new_line {
                deltas.push((idx, format!("\nOLD LINE:\n{}\n NEW LINE:\n{}\n", line, new_line)))
            }
        }

        if deltas.is_empty() {
            return None;
        }

        Some(deltas)
    }
}

use crate::{
    messages::{EditorConfigs, FileType},
    utils::trim_start_inplace,
};
use std::path::PathBuf;

use super::cursor::Cursor;

#[derive(Debug)]
pub struct Editor {
    pub configs: EditorConfigs,
    pub cursor: Cursor,
    pub content: Vec<String>,
    pub path: PathBuf,
    pub file_type: FileType,
}

impl Editor {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            configs: EditorConfigs::default(),
            cursor: Cursor::default(),
            content: content.lines().map(String::from).collect(),
            file_type: FileType::derive_type(&path),
            path,
        })
    }

    pub fn is_saved(&self) -> bool {
        if let Ok(file_content) = std::fs::read_to_string(&self.path) {
            return self
                .content
                .eq(&file_content.lines().map(String::from).collect::<Vec<_>>());
        };
        false
    }

    pub fn cut(&mut self) {
        self.cursor.cut(&mut self.content)
    }

    pub fn copy(&mut self) {
        self.cursor.copy(&self.content)
    }

    pub fn paste(&mut self) {
        if !self.cursor.selected.is_empty() {
            let _returned_for_action_log = self.cursor.remove(&mut self.content);
        }
        self.cursor.paste(&mut self.content)
    }

    pub fn up(&mut self) {
        self.cursor.navigate_up_content(&mut self.content)
    }

    pub fn select_up(&mut self) {
        self.cursor.select_up_content(&mut self.content)
    }

    pub fn scroll_up(&mut self) {
        self.cursor.scroll_up_content(&mut self.content)
    }

    pub fn swap_up(&mut self) {
        self.cursor.swap_up_line(&mut self.content);
        trim_start_inplace(&mut self.content[self.cursor.line]);
        self.get_indent()
    }

    pub fn down(&mut self) {
        self.cursor.navigate_down_content(&mut self.content)
    }

    pub fn select_down(&mut self) {
        self.cursor.select_down_content(&mut self.content)
    }

    pub fn scroll_down(&mut self) {
        self.cursor.scroll_down_content(&mut self.content)
    }

    pub fn swap_down(&mut self) {
        self.cursor.swap_down_line(&mut self.content);
        trim_start_inplace(&mut self.content[self.cursor.line]);
        self.get_indent()
    }

    pub fn left(&mut self) {
        self.cursor.navigate_left_content(&mut self.content)
    }

    pub fn jump_left(&mut self) {
        self.cursor.left_jump(&self.content)
    }

    pub fn select_left(&mut self) {
        self.cursor.select_left_content(&mut self.content)
    }

    pub fn right(&mut self) {
        self.cursor.navigate_right_content(&mut self.content)
    }

    pub fn jump_right(&mut self) {
        self.cursor.right_jump(&self.content)
    }

    pub fn select_right(&mut self) {
        self.cursor.select_right_content(&mut self.content)
    }

    pub fn push(&mut self, c: char) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            line.insert(self.cursor.char, c);
            self.cursor.char += 1;
            match c {
                '{' => line.insert(self.cursor.char, '}'),
                '(' => line.insert(self.cursor.char, ')'),
                '[' => line.insert(self.cursor.char, ']'),
                '"' => line.insert(self.cursor.char, '"'),
                '\'' => line.insert(self.cursor.char, '\''),
                _ => (),
            }
        } else {
            self.content.insert(self.cursor.line, c.to_string());
            self.cursor.char = 1;
        }
    }

    pub fn backspace(&mut self) {
        if !self.cursor.selected.is_empty() {
            let _returned_for_action_log = self.cursor.remove(&mut self.content);
            return;
        }
        // TODO needs work
        if self.cursor.line != 0 {
            let previous = self.content.get(self.cursor.line - 1).cloned();
            let current = self.content.get_mut(self.cursor.line);
            if let Some(line) = current {
                let (frist, second) = line.split_at(self.cursor.char);
                if frist.is_empty() {
                    if let Some(previous) = previous {
                        let prev_len = previous.len();
                        (*line) = format!("{}{}", previous, second);
                        self.cursor.line -= 1;
                        self.content.remove(self.cursor.line);
                        self.cursor.char = prev_len;
                        return;
                    }
                } else {
                    (*line) = format!("{}{}", &frist[..frist.len() - 1], second)
                }
            }
        } else if let Some(line) = self.content.get_mut(self.cursor.line) {
            let (first, second) = line.split_at(self.cursor.char);
            if !first.is_empty() {
                (*line) = format!("{}{}", &first[..first.len() - 1], second);
            };
        }
        if self.cursor.char != 0 {
            self.cursor.char -= 1;
        }
    }

    pub fn del(&mut self) {
        if !self.cursor.selected.is_empty() {
            let _returned_for_action_log = self.cursor.remove(&mut self.content);
            return;
        }
        // TODO needs work
        let next_line = self.content.get(self.cursor.line + 1).cloned();
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            let (first, second) = line.split_at(self.cursor.char);
            if second.is_empty() {
                if let Some(new_content) = next_line {
                    line.push_str(new_content.as_str());
                    self.content.remove(self.cursor.line + 1);
                }
            } else {
                (*line) = format!("{}{}", first, &second[1..]);
            }
        }
    }

    pub fn indent(&mut self) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            line.insert_str(self.cursor.char, &self.configs.indent);
            self.cursor.char += self.configs.indent.len();
        } else {
            self.content.insert(self.cursor.line, self.configs.indent.to_owned());
            self.cursor.char = self.configs.indent.len();
        }
    }

    pub fn indent_start(&mut self) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            line.insert_str(0, &self.configs.indent);
            self.cursor.char += self.configs.indent.len();
        }
    }

    pub fn unindent(&mut self) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            if line.starts_with(&self.configs.indent) {
                line.replace_range(..self.configs.indent.len(), "");
                self.cursor.char = self
                    .cursor
                    .char
                    .checked_sub(self.configs.indent.len())
                    .unwrap_or_default();
            }
        }
    }

    pub fn new_line(&mut self) {
        if self.content.is_empty() {
            self.content.push(String::new());
            self.cursor.line += 1;
            return;
        }
        let line = &self.content[self.cursor.line];
        let new_line = if line.len() > self.cursor.char {
            let (replace_line, new_line) = line.split_at(self.cursor.char);
            let new_line = String::from(new_line);
            self.content[self.cursor.line] = String::from(replace_line);
            new_line
        } else {
            String::new()
        };
        self.content.insert(self.cursor.line + 1, new_line);
        self.cursor.line += 1;
        self.cursor.char = 0;
        self.get_indent();
    }

    fn get_indent(&mut self) {
        if self.cursor.line == 0 {
            return;
        }

        let prev_line = &self.content[self.cursor.line - 1].clone();
        let curr_line = &mut self.content[self.cursor.line];
        let indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
        curr_line.insert_str(0, &indent);
        self.cursor.char = indent.len();

        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.configs.indent_after.contains(last) {
                if let Some(first) = curr_line.trim_start().chars().next() {
                    if (last, first) == ('{', '}') || (last, first) == ('(', ')') || (last, first) == ('[', ']') {
                        self.content.insert(self.cursor.line, indent);
                    }
                }
                self.content[self.cursor.line].insert_str(0, &self.configs.indent);
                self.cursor.char += self.configs.indent.len();
            }
        }
    }

    pub fn save(&self) {
        std::fs::write(&self.path, self.content.join("\n")).unwrap();
    }
}

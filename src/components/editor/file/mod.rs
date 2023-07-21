mod action;
mod clipboard;
mod select;
use clipboard::Clipboard;
pub use select::{CursorPosition, Select};
use tui::widgets::{List, ListItem};

use crate::{
    messages::{EditorConfigs, FileType},
    syntax::{Lexer, Theme},
    utils::{get_closing_char, trim_start_inplace},
};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Editor {
    pub linter: Lexer,
    pub configs: EditorConfigs,
    pub cursor: CursorPosition,
    select: Select,
    clipboard: Clipboard,
    should_paste_line: bool,
    // action_logger: ActionLogger,
    pub max_rows: u16,
    pub at_line: usize,
    pub content: Vec<String>,
    pub path: PathBuf,
    pub file_type: FileType,
}

impl Editor {
    pub fn from_path(path: PathBuf, configs: EditorConfigs) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let file_type = FileType::derive_type(&path);
        Ok(Self {
            linter: Lexer::from_type(&file_type, Theme::from(&configs.theme_file_in_config_dir)),
            configs,
            cursor: CursorPosition::default(),
            select: Select::default(),
            clipboard: Clipboard::default(),
            should_paste_line: false,
            // action_logger: ActionLogger::default(),
            max_rows: 0,
            at_line: 0,
            content: content.lines().map(String::from).collect(),
            file_type,
            path,
        })
    }

    pub fn get_list_widget(&mut self) -> (usize, List<'_>) {
        self.linter.reset(self.select.get());
        let max_digits = self.linter.line_number_max_digits(&self.content);
        let editor_content = List::new(
            self.content[self.at_line..]
                .iter()
                .enumerate()
                .map(|(idx, code_line)| self.linter.syntax_spans(idx + self.at_line, code_line))
                .collect::<Vec<ListItem>>(),
        );
        (max_digits, editor_content)
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
        if self.content.is_empty() {
            return;
        }
        self.should_paste_line = false;
        let cut_content = self.remove();
        self.clipboard.push(cut_content);
    }

    pub fn copy(&mut self) {
        self.should_paste_line = false;
        if let Some((from, to)) = self.select.get() {
            if from.line == to.line {
                self.clipboard
                    .push(self.content[from.line][from.char..to.char].to_owned());
            } else {
                let mut at_line = from.line;
                let mut clip_vec = Vec::new();
                clip_vec.push(self.content[from.line][from.char..].to_owned());
                while at_line < to.line {
                    at_line += 1;
                    if at_line != to.line {
                        clip_vec.push(self.content[at_line].to_owned())
                    } else {
                        clip_vec.push(self.content[at_line][..to.char].to_owned())
                    }
                }
                self.clipboard.push(clip_vec.join("\n"));
            }
        } else {
            self.should_paste_line = true;
            let mut line = self.content[self.cursor.line].to_owned();
            line.push('\n');
            self.clipboard.push(line);
        }
    }

    pub fn paste(&mut self) {
        if !self.select.is_empty() {
            let _returned_for_action_log = self.remove();
        }
        if let Some(clip) = self.clipboard.get() {
            self.insert_clip(clip)
        }
    }

    pub fn up(&mut self) {
        self.select.drop();
        self._up()
    }

    fn _up(&mut self) {
        if self.at_line >= self.cursor.line {
            self.scroll_up()
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn select_up(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._up();
        self.select.push(&self.cursor);
    }

    pub fn scroll_up(&mut self) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.up()
        }
    }

    pub fn swap_up(&mut self) {
        self.select.drop();
        if self.at_line >= self.cursor.line {
            self.scroll_up()
        };
        if self.cursor.line > 0 {
            let new_line = self.cursor.line - 1;
            self.content.swap(self.cursor.line, new_line);
            let (offset, indent) = self
                .configs
                .derive_indent_from(&self.content[new_line.checked_sub(1).unwrap_or(self.cursor.line)]);
            let line = &mut self.content[self.cursor.line];
            line.insert_str(0, &indent);
            self.cursor.char += offset;
            trim_start_inplace(line);
            self.cursor.line = new_line;
        }
    }

    pub fn down(&mut self) {
        self.select.drop();
        self._down();
    }

    fn _down(&mut self) {
        if self.content.is_empty() {
            return;
        }
        if self.cursor.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if self.content.len() - 1 > self.cursor.line {
            self.cursor.line += 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn select_down(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._down();
        self.select.push(&self.cursor);
    }

    pub fn scroll_down(&mut self) {
        if self.at_line < self.content.len() - 2 {
            self.at_line += 1;
            self.down()
        }
    }

    pub fn swap_down(&mut self) {
        self.select.drop();
        if self.content.is_empty() {
            return;
        }
        if self.cursor.line > self.max_rows as usize - 3 + self.at_line {
            self.at_line += 1;
        }
        if self.content.len() - 1 > self.cursor.line {
            let new_cursor_line = self.cursor.line + 1;
            self.content.swap(self.cursor.line, new_cursor_line);
            let (offset, indent) = self.configs.derive_indent_from(&self.content[self.cursor.line]);
            let line = &mut self.content[new_cursor_line];
            trim_start_inplace(line);
            line.insert_str(0, &indent);
            self.cursor.line = new_cursor_line;
            self.cursor.char += offset;
        }
    }

    pub fn left(&mut self) {
        self.select.drop();
        self._left();
    }

    fn _left(&mut self) {
        if self.cursor.char > 0 {
            self.cursor.char -= 1
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            if let Some(line) = self.content.get(self.cursor.line) {
                self.cursor.char = line.len();
            }
            if self.cursor.line < self.at_line {
                self.at_line -= 1;
            }
        }
    }

    pub fn jump_left(&mut self) {
        self.select.drop();
        self._jump_left();
    }

    pub fn jump_left_select(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._jump_left();
        self.select.push(&self.cursor);
    }

    fn _jump_left(&mut self) {
        let mut line = &self.content[self.cursor.line][..self.cursor.char];
        let mut last_was_char = false;
        if line.is_empty() && self.cursor.line > 0 {
            self._left();
            line = &self.content[self.cursor.line][..self.cursor.char];
        }
        for ch in line.chars().rev() {
            if last_was_char && !ch.is_alphabetic() || self.cursor.char == 0 {
                return;
            }
            self.cursor.char -= 1;
            last_was_char = ch.is_alphabetic();
        }
    }

    pub fn select_left(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._left();
        self.select.push(&self.cursor);
    }

    pub fn right(&mut self) {
        self.select.drop();
        self._right();
    }

    fn _right(&mut self) {
        if let Some(line) = self.content.get(self.cursor.line) {
            if line.len() > self.cursor.char {
                self.cursor.char += 1
            } else if self.content.len() - 1 > self.cursor.line {
                self.cursor.line += 1;
                self.cursor.char = 0;
                if self.cursor.line > self.max_rows as usize - 3 + self.at_line {
                    self.at_line += 1;
                }
            }
        }
    }

    pub fn jump_right(&mut self) {
        self.select.drop();
        self._jump_right();
    }

    pub fn jump_right_select(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._jump_right();
        self.select.push(&self.cursor);
    }

    pub fn _jump_right(&mut self) {
        let mut line = &self.content[self.cursor.line][self.cursor.char..];
        let mut last_was_char = false;
        if line.is_empty() && self.content.len() - 1 > self.cursor.line {
            self._right();
            line = &self.content[self.cursor.line][self.cursor.char..];
        }
        for ch in line.chars() {
            if last_was_char && !ch.is_alphabetic() {
                return;
            }
            self.cursor.char += 1;
            last_was_char = ch.is_alphabetic();
        }
    }

    pub fn select_right(&mut self) {
        self.select.init(self.cursor.line, self.cursor.char);
        self._right();
        self.select.push(&self.cursor);
    }

    pub fn new_line(&mut self) {
        if self.content.is_empty() {
            self.content.push(String::new());
            self.cursor.line += 1;
            return;
        }
        let prev_line = &mut self.content[self.cursor.line];
        let mut line = if prev_line.len() >= self.cursor.char {
            prev_line.split_off(self.cursor.char)
        } else {
            String::new()
        };
        let (offset, indent) = self.configs.derive_indent_from(prev_line);
        self.cursor.line += 1;
        self.cursor.char = offset;
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if (last, first) == ('{', '}') || (last, first) == ('(', ')') || (last, first) == ('[', ']') {
                    if let Some(bracket_indent) = indent.strip_prefix(&self.configs.indent) {
                        line.insert_str(0, bracket_indent);
                    }
                    self.content.insert(self.cursor.line, line);
                    self.content.insert(self.cursor.line, indent);
                    return;
                }
            }
        }
        line.insert_str(0, &indent);
        self.content.insert(self.cursor.line, line);
    }

    pub fn push(&mut self, ch: char) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            line.insert(self.cursor.char, ch);
            self.cursor.char += 1;
            if let Some(closing) = get_closing_char(ch) {
                line.insert(self.cursor.char, closing)
            }
        } else {
            self.content.insert(self.cursor.line, ch.to_string());
            self.cursor.char = 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.content.is_empty() {
            return;
        }
        if !self.select.is_empty() {
            let _returned_for_action_log = self.remove();
            return;
        }
        if self.cursor.line != 0 {
            if self.cursor.char == 0 {
                let current_line = self.content.remove(self.cursor.line);
                self.cursor.line -= 1;
                let prev_line = &mut self.content[self.cursor.line];
                self.cursor.char = prev_line.len();
                prev_line.push_str(&current_line);
            } else {
                let _returned_for_action_log = self.content[self.cursor.line].remove(self.cursor.char - 1);
                self.cursor.char -= 1;
            }
        } else if let Some(line) = self.content.get_mut(self.cursor.line) {
            if self.cursor.char != 0 {
                let _returned_for_action_log = line.remove(self.cursor.char - 1);
                self.cursor.char -= 1;
            }
        }
    }

    pub fn del(&mut self) {
        if self.content.is_empty() {
            return;
        }
        if !self.select.is_empty() {
            let _returned_for_action_log = self.remove();
            return;
        }
        if self.content[self.cursor.line].len() == self.cursor.char {
            if self.content.len() > self.cursor.line + 1 {
                let next_line = self.content.remove(self.cursor.line + 1);
                self.content[self.cursor.line].push_str(&next_line);
            }
        } else {
            let _returned_for_action_log = self.content[self.cursor.line].remove(self.cursor.char);
        }
    }

    pub fn indent(&mut self) {
        self.indent_at(self.cursor.char)
    }

    pub fn indent_start(&mut self) {
        self.indent_at(0)
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

    pub fn save(&self) {
        std::fs::write(&self.path, self.content.join("\n")).unwrap();
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.configs = new_cfg.clone();
        self.linter.theme = Theme::from(&self.configs.theme_file_in_config_dir);
    }

    fn remove(&mut self) -> String {
        if let Some((from, to)) = self.select.get() {
            let clip = if from.line == to.line {
                self.cursor.char = from.char;
                let data = self.content.remove(from.line);
                let mut payload = String::from(&data[..from.char]);
                payload.push_str(&data[to.char..]);
                self.content.insert(from.line, payload);
                data[from.char..to.char].to_owned()
            } else {
                let mut clip_vec = vec![self.content[from.line].split_off(from.char)];
                let mut last_line = to.line;
                while from.line < last_line {
                    last_line -= 1;
                    if from.line == last_line {
                        let final_clip = self.content.remove(from.line + 1);
                        let (clipped, remaining) = final_clip.split_at(to.char);
                        self.content[from.line].push_str(remaining);
                        clip_vec.push(clipped.to_owned())
                    } else {
                        clip_vec.push(self.content.remove(from.line + 1))
                    }
                }
                self.cursor.line = from.line;
                self.cursor.char = from.char;
                clip_vec.join("\n")
            };
            self.select.drop();
            clip
        } else {
            let mut clip = self.content.remove(self.cursor.line);
            clip.push('\n');
            if self.cursor.line >= self.content.len() && !self.content.is_empty() {
                self.cursor.line -= 1;
                self.cursor.char = self.content[self.cursor.line].len() - 1;
            } else {
                self.cursor.char = 0;
            }
            clip
        }
    }

    fn insert_clip(&mut self, clip: String) {
        let mut lines: Vec<_> = clip.split('\n').collect();
        if lines.is_empty() {
            return;
        }
        if self.should_paste_line && lines.len() == 2 && lines[1].is_empty() {
            self.content.insert(self.cursor.line, lines[0].into())
        } else if lines.len() == 1 {
            let text = lines[0];
            self.content[self.cursor.line].insert_str(self.cursor.char, lines[0]);
            self.cursor.char += text.len();
        } else {
            let line = self.content.remove(self.cursor.line);
            let (prefix, suffix) = line.split_at(self.cursor.char);
            let mut first_line = prefix.to_owned();
            if lines.len() == 1 {
                first_line.push_str(lines[0]);
                self.content.insert(self.cursor.line, first_line);
                self.content.insert(self.cursor.line + 1, suffix.to_owned());
            } else {
                first_line.push_str(lines.remove(0));
                self.content.insert(self.cursor.line, first_line);
                let last_idx = lines.len() - 1;
                for (idx, select) in lines.iter().enumerate() {
                    let next_line = if idx == last_idx {
                        let mut last_line = select.to_string();
                        self.cursor.char = last_line.len();
                        last_line.push_str(suffix);
                        last_line
                    } else {
                        select.to_string()
                    };
                    self.content.insert(self.cursor.line + 1, next_line);
                    self.down();
                }
            }
        }
    }

    fn indent_at(&mut self, idx: usize) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            line.insert_str(idx, &self.configs.indent);
            self.cursor.char += self.configs.indent.len();
        } else {
            self.content.insert(self.cursor.line, self.configs.indent.to_owned());
            self.cursor.char = self.configs.indent.len();
        }
    }

    fn adjust_cursor_max_char(&mut self) {
        if let Some(line) = self.content.get(self.cursor.line) {
            if line.len() < self.cursor.char {
                self.cursor.char = line.len()
            }
        }
    }
}

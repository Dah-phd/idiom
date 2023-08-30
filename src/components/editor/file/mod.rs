mod action;
mod clipboard;
mod select;
use clipboard::Clipboard;
pub use select::{CursorPosition, Offset, Select};
use tui::widgets::{List, ListItem};

use crate::{
    configs::{EditorConfigs, FileType},
    lsp::LSP,
    syntax::{Lexer, Theme},
    utils::{get_closing_char, trim_start_inplace},
};
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::Mutex;

use self::action::ActionLogger;

#[derive(Debug)]
pub struct Editor {
    pub cursor: CursorPosition,
    pub lsp: Option<Rc<Mutex<LSP>>>,
    pub file_type: FileType,
    pub path: PathBuf,
    pub at_line: usize,
    linter: Lexer,
    configs: EditorConfigs,
    select: Select,
    clipboard: Clipboard,
    action_logger: ActionLogger,
    max_rows: usize,
    content: Vec<String>,
}

impl Editor {
    pub fn from_path(path: PathBuf, mut configs: EditorConfigs) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let file_type = FileType::derive_type(&path);
        configs.update_by_file_type(&file_type);
        Ok(Self {
            linter: Lexer::from_type(&file_type, Theme::from(&configs.theme_file_in_config_dir)),
            configs,
            lsp: None,
            cursor: CursorPosition::default(),
            select: Select::default(),
            clipboard: Clipboard::default(),
            action_logger: ActionLogger::default(),
            max_rows: 0,
            at_line: 0,
            content: content.split('\n').map(String::from).collect(),
            file_type,
            path,
        })
    }

    pub fn get_list_widget(&mut self) -> (usize, List<'_>) {
        self.get_diagnostics();
        self.linter.set_select(self.select.get());
        let max_digits = self.linter.line_number_max_digits(&self.content);
        let render_till_line = self.content.len().min(self.at_line + self.max_rows);
        let editor_content = List::new(
            self.content[self.at_line..render_till_line]
                .iter()
                .enumerate()
                .map(|(idx, code_line)| self.linter.syntax_spans(idx + self.at_line, code_line))
                .collect::<Vec<ListItem>>(),
        );
        (max_digits, editor_content)
    }

    pub fn get_diagnostics(&mut self) {
        if let Some(lsp) = self.lsp.as_mut() {
            if let Ok(guard) = lsp.try_lock() {
                let diagnostics = guard.get_diagnostics(&self.path);
                if diagnostics.is_some() {
                    self.linter.diagnostics = diagnostics
                }
            }
        }
    }

    pub async fn update_lsp(&mut self) {
        if let Some(lsp) = self.lsp.as_mut() {
            if let Ok(mut guard) = lsp.try_lock() {
                if let Some((version, content_changes)) = self.action_logger.get_text_edits() {
                    guard.file_did_change(&self.path, version, content_changes).await;
                }
            }
        }
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
        if let Some((from, .., clip)) = self.select.extract_logged(&mut self.content, &mut self.action_logger) {
            self.cursor = from;
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.as_range()]);
            self.clipboard.push(clip);
        } else {
            self.action_logger
                .init_replace(self.cursor, &self.content[self.cursor.as_range()]);
            let mut clip = self.content.remove(self.cursor.line);
            clip.push('\n');
            if self.cursor.line >= self.content.len() && !self.content.is_empty() {
                self.cursor.line -= 1;
                self.cursor.char = self.content[self.cursor.line].len() - 1;
            } else {
                self.cursor.char = 0;
            }
            self.action_logger.finish_replace(self.cursor, &[]);
            self.clipboard.push(clip);
        }
    }

    pub fn copy(&mut self) {
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
            let mut line = self.content[self.cursor.line].to_owned();
            line.push('\n');
            self.clipboard.push(line);
        }
    }

    pub fn paste(&mut self) {
        if let Some((from, ..)) = self.select.extract_logged(&mut self.content, &mut self.action_logger) {
            self.cursor = from;
        } else {
            self.action_logger
                .init_replace(self.cursor, &self.content[self.cursor.as_range()]);
        };
        if let Some(clip) = self.clipboard.get() {
            let mut lines: Vec<_> = clip.split('\n').collect();
            let start_line = self.cursor.line;
            if lines.len() == 1 {
                let text = lines[0];
                self.content[self.cursor.line].insert_str(self.cursor.char, lines[0]);
                self.cursor.char += text.len();
            } else {
                let line = self.content.remove(self.cursor.line);
                let (prefix, suffix) = line.split_at(self.cursor.char);
                let mut first_line = prefix.to_owned();
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
            self.action_logger
                .finish_replace(self.cursor, &self.content[start_line..=self.cursor.line])
        } else {
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.as_range()])
        }
    }

    pub fn undo(&mut self) {
        if let Some(cursor) = self.action_logger.undo(&mut self.content) {
            self.cursor = cursor;
        }
    }

    pub fn redo(&mut self) {
        if let Some(cursor) = self.action_logger.redo(&mut self.content) {
            self.cursor = cursor;
        }
    }

    pub fn end_of_line(&mut self) {
        self.cursor.char = self.content[self.cursor.line].len();
    }

    pub fn end_of_file(&mut self) {
        if !self.content.is_empty() {
            self.cursor.line = self.content.len() - 1;
            self.cursor.char = self.content[self.cursor.line].len();
        }
    }

    pub fn start_of_file(&mut self) {
        self.at_line = 0;
        self.cursor.line = 0;
        self.cursor.char = 0;
    }

    pub fn start_of_line(&mut self) {
        self.cursor.char = 0;
        for ch in self.content[self.cursor.line].chars() {
            if !ch.is_whitespace() {
                break;
            }
            self.cursor.char += 1;
        }
    }

    pub fn go_to(&mut self, line: usize) {
        if self.content.len() >= line {
            self.cursor.line = line;
            self.cursor.char = 0;
        }
    }

    pub fn up(&mut self) {
        self.select.drop();
        self._up()
    }

    fn _up(&mut self) {
        if self.cursor.line > 0 {
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
        if self.cursor.line > 0 {
            let new_line = self.cursor.line - 1;
            self.action_logger.init_repalce_from_line(
                new_line,
                self.cursor,
                &self.content[new_line..=self.cursor.line],
            );
            let (char_offset, _) = self.swap(new_line, self.cursor.line);
            self.cursor.line = new_line;
            self.cursor.offset_char(char_offset);
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line + 1])
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
        if self.content.len() - 1 > self.cursor.line {
            let new_cursor_line = self.cursor.line + 1;
            self.action_logger
                .init_replace(self.cursor, &self.content[self.cursor.line..=new_cursor_line]);
            let (_, char_offset) = self.swap(self.cursor.line, new_cursor_line);
            self.cursor.offset_char(char_offset);
            self.cursor.line = new_cursor_line;
            self.action_logger
                .finish_replace(self.cursor, &self.content[new_cursor_line - 1..=new_cursor_line])
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
            self.action_logger
                .init_replace(self.cursor, &self.content[self.cursor.as_range()]);
            self.content.push(String::new());
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.line_range(0, 1)]);
            self.cursor.line += 1;
            return;
        }
        let prev_line = &mut self.content[self.cursor.line];
        self.action_logger.init_replace(self.cursor, &[prev_line.to_owned()]);
        let mut line = if prev_line.len() >= self.cursor.char {
            prev_line.split_off(self.cursor.char)
        } else {
            String::new()
        };
        let indent = self.configs.derive_indent_from(prev_line);
        line.insert_str(0, &indent);
        self.cursor.line += 1;
        self.cursor.char = indent.len();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if let Some(first) = line.trim_start().chars().next() {
                if [('{', '}'), ('(', ')'), ('[', ']')].contains(&(last, first)) {
                    self.configs.unindent_if_before_base_pattern(&mut line);
                    self.content.insert(self.cursor.line, line);
                    self.content.insert(self.cursor.line, indent);
                    self.action_logger
                        .finish_replace(self.cursor, &self.content[self.cursor.line_range(1, 2)]);
                    return;
                }
            }
        }
        self.content.insert(self.cursor.line, line);
        self.action_logger
            .finish_replace(self.cursor, &self.content[self.cursor.line_range(1, 1)]);
    }

    pub fn push(&mut self, ch: char) {
        if let Some((from, to)) = self.select.get_mut() {
            self.cursor = *from;
            self.cursor.char += 1;
            let replace = if let Some(closing) = get_closing_char(ch) {
                self.action_logger.init_replace_from_select(from, to, &self.content);
                self.content[from.line].insert(from.char, ch);
                from.char += 1;
                if from.line == to.line {
                    to.char += 1;
                }
                self.content[to.line].insert(to.char, closing);
                from.line..to.line
            } else {
                let (from, ..) = self
                    .select
                    .extract_logged(&mut self.content, &mut self.action_logger)
                    .unwrap();
                self.content[from.line].insert(from.char, ch);
                from.line..from.line + 1
            };
            self.action_logger.finish_replace(self.cursor, &self.content[replace]);
        } else if let Some(line) = self.content.get_mut(self.cursor.line) {
            self.action_logger.push_char(&self.cursor, line, ch);
            line.insert(self.cursor.char, ch);
            self.cursor.char += 1;
            if let Some(closing) = get_closing_char(ch) {
                line.insert(self.cursor.char, closing);
                self.action_logger.inser_char(&self.cursor, line, closing);
            }
        } else {
            self.action_logger.push_char(&self.cursor, "", ch);
            self.content.insert(self.cursor.line, ch.to_string());
            self.cursor.char = 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.content.is_empty() || self.cursor.line == 0 && self.cursor.char == 0 {
            return;
        }
        if let Some((from, ..)) = self.select.extract_logged(&mut self.content, &mut self.action_logger) {
            self.cursor = from;
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line]);
        } else if self.cursor.char == 0 {
            let prev_line_idx = self.cursor.line - 1;
            self.action_logger
                .init_replace(self.cursor, &self.content[prev_line_idx..=self.cursor.line]);
            let current_line = self.content.remove(self.cursor.line);
            self.cursor.line -= 1;
            let prev_line = &mut self.content[self.cursor.line];
            self.cursor.char = prev_line.len();
            prev_line.push_str(&current_line);
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line]);
        } else {
            let line = &mut self.content[self.cursor.line];
            self.action_logger.prep_buffer(&self.cursor, line);
            let offset = self.configs.backspace_indent_handler(line, self.cursor.char);
            self.cursor.offset_char(offset);
            self.action_logger.backspace(&self.cursor);
        }
    }

    pub fn del(&mut self) {
        if self.content.is_empty() {
            return;
        }
        if let Some((from, ..)) = self.select.extract_logged(&mut self.content, &mut self.action_logger) {
            self.cursor = from;
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line]);
        } else if self.content[self.cursor.line].len() == self.cursor.char {
            if self.content.len() > self.cursor.line + 1 {
                self.action_logger
                    .init_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line + 1]);
                let next_line = self.content.remove(self.cursor.line + 1);
                self.content[self.cursor.line].push_str(&next_line);
                self.action_logger
                    .finish_replace(self.cursor, &self.content[self.cursor.line..=self.cursor.line])
            }
        } else {
            let line = &mut self.content[self.cursor.line];
            self.action_logger.del(&self.cursor, line);
            line.remove(self.cursor.char);
        }
    }

    pub fn indent(&mut self) {
        if let Some((from, ..)) = self.select.extract_logged(&mut self.content, &mut self.action_logger) {
            self.indent_at(self.cursor.char);
            self.cursor = from;
            self.action_logger
                .finish_replace(self.cursor, &self.content[self.cursor.as_range()])
        } else {
            self.action_logger
                .prep_buffer(&self.cursor, &self.content[self.cursor.line]);
            self.indent_at(self.cursor.char);
            self.action_logger.buffer_str(&self.configs.indent, self.cursor);
        }
    }

    pub fn indent_start(&mut self) {
        self.indent_at(0)
    }

    pub fn unindent(&mut self) {
        if let Some(line) = self.content.get_mut(self.cursor.line) {
            if line.starts_with(&self.configs.indent) {
                self.action_logger.init_replace(self.cursor, &[line.to_owned()]);
                line.replace_range(..self.configs.indent.len(), "");
                self.cursor.diff_char(self.configs.indent.len());
                self.action_logger.finish_replace(self.cursor, &[line.to_owned()])
            }
        }
    }

    pub async fn save(&mut self) {
        if let Some(lsp) = self.lsp.as_mut() {
            lsp.lock().await.file_did_save(&self.path).await;
        }
        std::fs::write(&self.path, self.content.join("\n")).unwrap();
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.configs = new_cfg.clone();
        self.linter.theme = Theme::from(&self.configs.theme_file_in_config_dir);
    }

    fn get_and_indent_line(&mut self, line_idx: usize) -> (Offset, &mut String) {
        if line_idx > 0 {
            let (prev_split, current_split) = self.content.split_at_mut(line_idx);
            let prev = &prev_split[line_idx - 1];
            let line = &mut current_split[0];
            (self.configs.indent_from_prev(prev, line), line)
        } else {
            let line = &mut self.content[line_idx];
            (trim_start_inplace(line), line)
        }
    }

    fn swap(&mut self, from: usize, to: usize) -> (Offset, Offset) {
        // from should be always smaller than to - unchecked
        self.content.swap(from, to);
        let (offset, _) = self.get_and_indent_line(from);
        let (offset2, _) = self.get_and_indent_line(to);
        (offset, offset2)
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

    pub fn set_max_rows(&mut self, max_rows: u16) {
        self.max_rows = max_rows as usize;
        if self.cursor.line < self.at_line {
            self.at_line = self.cursor.line
        }
        if self.cursor.line > self.max_rows - 3 + self.at_line {
            self.at_line = self.cursor.line + 2 - self.max_rows
        }
    }
}

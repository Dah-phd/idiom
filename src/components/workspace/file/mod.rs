mod cursor;
mod utils;
use cursor::Cursor;
pub use cursor::{CursorPosition, Offset, Select};
use lsp_types::TextEdit;
use ratatui::widgets::{List, ListItem};

use crate::{
    configs::{EditorConfigs, FileType},
    events::Events,
    syntax::{Lexer, Theme},
    utils::{find_code_blocks, trim_start_inplace},
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

use self::utils::{copy_content, find_line_start, token_range_at};

type DocLen = usize;
type SelectLen = usize;
pub type DocStats<'a> = (DocLen, SelectLen, CursorPosition);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Editor {
    pub file_type: FileType,
    pub display: String,
    pub path: PathBuf,
    pub at_line: usize,
    pub lexer: Lexer,
    pub cursor: Cursor,
    max_rows: usize,
    content: Vec<String>,
}

#[allow(dead_code)]
impl Editor {
    pub fn from_path(path: PathBuf, mut configs: EditorConfigs, events: &Rc<RefCell<Events>>) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let file_type = FileType::derive_type(&path);
        let display = path.display().to_string();
        configs.update_by_file_type(&file_type);
        Ok(Self {
            lexer: Lexer::with_context(file_type, Theme::new(), events),
            cursor: Cursor::new(configs),
            max_rows: 0,
            at_line: 0,
            content: content.split('\n').map(String::from).collect(),
            file_type,
            display,
            path: path.canonicalize()?,
        })
    }

    pub fn get_list_widget_with_context(&mut self) -> (usize, List<'_>) {
        let max_digits = self.lexer.context(&self.content, self.cursor.select.get(), &self.path);
        let render_till_line = self.content.len().min(self.at_line + self.max_rows);
        let editor_content = List::new(
            self.content[self.at_line..render_till_line]
                .iter()
                .enumerate()
                .map(|(idx, code_line)| self.lexer.list_item(idx + self.at_line, code_line))
                .collect::<Vec<ListItem>>(),
        );
        (max_digits, editor_content)
    }

    pub fn get_stats(&self) -> DocStats {
        (self.content.len(), self.cursor.select.len(&self.content), self.cursor.position())
    }

    pub fn help(&mut self) {
        self.lexer.get_signitures(&self.path, &self.cursor.position());
    }

    pub fn declaration(&mut self) {
        self.lexer.go_to_declaration(&self.path, &self.cursor.position());
    }

    pub fn hover(&mut self) {
        self.lexer.get_hover(&self.path, &self.cursor.position());
    }

    pub fn start_renames(&mut self) {
        let line = &self.content[self.cursor.line];
        let token_range = token_range_at(line, self.cursor.char);
        self.lexer.start_renames(&self.cursor.position(), &line[token_range]);
    }

    pub async fn update_lsp(&mut self) {
        self.lexer.update_lsp(&self.path, self.cursor.get_text_edits()).await;
    }

    pub fn is_saved(&self) -> bool {
        if let Ok(file_content) = std::fs::read_to_string(&self.path) {
            return self.content.eq(&file_content.split('\n').map(String::from).collect::<Vec<_>>());
        };
        false
    }

    pub fn replace_select(&mut self, select: Select, new_clip: &str) {
        self.cursor.replace_select(select, new_clip, &mut self.content);
    }

    pub fn replace_token(&mut self, new: String) {
        self.cursor.replace_token(new, &mut self.content);
    }

    pub fn mass_replace(&mut self, selects: Vec<Select>, new_clip: &str) {
        for select in selects {
            self.cursor.replace_select(select, new_clip, &mut self.content);
        }
    }

    pub fn apply_file_edits(&mut self, edits: Vec<TextEdit>) {
        let cursor = self.cursor.position();
        for edit in edits {
            self.cursor.replace_select(edit.range.into(), edit.new_text, &mut self.content);
        }
        self.cursor.set_position(cursor);
    }

    pub fn go_to(&mut self, line: usize) {
        self.cursor.drop_select();
        if self.content.len() >= line {
            self.cursor.line = line;
            self.cursor.char = find_line_start(self.content[line].as_str());
            self.at_line = line.checked_sub(self.max_rows / 2).unwrap_or_default();
        }
    }

    pub fn go_to_select(&mut self, select: Select) {
        if let Select::Range(_, to) = select {
            self.cursor.set_position(to);
            self.at_line = to.line.checked_sub(self.max_rows / 2).unwrap_or_default();
            self.cursor.select = select;
        }
    }

    pub fn find(&mut self, pat: &str, buffer: &mut Vec<Select>) {
        buffer.clear();
        if pat.is_empty() {
            return;
        }
        for (line_idx, line_content) in self.content.iter().enumerate() {
            for (char_idx, _) in line_content.match_indices(pat) {
                buffer.push(Select::Range((line_idx, char_idx).into(), (line_idx, char_idx + pat.len()).into()));
            }
        }
    }

    pub fn find_with_line(&mut self, pat: &str) -> Vec<(Select, String)> {
        let mut buffer = Vec::new();
        if pat.is_empty() {
            return buffer;
        }
        for (line_idx, line_content) in self.content.iter().enumerate() {
            for (char_idx, _) in line_content.match_indices(pat) {
                let select = Select::Range((line_idx, char_idx).into(), (line_idx, char_idx + pat.len()).into());
                buffer.push((select, line_content.to_owned()));
            }
        }
        buffer
    }

    pub fn cut(&mut self) -> Option<String> {
        if self.content.is_empty() {
            return None;
        }
        Some(self.cursor.cut(&mut self.content))
    }

    pub fn copy(&mut self) -> Option<String> {
        if self.content.is_empty() {
            None
        } else if let Some((from, to)) = self.cursor.select.get() {
            Some(copy_content(from, to, &self.content))
        } else {
            Some(format!("{}\n", &self.content[self.cursor.line]))
        }
    }

    pub fn paste(&mut self, clip: String) {
        self.cursor.paste(clip, &mut self.content);
    }

    pub fn undo(&mut self) {
        self.cursor.undo(&mut self.content);
    }

    pub fn redo(&mut self) {
        self.cursor.redo(&mut self.content);
    }

    pub fn search_file(&self, pattern: &str) -> Vec<(usize, String)> {
        let mut buffer = Vec::new();
        find_code_blocks(&mut buffer, &self.content, pattern);
        buffer
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

    pub fn up(&mut self) {
        self.cursor.drop_select();
        self.move_up()
    }

    fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn select_up(&mut self) {
        self.cursor.init_select();
        self.move_up();
        self.cursor.push_to_select();
    }

    pub fn scroll_up(&mut self) {
        if self.at_line != 0 {
            self.at_line -= 1;
            self.up()
        }
    }

    pub fn swap_up(&mut self) {
        self.cursor.drop_select();
        if self.cursor.line == 0 {
            return;
        }
        let new_cursor_line = self.cursor.line - 1;
        let (char_offset, _) = self.cursor.swap_down(new_cursor_line, &mut self.content);
        self.cursor.line = new_cursor_line;
        self.cursor.offset_char(char_offset);
    }

    pub fn down(&mut self) {
        self.cursor.drop_select();
        self.move_down();
    }

    fn move_down(&mut self) {
        if self.content.is_empty() {
            return;
        }
        if self.content.len() - 1 > self.cursor.line {
            self.cursor.line += 1;
            self.adjust_cursor_max_char();
        }
    }

    pub fn select_down(&mut self) {
        self.cursor.init_select();
        self.move_down();
        self.cursor.push_to_select();
    }

    pub fn scroll_down(&mut self) {
        if self.at_line < self.content.len() - 2 {
            self.at_line += 1;
            self.down()
        }
    }

    pub fn swap_down(&mut self) {
        self.cursor.drop_select();
        if self.content.is_empty() {
            return;
        }
        if self.content.len() - 1 > self.cursor.line {
            let new_cursor_line = self.cursor.line + 1;
            let (_, char_offset) = self.cursor.swap_down(self.cursor.line, &mut self.content);
            self.cursor.offset_char(char_offset);
            self.cursor.line = new_cursor_line;
        }
    }

    pub fn left(&mut self) {
        self.cursor.drop_select();
        self.move_left();
    }

    fn move_left(&mut self) {
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
        self.cursor.drop_select();
        self._jump_left();
    }

    pub fn jump_left_select(&mut self) {
        self.cursor.init_select();
        self._jump_left();
        self.cursor.push_to_select();
    }

    fn _jump_left(&mut self) {
        let mut line = &self.content[self.cursor.line][..self.cursor.char];
        let mut last_was_char = false;
        if line.is_empty() && self.cursor.line > 0 {
            self.move_left();
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
        self.cursor.init_select();
        self.move_left();
        self.cursor.push_to_select();
    }

    pub fn right(&mut self) {
        self.cursor.drop_select();
        self.move_right();
    }

    fn move_right(&mut self) {
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
        self.cursor.drop_select();
        self._jump_right();
    }

    pub fn jump_right_select(&mut self) {
        self.cursor.init_select();
        self._jump_right();
        self.cursor.push_to_select();
    }

    pub fn _jump_right(&mut self) {
        let mut line = &self.content[self.cursor.line][self.cursor.char..];
        let mut last_was_char = false;
        if line.is_empty() && self.content.len() - 1 > self.cursor.line {
            self.move_right();
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
        self.cursor.init_select();
        self.move_right();
        self.cursor.push_to_select();
    }

    pub fn new_line(&mut self) {
        self.cursor.new_line(&mut self.content);
    }

    pub fn push(&mut self, ch: char) {
        self.cursor.push_char(ch, &mut self.content);
        self.lexer.get_autocomplete(&self.path, &self.cursor.position(), self.content[self.cursor.line].as_str());
    }

    pub fn backspace(&mut self) {
        self.cursor.backspace(&mut self.content);
    }

    pub fn del(&mut self) {
        self.cursor.del(&mut self.content);
    }

    pub fn indent(&mut self) {
        self.cursor.indent(&mut self.content);
    }

    pub fn indent_start(&mut self) {
        self.cursor.indent_start(&mut self.content);
    }

    pub fn unindent(&mut self) {
        self.cursor.unindent(&mut self.content);
    }

    pub fn save(&mut self) {
        if self.try_write_file() {
            if let Some(client) = self.lexer.lsp_client.as_mut() {
                let _ = client.file_did_save(&self.path);
            }
        }
    }

    pub fn try_write_file(&self) -> bool {
        if let Err(error) = std::fs::write(&self.path, self.content.join("\n")) {
            self.lexer.events.borrow_mut().overwrite(error.to_string());
            return false;
        }
        true
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.lexer.reload_theme();
        self.cursor.source_configs(new_cfg);
    }

    pub fn stringify(&self) -> String {
        self.content.join("\n")
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

#[cfg(test)]
pub mod test;

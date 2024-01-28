use super::cursor::{Cursor, CursorPosition};
use lsp_types::TextEdit;
use ratatui::widgets::{List, ListItem};

use crate::workspace::actions::Actions;
use crate::{
    configs::{EditorConfigs, FileType},
    global_state::GlobalState,
    syntax::Lexer,
};
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::utils::{copy_content, find_line_start, last_modified, token_range_at};

type DocLen = usize;
type SelectLen = usize;
pub type DocStats<'a> = (DocLen, SelectLen, CursorPosition);

#[allow(dead_code)]
pub struct Editor {
    pub file_type: FileType,
    pub display: String,
    pub path: PathBuf,
    pub lexer: Lexer,
    pub cursor: Cursor,
    pub actions: Actions,
    max_rows: usize,
    timestamp: Option<SystemTime>,
    content: Vec<String>,
}

impl Editor {
    pub fn from_path(path: PathBuf, mut cfg: EditorConfigs) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let file_type = FileType::derive_type(&path);
        let display = build_display(&path);
        cfg.update_by_file_type(&file_type);
        Ok(Self {
            lexer: Lexer::with_context(file_type, &path),
            cursor: Cursor::default(),
            actions: Actions::new(cfg),
            max_rows: 0,
            content: content.split('\n').map(String::from).collect(),
            file_type,
            display,
            timestamp: last_modified(&path),
            path,
        })
    }

    pub fn collect_widget(&mut self, gs: &mut GlobalState) -> List<'_> {
        self.actions.sync(&mut self.lexer, &self.content);
        self.lexer.context(&self.cursor, &self.content, gs);
        List::new(
            self.content
                .iter()
                .enumerate()
                .skip(self.cursor.at_line)
                .take(self.max_rows)
                .map(|(line_idx, content)| self.lexer.list_item(line_idx, content))
                .collect::<Vec<ListItem>>(),
        )
    }

    pub fn get_stats(&self) -> DocStats {
        (self.content.len(), self.cursor.select_len(&self.content), self.cursor.position())
    }

    pub fn help(&mut self) {
        self.lexer.help(self.cursor.position());
    }

    pub fn references(&mut self) {
        self.lexer.go_to_reference(self.cursor.position());
    }

    pub fn declarations(&mut self) {
        self.lexer.go_to_declaration(self.cursor.position());
    }

    pub fn select_token(&mut self) {
        let range = token_range_at(&self.content[self.cursor.line], self.cursor.char);
        if !range.is_empty() {
            self.cursor.set_char(range.end);
            self.cursor.select_set(
                CursorPosition { line: self.cursor.line, char: range.start },
                CursorPosition { line: self.cursor.line, char: range.end },
            )
        }
    }

    pub fn start_renames(&mut self) {
        let line = &self.content[self.cursor.line];
        let token_range = token_range_at(line, self.cursor.char);
        self.lexer.start_rename(self.cursor.position(), &line[token_range]);
    }

    pub fn is_saved(&self) -> bool {
        if let Ok(file_content) = std::fs::read_to_string(&self.path) {
            return self.content.eq(&file_content.split('\n').map(String::from).collect::<Vec<_>>());
        };
        false
    }

    pub fn replace_select(&mut self, from: CursorPosition, to: CursorPosition, new_clip: &str) {
        self.actions.replace_select(from, to, new_clip, &mut self.cursor, &mut self.content);
    }

    pub fn replace_token(&mut self, new: String) {
        self.actions.replace_token(new, &mut self.cursor, &mut self.content);
    }

    pub fn mass_replace(&mut self, mut ranges: Vec<(CursorPosition, CursorPosition)>, clip: String) {
        ranges.sort_by(|a, b| {
            let line_ord = b.0.line.cmp(&a.0.line);
            if let Ordering::Equal = line_ord {
                return b.0.char.cmp(&a.0.char);
            }
            line_ord
        });
        self.actions.mass_replace(&mut self.cursor, ranges, clip, &mut self.content);
    }

    pub fn apply_file_edits(&mut self, mut edits: Vec<TextEdit>) {
        edits.sort_by(|a, b| {
            let line_ord = b.range.start.line.cmp(&a.range.start.line);
            if let Ordering::Equal = line_ord {
                return b.range.start.character.cmp(&a.range.start.character);
            }
            line_ord
        });
        self.actions.apply_edits(edits, &mut self.content);
    }

    pub fn go_to(&mut self, line: usize) {
        self.cursor.select_drop();
        if self.content.len() >= line {
            self.cursor.line = line;
            self.cursor.char = find_line_start(self.content[line].as_str());
            self.cursor.at_line = line.saturating_sub(self.max_rows / 2);
        }
    }

    pub fn go_to_select(&mut self, from: CursorPosition, to: CursorPosition) {
        self.cursor.at_line = to.line.saturating_sub(self.max_rows / 2);
        self.cursor.select_set(from, to);
    }

    pub fn find(&mut self, pat: &str, buffer: &mut Vec<(CursorPosition, CursorPosition)>) {
        if pat.is_empty() {
            return;
        }
        for (line_idx, line_content) in self.content.iter().enumerate() {
            for (char_idx, _) in line_content.match_indices(pat) {
                buffer.push(((line_idx, char_idx).into(), (line_idx, char_idx + pat.len()).into()));
            }
        }
    }

    pub fn find_with_line(&mut self, pat: &str) -> Vec<((CursorPosition, CursorPosition), String)> {
        let mut buffer = Vec::new();
        if pat.is_empty() {
            return buffer;
        }
        for (line_idx, line_content) in self.content.iter().enumerate() {
            for (char_idx, _) in line_content.match_indices(pat) {
                buffer.push((
                    ((line_idx, char_idx).into(), (line_idx, char_idx + pat.len()).into()),
                    line_content.to_owned(),
                ));
            }
        }
        buffer
    }

    pub fn cut(&mut self) -> Option<String> {
        if self.content.is_empty() {
            return None;
        }
        Some(self.actions.cut(&mut self.cursor, &mut self.content))
    }

    pub fn copy(&mut self) -> Option<String> {
        if self.content.is_empty() {
            None
        } else if let Some((from, to)) = self.cursor.select_get() {
            Some(copy_content(from, to, &self.content))
        } else {
            Some(format!("{}\n", &self.content[self.cursor.line]))
        }
    }

    pub fn select_all(&mut self) {
        self.cursor.select_set(
            CursorPosition::default(),
            CursorPosition {
                line: self.content.len() - 1,
                char: self.content.last().map(|line| line.len()).unwrap_or_default(),
            },
        );
    }

    pub fn paste(&mut self, clip: String) {
        self.actions.paste(clip, &mut self.cursor, &mut self.content);
    }

    pub fn undo(&mut self) {
        self.actions.undo(&mut self.cursor, &mut self.content);
    }

    pub fn redo(&mut self) {
        self.actions.redo(&mut self.cursor, &mut self.content);
    }

    pub fn end_of_line(&mut self) {
        self.cursor.end_of_line(&self.content);
    }

    pub fn end_of_file(&mut self) {
        self.cursor.end_of_file(&self.content);
    }

    pub fn start_of_file(&mut self) {
        self.cursor.start_of_file();
    }

    pub fn start_of_line(&mut self) {
        self.cursor.start_of_line(&self.content);
    }

    pub fn up(&mut self) {
        self.cursor.up(&self.content);
    }

    pub fn select_up(&mut self) {
        self.cursor.select_up(&self.content);
    }

    pub fn scroll_up(&mut self) {
        self.cursor.scroll_up(&self.content);
    }

    pub fn swap_up(&mut self) {
        self.actions.swap_up(&mut self.cursor, &mut self.content);
    }

    pub fn down(&mut self) {
        self.cursor.down(&self.content);
    }

    pub fn select_down(&mut self) {
        self.cursor.select_down(&self.content);
    }

    pub fn scroll_down(&mut self) {
        self.cursor.scroll_down(&self.content);
    }

    pub fn swap_down(&mut self) {
        self.actions.swap_down(&mut self.cursor, &mut self.content);
    }

    pub fn left(&mut self) {
        self.cursor.left(&self.content);
    }

    pub fn jump_left(&mut self) {
        self.cursor.jump_left(&self.content);
    }

    pub fn jump_left_select(&mut self) {
        self.cursor.jump_left_select(&self.content);
    }

    pub fn select_left(&mut self) {
        self.cursor.select_left(&self.content);
    }

    pub fn right(&mut self) {
        self.cursor.right(&self.content);
    }

    pub fn jump_right(&mut self) {
        self.cursor.jump_right(&self.content);
    }

    pub fn jump_right_select(&mut self) {
        self.cursor.jump_right_select(&self.content);
    }

    pub fn select_right(&mut self) {
        self.cursor.select_right(&self.content);
    }

    pub fn new_line(&mut self) {
        self.actions.new_line(&mut self.cursor, &mut self.content);
    }

    pub fn push(&mut self, ch: char) {
        self.actions.push_char(ch, &mut self.cursor, &mut self.content);
        let line = &self.content[self.cursor.line];
        if self.lexer.should_autocomplete(self.cursor.char, line) {
            self.actions.force_sync(&mut self.lexer, &self.content);
            self.lexer.get_autocomplete(self.cursor.position(), line);
        }
    }

    pub fn backspace(&mut self) {
        self.actions.backspace(&mut self.cursor, &mut self.content);
    }

    pub fn del(&mut self) {
        self.actions.del(&mut self.cursor, &mut self.content);
    }

    pub fn indent(&mut self) {
        self.actions.indent(&mut self.cursor, &mut self.content);
    }

    pub fn indent_start(&mut self) {
        self.actions.indent_start(&mut self.cursor, &mut self.content);
    }

    pub fn unindent(&mut self) {
        self.actions.unindent(&mut self.cursor, &mut self.content);
    }

    pub fn save(&mut self, gs: &mut GlobalState) {
        if self.try_write_file(gs) {
            self.lexer.save_and_check_lsp(self.file_type, gs);
        }
    }

    pub fn try_write_file(&self, gs: &mut GlobalState) -> bool {
        if let Err(error) = std::fs::write(&self.path, self.content.join("\n")) {
            gs.error(error.to_string());
            return false;
        }
        true
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.lexer.reload_theme();
        self.actions.cfg = new_cfg.clone();
    }

    pub fn stringify(&self) -> String {
        let mut text = self.content.join("\n");
        text.push('\n');
        text
    }

    pub fn resize(&mut self, height: usize, width: usize) {
        self.max_rows = height;
        if self.cursor.line < self.cursor.at_line {
            self.cursor.at_line = self.cursor.line
        }
        if self.cursor.line > self.max_rows - 3 + self.cursor.at_line {
            self.cursor.at_line = self.cursor.line + 2 - self.max_rows
        }
        self.cursor.text_width = width.saturating_sub(self.lexer.line_number_offset + 1);
        self.lexer.set_text_width(width);
    }
}

fn build_display(path: &Path) -> String {
    let mut buffer = Vec::new();
    let mut text_path = path.display().to_string();
    if let Ok(base_path) = PathBuf::from("./").canonicalize().map(|p| p.display().to_string()) {
        if let Some(rel) = text_path.strip_prefix(&base_path).to_owned() {
            text_path = rel.to_owned();
        }
    }
    for part in text_path.split(MAIN_SEPARATOR).rev().take(2) {
        buffer.insert(0, part);
    }
    buffer.join(MAIN_SEPARATOR_STR)
}

#[cfg(test)]
pub mod test;

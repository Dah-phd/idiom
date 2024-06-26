use crate::{
    configs::{EditorConfigs, FileType},
    global_state::GlobalState,
    render::layout::Rect,
    syntax::Lexer,
    workspace::{
        actions::Actions,
        cursor::{Cursor, CursorPosition},
        line::{CodeLine, CodeLineContext, Context, EditorLine},
        utils::{copy_content, find_line_start, last_modified, token_range_at},
    },
};
use lsp_types::TextEdit;
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    time::SystemTime,
};

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
    pub content: Vec<CodeLine>,
    timestamp: Option<SystemTime>,
    pub line_number_offset: usize,
    last_render_at_line: Option<usize>,
}

impl Editor {
    pub fn from_path(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> std::io::Result<Self> {
        let content: Vec<_> =
            std::fs::read_to_string(&path)?.split('\n').map(|line| CodeLine::new(line.to_owned())).collect();
        let file_type = FileType::derive_type(&path);
        let display = build_display(&path);
        Ok(Self {
            line_number_offset: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
            lexer: Lexer::with_context(file_type, &path, gs),
            content,
            cursor: Cursor::default(),
            actions: Actions::new(cfg.get_indent_cfg(&file_type)),
            file_type,
            display,
            timestamp: last_modified(&path),
            path,
            last_render_at_line: None,
        })
    }

    #[inline]
    pub fn render(&mut self, gs: &mut GlobalState) {
        self.last_render_at_line.replace(self.cursor.at_line);
        self.sync(gs);
        let mut lines = gs.editor_area.into_iter();
        let mut ctx = CodeLineContext::collect_context(&mut self.lexer, &self.cursor, self.line_number_offset);
        for (line_idx, text) in self.content.iter_mut().enumerate().skip(self.cursor.at_line) {
            if self.cursor.line == line_idx && text.len() > self.cursor.text_width {
                text.wrapped_render(&mut ctx, &mut lines, &mut gs.writer);
            } else if let Some(line) = lines.next() {
                text.render(&mut ctx, line, &mut gs.writer);
            } else {
                break;
            };
        }
        for line in lines {
            line.render_empty(&mut gs.writer);
        }
        gs.render_stats(self.content.len(), self.cursor.select_len(&self.content), (&self.cursor).into());
        ctx.render_cursor(gs);
    }

    /// renders only updated lines
    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.last_render_at_line.is_none()
            || matches!(self.last_render_at_line, Some(idx) if idx != self.cursor.at_line)
        {
            return self.render(gs);
        }
        self.sync(gs);
        let mut lines = gs.editor_area.into_iter();
        let mut ctx = CodeLineContext::collect_context(&mut self.lexer, &self.cursor, self.line_number_offset);
        for (line_idx, text) in self.content.iter_mut().enumerate().skip(self.cursor.at_line) {
            if self.cursor.line == line_idx {
                if text.len() > self.cursor.text_width {
                    text.wrapped_render(&mut ctx, &mut lines, &mut gs.writer);
                } else if let Some(line) = lines.next() {
                    text.render(&mut ctx, line, &mut gs.writer);
                } else {
                    break;
                }
            } else if let Some(line) = lines.next() {
                text.fast_render(&mut ctx, line, &mut gs.writer);
            } else {
                break;
            };
        }
        for line in lines {
            line.render_empty(&mut gs.writer);
        }
        gs.render_stats(self.content.len(), self.cursor.select_len(&self.content), (&self.cursor).into());
        ctx.render_cursor(gs);
    }

    #[inline]
    pub fn clear_screen_cache(&mut self) {
        self.last_render_at_line = None;
    }

    #[inline]
    pub fn updated_rect(&mut self, rect: Rect, gs: &GlobalState) {
        let skip_offset = rect.row.saturating_sub(gs.editor_area.row) as usize;
        for line in self.content.iter_mut().skip(self.cursor.at_line + skip_offset).take(rect.width as usize) {
            line.clear_cache();
        }
    }

    #[inline]
    pub fn sync(&mut self, gs: &mut GlobalState) {
        let new_line_number_offset =
            if self.content.is_empty() { 0 } else { (self.content.len().ilog10() + 1) as usize };
        if new_line_number_offset != self.line_number_offset {
            self.line_number_offset = new_line_number_offset;
            self.last_render_at_line.take();
        };
        Lexer::context(self, gs);
        self.cursor.correct_cursor_position(&self.content);
    }

    pub fn get_stats(&self) -> DocStats {
        (self.content.len(), self.cursor.select_len(&self.content), (&self.cursor).into())
    }

    #[inline]
    pub fn update_path(&mut self, new_path: PathBuf) {
        self.display = build_display(&new_path);
        self.path = new_path;
    }

    pub fn help(&mut self, gs: &mut GlobalState) {
        self.lexer.help((&self.cursor).into(), &self.content, gs);
    }

    pub fn references(&mut self, gs: &mut GlobalState) {
        self.lexer.go_to_reference((&self.cursor).into(), gs);
    }

    pub fn declarations(&mut self, gs: &mut GlobalState) {
        self.lexer.go_to_declaration((&self.cursor).into(), gs);
    }

    pub fn select_token(&mut self) {
        let range = token_range_at(&self.content[self.cursor.line], self.cursor.char);
        if !range.is_empty() {
            self.cursor.select_set(
                CursorPosition { line: self.cursor.line, char: range.start },
                CursorPosition { line: self.cursor.line, char: range.end },
            )
        }
    }

    pub fn select_line(&mut self) {
        let start = CursorPosition { line: self.cursor.line, char: 0 };
        let next_line = self.cursor.line + 1;
        if self.content.len() > next_line {
            self.cursor.select_set(start, CursorPosition { line: next_line, char: 0 });
        } else {
            let char = self.content[start.line].len();
            if char == 0 {
                return;
            };
            self.cursor.select_set(start, CursorPosition { line: self.cursor.line, char });
        };
    }

    pub fn remove_line(&mut self) {
        self.select_line();
        if !self.cursor.select_is_none() {
            self.del();
        };
    }

    pub fn start_renames(&mut self) {
        let line = &self.content[self.cursor.line];
        let token_range = token_range_at(line, self.cursor.char);
        self.lexer.start_rename((&self.cursor).into(), &line[token_range]);
    }

    pub fn is_saved(&self) -> bool {
        if let Ok(file_content) = std::fs::read_to_string(&self.path) {
            return self
                .content
                .iter()
                .map(|l| l.to_string())
                .eq(file_content.split('\n').map(String::from).collect::<Vec<_>>());
        };
        false
    }

    pub fn insert_text_with_relative_offset(&mut self, insert: String) {
        self.actions.insert_top_cursor_relative_offset(insert, &mut self.cursor, &mut self.content);
    }

    pub fn replace_select(&mut self, from: CursorPosition, to: CursorPosition, new_clip: &str) {
        self.actions.replace_select(from, to, new_clip, &mut self.cursor, &mut self.content);
    }

    pub fn replace_token(&mut self, new: String) {
        self.actions.replace_token(new, &mut self.cursor, &mut self.content);
    }

    pub fn insert_snippet(&mut self, snippet: String, cursor_offset: Option<(usize, usize)>) {
        self.actions.insert_snippet(&mut self.cursor, snippet, cursor_offset, &mut self.content);
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
            self.cursor.char = find_line_start(&self.content[line]);
            self.cursor.at_line = line.saturating_sub(self.cursor.max_rows / 2);
        }
    }

    pub fn go_to_select(&mut self, from: CursorPosition, to: CursorPosition) {
        self.cursor.at_line = to.line.saturating_sub(self.cursor.max_rows / 2);
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
                    line_content.to_string(),
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

    pub fn mouse_cursor(&mut self, mut position: CursorPosition) {
        self.cursor.select_drop();
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        self.cursor.set_cursor_checked(position, &self.content);
    }

    pub fn mouse_select(&mut self, mut position: CursorPosition) {
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        self.cursor.set_cursor_checked_with_select(position, &self.content);
    }

    pub fn mouse_copy_paste(&mut self, mut position: CursorPosition, clip: Option<String>) -> Option<String> {
        if let Some((from, to)) = self.cursor.select_get() {
            return Some(copy_content(from, to, &self.content));
        };
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        self.cursor.set_cursor_checked(position, &self.content);
        self.paste(clip?);
        None
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

    pub fn push(&mut self, ch: char, gs: &mut GlobalState) {
        self.actions.push_char(ch, &mut self.cursor, &mut self.content);
        let line = &self.content[self.cursor.line];
        if self.lexer.should_autocomplete(self.cursor.char, line) {
            let line = line.to_string();
            self.actions.push_buffer();
            Lexer::sync(self, gs);
            self.lexer.get_autocomplete((&self.cursor).into(), line, gs);
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

    pub fn comment_out(&mut self) {
        self.actions.comment_out(self.file_type.comment_start(), &mut self.cursor, &mut self.content);
    }

    pub fn save(&mut self, gs: &mut GlobalState) {
        if self.try_write_file(gs) {
            self.lexer.save_and_check_lsp(gs);
            gs.success(format!("SAVED {}", self.path.display()));
        }
    }

    pub fn try_write_file(&self, gs: &mut GlobalState) -> bool {
        if let Err(error) =
            std::fs::write(&self.path, self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n"))
        {
            gs.error(error.to_string());
            return false;
        }
        true
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.actions.cfg = new_cfg.get_indent_cfg(&self.file_type);
    }

    #[inline]
    pub fn stringify(&self) -> String {
        let mut text = self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.cursor.max_rows = height;
        let offset = if self.content.is_empty() { 0 } else { (self.content.len().ilog10() + 1) as usize };
        self.cursor.text_width = width.saturating_sub(offset + 1);
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

impl Drop for Editor {
    fn drop(&mut self) {
        self.lexer.close(&self.path);
    }
}

#[cfg(test)]
pub mod test;

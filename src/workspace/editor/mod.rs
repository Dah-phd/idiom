mod controls;
mod utils;
use super::{
    actions::Actions,
    cursor::{Cursor, CursorPosition},
    line::EditorLine,
    renderer::Renderer,
    utils::{copy_content, find_line_start, token_range_at},
};
use crate::{
    configs::{EditorAction, EditorConfigs, FileType, IndentConfigs},
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    lsp::LSPError,
    syntax::{tokens::calc_wraps, Lexer},
};
use idiom_tui::{layout::Rect, Position};
use lsp_types::TextEdit;
use std::{cmp::Ordering, path::PathBuf};
use utils::{big_file_protection, build_display, calc_line_number_offset, FileUpdate};
pub use utils::{editor_from_data, text_editor_from_data};

pub struct Editor {
    pub file_type: FileType,
    pub display: String,
    pub path: PathBuf,
    pub lexer: Lexer,
    pub cursor: Cursor,
    pub content: Vec<EditorLine>,
    pub update_status: FileUpdate,
    pub line_number_offset: usize,
    pub last_render_at_line: Option<usize>,
    actions: Actions,
    renderer: Renderer,
    action_map: fn(&mut Self, EditorAction, gs: &mut GlobalState) -> bool,
    multi_positions: Vec<Cursor>,
}

impl Editor {
    pub fn from_path(
        path: PathBuf,
        file_type: FileType,
        cfg: &EditorConfigs,
        gs: &mut GlobalState,
    ) -> IdiomResult<Self> {
        big_file_protection(&path)?;
        let content = EditorLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let display = build_display(&path);
        let line_number_offset = calc_line_number_offset(content.len());
        Ok(Self {
            cursor: Cursor::sized(*gs.editor_area(), line_number_offset),
            multi_positions: Vec::new(),
            line_number_offset,
            lexer: Lexer::with_context(file_type, &path, gs),
            content,
            renderer: Renderer::code(),
            actions: Actions::new(cfg.get_indent_cfg(&file_type)),
            action_map: controls::single_cursor_map,
            file_type,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
        })
    }

    pub fn from_path_text(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> IdiomResult<Self> {
        big_file_protection(&path)?;
        gs.message(
            "The file is opened in text mode, beware idiom is not designed with plain text performance in mind!",
        );
        let mut content = EditorLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let display = build_display(&path);
        let line_number_offset = calc_line_number_offset(content.len());
        let cursor = Cursor::sized(*gs.editor_area(), line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            multi_positions: Vec::new(),
            line_number_offset,
            lexer: Lexer::text_lexer(&path, gs),
            content,
            renderer: Renderer::text(),
            actions: Actions::new(cfg.default_indent_cfg()),
            action_map: controls::single_cursor_map,
            file_type: FileType::Ignored,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
        })
    }

    pub fn from_path_md(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> IdiomResult<Self> {
        big_file_protection(&path)?;
        gs.message("The file is opened in MD mode, beware idiom is not designed with MD performance in mind!");
        let mut content = EditorLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let display = build_display(&path);
        let line_number_offset = calc_line_number_offset(content.len());
        let cursor = Cursor::sized(*gs.editor_area(), line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            multi_positions: Vec::new(),
            line_number_offset,
            lexer: Lexer::text_lexer(&path, gs),
            content,
            renderer: Renderer::markdown(),
            actions: Actions::new(cfg.default_indent_cfg()),
            action_map: controls::single_cursor_map,
            file_type: FileType::Ignored,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
        })
    }

    // RENDER

    #[inline]
    pub fn render(&mut self, gs: &mut GlobalState) {
        let new_offset = calc_line_number_offset(self.content.len());
        if new_offset != self.line_number_offset {
            self.line_number_offset = new_offset;
            self.last_render_at_line.take();
        };
        (self.renderer.render)(self, gs);
    }

    /// renders only updated lines
    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        let new_offset = calc_line_number_offset(self.content.len());
        if new_offset != self.line_number_offset {
            self.line_number_offset = new_offset;
            self.last_render_at_line.take();
        };
        (self.renderer.fast_render)(self, gs)
    }

    pub fn clear_ui(&mut self, gs: &GlobalState) {
        if let Some(rect) = self.lexer.clear_modal() {
            self.updated_rect(rect, gs);
        }
    }

    #[inline(always)]
    pub fn clear_screen_cache(&mut self, gs: &mut GlobalState) {
        self.lexer.refresh_lsp(gs);
        self.last_render_at_line = None;
    }

    pub fn updated_rect(&mut self, rect: Rect, gs: &GlobalState) {
        let skip_offset = rect.row.saturating_sub(gs.editor_area().row) as usize;
        for line in self.content.iter_mut().skip(self.cursor.at_line + skip_offset).take(rect.width) {
            line.cached.reset();
        }
    }

    // MAPPING

    #[inline]
    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> bool {
        (self.action_map)(self, action, gs)
    }

    pub fn enable_multi_cursors(&mut self) {
        self.multi_positions.clear();
        self.multi_positions.push(self.cursor.clone());
        self.action_map = controls::multi_cursor_map;
    }

    pub fn disable_multi_cursor(&mut self) {
        self.cursor.conjoin_cursor(&mut self.multi_positions);
        self.action_map = controls::single_cursor_map;
    }

    pub fn update_path(&mut self, new_path: PathBuf) -> Result<(), LSPError> {
        self.display = build_display(&new_path);
        self.path = new_path;
        self.lexer.update_path(&self.path)
    }

    pub fn file_type_set(&mut self, file_type: FileType, cfg: IndentConfigs, gs: &mut GlobalState) {
        self.actions.cfg = cfg;
        self.lexer = Lexer::with_context(file_type, &self.path, gs);
        self.file_type = file_type;
        match self.file_type {
            FileType::Ignored => self.renderer = Renderer::text(),
            _ => self.renderer = Renderer::code(),
        }
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
            let char = self.content[start.line].char_len();
            if char == 0 {
                return;
            };
            self.cursor.select_set(start, CursorPosition { line: self.cursor.line, char });
        };
    }

    pub fn go_to(&mut self, line: usize) {
        self.cursor.select_drop();
        if self.content.len() <= line {
            return;
        };
        self.cursor.line = line;
        self.cursor.char = find_line_start(&self.content[line]);
        self.cursor.at_line = line.saturating_sub(self.cursor.max_rows / 2);
    }

    pub fn go_to_select(&mut self, from: CursorPosition, to: CursorPosition) {
        self.cursor.at_line = to.line.saturating_sub(self.cursor.max_rows / 2);
        self.cursor.select_set(from, to);
    }

    pub fn find(&self, pat: &str, buffer: &mut Vec<(CursorPosition, CursorPosition)>) {
        if pat.is_empty() {
            return;
        }
        for (line_idx, line_content) in self.content.iter().enumerate() {
            for (char_idx, _) in line_content.match_indices(pat) {
                buffer.push(((line_idx, char_idx).into(), (line_idx, char_idx + pat.len()).into()));
            }
        }
    }

    pub fn find_with_select(&mut self, pat: &str) -> Vec<((CursorPosition, CursorPosition), String)> {
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

    pub fn is_saved(&self) -> IdiomResult<bool> {
        // for most source code files direct read should be faster
        let file_content = std::fs::read_to_string(&self.path)?;

        let mut counter = 0_usize;
        for expected in file_content.split('\n') {
            match self.content.get(counter) {
                Some(eline) if eline.content.as_str() == expected => {
                    counter += 1;
                }
                _ => return Ok(false),
            }
        }
        Ok(self.content.len() == counter)
    }

    pub fn rebase(&mut self, gs: &mut GlobalState) {
        if let Err(error) = big_file_protection(&self.path) {
            gs.error(format!("Failed to load file {error}"));
            return;
        };
        self.actions.clear();
        self.cursor.reset();
        self.lexer.close();
        let content = match std::fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(err) => {
                gs.error(format!("File rebase failed! ERR: {err}"));
                return;
            }
        };
        self.content = content.split('\n').map(|line| EditorLine::new(line.to_owned())).collect();
        match self.lexer.reopen(content, self.file_type) {
            Ok(()) => gs.success("File rebased!"),
            Err(err) => gs.error(format!("Filed to reactivate LSP after rebase! ERR: {err}")),
        }
    }

    pub fn save(&mut self, gs: &mut GlobalState) {
        if let Some(content) = self.try_write_file(gs) {
            self.update_status.deny();
            self.lexer.save_and_check_lsp(content, gs);
            gs.success(format!("SAVED {}", self.path.display()));
        }
    }

    pub fn try_write_file(&self, gs: &mut GlobalState) -> Option<String> {
        let content = self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        if let Err(error) = std::fs::write(&self.path, &content) {
            gs.error(error);
            return None;
        }
        Some(content)
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.actions.cfg = new_cfg.get_indent_cfg(&self.file_type);
    }

    pub fn stringify(&self) -> String {
        let mut text = self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.cursor.max_rows = height;
        self.line_number_offset = calc_line_number_offset(self.content.len());
        self.cursor.text_width = width.saturating_sub(self.line_number_offset + 1);
    }

    // EDITS

    pub fn insert_text_with_relative_offset(&mut self, insert: String) {
        self.actions.insert_top_cursor_relative_offset(insert, &mut self.cursor, &mut self.content, &mut self.lexer);
    }

    pub fn replace_select(&mut self, from: CursorPosition, to: CursorPosition, new_clip: &str) {
        self.actions.replace_select(from, to, new_clip, &mut self.cursor, &mut self.content, &mut self.lexer);
    }

    #[inline(always)]
    pub fn replace_token(&mut self, new: String) {
        self.actions.replace_token(new, &mut self.cursor, &mut self.content, &mut self.lexer);
    }

    #[inline(always)]
    pub fn insert_snippet(&mut self, snippet: String, cursor_offset: Option<(usize, usize)>) {
        self.actions.insert_snippet(&mut self.cursor, snippet, cursor_offset, &mut self.content, &mut self.lexer);
    }

    #[inline(always)]
    pub fn insert_snippet_with_select(&mut self, snippet: String, cursor_offset: (usize, usize), len: usize) {
        self.actions.insert_snippet_with_select(
            &mut self.cursor,
            snippet,
            cursor_offset,
            len,
            &mut self.content,
            &mut self.lexer,
        );
    }

    pub fn mass_replace(&mut self, mut ranges: Vec<(CursorPosition, CursorPosition)>, clip: String) {
        ranges.sort_by(|a, b| {
            let line_ord = b.0.line.cmp(&a.0.line);
            if let Ordering::Equal = line_ord {
                return b.0.char.cmp(&a.0.char);
            }
            line_ord
        });
        self.actions.mass_replace(&mut self.cursor, ranges, clip, &mut self.content, &mut self.lexer);
    }

    pub fn apply_file_edits(&mut self, mut edits: Vec<TextEdit>) {
        edits.sort_by(|a, b| {
            let line_ord = b.range.start.line.cmp(&a.range.start.line);
            if let Ordering::Equal = line_ord {
                return b.range.start.character.cmp(&a.range.start.character);
            }
            line_ord
        });
        self.actions.apply_edits(edits, &mut self.content, &mut self.lexer);
    }

    #[inline(always)]
    pub fn cut(&mut self) -> Option<String> {
        if self.content.is_empty() {
            return None;
        }
        Some(self.actions.cut(&mut self.cursor, &mut self.content, &mut self.lexer))
    }

    #[inline(always)]
    pub fn copy(&mut self) -> Option<String> {
        if self.content.is_empty() {
            None
        } else if let Some((from, to)) = self.cursor.select_get() {
            Some(copy_content(from, to, &self.content))
        } else {
            Some(format!("{}\n", &self.content[self.cursor.line]))
        }
    }

    pub fn paste(&mut self, clip: String) {
        self.actions.paste(clip, &mut self.cursor, &mut self.content, &mut self.lexer);
    }

    #[inline(always)]
    pub fn select_all(&mut self) {
        self.cursor.select_set(
            CursorPosition::default(),
            CursorPosition {
                line: self.content.len() - 1,
                char: self.content.last().map(|line| line.char_len()).unwrap_or_default(),
            },
        );
    }

    // MOUSE

    pub fn mouse_scroll_up(&mut self, select: bool, gs: &mut GlobalState) {
        let (taken, render_update) = self.lexer.map_modal_if_exists(EditorAction::ScrollUp, gs);
        if let Some(modal_rect) = render_update {
            self.updated_rect(modal_rect, gs);
        }
        if taken {
            return;
        };
        match select {
            true => {
                self.cursor.select_scroll_up(&self.content);
                self.cursor.select_scroll_up(&self.content);
            }
            false => {
                self.cursor.scroll_up(&self.content);
                self.cursor.scroll_up(&self.content);
            }
        }
    }

    pub fn mouse_scroll_down(&mut self, select: bool, gs: &mut GlobalState) {
        let (taken, render_update) = self.lexer.map_modal_if_exists(EditorAction::ScrollDown, gs);
        if let Some(modal_rect) = render_update {
            self.updated_rect(modal_rect, gs);
        }
        if taken {
            return;
        };
        match select {
            true => {
                self.cursor.select_scroll_down(&self.content);
                self.cursor.select_scroll_down(&self.content);
            }
            false => {
                self.cursor.scroll_down(&self.content);
                self.cursor.scroll_down(&self.content);
            }
        }
    }

    pub fn mouse_click(&mut self, position: Position, gs: &mut GlobalState) {
        if let Some(rect) = self.lexer.mouse_click_modal_if_exists(position, gs) {
            self.updated_rect(rect, gs);
            return;
        }
        let mut position = CursorPosition::from(position);
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        if self.cursor.select_is_none() && self.cursor == position {
            self.select_token();
            return;
        }
        self.cursor.select_drop();
        self.cursor.set_cursor_checked(position, &self.content);
    }

    pub fn mouse_menu_setup(&mut self, mut position: CursorPosition) {
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        match self.cursor.select_get() {
            Some((from, to)) if from <= position && position <= to => {
                return;
            }
            Some(..) => self.cursor.select_drop(),
            None => (),
        }
        self.cursor.set_cursor_checked(position, &self.content);
    }

    pub fn mouse_select(&mut self, mut position: CursorPosition) {
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_offset + 1);
        self.cursor.set_cursor_checked_with_select(position, &self.content);
    }

    pub fn mouse_moved(&mut self, row: u16, column: u16, gs: &GlobalState) {
        if let Some(rect) = self.lexer.mouse_moved_modal_if_exists(row, column) {
            self.updated_rect(rect, gs);
        };
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.lexer.close();
    }
}

#[cfg(test)]
pub mod tests;

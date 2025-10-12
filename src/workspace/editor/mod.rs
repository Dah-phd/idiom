mod controls;
mod modal;
mod utils;
use super::{
    actions::Actions,
    cursor::{Cursor, CursorPosition, WordRange},
    line::EditorLine,
    renderer::Renderer,
    utils::find_line_start,
};
use crate::{
    configs::{EditorAction, EditorConfigs, FileFamily, FileType, IndentConfigs},
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    lsp::LSPError,
    syntax::{tokens::calc_wraps, Lexer},
};
use controls::ControlMap;
use idiom_tui::{layout::Rect, Position};
use lsp_types::TextEdit;
pub use modal::EditorModal;
use std::path::PathBuf;
pub use utils::editor_from_data;
use utils::{big_file_protection, build_display, calc_line_number_offset, FileUpdate};

const WARN_TXT: &str = "The file is opened in text mode, \
    beware idiom is not designed with plain text performance in mind!";
const WARN_MD: &str = "The file is opened in markdown mode, \
    beware idiom is not designed with MD performance in mind!";

pub struct Editor {
    pub file_type: FileType,
    pub display: String,
    pub path: PathBuf,
    pub lexer: Lexer,
    pub cursor: Cursor,
    pub content: Vec<EditorLine>,
    pub update_status: FileUpdate,
    pub line_number_padding: usize,
    pub last_render_at_line: Option<usize>,
    pub controls: ControlMap,
    pub modal: EditorModal,
    actions: Actions,
    renderer: Renderer,
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
            line_number_padding: line_number_offset,
            lexer: Lexer::with_context(file_type, &path),
            content,
            renderer: Renderer::code(),
            actions: Actions::new(cfg.get_indent_cfg(file_type)),
            controls: ControlMap::default(),
            file_type,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
            modal: EditorModal::default(),
        })
    }

    pub fn from_path_text(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> IdiomResult<Self> {
        big_file_protection(&path)?;
        gs.message(WARN_TXT);
        let mut content = EditorLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let display = build_display(&path);
        let line_number_offset = calc_line_number_offset(content.len());
        let cursor = Cursor::sized(*gs.editor_area(), line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            line_number_padding: line_number_offset,
            lexer: Lexer::text_lexer(&path),
            content,
            renderer: Renderer::text(),
            actions: Actions::new(cfg.default_indent_cfg()),
            controls: ControlMap::default(),
            file_type: FileType::Text,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
            modal: EditorModal::default(),
        })
    }

    pub fn from_path_md(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> IdiomResult<Self> {
        big_file_protection(&path)?;
        gs.message(WARN_MD);
        let mut content = EditorLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let display = build_display(&path);
        let line_number_offset = calc_line_number_offset(content.len());
        let cursor = Cursor::sized(*gs.editor_area(), line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            line_number_padding: line_number_offset,
            lexer: Lexer::text_lexer(&path),
            content,
            renderer: Renderer::markdown(),
            actions: Actions::new(cfg.default_indent_cfg()),
            controls: ControlMap::default(),
            file_type: FileType::MarkDown,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
            modal: EditorModal::default(),
        })
    }

    // RENDER

    #[inline]
    pub fn render(&mut self, gs: &mut GlobalState) {
        let new_offset = calc_line_number_offset(self.content.len());
        if new_offset != self.line_number_padding {
            self.line_number_padding = new_offset;
            self.last_render_at_line.take();
        };
        (self.renderer.render)(self, gs);
    }

    /// renders only updated lines
    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        let new_offset = calc_line_number_offset(self.content.len());
        if new_offset != self.line_number_padding {
            self.line_number_padding = new_offset;
            self.last_render_at_line.take();
        };
        (self.renderer.fast_render)(self, gs)
    }

    /// check that lines have render cache
    /// estimates if there has been changes to the data within content
    #[inline]
    pub fn has_render_cache(&self) -> bool {
        let render_line_maches = matches!(self.last_render_at_line, Some(val) if val == self.cursor.at_line);
        render_line_maches && Renderer::all_lines_cached(self)
    }

    pub fn clear_ui(&mut self, gs: &GlobalState) {
        if let Some(rect) = self.modal.drop() {
            self.clear_lines_cache(rect, gs);
        }
    }

    #[inline(always)]
    pub fn clear_screen_cache(&mut self, gs: &mut GlobalState) {
        self.lexer.refresh_lsp(gs);
        self.last_render_at_line = None;
    }

    pub fn clear_lines_cache(&mut self, rect: Rect, gs: &GlobalState) {
        let skip_offset = rect.row.saturating_sub(gs.editor_area().row) as usize;
        for line in self.content.iter_mut().skip(self.cursor.at_line + skip_offset).take(rect.width) {
            line.cached.reset();
        }
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.controls.cursors
    }

    // MAPPING

    #[inline]
    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> bool {
        (self.controls.action_map)(self, action, gs)
    }

    pub fn update_path(&mut self, new_path: PathBuf) -> Result<(), LSPError> {
        self.display = build_display(&new_path);
        self.path = new_path;
        self.lexer.update_path(&self.path)
    }

    pub fn file_type_set(&mut self, file_type: FileType, cfg: IndentConfigs, gs: &mut GlobalState) {
        self.actions.cfg = cfg;
        self.file_type = file_type;
        match self.file_type.family() {
            FileFamily::Text => {
                self.renderer = Renderer::text();
                self.lexer = Lexer::text_lexer(&self.path);
                calc_wraps(&mut self.content, self.cursor.text_width);
            }
            FileFamily::MarkDown => {
                self.renderer = Renderer::markdown();
                self.lexer = Lexer::md_lexer(&self.path);
                calc_wraps(&mut self.content, self.cursor.text_width);
            }
            FileFamily::Code(..) => {
                self.renderer = Renderer::code();
                self.lexer = Lexer::with_context(file_type, &self.path);
            }
        };
        gs.force_screen_rebuild();
    }

    pub fn select_token(&mut self) {
        let Some(range) = WordRange::find_at(&self.content, self.cursor.get_position()) else {
            return;
        };
        let (from, to) = range.as_select();
        self.cursor.select_set(from, to);
    }

    fn select_line(&mut self) {
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

    #[inline(always)]
    fn select_all(&mut self) {
        self.cursor.select_set(
            CursorPosition::default(),
            CursorPosition {
                line: self.content.len() - 1,
                char: self.content.last().map(|line| line.char_len()).unwrap_or_default(),
            },
        );
    }

    pub fn go_to(&mut self, line: usize) {
        ControlMap::ensure_single_cursor(self);
        self.cursor.select_drop();
        if self.content.len() <= line {
            return;
        };
        self.cursor.line = line;
        self.cursor.char = find_line_start(&self.content[line]);
        self.cursor.at_line = line.saturating_sub(self.cursor.max_rows / 2);
    }

    pub fn go_to_select(&mut self, from: CursorPosition, to: CursorPosition) {
        ControlMap::ensure_single_cursor(self);
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

    pub fn find_with_text(&self, pat: &str) -> Vec<((CursorPosition, CursorPosition), String)> {
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
                Some(eline) if eline.as_str() == expected => {
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
        if !self.controls.cursors.is_empty() {
            self.controls.cursors.clear();
            self.renderer.single_cursor(self.file_type);
        }
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
        self.actions.cfg = new_cfg.get_indent_cfg(self.file_type);
    }

    pub fn stringify(&self) -> String {
        let mut text = self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.cursor.max_rows = height;
        self.line_number_padding = calc_line_number_offset(self.content.len());
        self.cursor.text_width = width.saturating_sub(self.line_number_padding + 1);
        for pos in self.controls.cursors.iter_mut() {
            pos.text_width = self.cursor.text_width;
        }
    }

    // EDITS (control map pass through)

    #[inline(always)]
    pub fn insert_text_with_relative_offset(&mut self, insert: String) {
        (self.controls.insert_import)(self, insert);
    }

    #[inline(always)]
    pub fn replace_select(&mut self, from: CursorPosition, to: CursorPosition, new_clip: &str) {
        (self.controls.replace_select)(self, from, to, new_clip);
    }

    #[inline(always)]
    pub fn replace_token(&mut self, new: String) {
        (self.controls.replace_token)(self, new);
    }

    #[inline(always)]
    pub fn insert_snippet(&mut self, snippet: String, cursor_offset: Option<(usize, usize)>) {
        (self.controls.insert_snippet)(self, snippet, cursor_offset);
    }

    #[inline(always)]
    pub fn insert_snippet_with_select(&mut self, snippet: String, cursor_offset: (usize, usize), len: usize) {
        (self.controls.insert_snippet_with_select)(self, snippet, cursor_offset, len);
    }

    #[inline(always)]
    pub fn mass_replace(&mut self, ranges: Vec<(CursorPosition, CursorPosition)>, clip: String) {
        (self.controls.mass_replace)(self, ranges, clip);
    }

    #[inline(always)]
    pub fn apply_file_edits(&mut self, edits: Vec<TextEdit>) {
        (self.controls.apply_file_edits)(self, edits)
    }

    #[inline(always)]
    pub fn copy(&mut self) -> Option<String> {
        (self.controls.copy)(self)
    }

    #[inline(always)]
    pub fn paste(&mut self, clip: String, gs: &mut GlobalState) {
        let (taken, modal_rect) = self.modal.paste_if_exists(&clip);
        if let Some(rect) = modal_rect {
            self.clear_lines_cache(rect, gs);
        }
        if taken {
            return;
        }
        (self.controls.paste)(self, clip)
    }

    // MOUSE

    pub fn mouse_scroll_up(&mut self, select: bool, gs: &mut GlobalState) {
        let (taken, render_update) = EditorModal::map_if_exists(self, EditorAction::ScrollUp, gs);
        if let Some(modal_rect) = render_update {
            self.clear_lines_cache(modal_rect, gs);
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
        let (taken, render_update) = EditorModal::map_if_exists(self, EditorAction::ScrollDown, gs);
        if let Some(modal_rect) = render_update {
            self.clear_lines_cache(modal_rect, gs);
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
        ControlMap::ensure_single_cursor(self);
        if let Some(rect) = EditorModal::mouse_click_if_exists(self, position, gs) {
            self.clear_lines_cache(rect, gs);
            return;
        }
        let position = self.mouse_parse(position);
        if self.cursor.select_is_none() && self.cursor == position {
            self.select_token();
            return;
        }
        self.cursor.select_drop();
        self.cursor.set_cursor_checked(position, &self.content);
    }

    pub fn mouse_multi_cursor(&mut self, position: Position) {
        self.modal.drop();
        self.last_render_at_line = None;
        let position = self.mouse_parse(position);
        if self.cursor == position {
            return;
        }
        controls::push_multicursor_position(self, position);
    }

    pub fn mouse_select_to(&mut self, position: Position, gs: &mut GlobalState) {
        ControlMap::ensure_single_cursor(self);
        if let Some(rect) = EditorModal::mouse_click_if_exists(self, position, gs) {
            self.clear_lines_cache(rect, gs);
            return;
        }
        let position = self.mouse_parse(position);
        self.cursor.select_to(position);
    }

    pub fn mouse_menu_setup(&mut self, position: Position) {
        let position = self.mouse_parse(position);
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
        position.char = position.char.saturating_sub(self.line_number_padding + 1);
        self.cursor.set_cursor_checked_with_select(position, &self.content);
    }

    pub fn mouse_moved(&mut self, row: u16, column: u16, gs: &GlobalState) {
        if let Some(rect) = self.modal.mouse_moved_if_exists(row, column) {
            self.clear_lines_cache(rect, gs);
        };
    }

    fn mouse_parse(&self, position: Position) -> CursorPosition {
        let mut position = CursorPosition::from(position);
        position.line += self.cursor.at_line;
        position.char = position.char.saturating_sub(self.line_number_padding + 1);
        position
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.lexer.close();
    }
}

#[cfg(test)]
pub mod tests;

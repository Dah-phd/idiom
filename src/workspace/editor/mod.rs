mod utils;
use super::{
    actions::Actions,
    cursor::{Cursor, CursorPosition},
    line::EditorLine,
    renderer::Renderer,
    utils::{copy_content, find_line_start, token_range_at},
};
use crate::{
    configs::{EditorAction, EditorConfigs, FileType},
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    lsp::LSPError,
    syntax::{tokens::calc_wraps, Lexer},
};
use idiom_tui::layout::Rect;
use lsp_types::TextEdit;
use std::{cmp::Ordering, path::PathBuf};
use utils::{big_file_protection, build_display, calc_line_number_offset, FileUpdate};

#[allow(dead_code)]
pub struct Editor {
    pub file_type: FileType,
    pub display: String,
    pub path: PathBuf,
    pub lexer: Lexer,
    pub cursor: Cursor,
    actions: Actions,
    pub content: Vec<EditorLine>,
    renderer: Renderer,
    pub update_status: FileUpdate,
    pub line_number_offset: usize,
    pub last_render_at_line: Option<usize>,
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
            cursor: Cursor::sized(gs, line_number_offset),
            line_number_offset,
            lexer: Lexer::with_context(file_type, &path, gs),
            content,
            renderer: Renderer::code(),
            actions: Actions::new(cfg.get_indent_cfg(&file_type)),
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
        let cursor = Cursor::sized(gs, line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            line_number_offset,
            lexer: Lexer::text_lexer(&path, gs),
            content,
            renderer: Renderer::text(),
            actions: Actions::new(cfg.default_indent_cfg()),
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
        let cursor = Cursor::sized(gs, line_number_offset);
        calc_wraps(&mut content, cursor.text_width);
        Ok(Self {
            cursor,
            line_number_offset,
            lexer: Lexer::text_lexer(&path, gs),
            content,
            renderer: Renderer::markdown(),
            actions: Actions::new(cfg.default_indent_cfg()),
            file_type: FileType::Ignored,
            display,
            update_status: FileUpdate::None,
            path,
            last_render_at_line: None,
        })
    }

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

    #[inline(always)]
    pub fn clear_screen_cache(&mut self, gs: &mut GlobalState) {
        self.lexer.refresh_lsp(gs);
        self.last_render_at_line = None;
    }

    pub fn updated_rect(&mut self, rect: Rect, gs: &GlobalState) {
        let skip_offset = rect.row.saturating_sub(gs.editor_area.row) as usize;
        for line in self.content.iter_mut().skip(self.cursor.at_line + skip_offset).take(rect.width) {
            line.cached.reset();
        }
    }

    pub fn update_path(&mut self, new_path: PathBuf) -> Result<(), LSPError> {
        self.display = build_display(&new_path);
        self.path = new_path;
        self.lexer.update_path(&self.path)
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> bool {
        let (taken, render_update) = self.lexer.map_modal_if_exists(action, gs);
        if let Some(modal_rect) = render_update {
            self.updated_rect(modal_rect, gs);
        }
        if taken {
            return true;
        };
        match action {
            // EDITS:
            EditorAction::Char(ch) => {
                self.actions.push_char(ch, &mut self.cursor, &mut self.content, &mut self.lexer);
                let line = &self.content[self.cursor.line];
                if self.lexer.should_autocomplete(self.cursor.char, line) {
                    let line = line.to_string();
                    self.actions.push_buffer(&mut self.lexer);
                    self.lexer.get_autocomplete((&self.cursor).into(), line, gs);
                }
                return true;
            }
            EditorAction::Backspace => {
                self.actions.backspace(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::Delete => {
                self.actions.del(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::NewLine => {
                self.actions.new_line(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::Indent => {
                self.actions.indent(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::RemoveLine => {
                self.select_line();
                if !self.cursor.select_is_none() {
                    self.actions.del(&mut self.cursor, &mut self.content, &mut self.lexer);
                    return true;
                };
            }
            EditorAction::IndentStart => {
                self.actions.indent_start(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::Unintent => {
                self.actions.unindent(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::SwapUp => {
                self.actions.swap_up(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::SwapDown => {
                self.actions.swap_down(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::Undo => {
                self.actions.undo(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::Redo => {
                self.actions.redo(&mut self.cursor, &mut self.content, &mut self.lexer);
                return true;
            }
            EditorAction::CommentOut => {
                self.actions.comment_out(
                    self.file_type.comment_start(),
                    &mut self.cursor,
                    &mut self.content,
                    &mut self.lexer,
                );
                return true;
            }
            EditorAction::Paste => {
                if let Some(clip) = gs.clipboard.pull() {
                    self.actions.paste(clip, &mut self.cursor, &mut self.content, &mut self.lexer);
                    return true;
                }
            }
            EditorAction::Cut => {
                if let Some(clip) = self.cut() {
                    gs.clipboard.push(clip);
                    return true;
                }
            }
            EditorAction::Copy => {
                if let Some(clip) = self.copy() {
                    gs.clipboard.push(clip);
                    return true;
                }
            }
            // CURSOR:
            EditorAction::Up => self.cursor.up(&self.content),
            EditorAction::Down => self.cursor.down(&self.content),
            EditorAction::Left => self.cursor.left(&self.content),
            EditorAction::Right => self.cursor.right(&self.content),
            EditorAction::SelectUp => self.cursor.select_up(&self.content),
            EditorAction::SelectDown => self.cursor.select_down(&self.content),
            EditorAction::SelectLeft => self.cursor.select_left(&self.content),
            EditorAction::SelectRight => self.cursor.select_right(&self.content),
            EditorAction::SelectToken => {
                let range = token_range_at(&self.content[self.cursor.line], self.cursor.char);
                if !range.is_empty() {
                    self.cursor.select_set(
                        CursorPosition { line: self.cursor.line, char: range.start },
                        CursorPosition { line: self.cursor.line, char: range.end },
                    )
                }
            }
            EditorAction::SelectLine => self.select_line(),
            EditorAction::SelectAll => self.select_all(),
            EditorAction::ScrollUp => self.cursor.scroll_up(&self.content),
            EditorAction::ScrollDown => self.cursor.scroll_down(&self.content),
            EditorAction::JumpLeft => self.cursor.jump_left(&self.content),
            EditorAction::JumpLeftSelect => self.cursor.jump_left_select(&self.content),
            EditorAction::JumpRight => self.cursor.jump_right(&self.content),
            EditorAction::JumpRightSelect => self.cursor.jump_right_select(&self.content),
            EditorAction::EndOfLine => self.cursor.end_of_line(&self.content),
            EditorAction::EndOfFile => self.cursor.end_of_file(&self.content),
            EditorAction::StartOfLine => self.cursor.start_of_line(&self.content),
            EditorAction::StartOfFile => self.cursor.start_of_file(),
            EditorAction::FindReferences => self.lexer.go_to_reference((&self.cursor).into(), gs),
            EditorAction::GoToDeclaration => self.lexer.go_to_declaration((&self.cursor).into(), gs),
            EditorAction::Help => self.lexer.help((&self.cursor).into(), &self.content, gs),
            EditorAction::LSPRename => {
                let line = &self.content[self.cursor.line];
                let token_range = token_range_at(line, self.cursor.char);
                self.lexer.start_rename((&self.cursor).into(), &line[token_range]);
            }
            EditorAction::RefreshUI => self.lexer.refresh_lsp(gs),
            EditorAction::Save => self.save(gs),
            EditorAction::Cancel => {
                if self.cursor.select_take().is_none() {
                    self.actions.push_buffer(&mut self.lexer);
                    return false;
                }
            }
            EditorAction::Close => return false,
        }
        self.actions.push_buffer(&mut self.lexer);
        true
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

    pub fn mouse_scroll_up(&mut self, gs: &mut GlobalState) {
        let (taken, render_update) = self.lexer.map_modal_if_exists(EditorAction::SelectUp, gs);
        if let Some(modal_rect) = render_update {
            self.updated_rect(modal_rect, gs);
        }
        if taken {
            return;
        };
        self.cursor.scroll_up(&self.content);
        self.cursor.scroll_up(&self.content);
    }

    pub fn mouse_scroll_down(&mut self, gs: &mut GlobalState) {
        let (taken, render_update) = self.lexer.map_modal_if_exists(EditorAction::SelectDown, gs);
        if let Some(modal_rect) = render_update {
            self.updated_rect(modal_rect, gs);
        }
        if taken {
            return;
        };
        self.cursor.scroll_down(&self.content);
        self.cursor.scroll_down(&self.content);
    }

    pub fn mouse_cursor(&mut self, mut position: CursorPosition) {
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

    pub fn rebase(&mut self, gs: &mut GlobalState) {
        if let Err(error) = big_file_protection(&self.path) {
            gs.error(format!("Failed to load file {}", error));
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
            Err(err) => gs.error(format!("Filed to reactivate LSP after rebase! ERR: {}", err)),
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
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.lexer.close();
    }
}

/// This is not a normal constructor for Editor
/// it should be used in cases where the content is present
/// or real file does not exists
pub fn editor_from_data(
    path: PathBuf,
    mut content: Vec<EditorLine>,
    file_type: FileType,
    cfg: &EditorConfigs,
    gs: &mut GlobalState,
) -> Editor {
    let lexer = match file_type {
        FileType::Ignored => Lexer::text_lexer(&path, gs),
        code_file_type => Lexer::with_context(code_file_type, &path, gs),
    };
    let display = build_display(&path);
    let line_number_offset = calc_line_number_offset(content.len());
    let cursor = Cursor::sized(gs, line_number_offset);
    calc_wraps(&mut content, cursor.text_width);
    Editor {
        actions: Actions::new(cfg.default_indent_cfg()),
        update_status: FileUpdate::None,
        renderer: Renderer::text(),
        last_render_at_line: None,
        cursor,
        line_number_offset,
        lexer,
        content,
        file_type,
        display,
        path,
    }
}

#[cfg(test)]
pub mod code_tests;

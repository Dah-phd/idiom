use crate::{
    configs::{EditorAction, EditorConfigs, FileType},
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    lsp::LSPError,
    render::layout::Rect,
    syntax::theme::Theme,
    workspace::{
        actions::internal::InternalActions,
        cursor::{Cursor, CursorPosition},
        line::{EditorLine, TextLine, TextType},
        utils::{copy_content, find_line_start, token_range_at},
    },
};
use lsp_types::TextEdit;
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

type DocLen = usize;
type SelectLen = usize;
pub type DocStats<'a> = (DocLen, SelectLen, CursorPosition);

#[allow(dead_code)]
pub struct TextEditor {
    pub style: TextType,
    pub display: String,
    pub path: PathBuf,
    pub cursor: Cursor,
    pub actions: InternalActions,
    pub content: Vec<TextLine>,
    theme: Theme,
    pub line_number_offset: usize,
    last_render_at_line: Option<usize>,
}

impl TextEditor {
    pub fn from_path(path: PathBuf, cfg: &EditorConfigs, gs: &mut GlobalState) -> IdiomResult<Self> {
        let content = TextLine::parse_lines(&path).map_err(IdiomError::GeneralError)?;
        let file_type = FileType::default();
        let display = build_display(&path);
        let style = match path.extension() {
            Some(ext) if ext.to_ascii_lowercase() == "md" => TextType::MarkDown,
            _ => TextType::Plain,
        };
        Ok(Self {
            style,
            line_number_offset: if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize },
            content,
            cursor: Cursor::default(),
            actions: InternalActions::new(cfg.get_indent_cfg(&file_type)),
            display,
            theme: gs.unwrap_or_default(Theme::new(), "theme.json: "),
            path,
            last_render_at_line: None,
        })
    }

    #[inline]
    pub fn render(&mut self, _: &mut GlobalState) {
        todo!()
    }

    /// renders only updated lines
    #[inline]
    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if !matches!(self.last_render_at_line, Some(idx) if idx == self.cursor.at_line) {
            return self.render(gs);
        }
        todo!()
    }

    #[inline]
    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> bool {
        match action {
            EditorAction::Char(ch) => {
                self.actions.push_char(ch, &mut self.cursor, &mut self.content);
                return true;
            }
            EditorAction::NewLine => self.actions.new_line(&mut self.cursor, &mut self.content),
            EditorAction::Indent => self.actions.indent(&mut self.cursor, &mut self.content),
            EditorAction::Backspace => self.actions.backspace(&mut self.cursor, &mut self.content),
            EditorAction::Delete => self.actions.del(&mut self.cursor, &mut self.content),
            EditorAction::RemoveLine => {
                self.select_line();
                if !self.cursor.select_is_none() {
                    self.actions.del(&mut self.cursor, &mut self.content);
                };
            }
            EditorAction::IndentStart => self.actions.indent_start(&mut self.cursor, &mut self.content),
            EditorAction::Unintent => self.actions.unindent(&mut self.cursor, &mut self.content),
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
            EditorAction::SwapUp => self.actions.swap_up(&mut self.cursor, &mut self.content),
            EditorAction::SwapDown => self.actions.swap_down(&mut self.cursor, &mut self.content),
            EditorAction::JumpLeft => self.cursor.jump_left(&self.content),
            EditorAction::JumpLeftSelect => self.cursor.jump_left_select(&self.content),
            EditorAction::JumpRight => self.cursor.jump_right(&self.content),
            EditorAction::JumpRightSelect => self.cursor.jump_right_select(&self.content),
            EditorAction::EndOfLine => self.cursor.end_of_line(&self.content),
            EditorAction::EndOfFile => self.cursor.end_of_file(&self.content),
            EditorAction::StartOfLine => self.cursor.start_of_line(&self.content),
            EditorAction::StartOfFile => self.cursor.start_of_file(),
            EditorAction::FindReferences => todo!(),
            EditorAction::GoToDeclaration => todo!(),
            EditorAction::Help => todo!(),
            EditorAction::LSPRename => {
                todo!()
            }
            EditorAction::CommentOut => self.actions.comment_out("//", &mut self.cursor, &mut self.content),
            EditorAction::Undo => self.actions.undo(&mut self.cursor, &mut self.content),
            EditorAction::Redo => self.actions.redo(&mut self.cursor, &mut self.content),
            EditorAction::Save => self.save(gs),
            EditorAction::Cancel => {
                if self.cursor.select_take().is_none() {
                    self.actions.push_buffer();
                    return false;
                }
            }
            EditorAction::Paste => {
                if let Some(clip) = gs.clipboard.pull() {
                    self.actions.paste(clip, &mut self.cursor, &mut self.content);
                }
            }
            EditorAction::Cut => {
                if let Some(clip) = self.cut() {
                    gs.clipboard.push(clip);
                }
            }
            EditorAction::Copy => {
                if let Some(clip) = self.copy() {
                    gs.clipboard.push(clip);
                }
            }
            EditorAction::Close => return false,
        }
        self.actions.push_buffer();
        true
    }

    #[inline(always)]
    pub fn clear_screen_cache(&mut self) {
        self.last_render_at_line = None;
    }

    #[inline]
    pub fn updated_rect(&mut self, rect: Rect, gs: &GlobalState) {
        let skip_offset = rect.row.saturating_sub(gs.editor_area.row) as usize;
        for line in self.content.iter_mut().skip(self.cursor.at_line + skip_offset).take(rect.width) {
            line.clear_cache();
        }
    }

    #[inline(always)]
    pub fn get_stats(&self) -> DocStats {
        (self.content.len(), self.cursor.select_len(&self.content), (&self.cursor).into())
    }

    #[inline(always)]
    pub fn update_path(&mut self, new_path: PathBuf) -> Result<(), LSPError> {
        self.display = build_display(&new_path);
        self.path = new_path;
        Ok(())
    }

    #[inline(always)]
    pub fn select_token(&mut self) {
        let range = token_range_at(&self.content[self.cursor.line], self.cursor.char);
        if !range.is_empty() {
            self.cursor.select_set(
                CursorPosition { line: self.cursor.line, char: range.start },
                CursorPosition { line: self.cursor.line, char: range.end },
            )
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn remove_line(&mut self) {
        self.select_line();
        if !self.cursor.select_is_none() {
            self.del();
        };
    }

    pub fn start_renames(&mut self) {
        todo!()
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

    #[inline(always)]
    pub fn insert_text_with_relative_offset(&mut self, insert: String) {
        self.actions.insert_top_cursor_relative_offset(insert, &mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn replace_select(&mut self, from: CursorPosition, to: CursorPosition, new_clip: &str) {
        self.actions.replace_select(from, to, new_clip, &mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn replace_token(&mut self, new: String) {
        self.actions.replace_token(new, &mut self.cursor, &mut self.content);
    }

    #[inline(always)]
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

    pub fn apply_file_edits(&mut self, _edits: Vec<TextEdit>) {
        panic!("Should not be called on plain text")
    }

    #[inline(always)]
    pub fn go_to(&mut self, line: usize) {
        self.cursor.select_drop();
        if self.content.len() >= line {
            self.cursor.line = line;
            self.cursor.char = find_line_start(&self.content[line]);
            self.cursor.at_line = line.saturating_sub(self.cursor.max_rows / 2);
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn cut(&mut self) -> Option<String> {
        if self.content.is_empty() {
            return None;
        }
        Some(self.actions.cut(&mut self.cursor, &mut self.content))
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

    #[inline(always)]
    pub fn paste(&mut self, clip: String) {
        self.actions.paste(clip, &mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn undo(&mut self) {
        self.actions.undo(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn end_of_line(&mut self) {
        self.cursor.end_of_line(&self.content);
    }

    #[inline(always)]
    pub fn end_of_file(&mut self) {
        self.cursor.end_of_file(&self.content);
    }

    #[inline(always)]
    pub fn start_of_file(&mut self) {
        self.cursor.start_of_file();
    }

    #[inline(always)]
    pub fn start_of_line(&mut self) {
        self.cursor.start_of_line(&self.content);
    }

    #[inline(always)]
    pub fn up(&mut self) {
        self.cursor.up(&self.content);
    }

    #[inline(always)]
    pub fn select_up(&mut self) {
        self.cursor.select_up(&self.content);
    }

    #[inline(always)]
    pub fn scroll_up(&mut self) {
        self.cursor.scroll_up(&self.content);
    }

    #[inline(always)]
    pub fn swap_up(&mut self) {
        self.actions.swap_up(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn down(&mut self) {
        self.cursor.down(&self.content);
    }

    #[inline(always)]
    pub fn select_down(&mut self) {
        self.cursor.select_down(&self.content);
    }

    #[inline(always)]
    pub fn scroll_down(&mut self) {
        self.cursor.scroll_down(&self.content);
    }

    #[inline(always)]
    pub fn swap_down(&mut self) {
        self.actions.swap_down(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn left(&mut self) {
        self.cursor.left(&self.content);
    }

    #[inline(always)]
    pub fn jump_left(&mut self) {
        self.cursor.jump_left(&self.content);
    }

    #[inline(always)]
    pub fn jump_left_select(&mut self) {
        self.cursor.jump_left_select(&self.content);
    }

    #[inline(always)]
    pub fn select_left(&mut self) {
        self.cursor.select_left(&self.content);
    }

    #[inline(always)]
    pub fn right(&mut self) {
        self.cursor.right(&self.content);
    }

    #[inline(always)]
    pub fn jump_right(&mut self) {
        self.cursor.jump_right(&self.content);
    }

    #[inline(always)]
    pub fn jump_right_select(&mut self) {
        self.cursor.jump_right_select(&self.content);
    }

    #[inline(always)]
    pub fn select_right(&mut self) {
        self.cursor.select_right(&self.content);
    }

    #[inline(always)]
    pub fn new_line(&mut self) {
        self.actions.new_line(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn push(&mut self, ch: char, _: &mut GlobalState) {
        self.actions.push_char(ch, &mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn backspace(&mut self) {
        self.actions.backspace(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn del(&mut self) {
        self.actions.del(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn indent(&mut self) {
        self.actions.indent(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn indent_start(&mut self) {
        self.actions.indent_start(&mut self.cursor, &mut self.content);
    }

    #[inline(always)]
    pub fn unindent(&mut self) {
        self.actions.unindent(&mut self.cursor, &mut self.content);
    }

    pub fn save(&mut self, gs: &mut GlobalState) {
        let content = self.content.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        match std::fs::write(&self.path, &content) {
            Ok(()) => gs.success(format!("SAVED {}", self.path.display())),
            Err(error) => gs.error(error.to_string()),
        }
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        self.actions.cfg = new_cfg.get_indent_cfg(&FileType::Ignored);
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

pub fn build_display(path: &Path) -> String {
    let mut buffer = Vec::new();
    let mut text_path = path.display().to_string();
    if let Ok(base_path) = PathBuf::from("./").canonicalize().map(|p| p.display().to_string()) {
        if text_path.starts_with(&base_path) {
            text_path.replace_range(..base_path.len(), "");
        }
    }
    for part in text_path.split(MAIN_SEPARATOR).rev().take(2) {
        buffer.insert(0, part);
    }
    buffer.join(MAIN_SEPARATOR_STR)
}

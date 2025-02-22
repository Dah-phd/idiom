mod buffer;
mod edits;
mod meta;

use super::{
    cursor::{Cursor, CursorPosition, Select},
    line::EditorLine,
    utils::{get_closing_char, is_closing_repeat},
};
use crate::{configs::IndentConfigs, syntax::Lexer, utils::Offset};
use buffer::ActionBuffer;
pub use edits::Edit;
use lsp_types::{TextDocumentContentChangeEvent, TextEdit};
pub use meta::{EditMetaData, EditType};

#[derive(Default)]
pub struct Actions {
    pub cfg: IndentConfigs,
    done: Vec<EditType>,
    undone: Vec<EditType>,
    buffer: ActionBuffer,
}

impl Actions {
    pub fn new(cfg: IndentConfigs) -> Self {
        Self { cfg, ..Default::default() }
    }

    pub fn swap_up(&mut self, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        if cursor.line == 0 {
            return;
        }
        cursor.select_drop();
        self.push_buffer(lexer);
        cursor.line -= 1;
        let (top, _, action) = Edit::swap_down(cursor.line, &self.cfg, content);
        cursor.char = top.offset(cursor.char);
        self.push_done(action, lexer, content);
    }

    pub fn swap_down(&mut self, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        if content.is_empty() || content.len() - 1 <= cursor.line {
            return;
        }
        cursor.select_drop();
        self.push_buffer(lexer);
        let (_, bot, action) = Edit::swap_down(cursor.line, &self.cfg, content);
        cursor.line += 1;
        cursor.char = bot.offset(cursor.char);
        self.push_done(action, lexer, content);
    }

    /// Insert new text at the top of the file preserving cursor/select relative position
    pub fn insert_top_cursor_relative_offset(
        &mut self,
        line: String,
        cursor: &mut Cursor,
        content: &mut Vec<EditorLine>,
        lexer: &mut Lexer,
    ) {
        self.push_buffer(lexer);
        let edit = Edit::replace_select(CursorPosition::default(), CursorPosition::default(), line, content);
        let offset = edit.meta.to - edit.meta.from;
        cursor.line += offset;
        cursor.at_line += offset;
        cursor.select_line_offset(offset);
        self.push_done(edit, lexer, content);
    }

    pub fn replace_token(&mut self, new: String, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        self.push_buffer(lexer);
        let action = Edit::replace_token(cursor.line, cursor.char, new, content);
        cursor.set_position(action.end_position());
        self.push_done(action, lexer, content);
    }

    pub fn replace_select(
        &mut self,
        from: CursorPosition,
        to: CursorPosition,
        clip: impl Into<String>,
        cursor: &mut Cursor,
        content: &mut Vec<EditorLine>,
        lexer: &mut Lexer,
    ) {
        self.push_buffer(lexer);
        cursor.select_drop();
        let action = Edit::replace_select(from, to, clip.into(), content);
        cursor.set_position(action.end_position());
        self.push_done(action, lexer, content);
    }

    pub fn insert_snippet(
        &mut self,
        c: &mut Cursor,
        snippet: String,
        cursor_offset: Option<(usize, usize)>,
        content: &mut Vec<EditorLine>,
        lexer: &mut Lexer,
    ) {
        self.push_buffer(lexer);
        let (position, action) = Edit::insert_snippet(c, snippet, cursor_offset, &self.cfg, content);
        c.set_position(position);
        self.push_done(action, lexer, content);
    }

    pub fn insert_snippet_with_select(
        &mut self,
        c: &mut Cursor,
        snippet: String,
        cursor_offset: (usize, usize),
        select_len: usize,
        content: &mut Vec<EditorLine>,
        lexer: &mut Lexer,
    ) {
        self.push_buffer(lexer);
        let (position, action) = Edit::insert_snippet(c, snippet, Some(cursor_offset), &self.cfg, content);
        let to = CursorPosition { line: position.line, char: position.char + select_len };
        c.select_set(position, to);
        self.push_done(action, lexer, content);
    }

    pub fn mass_replace(
        &mut self,
        cursor: &mut Cursor,
        ranges: Vec<Select>,
        clip: String,
        content: &mut Vec<EditorLine>,
        lexer: &mut Lexer,
    ) {
        self.push_buffer(lexer);
        cursor.select_drop();
        let actions = ranges
            .into_iter()
            .map(|(from, to)| Edit::replace_select(from, to, clip.to_owned(), content))
            .collect::<Vec<Edit>>();
        if let Some(last) = actions.last() {
            cursor.set_position(last.end_position());
        }
        self.push_done(actions, lexer, content);
    }

    pub fn apply_edits(&mut self, edits: Vec<TextEdit>, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        let actions = edits
            .into_iter()
            .map(|e| Edit::replace_select(e.range.start.into(), e.range.end.into(), e.new_text, content))
            .collect::<Vec<Edit>>();
        self.push_done(actions, lexer, content);
    }

    pub fn indent(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        match cursor.select_take() {
            Some((from, to)) => {
                if from.line == to.line {
                    self.push_done(Edit::replace_select(from, to, self.cfg.indent.to_owned(), content), lexer, content);
                } else {
                    let edits = self.indent_range(cursor, from, to, content);
                    self.push_done(edits, lexer, content);
                }
            }
            None => {
                self.push_done(Edit::insert_clip(cursor.into(), self.cfg.indent.to_owned(), content), lexer, content);
                cursor.add_to_char(self.cfg.indent.len());
            }
        }
    }

    pub fn indent_start(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        match cursor.select_take() {
            Some((from, to)) => {
                let edits = self.indent_range(cursor, from, to, content);
                self.push_done(edits, lexer, content);
            }
            None => {
                let start = CursorPosition { line: cursor.line, char: 0 };
                let edit = Edit::insert_clip(start, self.cfg.indent.to_owned(), content);
                cursor.add_to_char(self.cfg.indent.len());
                self.push_done(edit, lexer, content);
            }
        }
    }

    fn indent_range(
        &mut self,
        cursor: &mut Cursor,
        mut from: CursorPosition,
        mut to: CursorPosition,
        content: &mut [EditorLine],
    ) -> Vec<Edit> {
        let initial_select = (from, to);
        if from.char != 0 {
            from.char += self.cfg.indent.len();
        }
        let mut edit_lines = to.line - from.line;
        if to.char != 0 {
            to.char += self.cfg.indent.len();
            edit_lines += 1;
        };
        cursor.select_set(from, to);
        let mut edits = Vec::with_capacity(edit_lines);
        for (line_idx, text) in content.iter_mut().enumerate().skip(from.line).take(edit_lines) {
            text.insert_str(0, &self.cfg.indent);
            edits.push(Edit::record_in_line_insertion(
                CursorPosition { line: line_idx, char: 0 },
                self.cfg.indent.to_owned(),
            ))
        }
        add_select(&mut edits, Some(initial_select), Some((from, to)));
        edits
    }

    pub fn unindent(&mut self, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        self.push_buffer(lexer);
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                let initial_select = (from, to);
                let mut edit_lines = to.line - from.line;
                if to.char != 0 {
                    // include last line only if part of it is selected
                    edit_lines += 1;
                }
                let mut edits = Vec::new();
                for (line_idx, text) in content.iter_mut().enumerate().skip(from.line).take(edit_lines) {
                    if let Some((offset, edit)) = Edit::unindent(line_idx, text, &self.cfg.indent) {
                        if from.line == line_idx {
                            from.char = offset.offset(from.char);
                        }
                        if to.line == line_idx {
                            to.char = offset.offset(to.char);
                        }
                        edits.push(edit);
                    };
                }
                cursor.select_set(from, to);
                add_select(&mut edits, Some(initial_select), Some((from, to)));
                self.push_done(edits, lexer, content);
            }
            None => {
                let _ = content
                    .get_mut(cursor.line)
                    .and_then(|text| Edit::unindent(cursor.line, text, &self.cfg.indent))
                    .map(|(offset, edit)| {
                        self.push_done(edit, lexer, content);
                        cursor.char = offset.offset(cursor.char);
                    });
            }
        }
    }

    pub fn new_line(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        if content.is_empty() {
            cursor.set_position(CursorPosition { line: 0, char: 0 });
            content.push(Default::default());
            return;
        }
        self.push_buffer(lexer);
        match cursor.select_take() {
            Some((from, to)) => {
                let cut_edit = Edit::remove_select(from, to, content);
                let (new_position, new_line_edit) = Edit::new_line(from, &self.cfg, content);
                cursor.set_position(new_position);
                self.push_done(vec![cut_edit, new_line_edit], lexer, content)
            }
            None => {
                let (new_position, edit) = Edit::new_line(cursor.into(), &self.cfg, content);
                cursor.set_position(new_position);
                self.push_done(edit, lexer, content);
            }
        }
    }

    pub fn comment_out(&mut self, pat: &str, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        // TODO refactor
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                let from_char = from.char;
                let lines_n = to.line - from.line + 1;
                let cb = if select_is_commented(from.line, lines_n, pat, content) { uncomment } else { into_comment };
                let select = content.iter_mut().enumerate().skip(from.line).take(lines_n);
                let edits = select
                    .flat_map(|(line_idx, line)| {
                        (cb)(pat, line, CursorPosition { line: line_idx, char: cursor.char }).map(|(offset, edit)| {
                            if to.line == line_idx {
                                to.char = offset.offset(to.char);
                            }
                            if from.line == line_idx {
                                from.char = offset.offset(from.char);
                            }
                            edit
                        })
                    })
                    .collect::<Vec<Edit>>();
                if from.line == to.line {
                    if from_char == cursor.char {
                        cursor.select_set(to, from);
                    } else {
                        cursor.select_set(from, to);
                    }
                } else if from.line == cursor.line {
                    cursor.select_set(to, from);
                } else {
                    cursor.select_set(from, to);
                };
                self.push_done(edits, lexer, content);
            }
            _ => {
                let line = &mut content[cursor.line];
                if let Some((offset, edit)) = uncomment(pat, line, cursor.into()) {
                    self.push_done(edit, lexer, content);
                    cursor.char = offset.offset(cursor.char);
                } else if let Some((offset, edit)) = into_comment(pat, line, cursor.into()) {
                    self.push_done(edit, lexer, content);
                    cursor.char = offset.offset(cursor.char);
                }
            }
        }
    }

    pub fn push_char(&mut self, ch: char, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        match cursor.select_take() {
            Some((mut from, mut to)) => {
                self.push_buffer(lexer);
                match get_closing_char(ch) {
                    Some(closing) => {
                        content[to.line].insert(to.char, closing);
                        content[from.line].insert(from.char, ch);
                        let first_edit = Edit::record_in_line_insertion(to, closing.into()).select(from, to);
                        let second_edit = Edit::record_in_line_insertion(from, ch.into());
                        from.char += 1;
                        if from.line == to.line {
                            to.char += 1;
                        }
                        self.push_done(vec![first_edit, second_edit.new_select(from, to)], lexer, content);
                        cursor.set_position(to);
                        cursor.select_set(from, to);
                    }
                    None => {
                        cursor.set_position(from);
                        self.push_done(Edit::remove_select(from, to, content), lexer, content);
                        self.push_char_simple(ch, cursor, content, lexer);
                    }
                }
            }
            None => self.push_char_simple(ch, cursor, content, lexer),
        }
    }

    fn push_char_simple(&mut self, ch: char, cursor: &mut Cursor, content: &mut [EditorLine], lexer: &mut Lexer) {
        if let Some(line) = content.get_mut(cursor.line) {
            if is_closing_repeat(line, ch, cursor.char) {
            } else if let Some(closing) = get_closing_char(ch) {
                let new_text = format!("{ch}{closing}");
                line.insert_str(cursor.char, &new_text);
                self.push_buffer(lexer);
                self.push_done(Edit::record_in_line_insertion(cursor.into(), new_text), lexer, content);
            } else {
                line.insert(cursor.char, ch);
                if let Some(edit) = self.buffer.push(cursor.line, cursor.char, ch) {
                    lexer.sync_tokens(edit.meta);
                    self.done.push(edit.into());
                }

                let position = if cursor.char != 0 && !line.is_simple() {
                    lsp_types::Position::new(cursor.line as u32, (lexer.encode_position)(cursor.char, &line[..]) as u32)
                } else {
                    cursor.into()
                };

                lexer.sync_changes(vec![TextDocumentContentChangeEvent {
                    text: String::from(ch),
                    range: Some(lsp_types::Range::new(position, position)),
                    range_length: None,
                }]);
            }
            cursor.add_to_char(1);
        }
    }

    pub fn del(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        if content.is_empty() {
            return;
        }
        match cursor.select_take() {
            Some((from, to)) => {
                self.push_buffer(lexer);
                cursor.set_position(from);
                self.push_done(Edit::remove_select(from, to, content), lexer, content);
            }
            None if content[cursor.line].char_len() == cursor.char => {
                self.push_buffer(lexer);
                if content.len() > cursor.line + 1 {
                    self.push_done(Edit::merge_next_line(cursor.line, content), lexer, content);
                }
            }
            None => {
                let line = &mut content[cursor.line];
                let removed_char = line.remove(cursor.char);
                if let Some(edit) = self.buffer.del(cursor.line, cursor.char, removed_char) {
                    lexer.sync_tokens(edit.meta);
                    self.done.push(edit.into());
                }

                let (line_idx, char_idx) = if cursor.char != 0 && !line.is_simple() {
                    (cursor.line, (lexer.encode_position)(cursor.char, &line[..]))
                } else {
                    (cursor.line, cursor.char)
                };

                let start = lsp_types::Position::new(line_idx as u32, char_idx as u32);
                let end_char = char_idx + (lexer.char_lsp_pos)(removed_char);
                let end = lsp_types::Position::new(line_idx as u32, end_char as u32);
                lexer.sync_changes(vec![TextDocumentContentChangeEvent {
                    text: String::new(),
                    range: Some(lsp_types::Range { start, end }),
                    range_length: None,
                }]);
            }
        }
    }

    pub fn backspace(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        if content.is_empty() || cursor.line == 0 && cursor.char == 0 && cursor.select_is_none() {
            return;
        }
        match cursor.select_take() {
            Some((from, to)) => {
                self.push_buffer(lexer);
                cursor.set_position(from);
                self.push_done(Edit::remove_select(from, to, content), lexer, content);
            }
            None if cursor.char == 0 => {
                self.push_buffer(lexer);
                cursor.line -= 1;
                let edit = Edit::merge_next_line(cursor.line, content);
                cursor.set_char(edit.cursor.char);
                self.push_done(edit, lexer, content);
            }
            None => {
                if let Some(edit) =
                    self.buffer.backspace(cursor.line, cursor.char, &mut content[cursor.line], &self.cfg.indent)
                {
                    lexer.sync_tokens(edit.meta);
                    self.done.push(edit.into());
                }
                let new_char = self.buffer.last_char();
                let tmp_reverse = format!("{:width$}", "", width = cursor.char - new_char);
                cursor.set_char(new_char);
                let edit = Edit::single_line(cursor.into(), String::new(), tmp_reverse);
                lexer.sync_changes(vec![edit.text_change(lexer.encode_position, lexer.char_lsp_pos, content).1]);
            }
        }
    }

    pub fn undo(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        if let Some(action) = self.done.pop() {
            let (position, select) = action.apply_rev(content);
            lexer.sync_rev(&action, content);
            cursor.set_position(position);
            cursor.select_replace(select);
            self.undone.push(action);
        }
    }

    pub fn redo(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        if let Some(action) = self.undone.pop() {
            let (position, select) = action.apply(content);
            lexer.sync(&action, content);
            cursor.set_position(position);
            cursor.select_replace(select);
            self.done.push(action);
        }
    }

    pub fn paste(&mut self, clip: String, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) {
        self.push_buffer(lexer);
        let edit = match cursor.select_take() {
            Some((from, to)) => Edit::replace_select(from, to, clip, content),
            None => Edit::insert_clip(cursor.into(), clip, content),
        };
        cursor.set_position(edit.end_position());
        self.push_done(edit, lexer, content);
    }

    pub fn cut(&mut self, cursor: &mut Cursor, content: &mut Vec<EditorLine>, lexer: &mut Lexer) -> String {
        self.push_buffer(lexer);
        let edit = if let Some((from, to)) = cursor.select_take() {
            cursor.set_position(from);
            Edit::remove_select(from, to, content)
        } else {
            let action = Edit::remove_line(cursor.line, content);
            if content.is_empty() {
                content.push(Default::default());
                cursor.line = 0;
            } else if cursor.line >= content.len() && content.len() > 1 {
                cursor.line -= 1;
            }
            cursor.char = 0;
            action
        };
        let clip = edit.get_removed_text().to_owned();
        self.push_done(edit, lexer, content);
        clip
    }

    fn push_done(&mut self, edit: impl Into<EditType>, lexer: &mut Lexer, content: &mut [EditorLine]) {
        let action: EditType = edit.into();
        lexer.sync(&action, content);
        self.done.push(action);
    }

    pub fn push_buffer(&mut self, lexer: &mut Lexer) {
        if let Some(action) = self.buffer.collect() {
            lexer.sync_tokens(action.meta);
            self.undone.clear();
            self.done.push(action.into());
            // self.push_done(action, lexer, content);
        }
    }

    pub fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
        let _ = self.buffer.collect();
    }
}

#[inline]
fn add_select(edits: &mut [Edit], old: Option<Select>, new: Option<Select>) {
    if let Some(edit) = edits.first_mut() {
        edit.select = old;
    }
    if let Some(edit) = edits.last_mut() {
        edit.select = new;
    }
}

#[inline]
fn select_is_commented(from: usize, n: usize, pat: &str, content: &[EditorLine]) -> bool {
    content.iter().skip(from).take(n).all(|l| l.trim_start().starts_with(pat) || l.chars().all(|c| c.is_whitespace()))
}

#[inline]
fn into_comment(pat: &str, line: &mut EditorLine, cursor: CursorPosition) -> Option<(Offset, Edit)> {
    let idx = line.char_indices().flat_map(|(idx, c)| if c.is_whitespace() { None } else { Some(idx) }).next()?;
    let comment_start = format!("{pat} ");
    line.insert_str(idx, &comment_start);
    let offset = if cursor.char >= idx { Offset::Pos(comment_start.len()) } else { Offset::Pos(0) };
    Some((offset, Edit::record_in_line_insertion(CursorPosition { line: cursor.line, char: idx }, comment_start)))
}

#[inline]
fn uncomment(pat: &str, line: &mut EditorLine, cursor: CursorPosition) -> Option<(Offset, Edit)> {
    if !line.trim_start().starts_with(pat) {
        return None;
    }
    let idx = line.find(pat)?;
    let mut end_idx = idx + pat.len();
    end_idx += line[idx + pat.len()..].chars().take_while(|c| c.is_whitespace()).count();
    let offset = if cursor.char >= idx { Offset::Neg(end_idx - idx) } else { Offset::Neg(0) };
    Some((offset, Edit::remove_from_line(cursor.line, idx, end_idx, line)))
}

#[cfg(test)]
pub mod tests;

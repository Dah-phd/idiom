use std::{
    fmt::Debug,
    ops::{Add, AddAssign},
};

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

use crate::{
    configs::IndentConfigs,
    render::UTF8Safe,
    utils::Offset,
    workspace::{
        cursor::Cursor,
        line::EditorLine,
        utils::{clip_content, insert_clip, is_scope, remove_content, token_range_at},
        CursorPosition,
    },
};

#[derive(Debug)]
pub struct Edit {
    pub meta: EditMetaData,
    pub cursor: CursorPosition,
    pub reverse: String,
    pub text: String,
    pub select: Option<(CursorPosition, CursorPosition)>,
    pub new_select: Option<(CursorPosition, CursorPosition)>,
}

impl Edit {
    #[inline(always)]
    const fn without_select(cursor: CursorPosition, from: usize, to: usize, text: String, reverse: String) -> Self {
        let meta = EditMetaData { start_line: cursor.line, from, to };
        Self { meta, cursor, reverse, text, select: None, new_select: None }
    }

    #[inline(always)]
    pub const fn single_line(cursor: CursorPosition, text: String, reverse: String) -> Self {
        Self { meta: EditMetaData::line_changed(cursor.line), cursor, reverse, text, select: None, new_select: None }
    }

    pub fn swap_down(up_line: usize, cfg: &IndentConfigs, content: &mut [EditorLine]) -> (Offset, Offset, Self) {
        let to = up_line + 1;
        let reverse = format!("{}\n{}\n", content[up_line], content[to]);
        content.swap(up_line, to);
        let up_offset = cfg.indent_line(up_line, content);
        let down_offset = cfg.indent_line(to, content);
        let text = format!("{}\n{}\n", content[up_line], content[to]);
        let cursor = CursorPosition { line: up_line, char: 0 };
        (up_offset, down_offset, Self::without_select(cursor, 2, 2, text, reverse))
    }

    pub fn merge_next_line(line: usize, content: &mut Vec<EditorLine>) -> Self {
        let removed_line = content.remove(line + 1);
        let merged_to = &mut content[line];
        let cursor = CursorPosition { line, char: merged_to.char_len() };
        merged_to.push_line(removed_line);
        Self::without_select(cursor, 2, 1, String::new(), "\n".to_owned())
    }

    pub fn unindent(line: usize, text: &mut EditorLine, indent: &str) -> Option<(Offset, Self)> {
        let mut idx = 0;
        while text[idx..].starts_with(indent) {
            idx += indent.len();
        }
        if text[idx..].starts_with(' ') {
            let mut reverse = String::new();
            while text[idx..].starts_with(' ') {
                reverse.push(text.remove(idx));
            }
            return Some((
                Offset::Neg(reverse.len()),
                Self::single_line(CursorPosition { line, char: idx }, String::new(), reverse),
            ));
        };
        if idx != 0 {
            text.replace_range(0..indent.len(), "");
            return Some((
                Offset::Neg(indent.len()),
                Self::single_line(CursorPosition { line, char: 0 }, String::new(), indent.to_owned()),
            ));
        }
        None
    }

    /// Creates Edit record without performing the action
    /// does not support multi line insertion
    #[inline]
    pub fn record_in_line_insertion(position: CursorPosition, new_text: String) -> Self {
        Self::single_line(position, new_text, String::new())
    }

    #[inline]
    pub fn remove_from_line(line: usize, from: usize, to: usize, text: &mut EditorLine) -> Self {
        let reverse = text[from..to].to_owned();
        text.replace_range(from..to, "");
        Self::single_line(CursorPosition { line, char: from }, String::new(), reverse)
    }

    /// builds action from removed data
    #[inline]
    pub fn extract_from_start(line: usize, len: usize, text: &mut String) -> Self {
        let mut reverse = text.split_off(len);
        std::mem::swap(text, &mut reverse);
        Self::single_line(CursorPosition { line, char: 0 }, String::new(), reverse)
    }

    #[inline]
    pub fn insert_clip(cursor: CursorPosition, clip: String, content: &mut Vec<EditorLine>) -> Self {
        let end = insert_clip(&clip, content, cursor);
        let to = (end.line - cursor.line) + 1;
        Self::without_select(cursor, 1, to, clip, String::new())
    }

    #[inline]
    pub fn remove_line(line: usize, content: &mut Vec<EditorLine>) -> Self {
        let mut reverse = content.remove(line).unwrap();
        reverse.push('\n');
        Self::without_select(CursorPosition { line, char: 0 }, 2, 1, String::new(), reverse)
    }

    #[inline]
    pub fn remove_select(from: CursorPosition, to: CursorPosition, content: &mut Vec<EditorLine>) -> Self {
        Self {
            cursor: from,
            meta: EditMetaData { start_line: from.line, from: to.line - from.line + 1, to: 1 },
            reverse: clip_content(from, to, content),
            select: Some((from, to)),
            text: String::new(),
            new_select: None,
        }
    }

    #[inline]
    pub fn replace_select(
        from: CursorPosition,
        to: CursorPosition,
        clip: String,
        content: &mut Vec<EditorLine>,
    ) -> Self {
        let reverse_text_edit = clip_content(from, to, content);
        let end = if !clip.is_empty() { insert_clip(&clip, content, from) } else { from };
        let meta =
            EditMetaData { start_line: from.line, from: (to.line - from.line) + 1, to: (end.line - from.line) + 1 };
        Self { cursor: from, meta, reverse: reverse_text_edit, text: clip, select: Some((from, to)), new_select: None }
    }

    #[inline]
    pub fn replace_token(line: usize, char: usize, new_text: String, content: &mut [EditorLine]) -> Self {
        let code_line = &mut content[line];
        let range = token_range_at(code_line, char);
        let char = range.start;
        let reverse = code_line[range.clone()].to_owned();
        code_line.replace_range(range, &new_text);
        Self::single_line(CursorPosition { line, char }, new_text, reverse)
    }

    #[inline]
    pub fn new_line(
        mut cursor: CursorPosition,
        cfg: &IndentConfigs,
        content: &mut Vec<EditorLine>,
    ) -> (CursorPosition, Self) {
        let mut from_cursor = cursor;
        let mut reverse = String::new();
        let mut text = String::from('\n');
        let prev_line = &mut content[cursor.line];
        let mut line = prev_line.split_off(cursor.char);
        let indent = cfg.derive_indent_from(prev_line);
        line.insert_str(0, &indent);
        cursor.line += 1;
        cursor.char = indent.len();
        text.push_str(&indent);
        // expand scope
        if is_scope(&prev_line[..], &line[..]) {
            text.push('\n');
            if indent.len() >= cfg.indent.len() && cfg.unindent_if_before_base_pattern(&mut line) != 0 {
                text.push_str(&indent[..indent.len() - cfg.indent.len()]);
            }
            content.insert(cursor.line, line);
            content.insert(cursor.line, indent.into());
            return (cursor, Self::without_select(from_cursor, 1, 3, text, reverse));
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.char_len().rem_euclid(cfg.indent.len()) == 0 {
            from_cursor.char = 0;
            prev_line.push_content_to_buffer(&mut reverse);
            prev_line.clear();
        }
        content.insert(cursor.line, line);
        (cursor, Self::without_select(from_cursor, 1, 2, text, reverse))
    }

    #[inline]
    pub fn insert_snippet(
        c: &Cursor,
        snippet: String,
        cursor_offset: Option<(usize, usize)>,
        cfg: &IndentConfigs,
        content: &mut Vec<EditorLine>,
    ) -> (CursorPosition, Self) {
        let code_line = &mut content[c.line];
        let range = token_range_at(code_line, c.char);
        let from = CursorPosition { line: c.line, char: range.start };
        let to = CursorPosition { line: c.line, char: range.end };
        let indent = cfg.derive_indent_from(code_line);
        let snippet = snippet.replace('\n', &format!("\n{}", &indent));
        let new_cursor = cursor_offset.map(|(line, char)| CursorPosition {
            line: line + c.line,
            char: if line == 0 { from.char + char } else { indent.len() + char },
        });
        let edit = Edit::replace_select(from, to, snippet, content);
        (new_cursor.unwrap_or(edit.end_position()), edit)
    }

    /// UTILS

    #[inline]
    pub fn get_new_text(&self) -> &str {
        &self.text
    }

    #[inline]
    pub fn get_removed_text(&self) -> &str {
        &self.reverse
    }

    pub fn select(mut self, from: CursorPosition, to: CursorPosition) -> Self {
        self.select = Some((from, to));
        self
    }

    pub fn new_select(mut self, from: CursorPosition, to: CursorPosition) -> Self {
        self.new_select = Some((from, to));
        self
    }

    #[inline]
    pub fn start_position(&self) -> CursorPosition {
        self.cursor
    }

    #[inline]
    pub fn end_position(&self) -> CursorPosition {
        let line = self.meta.end_line();
        let mut char = self.text.split('\n').last().unwrap().char_len();
        if line == self.meta.start_line {
            char += self.cursor.char;
        };
        CursorPosition { line, char }
    }

    #[inline]
    pub fn end_position_rev(&self) -> CursorPosition {
        let line = self.meta.start_line + self.meta.from - 1;
        let mut char = self.reverse.split('\n').last().unwrap().char_len();
        if line == self.meta.start_line {
            char += self.cursor.char;
        };
        CursorPosition { line, char }
    }

    /// apply reverse edit (goes into undone)
    pub fn apply_rev(
        &self,
        content: &mut Vec<EditorLine>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position();
        remove_content(from, to, content);
        (insert_clip(&self.reverse, content, from), self.select)
    }

    /// apply edit (goes into done)
    pub fn apply(&self, content: &mut Vec<EditorLine>) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position_rev();
        remove_content(from, to, content);
        (insert_clip(&self.text, content, from), self.new_select)
    }

    #[inline(always)]
    pub fn text_change(
        &self,
        encoding: fn(usize, &str) -> usize,
        char_lsp: fn(char) -> usize,
        content: &[EditorLine],
    ) -> (EditMetaData, TextDocumentContentChangeEvent) {
        let mut cursor = self.cursor;
        let changed = self.meta.from - 1;
        let text = self.text.to_owned();
        let mut char = self.reverse.chars().rev().take_while(|ch| ch != &'\n').map(char_lsp).sum::<usize>();

        if cursor.char != 0 {
            let editor_line = &content[cursor.line];
            if !editor_line.is_simple() {
                cursor.char = (encoding)(cursor.char, &editor_line[..]);
            }
        }

        if changed == 0 {
            char += cursor.char;
        }
        let end = Position::new((cursor.line + changed) as u32, char as u32);
        let start = Position::from(cursor);
        (self.meta, TextDocumentContentChangeEvent { range: Some(Range::new(start, end)), text, range_length: None })
    }

    #[inline(always)]
    pub fn text_change_rev(
        &self,
        encoding: fn(usize, &str) -> usize,
        char_lsp: fn(char) -> usize,
        content: &[EditorLine],
    ) -> (EditMetaData, TextDocumentContentChangeEvent) {
        let rev_meta = self.meta.rev();
        let mut cursor = self.cursor;
        let changed = rev_meta.from - 1;
        let text = self.reverse.to_owned();
        let mut char = self.text.chars().rev().take_while(|ch| ch != &'\n').map(char_lsp).sum::<usize>();

        if cursor.char != 0 {
            let editor_line = &content[cursor.line];
            if !editor_line.is_simple() {
                cursor.char = (encoding)(cursor.char, &editor_line[..]);
            }
        }

        if changed == 0 {
            char += cursor.char;
        }
        let end = Position::new((cursor.line + changed) as u32, char as u32);
        let start = Position::from(cursor);
        (rev_meta, TextDocumentContentChangeEvent { range: Some(Range::new(start, end)), text, range_length: None })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct EditMetaData {
    pub start_line: usize,
    pub from: usize, // ignored after Add - is set to 0;
    pub to: usize,
}

impl Add for EditMetaData {
    type Output = Self;

    fn add(mut self, other: Self) -> Self::Output {
        let self_end_line = self.start_line + self.to;
        let other_end_line = other.start_line + other.to;
        self.start_line = std::cmp::min(self.start_line, other.start_line);
        if self_end_line > other_end_line {
            self.to = self_end_line - self.start_line;
            if other.from > other.to {
                self.to -= other.from - other.to;
            } else {
                self.to += other.to - other.from;
            }
        } else {
            // previous offset does not matter because we need the info for the last changed line
            self.to = other_end_line - self.start_line;
        };
        self.from = 0;
        self
    }
}

impl AddAssign for EditMetaData {
    fn add_assign(&mut self, other: Self) {
        let self_end_line = self.end_line();
        let other_end_line = other.end_line();
        self.start_line = std::cmp::min(self.start_line, other.start_line);
        if self_end_line > other_end_line {
            self.to = self_end_line - self.start_line;
            if other.from > other.to {
                self.to -= other.from - other.to;
            } else {
                self.to += other.to - other.from;
            }
        } else {
            // previous offset does not matter because we need the info for the last changed line
            self.to = other_end_line - self.start_line;
        };
        self.from = 0;
    }
}

impl EditMetaData {
    #[inline]
    pub const fn line_changed(start_line: usize) -> Self {
        Self { start_line, from: 1, to: 1 }
    }

    #[inline]
    pub const fn end_line(&self) -> usize {
        self.start_line + self.to - 1
    }

    #[inline]
    pub const fn rev(&self) -> Self {
        EditMetaData { start_line: self.start_line, from: self.to, to: self.from }
    }
}

impl Debug for EditMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} >> {}", self.from, self.to))
    }
}

impl From<EditMetaData> for lsp_types::Range {
    fn from(meta: EditMetaData) -> Self {
        let start = lsp_types::Position::new(meta.start_line as u32, 0);
        let end = lsp_types::Position::new((meta.start_line + meta.to) as u32, 0);
        lsp_types::Range::new(start, end)
    }
}

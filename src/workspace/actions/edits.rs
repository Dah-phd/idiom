use crate::{
    configs::IndentConfigs,
    syntax::Lexer,
    utils::Offset,
    workspace::{
        cursor::{Cursor, CursorPosition},
        line::EditorLine,
        utils::{clip_content, insert_clip, is_scope, remove_content, token_range_at},
    },
};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};
use std::{
    fmt::Debug,
    ops::{Add, AddAssign},
};

#[derive(Debug)]
pub struct Edit {
    pub meta: EditMetaData,
    pub reverse_text_edit: TextEdit,
    pub text_edit: TextEdit,
    pub select: Option<(CursorPosition, CursorPosition)>,
    pub new_select: Option<(CursorPosition, CursorPosition)>,
}

impl Edit {
    pub fn swap_down(up_line: usize, cfg: &IndentConfigs, content: &mut [impl EditorLine]) -> (Offset, Offset, Self) {
        let to = up_line + 1;
        let reverse_edit_text = format!("{}\n{}\n", content[up_line], content[to]);
        content.swap(up_line, to);
        let offset = cfg.indent_line(up_line, content);
        let offset2 = cfg.indent_line(to, content);
        let new_text = format!("{}\n{}\n", content[up_line], content[to]);
        let range = Range::new(Position::new(up_line as u32, 0), Position::new((to + 1) as u32, 0));
        (
            offset,
            offset2,
            Self {
                meta: EditMetaData { start_line: up_line, from: 2, to: 2 },
                reverse_text_edit: TextEdit::new(range, reverse_edit_text),
                text_edit: TextEdit::new(range, new_text),
                select: None,
                new_select: None,
            },
        )
    }

    pub fn merge_next_line(line: usize, content: &mut Vec<impl EditorLine>) -> Self {
        let removed_line = content.remove(line + 1);
        let merged_to = &mut content[line];
        let position_of_new_line = Position::new(line as u32, merged_to.char_len() as u32);
        merged_to.push_line(removed_line);
        Self {
            meta: EditMetaData { start_line: line, from: 2, to: 1 },
            reverse_text_edit: TextEdit::new(
                Range::new(position_of_new_line, position_of_new_line),
                String::from("\n"),
            ),
            text_edit: TextEdit::new(
                Range::new(position_of_new_line, Position::new((line + 1) as u32, 0)),
                String::new(),
            ),
            select: None,
            new_select: None,
        }
    }

    pub fn unindent(line: usize, text: &mut impl EditorLine, indent: &str) -> Option<(Offset, Self)> {
        let mut idx = 0;
        while text[idx..].starts_with(indent) {
            idx += indent.len();
        }
        if text[idx..].starts_with(' ') {
            let mut removed = String::new();
            while text[idx..].starts_with(' ') {
                removed.push(text.remove(idx));
            }
            let start = Position::new(line as u32, idx as u32);
            let end = Position::new(line as u32, (idx + removed.len()) as u32);
            return Some((
                Offset::Neg(removed.len()),
                Self {
                    meta: EditMetaData::line_changed(line),
                    reverse_text_edit: TextEdit::new(Range::new(start, start), removed),
                    text_edit: TextEdit::new(Range::new(start, end), String::new()),
                    select: None,
                    new_select: None,
                },
            ));
        };
        if idx != 0 {
            text.replace_range(0..indent.len(), "");
            let start = Position::new(line as u32, 0);
            let end = Position::new(line as u32, indent.len() as u32);
            return Some((
                Offset::Neg(indent.len()),
                Self {
                    meta: EditMetaData::line_changed(line),
                    reverse_text_edit: TextEdit::new(Range::new(start, start), indent.to_owned()),
                    text_edit: TextEdit::new(Range::new(start, end), String::new()),
                    select: None,
                    new_select: None,
                },
            ));
        }
        None
    }

    /// Creates Edit record without performing the action
    /// does not support multi line insertion
    pub fn record_in_line_insertion(position: Position, new_text: String) -> Self {
        Self {
            meta: EditMetaData::line_changed(position.line as usize),
            reverse_text_edit: TextEdit::new(
                Range::new(position, Position::new(position.line, position.character + new_text.len() as u32)),
                String::new(),
            ),
            text_edit: TextEdit::new(Range::new(position, position), new_text),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_from_line(at_line: usize, from: usize, to: usize, line: &mut impl EditorLine) -> Self {
        let start = Position::new(at_line as u32, from as u32);
        let old = line[from..to].to_owned();
        line.replace_range(from..to, "");
        Self {
            meta: EditMetaData::line_changed(at_line),
            reverse_text_edit: TextEdit::new(Range::new(start, start), old),
            text_edit: TextEdit::new(Range::new(start, Position::new(at_line as u32, to as u32)), String::new()),
            select: None,
            new_select: None,
        }
    }

    pub fn insert_clip(from: CursorPosition, clip: String, content: &mut Vec<impl EditorLine>) -> Self {
        let end = insert_clip(&clip, content, from);
        Self {
            meta: EditMetaData { start_line: from.line, from: 1, to: (end.line - from.line) + 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), end.into()), String::new()),
            text_edit: TextEdit::new(Range::new(from.into(), from.into()), clip),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_line(line: usize, content: &mut Vec<impl EditorLine>) -> Self {
        let mut removed_line = content.remove(line);
        removed_line.push('\n');
        let start = Position::new(line as u32, 0);
        Self {
            meta: EditMetaData { start_line: line, from: 2, to: 1 },
            reverse_text_edit: TextEdit::new(Range::new(start, start), removed_line.unwrap()),
            text_edit: TextEdit::new(Range::new(start, Position::new(line as u32 + 1, 0)), String::new()),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_select(from: CursorPosition, to: CursorPosition, content: &mut Vec<impl EditorLine>) -> Self {
        Self {
            meta: EditMetaData { start_line: from.line, from: to.line - from.line + 1, to: 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), from.into()), clip_content(from, to, content)),
            text_edit: TextEdit::new(Range::new(from.into(), to.into()), String::new()),
            select: Some((from, to)),
            new_select: None,
        }
    }

    pub fn new_line(
        mut cursor: CursorPosition,
        cfg: &IndentConfigs,
        content: &mut Vec<impl EditorLine>,
    ) -> (CursorPosition, Self) {
        let mut from_range = (cursor, cursor);
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
            let new_char = if indent.len() >= cfg.indent.len() && cfg.unindent_if_before_base_pattern(&mut line) != 0 {
                let new_char = indent.len() - cfg.indent.len();
                text.push_str(&indent[..new_char]);
                new_char
            } else {
                0
            };
            content.insert(cursor.line, line);
            content.insert(cursor.line, indent.into());
            let edit = Self {
                meta: EditMetaData { start_line: from_range.0.line, from: 1, to: 3 },
                reverse_text_edit: TextEdit {
                    new_text: reverse,
                    range: Range::new(
                        from_range.0.into(),
                        CursorPosition { line: cursor.line + 1, char: new_char }.into(),
                    ),
                },
                text_edit: TextEdit { new_text: text, range: Range::new(from_range.0.into(), from_range.1.into()) },
                select: None,
                new_select: None,
            };
            return (cursor, edit);
        }
        if prev_line.chars().all(|c| c.is_whitespace()) && prev_line.char_len().rem_euclid(cfg.indent.len()) == 0 {
            from_range.0.char = 0;
            prev_line.push_content_to_buffer(&mut reverse);
            prev_line.clear();
        }
        content.insert(cursor.line, line);
        let edit = Self {
            meta: EditMetaData { start_line: from_range.0.line, from: 1, to: 2 },
            text_edit: TextEdit { range: Range::new(from_range.0.into(), from_range.1.into()), new_text: text },
            reverse_text_edit: TextEdit { range: Range::new(from_range.0.into(), cursor.into()), new_text: reverse },
            select: None,
            new_select: None,
        };
        return (cursor, edit);
    }

    pub fn replace_select(
        from: CursorPosition,
        to: CursorPosition,
        clip: String,
        content: &mut Vec<impl EditorLine>,
    ) -> Self {
        let reverse_edit_text = clip_content(from, to, content);
        let end = if !clip.is_empty() { insert_clip(&clip, content, from) } else { from };
        Self {
            meta: EditMetaData { start_line: from.line, from: to.line - from.line + 1, to: (end.line - from.line) + 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), end.into()), reverse_edit_text),
            text_edit: TextEdit { range: Range::new(from.into(), to.into()), new_text: clip },
            select: Some((from, to)),
            new_select: None,
        }
    }

    pub fn replace_token(line: usize, char: usize, new_text: String, content: &mut [impl EditorLine]) -> Self {
        let code_line = &mut content[line];
        let range = token_range_at(code_line, char);
        let start = Position::new(line as u32, range.start as u32);
        let text_edit_range = Range::new(start, Position::new(line as u32, range.end as u32));
        let reverse_edit_range = Range::new(start, Position::new(line as u32, (range.start + new_text.len()) as u32));
        let replaced_text = code_line[range.clone()].to_owned();
        code_line.replace_range(range, &new_text);
        Self {
            meta: EditMetaData::line_changed(line),
            text_edit: TextEdit::new(text_edit_range, new_text),
            reverse_text_edit: TextEdit::new(reverse_edit_range, replaced_text),
            select: None,
            new_select: None,
        }
    }

    pub fn insert_snippet(
        c: &Cursor,
        snippet: String,
        cursor_offset: Option<(usize, usize)>,
        cfg: &IndentConfigs,
        content: &mut Vec<impl EditorLine>,
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
        (new_cursor.unwrap_or(edit.reverse_text_edit.range.end.into()), edit)
    }

    /// UTILS

    pub fn get_new_text(&self) -> &str {
        &self.text_edit.new_text
    }

    pub fn get_removed_text(&self) -> &str {
        &self.reverse_text_edit.new_text
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
        self.reverse_text_edit.range.start.into()
    }

    #[inline]
    pub fn end_position(&self) -> CursorPosition {
        self.reverse_text_edit.range.end.into()
    }

    #[inline]
    pub fn end_position_rev(&self) -> CursorPosition {
        self.text_edit.range.end.into()
    }

    /// apply reverse edit (goes into undone)
    pub fn apply_rev(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position();
        remove_content(from, to, content);
        events.push((self.meta.rev(), self.reverse_event()));
        (insert_clip(&self.reverse_text_edit.new_text, content, from), self.select)
    }

    /// apply edit (goes into done)
    pub fn apply(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position_rev();
        remove_content(from, to, content);
        events.push((self.meta, self.event()));
        (insert_clip(&self.text_edit.new_text, content, from), self.new_select)
    }

    pub fn reverse_event(&self) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent {
            range: Some(self.reverse_text_edit.range),
            range_length: None,
            text: self.reverse_text_edit.new_text.to_owned(),
        }
    }

    pub fn event(&self) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent {
            range: Some(self.text_edit.range),
            range_length: None,
            text: self.text_edit.new_text.to_owned(),
        }
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
    }
}

impl EditMetaData {
    #[inline]
    pub const fn line_changed(start_line: usize) -> Self {
        Self { start_line, from: 1, to: 1 }
    }

    #[inline]
    pub fn update_tokens(&self, content: &mut Vec<impl EditorLine>, lexer: &Lexer) {
        for line in content.iter_mut().skip(self.start_line).take(self.to) {
            line.rebuild_tokens(lexer);
        }
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

use crate::{
    configs::IndentConfigs,
    utils::Offset,
    workspace::{
        cursor::{Cursor, CursorPosition},
        line::EditorLine,
        utils::{clip_content, copy_content, insert_clip, remove_content, token_range_at},
    },
};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};
use std::fmt::Debug;

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
        let mut reverse_edit_text = content[up_line].as_str().to_owned();
        reverse_edit_text.push('\n');
        reverse_edit_text.push_str(&content[up_line + 1].as_str());
        reverse_edit_text.push('\n');
        let text_edit_range: (CursorPosition, CursorPosition) = ((up_line, 0).into(), (up_line + 2, 0).into());
        content.swap(up_line, to);
        let offset = cfg.indent_line(up_line, content);
        let offset2 = cfg.indent_line(to, content);
        let mut new_text = content[text_edit_range.0.line].as_str().to_owned();
        new_text.push('\n');
        new_text.push_str(&content[text_edit_range.0.line + 1].as_str());
        new_text.push('\n');
        let range = Range::new(Position::new(up_line as u32, 0), Position::new((up_line + 2) as u32, 0));
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
        let position_of_new_line = Position::new(line as u32, merged_to.len() as u32);
        merged_to.push_str(removed_line.as_str());
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

    /// builds action from removed data
    pub fn extract_from_start(line: usize, len: usize, text: &mut String) -> Self {
        let position = Position::new(line as u32, 0);
        let mut old_text = text.split_off(len);
        std::mem::swap(text, &mut old_text);
        Self {
            meta: EditMetaData::line_changed(line),
            text_edit: TextEdit::new(Range::new(position, Position::new(line as u32, len as u32)), String::new()),
            reverse_text_edit: TextEdit::new(Range::new(position, position), old_text),
            select: None,
            new_select: None,
        }
    }

    pub fn insert_clip(from: CursorPosition, clip: String, content: &mut Vec<impl EditorLine>) -> Self {
        let end = insert_clip(clip.to_owned(), content, from);
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

    pub fn replace_select(
        from: CursorPosition,
        to: CursorPosition,
        clip: String,
        content: &mut Vec<impl EditorLine>,
    ) -> Self {
        let reverse_edit_text = clip_content(from, to, content);
        let end = if !clip.is_empty() { insert_clip(clip.clone(), content, from) } else { from };
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
        let range = token_range_at(code_line.as_str(), char);
        let start = Position::new(line as u32, range.start as u32);
        let text_edit_range = Range::new(start, Position::new(line as u32, range.end as u32));
        let reverse_edit_range = Range::new(start, Position::new(line as u32, (range.start + new_text.len()) as u32));
        let replaced_text = code_line[range.clone()].to_owned();
        code_line.replace_range(range, &new_text);
        Self {
            meta: EditMetaData { start_line: line, from: 1, to: 1 },
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
        let range = token_range_at(code_line.as_str(), c.char);
        let from = CursorPosition { line: c.line, char: range.start };
        let to = CursorPosition { line: c.line, char: range.end };
        let indent = cfg.derive_indent_from(code_line.as_str());
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

    pub fn end_position(&self) -> CursorPosition {
        self.reverse_text_edit.range.end.into()
    }

    /// apply reverse edit (goes into undone)
    pub fn apply_rev(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.reverse_text_edit.range.start.into();
        remove_content(from, self.reverse_text_edit.range.end.into(), content);
        events.push((self.meta.rev(), self.reverse_event()));
        (insert_clip(self.reverse_text_edit.new_text.to_owned(), content, from), self.select)
    }

    /// apply edit (goes into done)
    pub fn apply(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.text_edit.range.start.into();
        remove_content(from, self.text_edit.range.end.into(), content);
        events.push((self.meta, self.event()));
        (insert_clip(self.text_edit.new_text.to_owned(), content, from), self.new_select)
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

#[derive(Clone, Copy)]
pub struct EditMetaData {
    pub start_line: usize,
    pub from: usize,
    pub to: usize,
}

impl EditMetaData {
    pub fn line_changed(start_line: usize) -> Self {
        Self { start_line, from: 1, to: 1 }
    }

    pub fn build_range(&self, content: &[impl EditorLine]) -> Option<Range> {
        let end_line = self.start_line + self.to - 1;
        Some(Range::new(
            Position::new(self.start_line as u32, 0),
            Position::new(end_line as u32, content.get(end_line)?.len() as u32),
        ))
    }

    fn rev(&self) -> Self {
        EditMetaData { start_line: self.start_line, from: self.to, to: self.from }
    }
}

impl Debug for EditMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} >> {}", self.from, self.to))
    }
}

#[derive(Debug)]
pub struct NewLineBuilder {
    pub reverse_edit_text: String,
    text_edit_range: (CursorPosition, CursorPosition),
    reverse_len: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl NewLineBuilder {
    /// initialize builder collecting select if exists
    pub fn new(cursor: &mut Cursor, content: &mut Vec<impl EditorLine>) -> Self {
        match cursor.select_take() {
            Some((from, to)) => {
                cursor.set_position(from);
                Self {
                    reverse_edit_text: clip_content(from, to, content),
                    reverse_len: to.line - from.line + 1,
                    text_edit_range: (from, to),
                    select: Some((from, to)),
                }
            }
            None => Self {
                text_edit_range: (cursor.into(), cursor.into()),
                reverse_edit_text: String::new(),
                reverse_len: 1,
                select: None,
            },
        }
    }

    pub fn finish(self, cursor: CursorPosition, content: &[impl EditorLine]) -> Edit {
        Edit {
            meta: EditMetaData {
                start_line: self.text_edit_range.0.line,
                from: self.reverse_len,
                to: cursor.line - self.text_edit_range.0.line + 1,
            },
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: copy_content(self.text_edit_range.0, cursor, content),
            },
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), cursor.into()),
                new_text: self.reverse_edit_text,
            },
            select: self.select,
            new_select: None,
        }
    }

    // UTILS
    pub fn and_clear_first_line(&mut self, line: &mut impl EditorLine) {
        self.text_edit_range.0.char = 0;
        self.reverse_edit_text.insert_str(0, line.as_str());
        line.clear();
    }
}

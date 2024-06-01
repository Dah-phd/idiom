use lsp_types::Position;

use crate::{
    configs::IndentConfigs,
    render::UTF8Safe,
    utils::Offset,
    workspace::{
        cursor::Cursor,
        line::EditorLine,
        utils::{clip_content, copy_content, insert_clip, remove_content, token_range_at},
        CursorPosition,
    },
};

use super::edits::EditMetaData;

#[derive(Debug)]
pub struct Edit {
    pub meta: EditMetaData,
    pub from: CursorPosition,
    pub reverse_text_edit: String,
    pub text_edit: String,
    pub select: Option<(CursorPosition, CursorPosition)>,
    pub new_select: Option<(CursorPosition, CursorPosition)>,
}

impl Edit {
    pub fn swap_down(up_line: usize, cfg: &IndentConfigs, content: &mut [impl EditorLine]) -> (Offset, Offset, Self) {
        let to = up_line + 1;
        let reverse_text_edit = format!("{}\n{}\n", content[up_line], content[to]);
        content.swap(up_line, to);
        let offset = cfg.indent_line(up_line, content);
        let offset2 = cfg.indent_line(to, content);
        (
            offset,
            offset2,
            Self {
                meta: EditMetaData { start_line: up_line, from: 2, to: 2 },
                reverse_text_edit,
                text_edit: format!("{}\n{}\n", content[up_line], content[to]),
                from: CursorPosition { line: up_line, char: 0 },
                select: None,
                new_select: None,
            },
        )
    }

    pub fn merge_next_line(line: usize, content: &mut Vec<impl EditorLine>) -> Self {
        let removed_line = content.remove(line + 1);
        let merged_to = &mut content[line];
        let from_char = CursorPosition { line, char: merged_to.char_len() };
        merged_to.push_line(removed_line);
        Self {
            from: from_char,
            meta: EditMetaData { start_line: line, from: 2, to: 1 },
            text_edit: String::new(),
            reverse_text_edit: "\n".to_owned(),
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
            return Some((
                Offset::Neg(removed.len()),
                Self {
                    from: CursorPosition { line, char: idx },
                    meta: EditMetaData::line_changed(line),
                    reverse_text_edit: removed,
                    text_edit: String::new(),
                    select: None,
                    new_select: None,
                },
            ));
        };
        if idx != 0 {
            text.replace_range(0..indent.len(), "");
            return Some((
                Offset::Neg(indent.len()),
                Self {
                    from: CursorPosition { line, char: 0 },
                    meta: EditMetaData::line_changed(line),
                    reverse_text_edit: indent.to_owned(),
                    text_edit: String::new(),
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
            from: position.into(),
            meta: EditMetaData::line_changed(position.line as usize),
            text_edit: new_text,
            reverse_text_edit: String::new(),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_from_line(at_line: usize, from: usize, to: usize, line: &mut impl EditorLine) -> Self {
        let old = line[from..to].to_owned();
        line.replace_range(from..to, "");
        Self {
            from: CursorPosition { line: at_line, char: from },
            meta: EditMetaData::line_changed(at_line),
            reverse_text_edit: old,
            text_edit: String::new(),
            select: None,
            new_select: None,
        }
    }

    /// builds action from removed data
    pub fn extract_from_start(line: usize, len: usize, text: &mut String) -> Self {
        let mut old_text = text.split_off(len);
        std::mem::swap(text, &mut old_text);
        Self {
            from: CursorPosition { line, char: 0 },
            meta: EditMetaData::line_changed(line),
            reverse_text_edit: old_text,
            text_edit: String::new(),
            select: None,
            new_select: None,
        }
    }

    pub fn insert_clip(from: CursorPosition, clip: String, content: &mut Vec<impl EditorLine>) -> Self {
        let end = insert_clip(&clip, content, from);
        Self {
            from,
            meta: EditMetaData { start_line: from.line, from: 1, to: (end.line - from.line) + 1 },
            text_edit: clip,
            reverse_text_edit: String::new(),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_line(line: usize, content: &mut Vec<impl EditorLine>) -> Self {
        let mut reverse_text_edit = content.remove(line).unwrap();
        reverse_text_edit.push('\n');
        Self {
            from: CursorPosition { line, char: 0 },
            meta: EditMetaData { start_line: line, from: 2, to: 1 },
            reverse_text_edit,
            text_edit: String::new(),
            select: None,
            new_select: None,
        }
    }

    pub fn remove_select(from: CursorPosition, to: CursorPosition, content: &mut Vec<impl EditorLine>) -> Self {
        Self {
            from,
            meta: EditMetaData { start_line: from.line, from: to.line - from.line + 1, to: 1 },
            reverse_text_edit: clip_content(from, to, content),
            select: Some((from, to)),
            text_edit: String::new(),
            new_select: None,
        }
    }

    pub fn replace_select(
        from: CursorPosition,
        to: CursorPosition,
        clip: String,
        content: &mut Vec<impl EditorLine>,
    ) -> Self {
        let reverse_text_edit = clip_content(from, to, content);
        let end = if !clip.is_empty() { insert_clip(&clip, content, from) } else { from };
        Self {
            from,
            meta: EditMetaData { start_line: from.line, from: to.line - from.line + 1, to: (end.line - from.line) + 1 },
            reverse_text_edit,
            text_edit: clip,
            select: Some((from, to)),
            new_select: None,
        }
    }

    pub fn replace_token(line: usize, char: usize, new_text: String, content: &mut [impl EditorLine]) -> Self {
        let code_line = &mut content[line];
        let range = token_range_at(code_line, char);
        let char = range.start;
        let reverse_text_edit = code_line[range.clone()].to_owned();
        code_line.replace_range(range, &new_text);
        Self {
            from: CursorPosition { line, char },
            meta: EditMetaData::line_changed(line),
            text_edit: new_text,
            reverse_text_edit,
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
        (new_cursor.unwrap_or(edit.end_position()), edit)
    }

    /// UTILS

    #[inline]
    pub fn get_new_text(&self) -> &str {
        &self.text_edit
    }

    #[inline]
    pub fn get_removed_text(&self) -> &str {
        &self.reverse_text_edit
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
        self.from
    }

    #[inline]
    pub fn end_position(&self) -> CursorPosition {
        let line = self.meta.end_line();
        let mut char = self.text_edit.split('\n').last().unwrap().char_len();
        if line == self.meta.start_line {
            char += self.from.char;
        };
        CursorPosition { line, char }
    }

    #[inline]
    pub fn end_position_rev(&self) -> CursorPosition {
        let line = self.meta.start_line + self.meta.from - 1;
        let mut char = self.reverse_text_edit.split('\n').last().unwrap().char_len();
        if line == self.meta.start_line {
            char += self.from.char;
        };
        CursorPosition { line, char }
    }

    /// apply reverse edit (goes into undone)
    pub fn apply_rev(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<LSPEvent>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position();
        remove_content(from, to, content);
        events.push(self.reverse_event());
        (insert_clip(&self.reverse_text_edit, content, from), self.select)
    }

    /// apply edit (goes into done)
    pub fn apply(
        &self,
        content: &mut Vec<impl EditorLine>,
        events: &mut Vec<LSPEvent>,
    ) -> (CursorPosition, Option<(CursorPosition, CursorPosition)>) {
        let from = self.start_position();
        let to = self.end_position_rev();
        remove_content(from, to, content);
        events.push(self.event());
        (insert_clip(&self.text_edit, content, from), self.new_select)
    }

    pub fn reverse_event(&self) -> LSPEvent {
        LSPEvent { meta: self.meta.rev(), from_char: self.from.char, text: self.reverse_text_edit.to_owned() }
    }

    pub fn event(&self) -> LSPEvent {
        LSPEvent { meta: self.meta, from_char: self.from.char, text: self.text_edit.to_owned() }
    }
}

pub struct LSPEvent {
    meta: EditMetaData,
    from_char: usize,
    text: String,
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
            from: self.text_edit_range.0,
            meta: EditMetaData {
                start_line: self.text_edit_range.0.line,
                from: self.reverse_len,
                to: cursor.line - self.text_edit_range.0.line + 1,
            },
            text_edit: copy_content(self.text_edit_range.0, cursor, content),
            reverse_text_edit: self.reverse_edit_text,
            select: self.select,
            new_select: None,
        }
    }

    // UTILS
    pub fn and_clear_first_line(&mut self, line: &mut impl EditorLine) {
        self.text_edit_range.0.char = 0;
        line.push_content_to_buffer(&mut self.reverse_edit_text);
        line.clear();
    }
}

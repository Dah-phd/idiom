use std::fmt::Debug;

use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};

use crate::{
    components::workspace::file::utils::{insert_clip, remove_content, token_range_at},
    configs::EditorConfigs,
    utils::Offset,
};

use super::{
    super::utils::{clip_content, copy_content},
    Cursor, CursorPosition,
};

#[derive(Debug)]
pub struct Edit {
    pub meta: EditMetaData,
    pub reverse_text_edit: TextEdit,
    pub text_edit: TextEdit,
}

impl Edit {
    pub fn swap_down(from: usize, cfg: &EditorConfigs, content: &mut [String]) -> (Offset, Offset, Self) {
        let to = from + 1;
        let mut reverse_edit_text = content[from].to_owned();
        reverse_edit_text.push('\n');
        reverse_edit_text.push_str(&content[from + 1]);
        reverse_edit_text.push('\n');
        let text_edit_range: (CursorPosition, CursorPosition) = ((from, 0).into(), (from + 2, 0).into());
        content.swap(from, to);
        let offset = cfg.indent_line(from, content);
        let offset2 = cfg.indent_line(to, content);
        let mut new_text = content[text_edit_range.0.line].to_owned();
        new_text.push('\n');
        new_text.push_str(&content[text_edit_range.0.line + 1]);
        new_text.push('\n');
        let range = Range::new(Position::new(from as u32, 0), Position::new((from + 2) as u32, 0));
        (
            offset,
            offset2,
            Self {
                meta: EditMetaData { from: 2, to: 2 },
                reverse_text_edit: TextEdit::new(range, reverse_edit_text),
                text_edit: TextEdit::new(range, new_text),
            },
        )
    }

    pub fn merge_next_line(from: usize, content: &mut Vec<String>) -> Self {
        let removed_line = content.remove(from + 1);
        let merged_to = &mut content[from];
        let position_of_new_line = Position::new(from as u32, merged_to.len() as u32);
        merged_to.push_str(removed_line.as_ref());
        Self {
            meta: EditMetaData { from: 1, to: 1 },
            reverse_text_edit: TextEdit::new(
                Range::new(position_of_new_line, position_of_new_line),
                String::from("\n"),
            ),
            text_edit: TextEdit::new(
                Range::new(position_of_new_line, Position::new((from + 1) as u32, 0)),
                String::new(),
            ),
        }
    }

    pub fn insertion(line: u32, char: u32, new_text: String) -> Self {
        Self {
            meta: EditMetaData { from: 1, to: 1 },
            reverse_text_edit: TextEdit::new(
                Range::new(Position::new(line, char), Position::new(line, char + new_text.len() as u32)),
                String::new(),
            ),
            text_edit: TextEdit::new(Range::new(Position::new(line, char), Position::new(line, char)), new_text),
        }
    }

    /// builds action from removed data
    pub fn extract_from_line(line: usize, char: usize, len: usize, text: &mut String) -> Self {
        let position = Position::new(line as u32, char as u32);
        let mut old_text = text.split_off(len);
        std::mem::swap(text, &mut old_text);
        Self {
            meta: EditMetaData { from: 1, to: 1 },
            text_edit: TextEdit::new(
                Range::new(position, Position::new(line as u32, (char + len) as u32)),
                String::new(),
            ),
            reverse_text_edit: TextEdit::new(Range::new(position, position), old_text),
        }
    }

    pub fn insert_clip(from: CursorPosition, clip: String, content: &mut Vec<String>) -> Self {
        let end = insert_clip(clip.clone(), content, from);
        Edit {
            meta: EditMetaData { from: 1, to: (end.line - from.line) + 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), end.into()), String::new()),
            text_edit: TextEdit::new(Range::new(from.into(), from.into()), clip),
        }
    }

    pub fn remove_line(line: usize, content: &mut Vec<String>) -> Self {
        let mut removed_line = content.remove(line);
        removed_line.push('\n');
        let start = Position::new(line as u32, 0);
        Edit {
            meta: EditMetaData { from: 2, to: 1 },
            reverse_text_edit: TextEdit::new(Range::new(start, start), removed_line),
            text_edit: TextEdit::new(Range::new(start, Position::new(line as u32 + 1, 0)), String::new()),
        }
    }

    pub fn remove_select(from: CursorPosition, to: CursorPosition, content: &mut Vec<String>) -> Self {
        Edit {
            meta: EditMetaData { from: to.line - from.line + 1, to: 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), from.into()), clip_content(&from, &to, content)),
            text_edit: TextEdit::new(Range::new(from.into(), to.into()), String::new()),
        }
    }

    pub fn replace_select(from: CursorPosition, to: CursorPosition, clip: String, content: &mut Vec<String>) -> Self {
        let reverse_edit_text = clip_content(&from, &to, content);
        let end = if !clip.is_empty() { insert_clip(clip.clone(), content, from) } else { from };
        Edit {
            meta: EditMetaData { from: to.line - from.line + 1, to: (end.line - from.line) + 1 },
            reverse_text_edit: TextEdit::new(Range::new(from.into(), end.into()), reverse_edit_text),
            text_edit: TextEdit { range: Range::new(from.into(), to.into()), new_text: clip },
        }
    }

    pub fn replace_token(line: usize, char: usize, new_text: String, content: &mut [String]) -> Self {
        let code_line = &mut content[line];
        let range = token_range_at(code_line, char);
        let start = Position::new(line as u32, range.start as u32);
        let text_edit_range = Range::new(start, Position::new(line as u32, range.end as u32));
        let reverse_edit_range = Range::new(start, Position::new(line as u32, (range.start + new_text.len()) as u32));
        let replaced_text = code_line[range.clone()].to_owned();
        code_line.replace_range(range, &new_text);
        Edit {
            meta: EditMetaData::default(),
            text_edit: TextEdit::new(text_edit_range, new_text),
            reverse_text_edit: TextEdit::new(reverse_edit_range, replaced_text),
        }
    }

    pub fn end_position(&self) -> CursorPosition {
        self.reverse_text_edit.range.start.into()
    }

    /// reverse the action (goes into undone)
    pub fn apply_rev(
        &self,
        content: &mut Vec<String>,
        events: &mut Vec<TextDocumentContentChangeEvent>,
    ) -> CursorPosition {
        let from = self.reverse_text_edit.range.start.into();
        remove_content(from, self.reverse_text_edit.range.end.into(), content);
        events.push(self.reverse_event());
        insert_clip(self.reverse_text_edit.new_text.to_owned(), content, from)
    }

    /// dose edit (goes into done)
    pub fn apply(&self, content: &mut Vec<String>, events: &mut Vec<TextDocumentContentChangeEvent>) -> CursorPosition {
        let from = self.text_edit.range.start.into();
        remove_content(from, self.text_edit.range.end.into(), content);
        events.push(self.event());
        insert_clip(self.text_edit.new_text.to_owned(), content, from)
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

#[derive(Debug)]
pub struct EditBuilder {
    pub reverse_edit_text: String,
    text_edit_range: (CursorPosition, CursorPosition),
    reverse_len: usize,
}

impl EditBuilder {
    // OPENERS
    /// initialize builder collecting select if exists
    pub fn init_alt(cursor: &mut Cursor, content: &mut Vec<String>) -> Self {
        if let Some((from, to)) = cursor.select.take() {
            cursor.set_position(from);
            return Self {
                reverse_edit_text: clip_content(&from, &to, content),
                reverse_len: to.line - from.line + 1,
                text_edit_range: (from, to),
            };
        }
        Self { text_edit_range: (cursor.into(), cursor.into()), reverse_edit_text: String::new(), reverse_len: 1 }
    }

    pub fn empty_at(position: CursorPosition) -> Self {
        Self { reverse_len: 1, reverse_edit_text: String::new(), text_edit_range: (position, position) }
    }

    pub fn raw_finish(self, position: CursorPosition, new_text: String) -> Edit {
        Edit {
            meta: EditMetaData { from: self.reverse_len, to: 1 },
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), position.into()),
                new_text: self.reverse_edit_text,
            },
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text,
            },
        }
    }

    pub fn finish(self, cursor: CursorPosition, content: &[String]) -> Edit {
        Edit {
            meta: EditMetaData { from: self.reverse_len, to: cursor.line - self.text_edit_range.0.line + 1 },
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: copy_content(self.text_edit_range.0, cursor, content),
            },
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), cursor.into()),
                new_text: self.reverse_edit_text,
            },
        }
    }

    // UTILS

    pub fn and_clear_first_line(&mut self, line: &mut String) {
        self.text_edit_range.0.char = 0;
        self.reverse_edit_text.insert_str(0, line);
        line.clear();
    }
}

pub struct EditMetaData {
    pub from: usize,
    pub to: usize,
}

impl Default for EditMetaData {
    fn default() -> Self {
        Self { from: 1, to: 1 }
    }
}

impl Debug for EditMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} >> {}", self.from, self.to))
    }
}

use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};

use crate::{components::workspace::file::utils::token_range_at, configs::EditorConfigs};

use super::{
    super::utils::{clip_content, copy_content},
    Cursor, CursorPosition, Select,
};

#[derive(Debug)]
pub struct Action {
    pub reverse_len: usize,
    pub reverse_text_edit: TextEdit,
    pub len: usize,
    pub text_edit: TextEdit,
}

impl Action {
    pub fn swap_down(from: usize, cfg: &EditorConfigs, content: &mut [String]) -> Self {
        let to = from + 1;
        let mut reverse_edit_text = content[from].to_owned();
        reverse_edit_text.push('\n');
        reverse_edit_text.push_str(&content[from + 1]);
        reverse_edit_text.push('\n');
        let text_edit_range: (CursorPosition, CursorPosition) = ((from, 0).into(), (from + 2, 0).into());
        content.swap(from, to);
        let _offset = cfg.indent_line(from, content);
        let _offset2 = cfg.indent_line(to, content);
        let mut new_text = content[text_edit_range.0.line].to_owned();
        new_text.push('\n');
        new_text.push_str(&content[text_edit_range.0.line + 1]);
        new_text.push('\n');
        let range = Range::new(Position::new(from as u32, 0), Position::new((from + 2) as u32, 0));
        Self {
            reverse_len: 2,
            reverse_text_edit: TextEdit::new(range, reverse_edit_text),
            len: 2,
            text_edit: TextEdit::new(range, new_text),
        }
    }

    pub fn merge_next_line(at_line: usize, content: &mut Vec<String>) -> Self {
        let removed_line = content.remove(at_line + 1);
        let merged_to = &mut content[at_line];
        let position_of_new_line = Position::new(at_line as u32, merged_to.len() as u32);
        merged_to.push_str(removed_line.as_ref());
        Self {
            reverse_len: 1,
            reverse_text_edit: TextEdit::new(
                Range::new(position_of_new_line, position_of_new_line),
                String::from("\n"),
            ),
            len: 1,
            text_edit: TextEdit::new(
                Range::new(position_of_new_line, Position::new((at_line + 1) as u32, 0)),
                String::new(),
            ),
        }
    }

    pub fn insertion(line: u32, char: u32, new_text: String) -> Self {
        Self {
            reverse_len: 1,
            reverse_text_edit: TextEdit::new(
                Range::new(Position::new(line, char), Position::new(line, char + new_text.len() as u32)),
                String::new(),
            ),
            len: 1,
            text_edit: TextEdit::new(Range::new(Position::new(line, char), Position::new(line, char)), new_text),
        }
    }

    /// builds action from removed data
    pub fn extract(line: u32, char: u32, old_text: String) -> Self {
        Self {
            len: 1,
            text_edit: TextEdit::new(
                Range::new(Position::new(line, char), Position::new(line, old_text.len() as u32)),
                String::new(),
            ),
            reverse_len: 1,
            reverse_text_edit: TextEdit::new(
                Range::new(Position::new(line, char), Position::new(line, char)),
                old_text,
            ),
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
        Action {
            len: 1,
            text_edit: TextEdit::new(text_edit_range, new_text),
            reverse_len: 1,
            reverse_text_edit: TextEdit::new(reverse_edit_range, replaced_text),
        }
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
pub struct ActionBuilder {
    pub reverse_edit_text: String,
    text_edit_range: (CursorPosition, CursorPosition),
    reverse_len: usize,
}

impl ActionBuilder {
    // OPENERS
    /// initialize builder collecting select if exists
    pub fn init(cursor: &mut Cursor, content: &mut Vec<String>) -> Self {
        if let Select::Range(from, to) = cursor.select.take() {
            cursor.set_position(from);
            return Self {
                reverse_edit_text: clip_content(&from, &to, content),
                reverse_len: to.line - from.line + 1,
                text_edit_range: (from, to),
            };
        }
        Self {
            text_edit_range: (cursor.position(), cursor.position()),
            reverse_edit_text: String::new(),
            reverse_len: 1,
        }
    }

    pub fn cut_range(from: CursorPosition, to: CursorPosition, content: &mut Vec<String>) -> Self {
        Self {
            reverse_edit_text: clip_content(&from, &to, content),
            reverse_len: to.line - from.line + 1,
            text_edit_range: (from, to),
        }
    }

    pub fn cut_line(line: usize, content: &mut Vec<String>) -> Self {
        let mut reverse_edit_text = content.remove(line);
        reverse_edit_text.push('\n');
        Self { reverse_edit_text, text_edit_range: ((line, 0).into(), (line + 1, 0).into()), reverse_len: 1 }
    }

    pub fn empty_at(position: CursorPosition) -> Self {
        Self { reverse_len: 1, reverse_edit_text: String::new(), text_edit_range: (position, position) }
    }

    pub fn push_clip(self, clip: String, end: &CursorPosition) -> Action {
        Action {
            len: self.text_edit_range.0.line - end.line + 1,
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: clip,
            },
            reverse_len: self.reverse_len,
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), end.into()),
                new_text: self.reverse_edit_text,
            },
        }
    }

    pub fn force_finish(self) -> Action {
        Action {
            reverse_len: self.reverse_len,
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.0.into()),
                new_text: self.reverse_edit_text,
            },
            len: 1,
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: String::new(),
            },
        }
    }

    pub fn raw_finish(self, position: CursorPosition, new_text: String) -> Action {
        Action {
            reverse_len: self.reverse_len,
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), position.into()),
                new_text: self.reverse_edit_text,
            },
            len: 1,
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text,
            },
        }
    }

    pub fn finish(self, cursor: CursorPosition, content: &[String]) -> Action {
        Action {
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: copy_content(&self.text_edit_range.0, &cursor, content),
            },
            reverse_len: self.reverse_len,
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), cursor.into()),
                new_text: self.reverse_edit_text,
            },
            len: cursor.line - self.text_edit_range.0.line + 1,
        }
    }

    // UTILS

    pub fn and_clear_first_line(&mut self, line: &mut String) {
        self.text_edit_range.0.char = 0;
        self.reverse_edit_text.insert_str(0, line);
        line.clear();
    }
}

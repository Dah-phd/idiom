use lsp_types::{Position, Range, TextDocumentContentChangeEvent, TextEdit};
use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

use crate::components::workspace::file::utils::token_range_at;

use super::{
    super::utils::{clip_content, copy_content},
    Cursor, CursorPosition, Select,
};

const TICK: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub struct Action {
    reverse_len: usize,
    pub reverse_text_edit: TextEdit,
    len: usize,
    pub text_edit: TextEdit,
}

impl Action {
    pub fn swap_lines() {
        // let mut reverse_edit_text = content[from].to_owned();
        // reverse_edit_text.push('\n');
        // reverse_edit_text.push_str(&content[from + 1]);
        // reverse_edit_text.push('\n');
        // Self { reverse_edit_text, text_edit_range: ((from, 0).into(), (from + 2, 0).into()), reverse_len: 2 }
        // let mut new_text = content[self.text_edit_range.0.line].to_owned();
        // new_text.push('\n');
        // new_text.push_str(&content[self.text_edit_range.0.line + 1]);
        // new_text.push('\n');
        // Action {
        //     reverse_len: self.reverse_len,
        //     reverse_text_edit: TextEdit {
        //         range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
        //         new_text: self.reverse_edit_text,
        //     },
        //     len: self.reverse_len,
        //     text_edit: TextEdit {
        //         range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
        //         new_text,
        //     },
        // }
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

    pub fn for_swap(content: &[String], from: usize) -> Self {
        let mut reverse_edit_text = content[from].to_owned();
        reverse_edit_text.push('\n');
        reverse_edit_text.push_str(&content[from + 1]);
        reverse_edit_text.push('\n');
        Self { reverse_edit_text, text_edit_range: ((from, 0).into(), (from + 2, 0).into()), reverse_len: 2 }
    }

    // FINISHERS

    pub fn finish_swap(self, content: &[String]) -> Action {
        let mut new_text = content[self.text_edit_range.0.line].to_owned();
        new_text.push('\n');
        new_text.push_str(&content[self.text_edit_range.0.line + 1]);
        new_text.push('\n');
        Action {
            reverse_len: self.reverse_len,
            reverse_text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text: self.reverse_edit_text,
            },
            len: self.reverse_len,
            text_edit: TextEdit {
                range: Range::new(self.text_edit_range.0.into(), self.text_edit_range.1.into()),
                new_text,
            },
        }
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

#[derive(Debug)]
pub struct ActionBuffer {
    at_line: usize,
    start_char: usize,
    pub last_char: usize,
    buffer: String,
    clock: Instant,
}

impl Default for ActionBuffer {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl ActionBuffer {
    fn new(line: usize, char: usize) -> Self {
        Self { at_line: line, start_char: char, last_char: char, buffer: String::new(), clock: Instant::now() }
    }

    pub fn timed_collect(&mut self) -> Option<Action> {
        if self.buffer.is_empty() || self.clock.elapsed() < TICK {
            return None;
        }
        let collected = std::mem::take(self);
        Some(collected.into())
    }

    pub fn collect(&mut self) -> Option<Action> {
        if self.buffer.is_empty() {
            return None;
        }
        let collected = std::mem::take(self);
        Some(collected.into())
    }

    pub fn push(&mut self, ch: char, line: usize, char: usize) -> Option<Action> {
        if line == self.at_line && char == self.last_char && self.check_time() {
            self.last_char += 1;
            self.buffer.push(ch);
            return None;
        }
        let mut action = std::mem::replace(self, Self::new(line, char));
        self.last_char += 1;
        self.buffer.push(ch);
        action.collect()
    }

    pub fn del(&mut self, line: usize, char: usize, text: &mut String) -> Option<Action> {
        if line == self.at_line && self.start_char == self.last_char && self.start_char == char && self.check_time() {
            self.buffer.push(text.remove(char));
            return None;
        }
        let mut action = std::mem::replace(self, Self::new(line, char));
        self.buffer.push(text.remove(char));
        action.collect()
    }

    pub fn backspace(&mut self, line: usize, char: usize, text: &mut String, indent: &str) -> Option<Action> {
        if line == self.at_line && self.start_char >= self.last_char && self.last_char == char && self.check_time() {
            self.backspace_indent_handler(char, text, indent);
            return None;
        }
        let mut action = std::mem::replace(self, Self::new(line, char));
        self.backspace_indent_handler(char, text, indent);
        action.collect()
    }

    fn backspace_indent_handler(&mut self, char: usize, line: &mut String, indent: &str) {
        let chars_after_indent = line[..char].trim_start_matches(indent);
        if chars_after_indent.is_empty() {
            self.buffer.push_str(indent);
            line.replace_range(..indent.len(), "");
            self.last_char -= indent.len();
            return;
        }
        if chars_after_indent.chars().all(|c| c.is_whitespace()) {
            self.last_char -= chars_after_indent.len();
            self.buffer.push_str(&line[self.last_char..char]);
            line.replace_range(self.last_char..char, "");
            return;
        }
        self.buffer.push(line.remove(char - 1));
        self.last_char -= 1;
    }

    pub fn check_time(&mut self) -> bool {
        if self.clock.elapsed() <= TICK {
            self.clock = Instant::now();
            return true;
        }
        false
    }
}

impl From<ActionBuffer> for Action {
    fn from(buffer: ActionBuffer) -> Self {
        match buffer.last_char.cmp(&buffer.start_char) {
            Ordering::Greater => Action {
                // push
                reverse_len: 1,
                reverse_text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                        Position::new(buffer.at_line as u32, buffer.last_char as u32),
                    ),
                    new_text: String::new(),
                },
                len: 1,
                text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                    ),
                    new_text: buffer.buffer,
                },
            },
            Ordering::Equal => Action {
                // del
                len: 1,
                text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                        Position::new(buffer.at_line as u32, (buffer.start_char - buffer.buffer.len()) as u32),
                    ),
                    new_text: String::new(),
                },
                reverse_len: 1,
                reverse_text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                    ),
                    new_text: buffer.buffer,
                },
            },
            Ordering::Less => Action {
                // backspace
                reverse_len: 1,
                reverse_text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.last_char as u32),
                        Position::new(buffer.at_line as u32, buffer.last_char as u32),
                    ),
                    new_text: buffer.buffer.chars().rev().collect(),
                },
                len: 1,
                text_edit: TextEdit {
                    range: Range::new(
                        Position::new(buffer.at_line as u32, buffer.last_char as u32),
                        Position::new(buffer.at_line as u32, buffer.start_char as u32),
                    ),
                    new_text: String::new(),
                },
            },
        }
    }
}

use crate::{
    render::UTF8Safe,
    workspace::{
        actions::edits::{Edit, EditMetaData},
        line::EditorLine,
    },
};
use lsp_types::{Position, Range, TextEdit};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(200);

#[derive(Default, Debug)]
pub enum ActionBuffer {
    #[default]
    None,
    Del(DelBuffer),
    Backspace(BackspaceBuffer),
    Text(TextBuffer),
}

impl ActionBuffer {
    pub fn collect(&mut self) -> Option<Edit> {
        std::mem::take(self).into()
    }

    pub fn timed_collect(&mut self) -> Option<Edit> {
        match std::mem::take(self) {
            Self::Text(buf) => {
                if buf.clock.elapsed() > TICK {
                    return Some(buf.into());
                };
                let _ = std::mem::replace(self, Self::Text(buf));
            }
            Self::Backspace(buf) => {
                if buf.clock.elapsed() > TICK {
                    return Some(buf.into());
                };
                let _ = std::mem::replace(self, Self::Backspace(buf));
            }
            Self::Del(buf) => {
                if buf.clock.elapsed() > TICK {
                    return Some(buf.into());
                };
                let _ = std::mem::replace(self, Self::Del(buf));
            }
            Self::None => (),
        }
        None
    }

    pub fn last_char(&self) -> usize {
        match self {
            Self::Backspace(buf) => buf.last,
            Self::Text(buf) => buf.last,
            _ => 0,
        }
    }

    pub fn push(&mut self, line: usize, char: usize, ch: char) -> Option<Edit> {
        if let Self::Text(buf) = self {
            return buf.push(line, char, ch);
        }
        std::mem::replace(self, Self::Text(TextBuffer::new(line, char, ch.into()))).into()
    }

    pub fn del(&mut self, line: usize, char: usize, text: &mut impl EditorLine) -> Option<Edit> {
        if let Self::Del(buf) = self {
            return buf.del(line, char, text);
        }
        std::mem::replace(self, Self::Del(DelBuffer::new(line, char, text))).into()
    }

    pub fn backspace(&mut self, line: usize, char: usize, text: &mut impl EditorLine, indent: &str) -> Option<Edit> {
        if let Self::Backspace(buf) = self {
            return buf.backspace(line, char, text, indent);
        }
        std::mem::replace(self, Self::Backspace(BackspaceBuffer::new(line, char, text, indent))).into()
    }
}

impl From<ActionBuffer> for Option<Edit> {
    fn from(buffer: ActionBuffer) -> Self {
        match buffer {
            ActionBuffer::None => None,
            ActionBuffer::Backspace(buf) => Some(buf.into()),
            ActionBuffer::Del(buf) => Some(buf.into()),
            ActionBuffer::Text(buf) => Some(buf.into()),
        }
    }
}

#[derive(Debug)]
pub struct DelBuffer {
    line: usize,
    char: usize,
    text: String,
    clock: Instant,
}

impl DelBuffer {
    fn new(line: usize, char: usize, text: &mut impl EditorLine) -> Self {
        Self { line, char, text: text.remove(char).into(), clock: Instant::now() }
    }

    fn del(&mut self, line: usize, char: usize, text: &mut impl EditorLine) -> Option<Edit> {
        if line == self.line && char == self.char && self.clock.elapsed() <= TICK {
            self.clock = Instant::now();
            self.text.push(text.remove(char));
            return None;
        }
        Some(std::mem::replace(self, Self::new(line, char, text)).into())
    }
}

impl From<DelBuffer> for Edit {
    fn from(buf: DelBuffer) -> Self {
        let start = Position::new(buf.line as u32, buf.char as u32);
        Edit {
            meta: EditMetaData::line_changed(buf.line),
            text_edit: TextEdit::new(
                Range::new(start, Position::new(buf.line as u32, (buf.char + buf.text.utf8_len()) as u32)),
                String::new(),
            ),
            reverse_text_edit: TextEdit::new(Range::new(start, start), buf.text),
            select: None,
            new_select: None,
        }
    }
}

#[derive(Debug)]
pub struct BackspaceBuffer {
    line: usize,
    char: u32,
    last: usize,
    text: String,
    clock: Instant,
}

impl BackspaceBuffer {
    fn new(line: usize, char: usize, text: &mut impl EditorLine, indent: &str) -> Self {
        let mut new = Self { line, last: char, char: char as u32, text: String::new(), clock: Instant::now() };
        new.backspace_indent_handler(char, text, indent);
        new
    }

    fn backspace(&mut self, line: usize, char: usize, text: &mut impl EditorLine, indent: &str) -> Option<Edit> {
        if line == self.line && self.last == char && self.clock.elapsed() <= TICK {
            self.backspace_indent_handler(char, text, indent);
            return None;
        }
        Some(std::mem::replace(self, Self::new(line, char, text, indent)).into())
    }

    fn backspace_indent_handler(&mut self, char: usize, text: &mut impl EditorLine, indent: &str) {
        let chars_after_indent = text[..char].trim_start_matches(indent);
        if chars_after_indent.is_empty() {
            self.text.push_str(indent);
            text.replace_till(indent.len(), "");
            self.last -= indent.len();
            return;
        }
        if chars_after_indent.chars().all(|c| c.is_whitespace()) {
            self.last -= chars_after_indent.len();
            self.text.push_str(&text[self.last..char]);
            text.replace_range(self.last..char, "");
            return;
        }
        self.text.push(text.remove(char - 1));
        self.last -= 1;
    }
}

impl From<BackspaceBuffer> for Edit {
    fn from(buf: BackspaceBuffer) -> Self {
        let end = Position::new(buf.line as u32, buf.last as u32);
        Edit {
            meta: EditMetaData::line_changed(buf.line),
            reverse_text_edit: TextEdit::new(Range::new(end, end), buf.text.chars().rev().collect()),
            text_edit: TextEdit::new(Range::new(end, Position::new(buf.line as u32, buf.char)), String::new()),
            select: None,
            new_select: None,
        }
    }
}

#[derive(Debug)]
pub struct TextBuffer {
    line: usize,
    char: u32,
    last: usize,
    text: String,
    clock: Instant,
}

impl TextBuffer {
    fn new(line: usize, char: usize, text: String) -> Self {
        Self { line, last: char + 1, char: char as u32, text, clock: Instant::now() }
    }

    fn push(&mut self, line: usize, char: usize, ch: char) -> Option<Edit> {
        if line == self.line && char == self.last && self.clock.elapsed() <= TICK {
            self.clock = Instant::now();
            self.last += 1;
            self.text.push(ch);
            return None;
        }
        Some(std::mem::replace(self, Self::new(line, char, ch.into())).into())
    }
}

impl From<TextBuffer> for Edit {
    fn from(buf: TextBuffer) -> Self {
        let start = Position::new(buf.line as u32, buf.char);
        Edit {
            meta: EditMetaData::line_changed(buf.line),
            reverse_text_edit: TextEdit::new(
                Range::new(start, Position::new(buf.line as u32, buf.last as u32)),
                String::new(),
            ),
            text_edit: TextEdit::new(Range::new(start, start), buf.text),
            select: None,
            new_select: None,
        }
    }
}

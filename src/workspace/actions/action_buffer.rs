use crate::workspace::{actions::Edit, line::EditorLine, CursorPosition};

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
            ActionBuffer::Backspace(buf) => buf.into(),
            ActionBuffer::Del(buf) => buf.into(),
            ActionBuffer::Text(buf) => buf.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DelBuffer {
    line: usize,
    char: usize,
    text: String,
}

impl DelBuffer {
    fn new(line: usize, char: usize, text: &mut impl EditorLine) -> Self {
        Self { line, char, text: text.remove(char).into() }
    }

    fn del(&mut self, line: usize, char: usize, text: &mut impl EditorLine) -> Option<Edit> {
        if line == self.line && char == self.char {
            self.text.push(text.remove(char));
            return None;
        }
        std::mem::replace(self, Self::new(line, char, text)).into()
    }
}

impl From<DelBuffer> for Option<Edit> {
    fn from(buf: DelBuffer) -> Self {
        if buf.text.is_empty() {
            return None;
        }
        Some(Edit::single_line(CursorPosition { line: buf.line, char: buf.char }, String::new(), buf.text))
    }
}

#[derive(Debug, Clone)]
pub struct BackspaceBuffer {
    line: usize,
    last: usize,
    text: String,
}

impl BackspaceBuffer {
    fn new(line: usize, char: usize, text: &mut impl EditorLine, indent: &str) -> Self {
        let mut new = Self { line, last: char, text: String::new() };
        new.backspace_indent_handler(char, text, indent);
        new
    }

    fn backspace(&mut self, line: usize, char: usize, text: &mut impl EditorLine, indent: &str) -> Option<Edit> {
        if line == self.line && self.last == char {
            self.backspace_indent_handler(char, text, indent);
            return None;
        }
        std::mem::replace(self, Self::new(line, char, text, indent)).into()
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

impl From<BackspaceBuffer> for Option<Edit> {
    fn from(buf: BackspaceBuffer) -> Self {
        if buf.text.is_empty() {
            return None;
        }
        Some(Edit::single_line(
            CursorPosition { line: buf.line, char: buf.last },
            String::new(),
            buf.text.chars().rev().collect(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    line: usize,
    char: u32,
    last: usize,
    text: String,
}

impl TextBuffer {
    fn new(line: usize, char: usize, text: String) -> Self {
        Self { line, last: char + 1, char: char as u32, text }
    }

    fn push(&mut self, line: usize, char: usize, ch: char) -> Option<Edit> {
        if line == self.line && char == self.last && (ch.is_alphabetic() || ch == '_') {
            self.last += 1;
            self.text.push(ch);
            return None;
        }
        std::mem::replace(self, Self::new(line, char, ch.into())).into()
    }
}

impl From<TextBuffer> for Option<Edit> {
    fn from(buf: TextBuffer) -> Self {
        if buf.text.is_empty() {
            return None;
        }
        Some(Edit::single_line(CursorPosition { line: buf.line, char: buf.char as usize }, buf.text, String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::edits::Edit;
    use crate::workspace::line::{CodeLine, EditorLine};
    use crate::workspace::CursorPosition;

    use super::ActionBuffer;

    #[test]
    fn test_del() {
        let mut code_line = CodeLine::new("0123456789".to_owned());
        let mut buf = ActionBuffer::None;
        buf.del(0, 7, &mut code_line);
        buf.del(0, 7, &mut code_line);
        buf.del(0, 7, &mut code_line);
        if let ActionBuffer::Del(buf) = buf {
            let m_edit: Option<Edit> = buf.clone().into();
            let edit = m_edit.unwrap();
            assert!(edit.text.is_empty());
            assert_eq!(edit.reverse, "789");
            assert_eq!(edit.cursor, CursorPosition { line: 0, char: 7 });
            return;
        }
        panic!("Expected Del buf!")
    }

    #[test]
    fn test_backspace() {
        let mut code_line = CodeLine::new("          1".to_owned());
        let indent = "    ";
        let mut buf = ActionBuffer::None;
        buf.backspace(0, 11, &mut code_line, indent);
        buf.backspace(0, 10, &mut code_line, indent);
        buf.backspace(0, 8, &mut code_line, indent);
        if let ActionBuffer::Backspace(buf) = buf {
            let m_edit: Option<Edit> = buf.clone().into();
            let edit = m_edit.unwrap();
            assert!(edit.text.is_empty());
            assert_eq!(edit.reverse, "      1");
            assert_eq!(code_line.unwrap(), indent);
            assert_eq!(edit.cursor, CursorPosition { line: 0, char: 4 });
            return;
        }
        panic!("Expected Backspace buf!")
    }

    #[test]
    fn test_text() {
        let mut buf = ActionBuffer::None;
        buf.push(0, 0, 'a');
        buf.push(0, 1, 'b');
        buf.push(0, 2, 'c');
        if let Some(edit) = buf.push(0, 3, ' ') {
            assert!(edit.reverse.is_empty());
            assert_eq!(edit.text, "abc");
            assert_eq!(edit.cursor, CursorPosition { line: 0, char: 0 });
        } else {
            panic!("Expected edit!")
        }
        buf.push(0, 4, 'a');
        buf.push(0, 5, '_');
        if let Some(edit) = buf.push(0, 6, '1') {
            assert!(edit.reverse.is_empty());
            assert_eq!(edit.text, " a_");
            assert_eq!(edit.cursor, CursorPosition { line: 0, char: 3 });
        } else {
            panic!("Expected edit!")
        }
    }
}

use super::super::{actions::Edit, line::EditorLine, CursorPosition};
use crate::{syntax::Lexer, workspace::cursor::Cursor};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

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

    pub fn push(
        &mut self,
        cursor: &mut Cursor,
        ch: char,
        code_text: &mut EditorLine,
        lexer: &Lexer,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if let Self::Text(buf) = self {
            return buf.push(cursor, ch, code_text, lexer);
        }
        let (new, event) = TextBuffer::new(cursor, ch, code_text, lexer);
        (std::mem::replace(self, Self::Text(new)).into(), event)
    }

    pub fn del(
        &mut self,
        cursor: &Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if let Self::Del(buf) = self {
            return buf.del(cursor, text, lexer);
        }
        let (new, event) = DelBuffer::new(cursor, text, lexer);
        (std::mem::replace(self, Self::Del(new)).into(), event)
    }

    pub fn backspace(
        &mut self,
        cursor: &mut Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
        indent: &str,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if let Self::Backspace(buf) = self {
            return buf.backspace(cursor, text, lexer, indent);
        }
        let (new, event) = BackspaceBuffer::new(cursor, text, lexer, indent);
        (std::mem::replace(self, Self::Backspace(new)).into(), event)
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
    text: String,
    line: usize,
    char: usize,
    change_start: Position,
}

impl DelBuffer {
    fn new(cursor: &Cursor, text: &mut EditorLine, lexer: &Lexer) -> (Self, TextDocumentContentChangeEvent) {
        let line = cursor.line;
        let char = cursor.char;
        let change_start = if text.is_simple() {
            Position::new(line as u32, cursor.char as u32)
        } else {
            Position::new(line as u32, (lexer.encode_position)(cursor.char, &text[..]) as u32)
        };
        let removed = text.remove(char);
        let end = Position::new(change_start.line, change_start.character + ((lexer.char_lsp_pos)(removed)) as u32);
        (
            Self { line, char, change_start, text: String::from(removed) },
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(change_start, end)),
                range_length: None,
            },
        )
    }

    fn del(
        &mut self,
        cursor: &Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if cursor.line == self.line && cursor.char == self.char {
            let removed = text.remove(cursor.char);
            let end_character = self.change_start.character + ((lexer.char_lsp_pos)(removed)) as u32;
            let end = Position::new(self.change_start.line, end_character);
            self.text.push(removed);
            return (
                None,
                TextDocumentContentChangeEvent {
                    text: String::new(),
                    range: Some(Range::new(self.change_start, end)),
                    range_length: None,
                },
            );
        }
        let (new, event) = Self::new(cursor, text, lexer);
        (std::mem::replace(self, new).into(), event)
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
    fn new(
        cursor: &mut Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
        indent: &str,
    ) -> (Self, TextDocumentContentChangeEvent) {
        let mut new = Self { line: cursor.line, last: cursor.char, text: String::new() };
        let event = new.backspace_indent_handler(cursor, text, lexer, indent);
        cursor.set_char(new.last);
        (new, event)
    }

    fn backspace(
        &mut self,
        cursor: &mut Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
        indent: &str,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if cursor.line == self.line && self.last == cursor.char {
            let event = self.backspace_indent_handler(cursor, text, lexer, indent);
            cursor.set_char(self.last);
            return (None, event);
        }
        let (new, event) = Self::new(cursor, text, lexer, indent);
        (std::mem::replace(self, new).into(), event)
    }

    /// handles only whitespace logic - no encoding needed
    fn backspace_indent_handler(
        &mut self,
        cursor: &mut Cursor,
        text: &mut EditorLine,
        lexer: &Lexer,
        indent: &str,
    ) -> TextDocumentContentChangeEvent {
        let char = cursor.char;
        let line = cursor.line as u32;
        let chars_after_indent = text[..char].trim_start_matches(indent);

        let range = if chars_after_indent.is_empty() {
            self.last -= indent.len();
            self.text.push_str(indent);
            text.tokens.remove_tokens_till(indent.len());
            text.replace_till(indent.len(), "");
            Range::new(Position::new(line, self.last as u32), Position::new(line, char as u32))
        } else if chars_after_indent.chars().all(|c| c.is_whitespace()) {
            let removed_count = chars_after_indent.len();
            self.last -= removed_count;
            self.text.push_str(&text[self.last..char]);
            text.tokens.remove_tokens_till(removed_count);
            text.replace_till(removed_count, "");
            Range::new(Position::new(line, self.last as u32), Position::new(line, char as u32))
        } else {
            self.last -= 1;
            let ch = text.remove(self.last);
            self.text.push(ch);
            let character = match text.is_simple() {
                true => self.last,
                false => (lexer.encode_position)(self.last, text.content.as_str()),
            };
            let start = Position::new(line, character as u32);
            let end = Position::new(line, (character + (lexer.char_lsp_pos)(ch)) as u32);
            Range::new(start, end)
        };
        TextDocumentContentChangeEvent { text: String::new(), range: Some(range), range_length: None }
    }
}

impl From<BackspaceBuffer> for Option<Edit> {
    fn from(buf: BackspaceBuffer) -> Self {
        if buf.text.is_empty() {
            return None;
        }
        let cursor = CursorPosition { line: buf.line, char: buf.last };
        Some(Edit::single_line(cursor, String::new(), buf.text.chars().rev().collect()))
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    last: usize,
    line: usize,
    char: usize,
    text: String,
}

impl TextBuffer {
    fn new(
        cursor: &mut Cursor,
        ch: char,
        text: &mut EditorLine,
        lexer: &Lexer,
    ) -> (Self, TextDocumentContentChangeEvent) {
        let char = cursor.char;
        let pos = if cursor.char != 0 && !text.is_simple() {
            Position::new(cursor.line as u32, (lexer.encode_position)(cursor.char, &text[..]) as u32)
        } else {
            cursor.into()
        };
        text.insert(cursor.char, ch);
        cursor.add_to_char(1);
        (
            Self { line: cursor.line, last: cursor.char, char, text: String::from(ch) },
            TextDocumentContentChangeEvent {
                text: String::from(ch),
                range: Some(Range::new(pos, pos)),
                range_length: None,
            },
        )
    }

    fn push(
        &mut self,
        cursor: &mut Cursor,
        ch: char,
        text: &mut EditorLine,
        lexer: &Lexer,
    ) -> (Option<Edit>, TextDocumentContentChangeEvent) {
        if cursor.line == self.line && cursor.char == self.last && (ch.is_alphabetic() || ch == '_') {
            let pos = match cursor.char != 0 && !text.is_simple() {
                true => Position::new(cursor.line as u32, (lexer.encode_position)(cursor.char, &text[..]) as u32),
                false => cursor.into(),
            };
            self.text.push(ch);
            text.insert(cursor.char, ch);
            cursor.add_to_char(1);
            self.last = cursor.char;
            return (
                None,
                TextDocumentContentChangeEvent {
                    text: String::from(ch),
                    range: Some(Range::new(pos, pos)),
                    range_length: None,
                },
            );
        }
        let (new, event) = Self::new(cursor, ch, text, lexer);
        (std::mem::replace(self, new).into(), event)
    }
}

impl From<TextBuffer> for Option<Edit> {
    fn from(buf: TextBuffer) -> Self {
        if buf.text.is_empty() {
            return None;
        }
        Some(Edit::single_line(CursorPosition { line: buf.line, char: buf.char }, buf.text, String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::edits::Edit;
    use super::ActionBuffer;
    use crate::configs::FileType;
    use crate::global_state::GlobalState;
    use crate::render::backend::{Backend, BackendProtocol};
    use crate::syntax::{
        tests::{char_lsp_utf8, encode_pos_utf8},
        Lexer,
    };
    use crate::workspace::actions::buffer::DelBuffer;
    use crate::workspace::line::EditorLine;
    use crate::workspace::{cursor::Cursor, CursorPosition};
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
    use std::path::PathBuf;

    fn create_lexer() -> Lexer {
        let mut gs = GlobalState::new(Backend::init()).unwrap();
        let path = PathBuf::new();
        Lexer::with_context(FileType::Rust, &path, &mut gs)
    }

    fn create_lexer_utf8() -> Lexer {
        let mut lexer = create_lexer();
        lexer.encode_position = encode_pos_utf8;
        lexer.char_lsp_pos = char_lsp_utf8;
        lexer
    }

    #[test]
    fn del() {
        let lexer = create_lexer();
        let mut code_text = EditorLine::new("0123456789".to_owned());
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_char(7);
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 7), Position::new(0, 8))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 7), Position::new(0, 8))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 7), Position::new(0, 8))),
                range_length: None,
            }
        );
        assert!(edit.is_none());

        let edit = buf.collect().unwrap();
        let mut content = vec![code_text];
        edit.apply_rev(&mut content);
        assert_eq!(content[0].content, "0123456789");
        assert_eq!(edit.text, "");
        assert_eq!(edit.reverse, "789");
        assert_eq!(edit.cursor, CursorPosition { line: 0, char: 7 });
    }

    #[test]
    fn del_complx() {
        let lexer = create_lexer_utf8();
        let mut code_text = EditorLine::new("012🙀4567🙀9".to_owned());
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_char(7);
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 10), Position::new(0, 11))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 10), Position::new(0, 14))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        let (edit, event) = buf.del(&cursor, &mut code_text, &lexer);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 10), Position::new(0, 11))),
                range_length: None,
            }
        );
        assert!(edit.is_none());

        assert!(matches!(
            buf, ActionBuffer::Del(DelBuffer { text, change_start, ..}) if text == "7🙀9" && change_start == Position::new(0, 10)
        ));
    }

    #[test]
    fn backspace() {
        let lexer = create_lexer();
        let mut code_text = EditorLine::new("          1".to_owned());
        let indent = "    ";
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_position((0, 11).into());
        buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        if let ActionBuffer::Backspace(buf) = buf {
            let m_edit: Option<Edit> = buf.clone().into();
            let edit = m_edit.unwrap();
            assert!(edit.text.is_empty());
            assert_eq!(edit.reverse, "      1");
            assert_eq!(code_text.unwrap(), indent);
            assert_eq!(edit.cursor, CursorPosition { line: 0, char: 4 });
            return;
        }
        panic!("Expected Backspace buf!")
    }

    #[test]
    fn backspace_indent() {
        let lexer = create_lexer_utf8();
        let mut code_text = EditorLine::new("          🙀".to_owned());
        let indent = "    ";
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_position((0, 10).into());
        let (edit, event) = buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 8), Position::new(0, 10))),
                range_length: None
            }
        );
        assert_eq!(code_text.content.as_str(), "        🙀");
        assert_eq!(cursor.char, 8);
        assert!(edit.is_none());
        let (edit, event) = buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 4), Position::new(0, 8))),
                range_length: None
            }
        );
        assert_eq!(code_text.content.as_str(), "    🙀");
        assert_eq!(cursor.char, 4);
        assert!(edit.is_none());
        assert!(matches!(buf, ActionBuffer::Backspace(..)));
    }

    #[test]
    fn backspace_complex() {
        let lexer = create_lexer_utf8();
        let mut code_text = EditorLine::new("        🙀🙀1🙀2".to_owned());
        let indent = "    ";
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_position((0, 12).into());
        let (edit, event) = buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        assert_eq!("        🙀🙀12", code_text.content);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 17), Position::new(0, 21))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        cursor.set_position((0, 9).into());
        let (edit, event) = buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        assert_eq!("        🙀12", code_text.content);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 8), Position::new(0, 12))),
                range_length: None,
            }
        );
        assert!(edit.is_some());
        let (edit, event) = buf.backspace(&mut cursor, &mut code_text, &lexer, indent);
        assert_eq!("    🙀12", code_text.content);
        assert_eq!(
            event,
            TextDocumentContentChangeEvent {
                text: String::new(),
                range: Some(Range::new(Position::new(0, 4), Position::new(0, 8))),
                range_length: None,
            }
        );
        assert!(edit.is_none());
        assert!(matches!(buf, ActionBuffer::Backspace(..)));
    }

    #[test]
    fn text() {
        let lexer = create_lexer();
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        let mut code_text = EditorLine::from("");
        let (edit, event) = buf.push(&mut cursor, 'a', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 0), Position::new(0, 0)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('a'), range, range_length: None });
        assert!(edit.is_none());

        let (edit, event) = buf.push(&mut cursor, 'b', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 1), Position::new(0, 1)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('b'), range, range_length: None });
        assert!(edit.is_none());

        let (edit, event) = buf.push(&mut cursor, 'c', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 2), Position::new(0, 2)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('c'), range, range_length: None });
        assert!(edit.is_none());

        let (maybe_edit, event) = buf.push(&mut cursor, ' ', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 3), Position::new(0, 3)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from(' '), range, range_length: None });
        assert!(
            matches!(maybe_edit, Some(edit) if edit.reverse.is_empty() && edit.text == "abc" && edit.cursor == CursorPosition { line: 0, char: 0 })
        );

        let (edit, event) = buf.push(&mut cursor, 'a', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 4), Position::new(0, 4)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('a'), range, range_length: None });
        assert!(edit.is_none());

        let (edit, event) = buf.push(&mut cursor, '_', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 5), Position::new(0, 5)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('_'), range, range_length: None });
        assert!(edit.is_none());

        let (maybe_edit, event) = buf.push(&mut cursor, '1', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 6), Position::new(0, 6)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('1'), range, range_length: None });
        assert!(
            matches!(maybe_edit, Some(edit) if edit.reverse.is_empty() && edit.text == " a_" && edit.cursor == CursorPosition { line: 0, char: 3 })
        );

        assert!(
            matches!(buf, ActionBuffer::Text(buffer) if buffer.text == "1" && buffer.line == 0 && buffer.char == 6)
        );
    }

    #[test]
    fn test_complex() {
        let lexer = create_lexer_utf8();
        let mut buf = ActionBuffer::None;
        let mut cursor = Cursor::default();
        cursor.set_position((0, 6).into());
        let mut code_text = EditorLine::from("tesxt🙀asd32ra 🙀dw");
        let (no_edit, event) = buf.push(&mut cursor, 'b', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 9), Position::new(0, 9)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('b'), range, range_length: None });
        assert!(no_edit.is_none());

        let (no_edit, event) = buf.push(&mut cursor, 'b', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 10), Position::new(0, 10)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('b'), range, range_length: None });
        assert!(no_edit.is_none());
        assert_eq!(code_text.content, "tesxt🙀bbasd32ra 🙀dw");

        let (edit, event) = buf.push(&mut cursor, '🙀', &mut code_text, &lexer);
        let range = Some(Range::new(Position::new(0, 11), Position::new(0, 11)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('🙀'), range, range_length: None });
        assert!(
            matches!(edit, Some(edit) if edit.reverse.is_empty() && edit.text == "bb" && edit.cursor == CursorPosition {line: 0, char: 6})
        );
        assert_eq!(code_text.content, "tesxt🙀bb🙀asd32ra 🙀dw");

        cursor.set_char(8);
        let (edit, event) = buf.push(&mut cursor, 'x', &mut code_text, &lexer);
        assert!(edit.is_some());
        assert_eq!(code_text.content, "tesxt🙀bbx🙀asd32ra 🙀dw");
        let range = Some(Range::new(Position::new(0, 11), Position::new(0, 11)));
        assert_eq!(event, TextDocumentContentChangeEvent { text: String::from('x'), range, range_length: None });
        assert!(
            matches!(buf, ActionBuffer::Text(text) if text.text == "x" && text.line == 0 && text.char == 8 && text.last == 9)
        );
    }
}

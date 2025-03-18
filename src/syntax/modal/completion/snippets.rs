use crate::global_state::IdiomEvent;
use lsp_types::{CompletionItem, CompletionTextEdit, InsertTextFormat};
use std::str::Chars;

pub fn parse_completion_item(item: CompletionItem) -> IdiomEvent {
    let parser = match item.insert_text_format {
        Some(InsertTextFormat::SNIPPET) => parse_snippet,
        _ => IdiomEvent::AutoComplete,
    };
    if let Some(text) = item.insert_text {
        return (parser)(text);
    }
    if let Some(edit) = item.text_edit {
        match edit {
            CompletionTextEdit::Edit(edit) => {
                return (parser)(edit.new_text);
            }
            CompletionTextEdit::InsertAndReplace(edit) => {
                return (parser)(edit.new_text);
            }
        };
    }
    IdiomEvent::AutoComplete(item.label)
}

#[derive(Default)]
struct WrappedBuffer {
    inner: String,
    char: usize,
    line: usize,
}

impl WrappedBuffer {
    #[inline]
    fn end_line(&mut self) {
        self.inner.push('\n');
        self.line += 1;
        self.char = 0;
    }

    #[inline]
    fn push(&mut self, ch: char) {
        self.inner.push(ch);
        self.char += 1;
    }

    #[inline]
    fn push_str(&mut self, string: &str) {
        self.char += string.len();
        self.inner.push_str(string);
    }

    #[inline]
    fn snapshot_position(&self) -> (usize, usize) {
        (self.line, self.char)
    }
}

/// Example:
/// "push(${1:value})$0"
/// TODO refactor
fn parse_snippet(snippet: String) -> IdiomEvent {
    let mut cursor_offset = None;
    let mut relative_select = None;
    let mut buffer = WrappedBuffer::default();
    let mut chars = snippet.chars();
    while let Some(ch) = chars.next() {
        match ch {
            // new line
            '\n' => {
                buffer.end_line();
            }
            // expression
            '$' => {
                let next_ch = match chars.next() {
                    Some(next_ch) => next_ch,
                    None => {
                        buffer.push(ch);
                        return IdiomEvent::Snippet { snippet: buffer.inner, cursor_offset, relative_select };
                    }
                };

                match next_ch {
                    // positional
                    '0'..'9' => {
                        if cursor_offset.is_none() {
                            cursor_offset.replace(buffer.snapshot_position());
                        }
                        if let Some(non_num_ch) = skip_numbers(&mut chars) {
                            match non_num_ch {
                                '\n' => {
                                    buffer.end_line();
                                }
                                _ => {
                                    buffer.push(non_num_ch);
                                }
                            }
                        }
                    }
                    // named arg
                    '{' => {
                        let (number, maybe_char) = collect_numbers(&mut chars);
                        match maybe_char {
                            Some(':') if !number.is_empty() => {
                                let (name, end_char) = collect_name(&mut chars);
                                match end_char {
                                    Some('}') => {
                                        if relative_select.is_none() {
                                            relative_select.replace((buffer.snapshot_position(), name.len()));
                                        }
                                        buffer.push_str(&name);
                                    }
                                    Some('\n') => {
                                        buffer.inner.push(ch);
                                        buffer.inner.push(next_ch);
                                        buffer.inner.push_str(&number);
                                        buffer.inner.push(':');
                                        buffer.inner.push_str(&name);
                                        buffer.end_line();
                                    }
                                    Some(other) => {
                                        buffer.push(ch);
                                        buffer.push(next_ch);
                                        buffer.push_str(&number);
                                        buffer.push(':');
                                        buffer.push_str(&name);
                                        buffer.push(other);
                                    }
                                    None => {
                                        buffer.inner.push(ch);
                                        buffer.inner.push(next_ch);
                                        buffer.inner.push_str(&number);
                                        buffer.inner.push(':');
                                        buffer.inner.push_str(&name);
                                    }
                                }
                            }
                            Some('\n') => {
                                buffer.inner.push(ch);
                                buffer.inner.push(next_ch);
                                buffer.inner.push_str(&number);
                                buffer.end_line();
                            }
                            Some(other) => {
                                buffer.push(ch);
                                buffer.push(next_ch);
                                buffer.push_str(&number);
                                buffer.push(other);
                            }
                            None => {
                                buffer.inner.push(ch);
                                buffer.inner.push(next_ch);
                                buffer.inner.push_str(&number);
                            }
                        }
                    }
                    // new line after $
                    '\n' => {
                        buffer.inner.push(ch);
                        buffer.end_line();
                    }
                    // unexpected char for expression
                    _ => {
                        buffer.push(ch);
                        buffer.push(next_ch);
                    }
                }
            }
            _ => buffer.push(ch),
        }
    }
    IdiomEvent::Snippet { snippet: buffer.inner, cursor_offset, relative_select }
}

fn skip_numbers(chars: &mut Chars) -> Option<char> {
    while let Some(ch) = chars.next() {
        if !ch.is_numeric() {
            return Some(ch);
        }
    }
    None
}

fn collect_numbers(chars: &mut Chars) -> (String, Option<char>) {
    let mut number = String::new();
    while let Some(ch) = chars.next() {
        if !ch.is_numeric() {
            return (number, Some(ch));
        }
        number.push(ch);
    }
    (number, None)
}

fn collect_name(chars: &mut Chars) -> (String, Option<char>) {
    let mut name = String::new();
    while let Some(ch) = chars.next() {
        if ch.is_alphabetic() || ch.is_numeric() || " _&".contains(ch) {
            name.push(ch);
            continue;
        }
        return (name, Some(ch));
    }
    (name, None)
}

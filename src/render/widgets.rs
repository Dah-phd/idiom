use super::{backend::BackendProtocol, layout::RectIter};
use crate::render::{
    backend::{Backend, Style},
    layout::{Line, Rect},
    UTF8Safe,
};

pub trait Widget {
    fn width(&self) -> usize;
    fn char_len(&self) -> usize;
    fn len(&self) -> usize;
    fn print(&self, backend: &mut Backend);
    fn print_at(&self, line: Line, backend: &mut Backend);
    fn wrap(&self, lines: RectIter, backend: &mut Backend);
}

pub struct Text {
    text: String,
    char_len: usize,
    width: usize,
    style: Option<Style>,
}

impl Widget for Text {
    fn char_len(&self) -> usize {
        self.char_len
    }

    fn width(&self) -> usize {
        self.width
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn print(&self, backend: &mut Backend) {
        backend.print(&self.text);
    }

    fn print_at(&self, line: Line, backend: &mut Backend) {
        backend.print_at(line.row, line.col, &self.text);
    }

    fn wrap(&self, lines: RectIter, backend: &mut Backend) {}
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: None }
    }
}

impl From<(String, Style)> for Text {
    fn from((text, style): (String, Style)) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: Some(style) }
    }
}

impl From<(Style, String)> for Text {
    fn from((style, text): (Style, String)) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: Some(style) }
    }
}

pub struct StyledLine {
    inner: Vec<Text>,
}

impl StyledLine {
    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) -> bool {
        let mut current_line = match lines.next() {
            Some(line) => line,
            None => return false,
        };
        let mut remaining = current_line.width;
        backend.go_to(current_line.row, current_line.col);
        for word in self.inner.iter() {
            todo!()
        }
        true
    }

    fn render(&self, line: Line, backend: &mut Backend) {
        let mut builder = line.unsafe_builder(backend);
        for word in self.inner.iter() {
            if !match word.style {
                Some(style) => builder.push_styled(&word.text, style),
                None => builder.push(&word.text),
            } {
                return;
            }
        }
    }

    fn render_rev(&self, line: Line, backend: &mut Backend) {
        let mut builder = line.unsafe_builder_rev(backend);
        for word in self.inner.iter() {
            if !match word.style {
                Some(style) => builder.push_styled(&word.text, style),
                None => builder.push(&word.text),
            } {
                return;
            }
        }
    }
}

impl From<Vec<Text>> for StyledLine {
    fn from(inner: Vec<Text>) -> Self {
        Self { inner }
    }
}

#[allow(dead_code)]
pub fn paragraph<'a>(area: Rect, text: impl Iterator<Item = &'a str>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for text_line in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.width() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render(text_slice, backend);
                        } else {
                            line.render(text_line[at_char..].as_ref(), backend);
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return;
                        }
                    }
                } else {
                    line.render(text_line, backend);
                };
            }
            None => return,
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}

pub fn alt_paragraph_styled<'a>(area: Rect, text: impl Iterator<Item = &'a StyledLine>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for line_text in text {
        if !line_text.wrap(&mut lines, backend) {
            return;
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}

pub fn paragraph_styled<'a>(area: Rect, text: impl Iterator<Item = (&'a str, Style)>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for (text_line, style) in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.width() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render_styled(text_slice, style, backend);
                        } else {
                            line.render_styled(text_line[at_char..].as_ref(), style, backend);
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return;
                        }
                    }
                } else {
                    line.render_styled(text_line, style, backend);
                };
            }
            None => return,
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}

use crossterm::style;

use super::{backend::BackendProtocol, layout::RectIter, utils::WriteChunks};
use crate::render::{
    backend::{Backend, Style},
    layout::{Line, Rect},
    UTF8Safe,
};

/// Trait that allows faster rendering without checks and can reduce complexity
pub trait Writable {
    /// check if the line can be rendered as ascii - no control chars should be included
    fn is_simple(&self) -> bool;
    /// width when rendered
    fn width(&self) -> usize;
    fn char_len(&self) -> usize;
    fn len(&self) -> usize;
    /// directly render no checks or bounds
    fn print(&self, backend: &mut Backend);
    /// print truncated
    unsafe fn print_truncated(&self, width: usize, backend: &mut Backend);
    /// print truncated start
    unsafe fn print_truncated_start(&self, width: usize, backend: &mut Backend);
    /// prints bounded by line
    fn print_at(&self, line: Line, backend: &mut Backend);
    /// wraps within rect
    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend);
}

pub struct Text {
    text: String,
    char_len: usize,
    width: usize,
    style: Option<Style>,
}

impl Text {
    pub fn new(text: String, style: Option<Style>) -> Self {
        Self { char_len: text.char_len(), width: text.width(), style, text }
    }
}

impl Writable for Text {
    #[inline(always)]
    fn is_simple(&self) -> bool {
        self.char_len == self.text.len()
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.char_len
    }

    #[inline(always)]
    fn width(&self) -> usize {
        self.width
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.text.len()
    }

    fn print(&self, backend: &mut Backend) {
        backend.print(&self.text);
    }

    unsafe fn print_truncated(&self, width: usize, backend: &mut Backend) {
        let text = if self.is_simple() { self.text.get_unchecked(..width) } else { self.text.truncate_width(width) };
        match self.style {
            Some(style) => backend.print_styled(text, style),
            None => backend.print(text),
        }
    }

    unsafe fn print_truncated_start(&self, width: usize, backend: &mut Backend) {
        let text = if self.is_simple() { self.text.get_unchecked(width..) } else { self.text.truncate_width(width) };
        match self.style {
            Some(style) => backend.print_styled(text, style),
            None => backend.print(text),
        }
    }

    fn print_at(&self, line: Line, backend: &mut Backend) {
        let Line { width, row, col } = line;
        let text = if self.width > width { self.text.truncate_width(width) } else { &self.text };

        match self.style {
            Some(style) => backend.print_styled_at(row, col, text, style),
            None => backend.print_at(row, col, text),
        }
    }

    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) {
        for (width, text_chunk) in WriteChunks::new(&self.text, lines.width()) {
            match lines.next() {
                Some(line) => backend.print_at(line.row, line.col, text_chunk),
                None => return,
            }
        }
    }
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

impl Writable for StyledLine {
    fn is_simple(&self) -> bool {
        self.inner.iter().all(|text| text.is_simple())
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.inner.iter().fold(0, |sum, text| sum + text.char_len)
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.inner.iter().fold(0, |sum, text| sum + text.len())
    }

    fn width(&self) -> usize {
        self.inner.iter().fold(0, |sum, text| sum + text.width)
    }

    fn print(&self, backend: &mut Backend) {
        for text in self.inner.iter() {
            text.print(backend)
        }
    }

    unsafe fn print_truncated(&self, mut width: usize, backend: &mut Backend) {
        for text in self.inner.iter() {
            if text.width > width {
                text.print_truncated(width, backend);
                return;
            }
            width -= text.width;
            text.print(backend);
        }
    }

    unsafe fn print_truncated_start(&self, width: usize, backend: &mut Backend) {
        let mut skipped = self.width() - width;
        let mut iter = self.inner.iter();
        for text in iter.by_ref() {
            if text.width > skipped {
                text.print_truncated_start(skipped, backend);
                break;
            }
            skipped -= text.width;
        }

        for text in iter {
            text.print(backend);
        }
    }

    fn print_at(&self, line: Line, backend: &mut Backend) {
        let Line { row, col, mut width } = line;
        backend.go_to(row, col);
        for text in self.inner.iter() {
            if width < text.width {
                let truncated_text = text.text.truncate_width(width);
                match text.style {
                    Some(style) => backend.print_styled(truncated_text, style),
                    None => backend.print(truncated_text),
                };
                return;
            }
            width -= text.width;
            text.print(backend);
        }
    }

    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) {}
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

#[cfg(test)]
mod tests;

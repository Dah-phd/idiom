use super::{backend::BackendProtocol, layout::RectIter, utils::WriteChunks};
use crate::render::{
    backend::{Backend, Style},
    layout::{Line, Rect},
    UTF8Safe,
};
use std::fmt::Display;
use unicode_width::UnicodeWidthChar;

/// Trait that allows faster rendering without checks and can reduce complexity
pub trait Writable: Display {
    /// check if the line can be rendered as ascii - no control chars should be included
    fn is_simple(&self) -> bool;
    /// width when rendered
    fn width(&self) -> usize;
    fn char_len(&self) -> usize;
    fn len(&self) -> usize;
    /// directly render no checks or bounds
    fn print(&self, backend: &mut Backend);
    /// prints bounded by line
    fn print_at(&self, line: Line, backend: &mut Backend);
    /// wraps within rect
    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend);
    /// print truncated
    unsafe fn print_truncated(&self, width: usize, backend: &mut Backend);
    /// print truncated start
    unsafe fn print_truncated_start(&self, width: usize, backend: &mut Backend);
}

/// Represents word with additional meta data such as width, style and number of chars, useful when rendering multiple times the same string
#[derive(Clone, PartialEq)]
pub struct Text {
    text: String,
    char_len: usize,
    width: usize,
    style: Option<Style>,
}

impl Text {
    #[inline]
    pub fn new(text: String, style: Option<Style>) -> Self {
        Self { char_len: text.char_len(), width: text.width(), style, text }
    }

    #[inline]
    pub fn raw(text: String) -> Self {
        Self { char_len: text.char_len(), width: text.width(), style: None, text }
    }

    #[inline]
    pub fn style(&self) -> Option<Style> {
        self.style
    }

    #[inline]
    pub fn set_style(&mut self, style: Option<Style>) {
        self.style = style;
    }

    #[inline]
    pub fn simple_wrap(&self, lines: &mut RectIter, backend: &mut Backend) {
        let max_width = match lines.move_cursor(backend) {
            Some(width) => width,
            None => return,
        };
        if max_width > self.width {
            match self.style {
                Some(style) => backend.print_styled(&self.text, style),
                None => backend.print(&self.text),
            };
            backend.pad(max_width - self.width);
        } else {
            let mut remaining = self.width;
            let mut start = 0;
            match self.style {
                Some(style) => loop {
                    if remaining > max_width {
                        backend.print_styled(&self.text[start..start + max_width], style);
                        remaining -= max_width;
                        start += max_width;
                    } else {
                        backend.print_styled(&self.text[start..], style);
                        backend.pad(max_width - remaining);
                        return;
                    }
                    if lines.move_cursor(backend).is_none() {
                        return;
                    }
                },
                None => loop {
                    if remaining < max_width {
                        backend.print(&self.text[start..]);
                        backend.pad(max_width - remaining);
                    } else {
                        backend.print(&self.text[start..start + max_width]);
                        remaining -= max_width;
                        start += max_width;
                    }
                    if lines.move_cursor(backend).is_none() {
                        return;
                    }
                },
            }
        }
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
        match self.style {
            Some(style) => backend.print_styled(&self.text, style),
            None => backend.print(&self.text),
        }
    }

    unsafe fn print_truncated(&self, width: usize, backend: &mut Backend) {
        if self.is_simple() {
            match self.style {
                Some(style) => backend.print_styled(self.text.get_unchecked(..width), style),
                None => backend.print(self.text.get_unchecked(..width)),
            }
        } else {
            let (remaining_w, text) = self.text.truncate_width(width);
            match self.style {
                Some(style) => backend.print_styled(text, style),
                None => backend.print(text),
            }
            if remaining_w != 0 {
                backend.pad(remaining_w);
            }
        };
    }

    unsafe fn print_truncated_start(&self, width: usize, backend: &mut Backend) {
        if self.is_simple() {
            match self.style {
                Some(style) => backend.print_styled(self.text.get_unchecked(self.len() - width..), style),
                None => backend.print(self.text.get_unchecked(self.len() - width..)),
            }
        } else {
            let (remaining_w, text) = self.text.truncate_width_start(width);
            if remaining_w != 0 {
                backend.pad(remaining_w);
            }
            match self.style {
                Some(style) => backend.print_styled(text, style),
                None => backend.print(text),
            }
        };
    }

    fn print_at(&self, line: Line, backend: &mut Backend) {
        let Line { width, row, col } = line;
        backend.go_to(row, col);
        if self.width > width {
            unsafe { self.print_truncated(width, backend) };
            return;
        }
        let pad_width = width - self.width;
        self.print(backend);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
    }

    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) {
        if self.is_simple() {
            return self.simple_wrap(lines, backend);
        }
        let max_width = lines.width();
        let chunks = WriteChunks::new(&self.text, max_width);
        match self.style {
            Some(style) => {
                for (width, text_chunk) in chunks {
                    if lines.move_cursor(backend).is_none() {
                        return;
                    }
                    backend.print_styled(text_chunk, style);
                    if width < max_width {
                        backend.pad(max_width - width);
                    }
                }
            }
            None => {
                for (width, text_chunk) in chunks {
                    if lines.move_cursor(backend).is_none() {
                        return;
                    }
                    backend.print(text_chunk);
                    if width < max_width {
                        backend.pad(max_width - width);
                    }
                }
            }
        }
    }
}

/// Collection of styled texts, useful when rendering multiple times the same string, as it holds meta data for width / charcer len of words
#[derive(Clone, PartialEq)]
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
                let truncated_text = text.text.truncate_width(width).1;
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

    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) {
        let mut width = match lines.move_cursor(backend) {
            Some(width) => width,
            None => return,
        };
        for word in self.inner.iter() {
            if word.width > width {
                if width != 0 {
                    continue;
                };
                width = match lines.move_cursor(backend) {
                    Some(new_width) => new_width,
                    None => return,
                }
            }
            width -= word.width;
            word.print(backend);
        }
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

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: None }
    }
}

impl From<char> for Text {
    #[inline]
    fn from(value: char) -> Self {
        Self {
            char_len: 1,
            width: UnicodeWidthChar::width(value).unwrap_or_default(),
            text: value.to_string(),
            style: None,
        }
    }
}

impl From<(String, Style)> for Text {
    #[inline]
    fn from((text, style): (String, Style)) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: Some(style) }
    }
}

impl From<(Style, String)> for Text {
    #[inline]
    fn from((style, text): (Style, String)) -> Self {
        Self { char_len: text.char_len(), width: text.width(), text, style: Some(style) }
    }
}

impl Display for StyledLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for text in self.inner.iter() {
            text.fmt(f)?;
        }
        Ok(())
    }
}

impl From<Vec<Text>> for StyledLine {
    fn from(inner: Vec<Text>) -> Self {
        Self { inner }
    }
}

#[cfg(test)]
mod tests;

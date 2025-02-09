use std::str::{CharIndices, Chars};
use unicode_width::UnicodeWidthChar;

/// Iterate over str getting chars and corresponding widths
/// in case char has no width or exceeds provided limit returns error char with 1 width
#[derive(Clone)]
pub struct CharLimitedWidths<'a> {
    chars: Chars<'a>,
    limit: usize,
}

impl<'a> CharLimitedWidths<'a> {
    pub fn new(text: &'a str, width_limit: usize) -> Self {
        let chars = text.chars();
        Self { chars, limit: width_limit }
    }
}

impl<'a> Iterator for CharLimitedWidths<'a> {
    type Item = (char, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.chars.next()?;
        match ch.width() {
            Some(width) if width <= self.limit => Some((ch, width)),
            _ => Some(('⚠', 1)),
        }
    }
}

impl<'a> DoubleEndedIterator for CharLimitedWidths<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ch = self.chars.next_back()?;
        match ch.width() {
            Some(width) if width <= self.limit => Some((ch, width)),
            _ => Some(('⚠', 1)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct StrChunks<'a> {
    pub text: &'a str,
    pub width: usize,
}

pub struct ByteChunks<'a> {
    pub width: usize,
    text: &'a str,
}

impl<'a> ByteChunks<'a> {
    pub fn new(text: &'a str, width: usize) -> Self {
        Self { text, width }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width == 0
    }
}

impl<'a> Iterator for ByteChunks<'a> {
    type Item = StrChunks<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.text.len() >= self.width {
            let result = self.text.get(..self.width).map(|text| StrChunks { text, width: self.width });
            self.text = unsafe { self.text.get_unchecked(self.width..) };
            return result;
        }
        if !self.text.is_empty() {
            let result = StrChunks { width: self.text.len(), text: self.text };
            self.text = "";
            return Some(result);
        }
        None
    }
}

pub struct WriteChunks<'a> {
    pub width: usize,
    at_byte: usize,
    text: &'a str,
    inner: CharIndices<'a>,
    width_offset: usize,
}

impl<'a> WriteChunks<'a> {
    pub fn new(text: &'a str, width: usize) -> Self {
        Self { inner: text.char_indices(), text, at_byte: 0, width, width_offset: 0 }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width == 0
    }
}

impl<'a> Iterator for WriteChunks<'a> {
    type Item = StrChunks<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.width == 0 {
            return None;
        }
        let start = self.at_byte;
        let mut width = self.width_offset;
        for (idx, ch) in self.inner.by_ref() {
            let current_w = UnicodeWidthChar::width(ch).unwrap_or_default();
            if self.width < width + current_w {
                if current_w > self.width {
                    self.width = 0;
                    return None;
                }
                self.width_offset = current_w;
                self.at_byte = idx;
                return Some(StrChunks { width, text: unsafe { self.text.get_unchecked(start..self.at_byte) } });
            };
            width += current_w;
        }
        self.width = 0;
        return Some(StrChunks { width, text: unsafe { self.text.get_unchecked(start..) } });
        // (width, unsafe { self.text.get_unchecked(start..) }));
    }
}

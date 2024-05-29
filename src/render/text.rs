use std::fmt::Display;
use std::ops::Range;

use crate::render::UTF8Safe;

use super::utils::UTF8SafeStringExt;

/// String wrapper with precalculated utf8_len and render width
#[derive(Default)]
pub struct Text {
    inner: String,
    utf8_len: usize,
    width: usize,
}

impl Text {
    fn new(string: impl Into<String>) -> Self {
        let inner = string.into();
        let utf8_len = inner.char_len();
        let width = inner.width();
        Self { inner, utf8_len, width }
    }

    #[inline]
    fn len(&self) -> usize {
        self.utf8_len
    }

    #[inline]
    fn replace_from(&mut self, from: usize, string: &str) {
        if self.utf8_len == self.inner.len() {
            self.utf8_len += string.char_len();
            self.width += string.width();
            self.inner.truncate(from);
            self.inner.push_str(string);
        } else {
            self.inner.utf8_replace_from(from, string);
            self.recalc();
        }
    }

    #[inline]
    fn replace_till(&mut self, to: usize, string: &str) {
        if self.utf8_len == self.inner.len() {
            self.utf8_len += string.char_len();
            self.width += string.width();
            self.inner.replace_range(..to, string);
        } else {
            self.inner.utf8_replace_till(to, string);
            self.recalc();
        }
    }

    #[inline]
    fn replace_range(&mut self, range: Range<usize>, string: &str) {
        if self.utf8_len == self.inner.len() {
            self.utf8_len += string.char_len();
            self.width += string.width();
            self.inner.replace_range(range, string);
        } else {
            self.inner.utf8_replace_range(range, string);
            self.recalc();
        }
    }

    #[inline]
    fn remove(&mut self, idx: usize) -> char {
        if self.inner.len() == self.utf8_len {
            self.utf8_len -= 1;
            let ch = self.inner.remove(idx);
            ch
        } else {
            let ch = self.inner.utf8_remove(idx);
            self.recalc();
            ch
        }
    }

    #[inline]
    fn truncate_width(&self, width: usize) -> &str {
        if self.width > width {
            if self.inner.len() == self.utf8_len {
                unsafe { self.inner.get_unchecked(..width) }
            } else {
                self.inner.truncate_width(width)
            }
        } else {
            &self.inner
        }
    }

    #[inline]
    fn recalc(&mut self) {
        self.utf8_len = self.inner.char_len();
        self.width = self.inner.width();
    }
}

impl Display for Text {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl From<&str> for Text {
    #[inline]
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Text {
    #[inline]
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

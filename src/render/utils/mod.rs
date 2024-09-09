mod chunks;
pub use chunks::{ByteChunks, StrChunks, WriteChunks};
use std::ops::Range;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Trait allowing UTF8 safe operations on str/String
pub trait UTF8Safe {
    /// returns str that will fit into width of columns, removing chars at the end returning info about remaining width
    fn truncate_width(&self, width: usize) -> (usize, &str);
    /// returns str that will fit into width of columns, removing chars from the start returng info about remaining width
    fn truncate_width_start(&self, width: usize) -> (usize, &str);
    /// return Some(&str) if wider than allowed width
    fn truncate_if_wider(&self, width: usize) -> Result<&str, usize>;
    /// return Some(&str) truncated from start if wider than allowed width
    fn truncate_if_wider_start(&self, width: usize) -> Result<&str, usize>;
    /// returns display len of the str
    fn width(&self) -> usize;
    /// calcs the width at position
    fn width_at(&self, at: usize) -> usize;
    /// returns utf8 chars len
    fn char_len(&self) -> usize;
    /// utf16 len
    fn utf16_len(&self) -> usize;
    /// return utf8 split at char idx
    fn utf8_split_at(&self, mid: usize) -> (&str, &str);
    /// splits utf8 if not ascii (needs precalculated utf8 len)
    fn utf8_cached_split_at(&self, mid: usize, utf8_len: usize) -> (&str, &str);
    /// limits str within range based on utf char locations
    /// panics if out of bounds
    fn utf8_unsafe_get(&self, from: usize, to: usize) -> &str;
    /// removes "from" chars from the begining of the string
    /// panics if out of bounds
    fn utf8_unsafe_get_from(&self, from: usize) -> &str;
    /// limits str to char idx
    /// panics if out of bounds
    fn utf8_unsafe_get_to(&self, to: usize) -> &str;
    /// get checked utf8 slice
    fn utf8_get(&self, from: usize, to: usize) -> Option<&str>;
    /// get checked utf8 from
    fn utf8_get_from(&self, from: usize) -> Option<&str>;
    /// get checked utf8 to
    fn utf8_get_to(&self, to: usize) -> Option<&str>;
}

/// String specific extension
pub trait UTF8SafeStringExt {
    fn utf8_insert(&mut self, idx: usize, ch: char);
    fn utf8_insert_str(&mut self, idx: usize, string: &str);
    fn utf8_remove(&mut self, idx: usize) -> char;
    fn utf8_replace_range(&mut self, range: Range<usize>, string: &str);
    fn utf8_replace_till(&mut self, to: usize, string: &str);
    fn utf8_replace_from(&mut self, from: usize, string: &str);
    fn utf8_split_off(&mut self, at: usize) -> Self;
}

impl UTF8Safe for str {
    #[inline(always)]
    fn truncate_width(&self, mut width: usize) -> (usize, &str) {
        let mut end = 0;
        for char in self.chars() {
            let char_width = UnicodeWidthChar::width(char).unwrap_or(0);
            if char_width > width {
                return (width, unsafe { self.get_unchecked(..end) });
            };
            width -= char_width;
            end += char.len_utf8();
        }
        (width, self)
    }

    #[inline(always)]
    fn truncate_width_start(&self, mut width: usize) -> (usize, &str) {
        let mut start = 0;
        for char in self.chars().rev() {
            let char_width = UnicodeWidthChar::width(char).unwrap_or(0);
            if char_width > width {
                return (width, unsafe { self.get_unchecked(self.len() - start..) });
            }
            width -= char_width;
            start += char.len_utf8();
        }
        (width, self)
    }

    #[inline(always)]
    fn truncate_if_wider(&self, width: usize) -> Result<&str, usize> {
        let mut end = 0;
        let mut current_width = 0;
        for char in self.chars() {
            current_width += UnicodeWidthChar::width(char).unwrap_or(0);
            if current_width > width {
                return Ok(unsafe { self.get_unchecked(..end) });
            };
            end += char.len_utf8();
        }
        Err(current_width)
    }

    #[inline(always)]
    fn truncate_if_wider_start(&self, width: usize) -> Result<&str, usize> {
        let mut start = 0;
        let mut current_width = 0;
        for char in self.chars().rev() {
            current_width += UnicodeWidthChar::width(char).unwrap_or(0);
            if current_width > width {
                return Ok(unsafe { self.get_unchecked(self.len() - start..) });
            }
            start += char.len_utf8();
        }
        Err(current_width)
    }

    #[inline(always)]
    fn width(&self) -> usize {
        UnicodeWidthStr::width(self)
    }

    #[inline(always)]
    fn width_at(&self, at: usize) -> usize {
        self.chars().take(at).fold(0, |l, r| l + UnicodeWidthChar::width(r).unwrap_or(0))
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.chars().count()
    }

    #[inline(always)]
    fn utf16_len(&self) -> usize {
        self.chars().fold(0, |sum, ch| sum + ch.len_utf16())
    }

    #[inline(always)]
    fn utf8_split_at(&self, mid: usize) -> (&str, &str) {
        self.split_at(prev_char_bytes_end(self, mid))
    }

    #[inline(always)]
    fn utf8_cached_split_at(&self, mid: usize, utf8_len: usize) -> (&str, &str) {
        if self.len() == utf8_len {
            return self.split_at(mid);
        }
        self.utf8_split_at(mid)
    }

    #[inline(always)]
    fn utf8_get(&self, from: usize, to: usize) -> Option<&str> {
        maybe_prev_char_bytes_end(self, from)
            .and_then(|from_checked| Some(from_checked..maybe_prev_char_bytes_end(self, to)?))
            .map(|range| unsafe { self.get_unchecked(range) })
    }

    #[inline(always)]
    fn utf8_get_from(&self, from: usize) -> Option<&str> {
        maybe_prev_char_bytes_end(self, from).map(|from_checked| unsafe { self.get_unchecked(from_checked..) })
    }

    #[inline(always)]
    fn utf8_get_to(&self, to: usize) -> Option<&str> {
        maybe_prev_char_bytes_end(self, to).map(|to_checked| unsafe { self.get_unchecked(..to_checked) })
    }

    #[inline(always)]
    fn utf8_unsafe_get(&self, from: usize, to: usize) -> &str {
        unsafe { self.get_unchecked(prev_char_bytes_end(self, from)..prev_char_bytes_end(self, to)) }
    }

    #[inline(always)]
    fn utf8_unsafe_get_from(&self, from: usize) -> &str {
        unsafe { self.get_unchecked(prev_char_bytes_end(self, from)..) }
    }

    #[inline(always)]
    fn utf8_unsafe_get_to(&self, to: usize) -> &str {
        unsafe { self.get_unchecked(..prev_char_bytes_end(self, to)) }
    }
}

impl UTF8Safe for String {
    #[inline(always)]
    fn truncate_width(&self, width: usize) -> (usize, &str) {
        self.as_str().truncate_width(width)
    }

    #[inline(always)]
    fn truncate_width_start(&self, width: usize) -> (usize, &str) {
        self.as_str().truncate_width_start(width)
    }

    #[inline(always)]
    fn truncate_if_wider(&self, width: usize) -> Result<&str, usize> {
        self.as_str().truncate_if_wider(width)
    }

    #[inline(always)]
    fn truncate_if_wider_start(&self, width: usize) -> Result<&str, usize> {
        self.as_str().truncate_if_wider_start(width)
    }

    #[inline(always)]
    fn width(&self) -> usize {
        UnicodeWidthStr::width(self.as_str())
    }

    #[inline(always)]
    fn width_at(&self, at: usize) -> usize {
        self.as_str().width_at(at)
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.chars().count()
    }

    #[inline(always)]
    fn utf16_len(&self) -> usize {
        self.as_str().utf16_len()
    }

    #[inline(always)]
    fn utf8_split_at(&self, mid: usize) -> (&str, &str) {
        self.as_str().utf8_split_at(mid)
    }

    #[inline(always)]
    fn utf8_cached_split_at(&self, mid: usize, utf8_len: usize) -> (&str, &str) {
        self.as_str().utf8_cached_split_at(mid, utf8_len)
    }

    #[inline(always)]
    fn utf8_get(&self, from: usize, to: usize) -> Option<&str> {
        self.as_str().utf8_get(from, to)
    }

    #[inline(always)]
    fn utf8_get_from(&self, from: usize) -> Option<&str> {
        self.as_str().utf8_get_from(from)
    }

    #[inline(always)]
    fn utf8_get_to(&self, to: usize) -> Option<&str> {
        self.as_str().utf8_get_to(to)
    }

    #[inline(always)]
    fn utf8_unsafe_get(&self, from: usize, to: usize) -> &str {
        self.as_str().utf8_unsafe_get(from, to)
    }

    #[inline(always)]
    fn utf8_unsafe_get_from(&self, from: usize) -> &str {
        self.as_str().utf8_unsafe_get_from(from)
    }

    #[inline(always)]
    fn utf8_unsafe_get_to(&self, to: usize) -> &str {
        self.as_str().utf8_unsafe_get_to(to)
    }
}

impl UTF8SafeStringExt for String {
    #[inline(always)]
    fn utf8_insert(&mut self, idx: usize, ch: char) {
        self.insert(prev_char_bytes_end(self, idx), ch);
    }

    #[inline(always)]
    fn utf8_insert_str(&mut self, idx: usize, string: &str) {
        self.insert_str(prev_char_bytes_end(self, idx), string)
    }

    #[inline(always)]
    fn utf8_remove(&mut self, idx: usize) -> char {
        self.remove(prev_char_bytes_end(self, idx))
    }

    #[inline(always)]
    fn utf8_replace_range(&mut self, range: Range<usize>, text: &str) {
        let start = prev_char_bytes_end(self, range.start);
        let end = prev_char_bytes_end(self, range.end);
        self.replace_range(start..end, text);
    }

    #[inline(always)]
    fn utf8_replace_from(&mut self, from: usize, string: &str) {
        self.truncate(prev_char_bytes_end(self, from));
        self.push_str(string);
    }

    #[inline(always)]
    fn utf8_replace_till(&mut self, to: usize, string: &str) {
        self.replace_range(..prev_char_bytes_end(self, to), string);
    }

    #[inline(always)]
    fn utf8_split_off(&mut self, at: usize) -> Self {
        self.split_off(prev_char_bytes_end(self, at))
    }
}

#[inline(always)]
fn prev_char_bytes_end(text: &str, idx: usize) -> usize {
    if idx == 0 {
        return idx;
    }
    if let Some((byte_idx, ch)) = text.char_indices().nth(idx - 1) {
        return byte_idx + ch.len_utf8();
    }
    panic!("Index out of bound! Max len {} with index {}", text.char_len(), idx)
}

#[inline(always)]
fn maybe_prev_char_bytes_end(text: &str, idx: usize) -> Option<usize> {
    if idx == 0 {
        return Some(idx);
    }
    text.char_indices().nth(idx - 1).map(|(byte_idx, ch)| byte_idx + ch.len_utf8())
}

#[cfg(test)]
mod tests;

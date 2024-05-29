use std::{ops::Range, usize};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Trait allowing UTF8 safe operations on str/String
#[allow(dead_code)]
pub trait UTF8Safe {
    /// returns str that will fit into width of columns, removing chars at the end that will not fit
    fn truncate_width<'a>(&'a self, width: usize) -> &'a str;
    /// returns str that will fit into width of columns, removing chars from the start that will not fit
    fn truncate_width_start<'a>(&'a self, width: usize) -> &'a str;
    /// returns display len of the str
    fn width(&self) -> usize;
    /// calcs the width at position
    fn width_at(&self, at: usize) -> usize;
    /// returns utf8 chars len
    fn utf8_len(&self) -> usize;
    /// return utf8 split at char idx
    fn utf8_split_at<'a>(&'a self, mid: usize) -> (&'a str, &'a str);
    /// splits utf8 if not ascii (needs precalculated utf8 len)
    fn utf8_cached_split_at<'a>(&'a self, mid: usize, utf8_len: usize) -> (&'a str, &'a str);
    /// limits str within range based on utf char locations
    fn utf8_unsafe_get<'a>(&'a self, from: usize, to: usize) -> &'a str;
    /// removes "from" chars from the begining of the string
    fn utf8_unsafe_get_from<'a>(&'a self, from: usize) -> &'a str;
    /// limits str to char idx
    fn utf8_unsafe_get_till<'a>(&'a self, to: usize) -> &'a str;
    /// get checked utf8 slice
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> Option<&'a str>;
    /// get checked utf8 from
    fn utf8_get_from<'a>(&'a self, from: usize) -> Option<&'a str>;
    /// get checked utf8 to
    fn utf8_get_till<'a>(&'a self, to: usize) -> Option<&'a str>;
}

/// String specific extension
#[allow(dead_code)]
pub trait UTF8SafeStringExt {
    fn utf8_insert(&mut self, idx: usize, ch: char);
    fn utf8_insert_str(&mut self, idx: usize, string: &str);
    fn utf8_remove(&mut self, idx: usize) -> char;
    fn utf8_replace_range(&mut self, range: Range<usize>, string: &str);
    fn utf8_replace_till(&mut self, to: usize, string: &str);
    fn utf8_replace_from(&mut self, from: usize, string: &str);
}

impl UTF8Safe for str {
    #[inline]
    fn truncate_width<'a>(&'a self, mut width: usize) -> &'a str {
        let mut end = 0;
        for char in self.chars() {
            let char_width = UnicodeWidthChar::width(char).unwrap_or(0);
            if char_width > width {
                return unsafe { self.get_unchecked(..end) };
            };
            width -= char_width;
            end += char.len_utf8();
        }
        self
    }

    #[inline]
    fn truncate_width_start<'a>(&'a self, mut width: usize) -> &'a str {
        let mut start = 0;
        for char in self.chars().rev() {
            let char_width = UnicodeWidthChar::width(char).unwrap_or(0);
            if char_width > width {
                return unsafe { self.get_unchecked(self.len() - start..) };
            }
            width -= char_width;
            start += char.len_utf8();
        }
        self
    }

    #[inline]
    fn width(&self) -> usize {
        UnicodeWidthStr::width(self)
    }

    #[inline]
    fn width_at(&self, at: usize) -> usize {
        self.chars().take(at).fold(0, |l, r| l + UnicodeWidthChar::width(r).unwrap_or(0))
    }

    #[inline]
    fn utf8_len(&self) -> usize {
        self.chars().count()
    }

    #[inline]
    fn utf8_split_at<'a>(&'a self, mid: usize) -> (&'a str, &'a str) {
        self.split_at(prev_char_bytes_end(self, mid))
    }

    #[inline]
    fn utf8_cached_split_at<'a>(&'a self, mid: usize, utf8_len: usize) -> (&'a str, &'a str) {
        if self.len() == utf8_len {
            return self.split_at(mid);
        }
        self.utf8_split_at(mid)
    }

    #[inline]
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> Option<&'a str> {
        maybe_prev_char_bytes_end(self, from)
            .and_then(|from_checked| Some(from_checked..maybe_prev_char_bytes_end(self, to)?))
            .map(|range| unsafe { self.get_unchecked(range) })
    }

    #[inline]
    fn utf8_get_from<'a>(&'a self, from: usize) -> Option<&'a str> {
        maybe_prev_char_bytes_end(self, from).map(|from_checked| unsafe { self.get_unchecked(from_checked..) })
    }

    #[inline]
    fn utf8_get_till<'a>(&'a self, to: usize) -> Option<&'a str> {
        maybe_prev_char_bytes_end(self, to).map(|to_checked| unsafe { self.get_unchecked(..to_checked) })
    }

    #[inline]
    fn utf8_unsafe_get<'a>(&'a self, from: usize, to: usize) -> &'a str {
        unsafe { self.get_unchecked(prev_char_bytes_end(self, from)..prev_char_bytes_end(self, to)) }
    }

    #[inline]
    fn utf8_unsafe_get_from<'a>(&'a self, from: usize) -> &'a str {
        unsafe { self.get_unchecked(prev_char_bytes_end(self, from)..) }
    }

    #[inline]
    fn utf8_unsafe_get_till<'a>(&'a self, to: usize) -> &'a str {
        unsafe { self.get_unchecked(..prev_char_bytes_end(self, to)) }
    }
}

impl UTF8Safe for String {
    #[inline]
    fn truncate_width<'a>(&'a self, width: usize) -> &'a str {
        self.as_str().truncate_width(width)
    }

    #[inline]
    fn truncate_width_start<'a>(&'a self, width: usize) -> &'a str {
        self.as_str().truncate_width_start(width)
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
    fn utf8_len(&self) -> usize {
        self.chars().count()
    }

    #[inline(always)]
    fn utf8_split_at<'a>(&'a self, mid: usize) -> (&'a str, &'a str) {
        self.as_str().utf8_split_at(mid)
    }

    #[inline(always)]
    fn utf8_cached_split_at<'a>(&'a self, mid: usize, utf8_len: usize) -> (&'a str, &'a str) {
        self.as_str().utf8_cached_split_at(mid, utf8_len)
    }

    #[inline]
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> Option<&'a str> {
        self.as_str().utf8_get(from, to)
    }

    #[inline]
    fn utf8_get_from<'a>(&'a self, from: usize) -> Option<&'a str> {
        self.as_str().utf8_get_from(from)
    }

    #[inline]
    fn utf8_get_till<'a>(&'a self, to: usize) -> Option<&'a str> {
        self.as_str().utf8_get_till(to)
    }

    #[inline]
    fn utf8_unsafe_get<'a>(&'a self, from: usize, to: usize) -> &'a str {
        self.as_str().utf8_unsafe_get(from, to)
    }

    #[inline]
    fn utf8_unsafe_get_from<'a>(&'a self, from: usize) -> &'a str {
        self.as_str().utf8_unsafe_get_from(from)
    }

    #[inline(always)]
    fn utf8_unsafe_get_till<'a>(&'a self, to: usize) -> &'a str {
        self.as_str().utf8_unsafe_get_till(to)
    }
}

impl UTF8SafeStringExt for String {
    fn utf8_insert(&mut self, idx: usize, ch: char) {
        self.insert(prev_char_bytes_end(self, idx), ch);
    }

    fn utf8_insert_str(&mut self, idx: usize, string: &str) {
        self.insert_str(prev_char_bytes_end(self, idx), string)
    }

    fn utf8_remove(&mut self, idx: usize) -> char {
        self.remove(prev_char_bytes_end(&self, idx))
    }

    fn utf8_replace_range(&mut self, range: Range<usize>, text: &str) {
        let start = prev_char_bytes_end(self, range.start);
        let end = prev_char_bytes_end(self, range.end);
        self.replace_range(start..end, text);
    }

    fn utf8_replace_from(&mut self, from: usize, string: &str) {
        self.truncate(prev_char_bytes_end(self, from));
        self.push_str(string);
    }

    fn utf8_replace_till(&mut self, to: usize, string: &str) {
        self.replace_range(..prev_char_bytes_end(self, to), string);
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
    panic!("Index out of bound! Max len {} with index {}", text.utf8_len(), idx)
}

#[inline(always)]
fn maybe_prev_char_bytes_end(text: &str, idx: usize) -> Option<usize> {
    if idx == 0 {
        return Some(idx);
    }
    text.char_indices().nth(idx - 1).map(|(byte_idx, ch)| byte_idx + ch.len_utf8())
}

#[cfg(test)]
mod test {
    use super::{UTF8Safe, UTF8SafeStringExt};
    const TEXT: &str = "123🚀13";

    #[test]
    fn test_utf8_insert_str() {
        let mut s = String::new();
        s.utf8_insert_str(0, TEXT);
        assert!(&s == TEXT);
        s.utf8_insert_str(4, TEXT);
        assert!(&s == "123🚀123🚀1313");
    }

    #[test]
    fn test_utf8_insert() {
        let mut s = String::new();
        s.utf8_insert(0, '🚀');
        assert!(&s == "🚀");
        s.utf8_insert(1, '🚀');
        s.utf8_insert(2, 'r');
        assert!(&s == "🚀🚀r");
    }

    #[test]
    #[should_panic]
    fn test_truncate() {
        let mut s = String::from(TEXT);
        s.truncate(4);
    }

    #[test]
    fn test_truncate_utf8() {
        assert_eq!("123", TEXT.truncate_width(4));
        assert_eq!(3, TEXT.truncate_width(4).len());
        assert_eq!("123🚀", TEXT.truncate_width(5));
        assert_eq!(7, TEXT.truncate_width(5).len());
        assert_eq!(4, TEXT.truncate_width(5).chars().count());
        assert_eq!("🚀13", TEXT.truncate_width_start(4));
        assert_eq!("13", TEXT.truncate_width_start(3));
    }

    #[test]
    #[should_panic]
    fn test_split_std() {
        let _ = TEXT.split_at(4);
    }

    #[test]
    fn test_split_utf8() {
        assert_eq!(TEXT.split_at(3), TEXT.utf8_split_at(3));
        assert_eq!(("123🚀", "13"), TEXT.utf8_split_at(4));
    }

    /// example issue
    #[test]
    #[should_panic]
    fn test_replace_range() {
        let mut s = String::from(TEXT);
        s.replace_range(4.., ""); // in char boundry
    }

    #[test]
    fn test_utf8_replace_range() {
        let mut s = String::new();
        s.replace_range(0..0, "asd");
        assert!(&s == "asd");
        s.clear();
        s.utf8_replace_range(0..0, "🚀🚀");
        assert_eq!(&s, "🚀🚀");
        s.utf8_replace_range(1..2, "asd");
        assert_eq!(&s, "🚀asd");
    }

    #[test]
    #[should_panic]
    fn test_utf8_replace_range_panic() {
        let mut s = String::new();
        s.utf8_replace_range(0..1, "panic");
    }

    #[test]
    fn test_replace_from() {
        let mut s = String::from("text");
        s.utf8_replace_from(0, "123");
        assert!(&s == "123");
        s.clear();
        s.utf8_replace_from(0, "123");
        assert!(&s == "123");
    }

    #[test]
    fn test_replace_till() {
        let mut s = String::from("🚀🚀");
        s.utf8_replace_till(1, "asd");
        assert!(&s == "asd🚀");
        s.clear();
        s.utf8_replace_till(0, "🚀");
        assert_eq!(&s, "🚀");
    }

    #[test]
    fn test_utf8_replaces() {
        let mut s = String::from(TEXT);
        let mut std_s = s.clone();
        s.utf8_replace_from(4, "replace_with");
        std_s.replace_range(7.., "replace_with");
        assert_eq!(s, std_s);
    }

    #[test]
    fn test_utf8_str() {
        assert_eq!(TEXT.len(), 9);
        assert_eq!(TEXT.utf8_len(), 6);
        assert_eq!(TEXT.width(), 7);
    }

    /// represent issue solved by UTF8 traits
    #[test]
    #[should_panic]
    fn test_std_remove() {
        let mut s = String::from(TEXT);
        s.remove(4); // in char boundry
    }

    #[test]
    fn test_utf8_remove() {
        let mut s = String::from(TEXT);
        assert_eq!(s.len(), 9);
        assert_eq!(s.utf8_len(), 6);
        assert_eq!(s.width(), 7);
        assert_eq!(s.utf8_remove(4), '1');
        assert_eq!(s.utf8_remove(3), '🚀');
        assert_eq!(&s, "1233");
    }

    #[test]
    fn test_utf8_get() {
        assert_eq!(TEXT.utf8_get(0, 10), None);
        assert_eq!(TEXT.utf8_get(0, 3), Some("123"));
        assert_eq!(TEXT.utf8_get(3, 4), Some("🚀"));
    }

    #[test]
    fn test_utf8_get_from() {
        assert_eq!(TEXT.utf8_get_from(10), None);
        assert_eq!(TEXT.utf8_get_from(0), Some(TEXT));
        assert_eq!(TEXT.utf8_get_from(3), Some("🚀13"));
        assert_eq!(TEXT.utf8_get_from(4), Some("13"));
    }

    #[test]
    fn test_utf8_get_till() {
        assert_eq!(TEXT.utf8_get_till(10), None);
        assert_eq!(TEXT.utf8_get_till(3), Some("123"));
        assert_eq!(TEXT.utf8_get_till(4), Some("123🚀"));
    }

    #[test]
    #[should_panic]
    fn test_utf8_remove_panic() {
        let mut s = String::new();
        s.utf8_remove(0);
    }
}

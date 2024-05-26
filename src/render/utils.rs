use std::ops::Range;

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
    /// limits str to char idx
    fn utf8_get_till<'a>(&'a self, to: usize) -> &'a str;
    /// limits str within range based on utf char locations
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> &'a str;
    /// removes "from" chars from the begining of the string
    fn utf8_get_unbound<'a>(&'a self, from: usize) -> &'a str;
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
        self.split_at(derive_byte_idx(self, mid))
    }

    #[inline]
    fn utf8_cached_split_at<'a>(&'a self, mid: usize, utf8_len: usize) -> (&'a str, &'a str) {
        if self.len() == utf8_len {
            return self.split_at(mid);
        }
        self.utf8_split_at(mid)
    }

    #[inline]
    fn utf8_get_till<'a>(&'a self, to: usize) -> &'a str {
        unsafe { self.get_unchecked(..find_bytes_end_after(self, to)) }
    }

    #[inline]
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> &'a str {
        unsafe { self.get_unchecked(find_bytes_end_after(self, from)..find_bytes_end_after(self, to)) }
    }

    #[inline]
    fn utf8_get_unbound<'a>(&'a self, from: usize) -> &'a str {
        unsafe { self.get_unchecked(find_bytes_end_after(self, from)..) }
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

    #[inline(always)]
    fn utf8_get_till<'a>(&'a self, to: usize) -> &'a str {
        self.as_str().utf8_get_till(to)
    }

    #[inline]
    fn utf8_get<'a>(&'a self, from: usize, to: usize) -> &'a str {
        self.as_str().utf8_get(from, to)
    }

    #[inline]
    fn utf8_get_unbound<'a>(&'a self, from: usize) -> &'a str {
        self.as_str().utf8_get_unbound(from)
    }
}

impl UTF8SafeStringExt for String {
    fn utf8_insert(&mut self, idx: usize, ch: char) {
        self.insert(find_bytes_end_after(self, idx), ch);
    }

    fn utf8_insert_str(&mut self, idx: usize, string: &str) {
        self.insert_str(find_bytes_end_after(self, idx), string)
    }

    fn utf8_remove(&mut self, idx: usize) -> char {
        self.remove(derive_byte_idx(&self, idx))
    }

    fn utf8_replace_range(&mut self, range: Range<usize>, text: &str) {
        let start = derive_byte_idx(self, range.start);
        let end = derive_byte_idx(self, range.end);
        self.replace_range(start..end, text);
    }

    fn utf8_replace_from(&mut self, from: usize, string: &str) {
        self.truncate(derive_byte_idx(self, from));
        self.push_str(string);
    }

    fn utf8_replace_till(&mut self, to: usize, string: &str) {
        self.replace_range(..derive_byte_idx(self, to), string);
    }
}

#[inline(always)]
fn derive_byte_idx(text: &str, idx: usize) -> usize {
    if let Some(byte_idx) = text.char_indices().nth(idx).map(|(byte_idx, ..)| byte_idx) {
        return byte_idx;
    }
    panic!("Index out of bound! Max len {} with index {}", text.utf8_len(), idx);
}

#[inline(always)]
fn find_bytes_end_after(text: &str, after: usize) -> usize {
    text.char_indices().take(after).last().map(|(byte_idx, ch)| byte_idx + ch.len_utf8()).unwrap_or(0)
}

#[cfg(test)]
mod test {
    use super::{UTF8Safe, UTF8SafeStringExt};
    const TEXT: &str = "123🚀13";
    const ASCII_TEXT: &str = "123abc";

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

    /// ensures on ascii behavior is the same
    #[test]
    fn test_utf8_to_std_cmp() {
        let mut s = String::from(ASCII_TEXT);
        let mut std_s = s.clone();
        s.utf8_replace_range(0..1, "3");
        std_s.replace_range(0..1, "3");
        assert_eq!(s, std_s);
        s.utf8_replace_from(4, "bumba");
        std_s.replace_range(4.., "bumba");
        assert_eq!(s, std_s);
        s.utf8_replace_till(2, "");
        std_s.replace_range(..2, "");
        assert_eq!(s, std_s);
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
}

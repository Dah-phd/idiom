use idiom_tui::UTFSafeStringExt;

pub struct Encoding {
    pub char_len: fn(char) -> usize,
    pub encode_position: fn(usize, &str) -> usize,
    pub insert_char_with_idx: fn(&mut String, usize, char) -> usize,
    pub remove_char_with_idx: fn(&mut String, usize) -> (usize, char),
}

impl Encoding {
    pub fn utf32() -> Self {
        Self {
            char_len: char_lsp_pos,
            encode_position: encode_pos_utf32,
            insert_char_with_idx: utf32_insert_char,
            remove_char_with_idx: utf32_remove_char,
        }
    }

    pub fn utf16() -> Self {
        Self {
            char_len: char::len_utf16,
            encode_position: encode_pos_utf16,
            insert_char_with_idx: UTFSafeStringExt::insert_at_char_with_utf16_idx,
            remove_char_with_idx: UTFSafeStringExt::remove_at_char_with_utf16_idx,
        }
    }

    pub fn utf8() -> Self {
        Self {
            char_len: char::len_utf8,
            encode_position: encode_pos_utf8,
            insert_char_with_idx: UTFSafeStringExt::insert_at_char_with_utf8_idx,
            remove_char_with_idx: UTFSafeStringExt::remove_at_char_with_utf8_idx,
        }
    }
}

#[inline]
fn utf32_insert_char(text: &mut String, idx: usize, ch: char) -> usize {
    text.insert_at_char(idx, ch);
    idx
}

#[inline]
fn utf32_remove_char(text: &mut String, idx: usize) -> (usize, char) {
    (idx, text.remove_at_char(idx))
}

#[inline]
fn encode_pos_utf8(char_idx: usize, from_str: &str) -> usize {
    from_str.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf8())
}

#[inline]
fn encode_pos_utf16(char_idx: usize, from_str: &str) -> usize {
    from_str.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf16())
}

#[inline]
fn encode_pos_utf32(char_idx: usize, _: &str) -> usize {
    char_idx
}

#[inline]
fn char_lsp_pos(_: char) -> usize {
    1
}

mod context;
mod status;
pub use status::{Reduction, RenderStatus};

use crate::editor::syntax::{tokens::TokenLine, DiagnosticInfo, DiagnosticLine, Encoding, Lang, Token};
pub use context::LineContext;
use idiom_tui::{utils::UTFSafeStringExt, UTFSafe};
use std::{
    fmt::Display,
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    path::Path,
};

/// Used to represent code, has simpler wrapping as cpde lines shoud be shorter than 120 chars in most cases
#[derive(Default)]
pub struct EditorLine {
    content: String,
    // keeps trach of utf8 char len
    char_len: usize,
    // syntax
    tokens: TokenLine,
    diagnostics: Option<DiagnosticLine>,
    // used for caching - 0 is reseved for file tabs and can be used to reset line
    pub cached: RenderStatus,
}

impl EditorLine {
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.content.as_str()
    }

    #[inline(always)]
    pub fn content(&self) -> &String {
        &self.content
    }

    #[inline]
    pub fn parse_lines<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, String> {
        Ok(std::fs::read_to_string(path)
            .map_err(|err| err.to_string())?
            .split('\n')
            .map(|line| EditorLine::new(line.to_owned()))
            .collect())
    }

    #[inline]
    pub fn is_simple(&self) -> bool {
        self.content.len() == self.char_len
    }

    #[inline]
    pub fn unwrap(self) -> String {
        self.content
    }

    #[inline]
    pub fn get_char(&self, pos: usize) -> Option<char> {
        if self.is_simple() {
            if self.content.len() <= pos {
                return None;
            }
            Some(self.content.as_bytes()[pos] as char)
        } else {
            self.chars().nth(pos)
        }
    }

    #[inline]
    pub fn get(&self, from: usize, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..to);
        }
        self.content.get_char_range(from, to)
    }

    #[inline]
    pub fn get_from(&self, from: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..);
        }
        self.content.get_from_char(from)
    }

    #[inline]
    pub fn get_to(&self, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(..to);
        }
        self.content.get_to_char(to)
    }

    #[inline]
    pub fn replace_till(&mut self, to: usize, string: &str) {
        self.cached.reset();
        if self.content.len() == self.char_len {
            self.char_len += string.char_len();
            self.char_len -= to;
            return self.content.replace_range(..to, string);
        }
        self.char_len += string.char_len();
        self.char_len -= to;
        self.content.replace_till_char(to, string)
    }

    #[inline]
    pub fn replace_from(&mut self, from: usize, string: &str) {
        self.cached.reset();
        if self.content.len() == self.char_len {
            self.char_len = from + string.char_len();
            self.content.truncate(from);
            return self.content.push_str(string);
        }
        self.char_len = from + string.char_len();
        self.content.replace_from_char(from, string)
    }

    #[inline]
    pub fn replace_range(&mut self, range: Range<usize>, string: &str) {
        self.cached.reset();
        if self.char_len == self.content.len() {
            self.char_len += string.char_len();
            self.char_len -= range.len();
            return self.content.replace_range(range, string);
        }
        self.char_len += string.char_len();
        self.char_len -= range.len();
        self.content.replace_char_range(range, string)
    }

    #[inline]
    pub fn starts_with(&self, pat: &str) -> bool {
        self.content.starts_with(pat)
    }

    #[inline]
    pub fn ends_with(&self, pat: &str) -> bool {
        self.content.ends_with(pat)
    }

    #[inline]
    pub fn find(&self, pat: &str) -> Option<usize> {
        self.content.find(pat)
    }

    // no need to handle non ascii inserts because - triggers from keyboard
    #[inline]
    pub fn insert_simple(&mut self, idx: usize, ch: char, encoding: &Encoding) {
        self.cached.reset();
        self.char_len += 1;
        if self.char_len == self.content.len() {
            // base update on delta start
            self.tokens.increment_before_encoded(idx);
            self.content.insert(idx, ch);
        } else {
            let encoded_idx = (encoding.insert_char_with_idx)(&mut self.content, idx, ch);
            self.tokens.increment_before_encoded(encoded_idx);
        }
    }

    // no need to handle non ascii inserts because - triggers from keyboard
    #[inline]
    pub fn push_simple(&mut self, ch: char) {
        self.cached.reset();
        self.tokens.increment_before_encoded(self.char_len);
        self.char_len += 1;
        self.content.push(ch);
    }

    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        self.cached.reset();
        if self.char_len == self.content.len() {
            self.char_len += string.char_len();
            self.content.insert_str(idx, string);
        } else {
            self.char_len += string.char_len();
            self.content.insert_str_at_char(idx, string);
        }
    }

    #[inline]
    pub fn push_str(&mut self, string: &str) {
        self.cached.reset();
        self.char_len += string.char_len();
        self.content.push_str(string);
    }

    #[inline]
    pub fn push_line(&mut self, line: Self) {
        self.cached.reset();
        self.char_len += line.char_len;
        self.content.push_str(&line.content)
    }

    #[inline]
    pub fn insert_content_to_buffer(&self, idx: usize, buffer: &mut String) {
        buffer.insert_str(idx, &self.content)
    }

    #[inline]
    pub fn push_content_to_buffer(&self, buffer: &mut String) {
        buffer.push_str(&self.content)
    }

    #[inline]
    pub fn remove(&mut self, idx: usize, encoding: &Encoding) -> char {
        self.cached.reset();
        if self.content.len() == self.char_len {
            self.tokens.decrement_at_encoded(idx);
            self.char_len -= 1;
            self.content.remove(idx)
        } else {
            self.char_len -= 1;
            let (encoded_idx, ch) = (encoding.remove_char_with_idx)(&mut self.content, idx);
            for _ in 0..(encoding.char_len)(ch) {
                self.tokens.decrement_at_encoded(encoded_idx);
            }
            ch
        }
    }

    #[inline]
    pub fn trim_start(&self) -> &str {
        self.content.trim_start()
    }

    #[inline]
    pub fn trim_start_counted(&self) -> (usize, &str) {
        let count = self.content.chars().take_while(|c| c.is_whitespace()).count();
        (count, &self.content[count..])
    }

    #[inline]
    pub fn trim_end(&self) -> &str {
        self.content.trim_end()
    }

    #[inline]
    pub fn chars(&self) -> std::str::Chars<'_> {
        self.content.chars()
    }

    #[inline]
    pub fn char_indices(&self) -> std::str::CharIndices<'_> {
        self.content.char_indices()
    }

    #[inline]
    pub fn match_indices<'a>(&self, pat: &'a str) -> std::str::MatchIndices<'_, &'a str> {
        self.content.match_indices(pat)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.content.clear();
        self.char_len = 0;
        self.cached.reset();
    }

    #[inline]
    pub fn split_off(&mut self, at: usize) -> Self {
        self.cached.reset();
        if at == 0 {
            return std::mem::take(self);
        }
        if self.content.len() == self.char_len {
            let content = self.content.split_off(at);
            if !content.is_empty() {
                self.char_len = self.content.len();
                self.tokens.clear();
            }
            Self { char_len: content.len(), content, ..Default::default() }
        } else {
            let content = self.content.split_off_at_char(at);
            if !content.is_empty() {
                self.char_len = self.content.char_len();
                self.tokens.clear();
            }
            Self { char_len: content.char_len(), content, ..Default::default() }
        }
    }

    #[inline]
    pub fn split_at(&self, mid: usize) -> (&str, &str) {
        if self.content.len() == self.char_len {
            self.content.split_at(mid)
        } else {
            self.content.split_at_char(mid)
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.content.len()
    }

    #[inline(always)]
    pub fn char_len(&self) -> usize {
        self.char_len
    }

    #[inline(always)]
    pub fn is_empry(&self) -> bool {
        self.content.is_empty()
    }

    #[inline]
    pub fn unsafe_utf8_idx_at(&self, char_idx: usize) -> usize {
        if char_idx > self.char_len {
            panic!("Index out of bounds! Index {} where max is {}", char_idx, self.char_len);
        }
        if self.char_len == self.content.len() {
            return char_idx;
        };
        self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf8())
    }

    #[inline]
    pub fn unsafe_utf16_idx_at(&self, char_idx: usize) -> usize {
        if char_idx > self.char_len {
            panic!("Index out of bounds! Index {} where max is {}", char_idx, self.char_len);
        }
        if self.is_simple() {
            return char_idx;
        }
        self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf16())
    }

    #[inline]
    pub fn unsafe_utf8_to_idx(&self, utf8_idx: usize) -> usize {
        for (idx, (byte_idx, ..)) in self.content.char_indices().enumerate() {
            if byte_idx == utf8_idx {
                return idx;
            }
        }
        panic!("Index out of bounds! Index {} where max is {}", utf8_idx, self.content.len());
    }

    #[inline]
    pub fn unsafe_utf16_to_idx(&self, utf16_idx: usize) -> usize {
        let mut sum = 0;
        for (pos, ch) in self.content.chars().enumerate() {
            if sum == utf16_idx {
                return pos;
            }
            sum += ch.len_utf16();
        }
        panic!("Index out of bounds! Index {utf16_idx} where max is {sum}")
    }

    #[inline]
    pub fn utf16_len(&self) -> usize {
        self.content.chars().fold(0, |sum, ch| sum + ch.len_utf16())
    }

    pub fn new(content: String) -> Self {
        Self { char_len: content.char_len(), content, ..Default::default() }
    }

    pub fn empty() -> Self {
        Self { ..Default::default() }
    }

    #[inline]
    pub fn push_token(&mut self, token: Token) {
        self.cached.reset();
        self.tokens.push(token);
    }

    #[inline]
    pub fn replace_tokens(&mut self, tokens: TokenLine) {
        self.cached.reset();
        self.tokens = tokens;
        if let Some(diagnostics) = self.diagnostics.as_ref() {
            for diagnostic in diagnostics.iter() {
                self.tokens.mark_diagnostics(diagnostic);
            }
        };
    }

    #[inline]
    pub fn set_diagnostics(&mut self, diagnostics: DiagnosticLine) {
        self.cached.reset();
        for diagnostic in diagnostics.iter() {
            self.tokens.mark_diagnostics(diagnostic);
        }
        self.diagnostics.replace(diagnostics);
    }

    /// does not mark the line for render may cause render artefacts
    /// also the tokesn are not verified - may cause panics
    /// proceed with caution
    #[inline]
    pub fn tokens_mut_unchecked(&mut self) -> &mut TokenLine {
        &mut self.tokens
    }

    #[inline]
    pub fn tokens_mut(&mut self) -> &mut TokenLine {
        self.cached.reset();
        &mut self.tokens
    }

    #[inline]
    pub fn tokens(&self) -> &TokenLine {
        &self.tokens
    }

    #[inline]
    pub fn iter_tokens(&self) -> impl Iterator<Item = &Token> {
        self.tokens.iter()
    }

    #[inline]
    pub fn diagnostics(&self) -> &Option<DiagnosticLine> {
        &self.diagnostics
    }

    #[inline]
    pub fn diagnostic_info(&self, lang: &Lang) -> Option<DiagnosticInfo> {
        self.diagnostics.as_ref().map(|d| d.collect_info(lang))
    }

    #[inline]
    pub fn drop_diagnostics(&mut self) {
        if self.diagnostics.take().is_some() {
            self.tokens.drop_diagnostics();
            self.cached.reset();
        };
    }

    pub fn generate_skipped_chars_simple(&mut self, cursor_idx: usize, line_width: usize) -> (usize, Reduction) {
        self.cached.generate_skipped_chars_simple(cursor_idx, line_width)
    }

    pub fn generate_skipped_chars_complex(&mut self, cursor_idx: usize, line_width: usize) -> usize {
        self.cached.generate_skipped_chars_complex(self.content.as_str(), self.char_len(), cursor_idx, line_width)
    }
}

impl Display for EditorLine {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.content.fmt(f)
    }
}

impl From<String> for EditorLine {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl From<&str> for EditorLine {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}

impl Index<Range<usize>> for EditorLine {
    type Output = str;
    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        if self.char_len == self.content.len() {
            &self.content[index]
        } else {
            self.content.unchecked_get_char_range(index.start, index.end)
        }
    }
}

impl Index<RangeTo<usize>> for EditorLine {
    type Output = str;
    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        if self.char_len == self.content.len() {
            &self.content[index]
        } else {
            self.content.unchecked_get_to_char(index.end)
        }
    }
}

impl Index<RangeFrom<usize>> for EditorLine {
    type Output = str;
    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        if self.char_len == self.content.len() {
            &self.content[index]
        } else {
            self.content.unchecked_get_from_char(index.start)
        }
    }
}

impl Index<RangeFull> for EditorLine {
    type Output = str;
    fn index(&self, _: RangeFull) -> &Self::Output {
        &self.content
    }
}

impl From<EditorLine> for String {
    fn from(val: EditorLine) -> Self {
        val.content
    }
}

#[cfg(test)]
mod tests;

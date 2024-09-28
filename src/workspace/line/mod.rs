mod context;
// mod render;
mod status;
use status::RenderStatus;

use crate::{
    render::{utils::UTF8SafeStringExt, UTF8Safe},
    syntax::{tokens::TokenLine, DiagnosticLine, Lang, Token},
    // workspace::line::EditorLine,
};
pub use context::LineContext;
use std::{
    fmt::Display,
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    path::Path,
};

/// Used to represent code, has simpler wrapping as cpde lines shoud be shorter than 120 chars in most cases
#[derive(Default)]
pub struct EditorLine {
    pub content: String,
    // keeps trach of utf8 char len
    pub char_len: usize,
    // syntax
    pub tokens: TokenLine,
    pub diagnostics: Option<DiagnosticLine>,
    // used for caching - 0 is reseved for file tabs and can be used to reset line
    pub cached: RenderStatus,
}

impl EditorLine {
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
    pub fn get(&self, from: usize, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..to);
        }
        self.content.utf8_get(from, to)
    }

    #[inline]
    pub fn get_from(&self, from: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..);
        }
        self.content.utf8_get_from(from)
    }

    #[inline]
    pub fn get_to(&self, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(..to);
        }
        self.content.utf8_get_to(to)
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
        self.content.utf8_replace_till(to, string)
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
        self.content.utf8_replace_from(from, string)
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
        self.content.utf8_replace_range(range, string)
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

    #[inline]
    pub fn insert(&mut self, idx: usize, ch: char) {
        self.cached.reset();
        self.tokens.increment_at(idx);
        if self.char_len == self.content.len() {
            // base update on delta start
            self.char_len += 1;
            self.content.insert(idx, ch);
        } else {
            self.char_len += 1;
            self.content.utf8_insert(idx, ch);
        }
    }

    #[inline]
    pub fn push(&mut self, ch: char) {
        self.cached.reset();
        if self.char_len == self.tokens.char_len() {
            self.tokens.increment_end();
        }
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
            self.content.utf8_insert_str(idx, string);
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
    pub fn remove(&mut self, idx: usize) -> char {
        self.cached.reset();
        self.tokens.decrement_at(idx);
        if self.content.len() == self.char_len {
            self.char_len -= 1;
            return self.content.remove(idx);
        }
        self.char_len -= 1;
        self.content.utf8_remove(idx)
    }

    #[inline]
    pub fn trim_start(&self) -> &str {
        self.content.trim_start()
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
    pub fn match_indices<'a>(&self, pat: &'a str) -> std::str::MatchIndices<&'a str> {
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
        if self.content.len() == self.char_len {
            let content = self.content.split_off(at);
            if !content.is_empty() {
                self.char_len = self.content.len();
                self.tokens.clear();
            }
            return Self {
                char_len: content.len(),
                content,
                diagnostics: self.diagnostics.take(),
                ..Default::default()
            };
        }
        let content = self.content.utf8_split_off(at);
        if !content.is_empty() {
            self.char_len = self.content.char_len();
            self.tokens.clear();
        }
        Self { char_len: content.char_len(), content, diagnostics: self.diagnostics.take(), ..Default::default() }
    }

    #[inline]
    pub fn split_at(&self, mid: usize) -> (&str, &str) {
        if self.content.len() == self.char_len {
            self.content.split_at(mid)
        } else {
            self.content.utf8_split_at(mid)
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.content.len()
    }

    #[inline(always)]
    pub fn char_len(&self) -> usize {
        self.char_len
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
        panic!("Index out of bounds! Index {} where max is {}", utf16_idx, sum)
    }

    #[inline]
    pub fn utf16_len(&self) -> usize {
        self.content.chars().fold(0, |sum, ch| sum + ch.len_utf16())
    }
}

impl EditorLine {
    pub fn new(content: String) -> Self {
        Self { char_len: content.char_len(), content, ..Default::default() }
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
            for diagnostic in diagnostics.data.iter() {
                self.tokens.mark_diagnostics(diagnostic);
            }
        };
    }

    #[inline]
    pub fn set_diagnostics(&mut self, diagnostics: DiagnosticLine) {
        self.cached.reset();
        for diagnostic in diagnostics.data.iter() {
            self.tokens.mark_diagnostics(diagnostic);
        }
        self.diagnostics.replace(diagnostics);
    }

    #[inline(always)]
    pub fn tokens_mut(&mut self) -> &mut TokenLine {
        self.clear_cache();
        &mut self.tokens
    }

    #[inline]
    pub fn iter_tokens(&self) -> impl Iterator<Item = &Token> {
        self.tokens.iter()
    }

    #[inline]
    pub fn diagnostic_info(&self, lang: &Lang) -> Option<crate::syntax::DiagnosticInfo> {
        self.diagnostics.as_ref().map(|d| d.collect_info(lang))
    }

    #[inline]
    pub fn drop_diagnostics(&mut self) {
        if self.diagnostics.take().is_some() {
            self.tokens.drop_diagnostics();
            self.cached.reset();
        };
    }

    #[inline]
    pub fn clear_cache(&mut self) {
        self.cached.reset();
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

impl From<&'static str> for EditorLine {
    fn from(value: &'static str) -> Self {
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
            self.content.utf8_unsafe_get(index.start, index.end)
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
            self.content.utf8_unsafe_get_to(index.end)
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
            self.content.utf8_unsafe_get_from(index.start)
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

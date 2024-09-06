mod code;

pub use code::{CodeLine, CodeLineContext};
use std::{
    fmt::Display,
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    path::Path,
    str::{CharIndices, Chars, MatchIndices},
};

/// The trait can be used in future to add rope version for non code text
pub trait EditorLine:
    Into<String>
    + Default
    + Sized
    + Index<Range<usize>, Output = str>
    + Index<RangeTo<usize>, Output = str>
    + Index<RangeFrom<usize>, Output = str>
    + Index<RangeFull, Output = str>
    + From<String>
    + From<&'static str>
    + Display
{
    type Context<'a>;
    type Error;

    /// init
    fn parse_lines<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Self::Error>;

    /// assumption is that control chars will not be present in file -> confirms utf8 idx is always 1 byte
    fn is_simple(&self) -> bool;
    fn insert(&mut self, idx: usize, ch: char);
    fn push(&mut self, ch: char);
    fn insert_str(&mut self, idx: usize, string: &str);
    fn push_str(&mut self, string: &str);
    fn push_line(&mut self, line: Self);

    /// UTF ENOCODING
    fn len(&self) -> usize;
    fn char_len(&self) -> usize;
    fn utf16_len(&self) -> usize;
    fn get(&self, from: usize, to: usize) -> Option<&str>;
    fn get_from(&self, from: usize) -> Option<&str>;
    fn get_to(&self, to: usize) -> Option<&str>;
    /// panics if out of bounds
    fn unsafe_utf8_idx_at(&self, char_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf16_idx_at(&self, char_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf8_to_idx(&self, utf8_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf16_to_idx(&self, utf16_idx: usize) -> usize;
    fn replace_till(&mut self, to: usize, string: &str);
    fn replace_from(&mut self, from: usize, string: &str);
    fn replace_range(&mut self, range: Range<usize>, string: &str);
    fn split_off(&mut self, at: usize) -> Self;
    fn split_at(&self, mid: usize) -> (&str, &str);
    fn remove(&mut self, idx: usize) -> char;
    fn trim_start(&self) -> &str;
    fn trim_end(&self) -> &str;
    fn chars(&self) -> Chars<'_>;
    fn char_indices(&self) -> CharIndices<'_>;
    fn match_indices<'a>(&self, pat: &'a str) -> MatchIndices<&'a str>;
    fn starts_with(&self, pat: &str) -> bool;
    fn ends_with(&self, pat: &str) -> bool;
    fn find(&self, pat: &str) -> Option<usize>;
    fn push_content_to_buffer(&self, buffer: &mut String);
    fn insert_content_to_buffer(&self, at: usize, buffer: &mut String);
    fn clear(&mut self);
    fn unwrap(self) -> String;
}

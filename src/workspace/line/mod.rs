mod code;
use crate::{
    render::{backend::Backend, layout::Line as LineInfo},
    syntax::{DiagnosticLine, Lexer, Token},
};
pub use code::CodeLine;
use std::{
    fmt::Display,
    ops::{Index, Range, RangeBounds, RangeFrom, RangeFull, RangeTo},
    slice::SliceIndex,
};

pub trait Line:
    Into<String>
    + Default
    + Sized
    + Index<Range<usize>, Output = str>
    + Index<RangeTo<usize>, Output = str>
    + Index<RangeFrom<usize>, Output = str>
    + Index<RangeFull, Output = str>
    + From<String>
    + Display
{
    fn insert(&mut self, idx: usize, ch: char);
    fn as_str(&self) -> &str;
    fn push(&mut self, ch: char);
    fn insert_str(&mut self, idx: usize, string: &str);
    fn push_str(&mut self, string: &str);
    fn len(&self) -> usize;
    fn replace_range(&mut self, range: impl RangeBounds<usize>, string: &str);
    fn string(&self) -> &String;
    fn string_mut(&mut self) -> &mut String;
    fn split_off(&mut self, at: usize) -> String;
    fn split_at(&self, mid: usize) -> (&str, &str);
    fn remove(&mut self, idx: usize) -> char;
    fn clear(&mut self);
    fn unwrap(self) -> String;
    fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&I::Output>;
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine);
    fn drop_diagnostics(&mut self);
    fn push_token(&mut self, token: Token);
    fn replace_tokens(&mut self, tokens: Vec<Token>);
    fn wrapped_render(
        &mut self,
        idx: usize,
        line: LineInfo,
        limit: usize,
        lexer: &mut Lexer,
        writer: &mut Backend,
    ) -> std::io::Result<usize>;
    fn render(&mut self, idx: usize, line: LineInfo, lexer: &mut Lexer, writer: &mut Backend) -> std::io::Result<()>;
    fn fast_render(
        &mut self,
        idx: usize,
        line: LineInfo,
        lexer: &mut Lexer,
        writer: &mut Backend,
    ) -> std::io::Result<()>;
    unsafe fn get_unchecked<I: SliceIndex<str>>(&self, i: I) -> &I::Output;
}

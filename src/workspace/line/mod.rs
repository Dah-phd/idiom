mod code;
use crate::{
    global_state::GlobalState,
    render::{
        backend::Backend,
        layout::{Line, RectIter},
    },
    syntax::{DiagnosticInfo, DiagnosticLine, Lang, Lexer, Token},
};
pub use code::{CodeLine, CodeLineContext};
use std::{
    fmt::Display,
    io::Result,
    ops::{Index, Range, RangeBounds, RangeFrom, RangeFull, RangeTo},
    slice::SliceIndex,
    str::{CharIndices, Chars, MatchIndices},
};

type LineWidth = usize;

pub trait EditorLine:
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
    fn push(&mut self, ch: char);
    fn insert_str(&mut self, idx: usize, string: &str);
    fn push_str(&mut self, string: &str);
    fn push_line(&mut self, line: Self);
    fn len(&self) -> usize;
    fn replace_range(&mut self, range: impl RangeBounds<usize>, string: &str);
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
    fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&I::Output>;
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine);
    fn diagnostic_info(&self, lang: &Lang) -> Option<DiagnosticInfo>;
    fn drop_diagnostics(&mut self);
    fn push_token(&mut self, token: Token);
    fn replace_tokens(&mut self, tokens: Vec<Token>);
    fn wrapped_render(&mut self, ctx: &mut impl Context, lines: &mut RectIter, writer: &mut Backend) -> Result<()>;
    fn render(&mut self, ctx: &mut impl Context, line: Line, writer: &mut Backend) -> Result<()>;
    fn fast_render(&mut self, ctx: &mut impl Context, line: Line, writer: &mut Backend) -> Result<()>;
    unsafe fn get_unchecked<I: SliceIndex<str>>(&self, i: I) -> &I::Output;
}

pub trait Context {
    fn setup_line(&mut self, line: Line, backend: &mut Backend) -> Result<LineWidth>;
    fn get_select(&mut self) -> Option<Range<usize>>;
    fn lexer(&self) -> &Lexer;
    fn count_skipped_to_cursor(&mut self, wrap_len: usize, remaining_lines: usize) -> usize;
    fn render_cursor(self, gs: &mut GlobalState) -> Result<()>;
}

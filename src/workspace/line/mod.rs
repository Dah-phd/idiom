mod code;
mod render;
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
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    str::{CharIndices, Chars, MatchIndices},
};

type LineWidth = usize;
type Select = Range<usize>;

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
    /// assumption is that control chars will not be present in file
    fn is_ascii(&self) -> bool;
    fn insert(&mut self, idx: usize, ch: char);
    fn push(&mut self, ch: char);
    fn insert_str(&mut self, idx: usize, string: &str);
    fn push_str(&mut self, string: &str);
    fn push_line(&mut self, line: Self);
    fn len(&self) -> usize;
    fn utf16_len(&self) -> usize;
    /// panics if out of bounds
    fn unsafe_utf8_idx_at(&self, char_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf16_idx_at(&self, char_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf8_to_idx(&self, utf8_idx: usize) -> usize;
    /// panics if out of bounds
    fn unsafe_utf16_to_idx(&self, utf16_idx: usize) -> usize;
    fn char_len(&self) -> usize;
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
    fn get(&self, from: usize, to: usize) -> Option<&str>;
    fn get_from(&self, from: usize) -> Option<&str>;
    fn get_to(&self, to: usize) -> Option<&str>;
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine);
    fn diagnostic_info(&self, lang: &Lang) -> Option<DiagnosticInfo>;
    fn drop_diagnostics(&mut self);
    fn iter_tokens(&self) -> impl Iterator<Item = &Token>;
    fn push_token(&mut self, token: Token);
    fn replace_tokens(&mut self, tokens: Vec<Token>);
    fn rebuild_tokens(&mut self, lexer: &Lexer);
    fn full_render(&mut self, ctx: &mut impl Context, lines: &mut RectIter, backend: &mut Backend);
    fn render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend);
    fn fast_render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend);
    fn clear_cache(&mut self);
}

pub trait Context {
    fn setup_with_select(&mut self, line: Line, backend: &mut Backend) -> (LineWidth, Option<Select>);
    fn setup_line(&mut self, line: Line, backend: &mut Backend) -> LineWidth;
    fn setup_wrap(&self) -> String;
    fn cursor_char(&self) -> usize;
    fn skip_line(&mut self);
    fn lexer(&self) -> &Lexer;
    fn get_select(&self, width: usize) -> Option<Select>;
    fn count_skipped_to_cursor(&mut self, wrap_len: usize, remaining_lines: usize) -> WrappedCursor;
    fn count_skipped_to_cursor_complex(
        &mut self,
        line: &impl EditorLine,
        wrap_len: usize,
        remaining_lines: usize,
    ) -> (WrappedCursor, usize);
    fn render_cursor(self, gs: &mut GlobalState);
}

pub struct WrappedCursor {
    pub flat_char_idx: usize,
    pub skip_lines: usize,
    pub skip_chars: usize,
}

#[cfg(test)]
mod tests;

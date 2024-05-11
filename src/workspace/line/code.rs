use crate::{
    global_state::GlobalState,
    render::{
        backend::{color, Backend, BackendProtocol, Style},
        layout::{Line, RectIter},
        utils::truncate_str,
    },
    syntax::{DiagnosticLine, Lang, Lexer, Token},
    workspace::{
        cursor::Cursor,
        line::{
            utils::{build_line, build_line_select, shrank_line, wrapped_line, wrapped_line_select},
            Context, EditorLine,
        },
        CursorPosition,
    },
};
use std::{
    cmp::Ordering,
    fmt::Display,
    io::Write,
    ops::{Index, Range, RangeBounds, RangeFrom, RangeFull, RangeTo},
    slice::SliceIndex,
};

use super::utils::inline_diagnostics;

#[derive(Default)]
pub struct CodeLine {
    content: String,
    // syntax
    tokens: Vec<Token>,
    diagnostics: Option<DiagnosticLine>,
    // used for caching - 0 is reseved for file tabs and can be used to reset line
    rendered_at: u16,
}

impl Display for CodeLine {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.content.fmt(f)
    }
}

impl From<String> for CodeLine {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl CodeLine {
    pub fn new(content: String) -> Self {
        Self { content, tokens: Vec::new(), diagnostics: None, rendered_at: 0 }
    }
}

impl Index<Range<usize>> for CodeLine {
    type Output = str;
    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.content[index]
    }
}

impl Index<RangeTo<usize>> for CodeLine {
    type Output = str;
    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.content[index]
    }
}

impl Index<RangeFrom<usize>> for CodeLine {
    type Output = str;
    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.content[index]
    }
}

impl Index<RangeFull> for CodeLine {
    type Output = str;
    #[inline]
    fn index(&self, index: RangeFull) -> &Self::Output {
        &self.content[index]
    }
}

impl EditorLine for CodeLine {
    #[inline]
    fn unwrap(self) -> String {
        self.content
    }

    #[inline]
    fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&I::Output> {
        self.content.get(i)
    }

    #[inline]
    unsafe fn get_unchecked<I: SliceIndex<str>>(&self, i: I) -> &I::Output {
        self.content.get_unchecked(i)
    }

    #[inline]
    fn replace_range(&mut self, range: impl RangeBounds<usize>, string: &str) {
        self.rendered_at = 0;
        self.content.replace_range(range, string);
    }

    #[inline]
    fn starts_with(&self, pat: &str) -> bool {
        self.content.starts_with(pat)
    }

    #[inline]
    fn ends_with(&self, pat: &str) -> bool {
        self.content.ends_with(pat)
    }

    #[inline]
    fn find(&self, pat: &str) -> Option<usize> {
        self.content.find(pat)
    }

    #[inline]
    fn insert(&mut self, idx: usize, ch: char) {
        self.rendered_at = 0;
        self.content.insert(idx, ch);
    }

    #[inline]
    fn push(&mut self, ch: char) {
        self.rendered_at = 0;
        self.content.push(ch);
    }

    #[inline]
    fn insert_str(&mut self, idx: usize, string: &str) {
        self.rendered_at = 0;
        self.content.insert_str(idx, string);
    }

    #[inline]
    fn push_str(&mut self, string: &str) {
        self.rendered_at = 0;
        self.content.push_str(string);
    }

    #[inline]
    fn push_line(&mut self, line: Self) {
        self.rendered_at = 0;
        self.content.push_str(&line.content)
    }

    #[inline]
    fn insert_content_to_buffer(&self, idx: usize, buffer: &mut String) {
        buffer.insert_str(idx, &self.content)
    }

    #[inline]
    fn push_content_to_buffer(&self, buffer: &mut String) {
        buffer.push_str(&self.content)
    }

    #[inline]
    fn remove(&mut self, idx: usize) -> char {
        self.rendered_at = 0;
        self.content.remove(idx)
    }

    #[inline]
    fn trim_start(&self) -> &str {
        &self.content.trim_start()
    }

    #[inline]
    fn trim_end(&self) -> &str {
        &self.content.trim_end()
    }

    #[inline]
    fn chars(&self) -> std::str::Chars<'_> {
        self.content.chars()
    }

    #[inline]
    fn char_indices(&self) -> std::str::CharIndices<'_> {
        self.content.char_indices()
    }

    #[inline]
    fn match_indices<'a>(&self, pat: &'a str) -> std::str::MatchIndices<&'a str> {
        self.content.match_indices(pat)
    }

    #[inline]
    fn clear(&mut self) {
        self.tokens.clear();
        self.content.clear();
        self.rendered_at = 0;
    }

    #[inline]
    fn split_off(&mut self, at: usize) -> Self {
        self.rendered_at = 0;
        Self::from(self.content.split_off(at))
    }

    #[inline]
    fn split_at(&self, mid: usize) -> (&str, &str) {
        self.content.split_at(mid)
    }

    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }

    #[inline]
    fn push_token(&mut self, token: Token) {
        self.rendered_at = 0;
        self.tokens.push(token);
    }

    #[inline]
    fn replace_tokens(&mut self, tokens: Vec<Token>) {
        self.rendered_at = 0;
        self.tokens = tokens;
        if let Some(diagnostics) = self.diagnostics.as_ref() {
            for diagnostic in diagnostics.data.iter() {
                for token in self.tokens.iter_mut() {
                    diagnostic.check_and_update(token);
                }
            }
        };
    }

    #[inline]
    fn rebuild_tokens(&mut self, lexer: &Lexer) {
        self.rendered_at = 0;
        self.tokens.clear();
        Token::parse(&lexer.lang, &lexer.theme, &self.content, &mut self.tokens);
    }

    #[inline]
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine) {
        self.rendered_at = 0;
        for diagnostic in diagnostics.data.iter() {
            for token in self.tokens.iter_mut() {
                diagnostic.check_and_update(token);
            }
        }
        self.diagnostics.replace(diagnostics);
    }

    #[inline]
    fn diagnostic_info(&self, lang: &Lang) -> Option<crate::syntax::DiagnosticInfo> {
        self.diagnostics.as_ref().map(|d| d.collect_info(lang))
    }

    #[inline]
    fn drop_diagnostics(&mut self) {
        if self.diagnostics.take().is_some() {
            for token in self.tokens.iter_mut() {
                token.drop_diagstic();
            }
            self.rendered_at = 0;
        };
    }

    #[inline]
    fn wrapped_render(
        &mut self,
        ctx: &mut impl Context,
        lines: &mut RectIter,
        backend: &mut Backend,
    ) -> std::io::Result<()> {
        let wrap_len = match lines.next() {
            Some(line) => ctx.setup_line(line, backend)?,
            None => return Ok(()),
        };
        match ctx.get_select() {
            Some(select) => wrapped_line_select(&self.content, &self.tokens, ctx, wrap_len, lines, select, backend),
            None => wrapped_line(&self.content, &self.tokens, ctx, wrap_len, lines, backend),
        }?;
        backend.reset_style()?;
        backend.flush()
    }

    #[inline]
    fn render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend) -> std::io::Result<()> {
        if self.tokens.is_empty() {
            Token::parse(&ctx.lexer().lang, &ctx.lexer().theme, &self.content, &mut self.tokens);
        };
        self.rendered_at = line.row;
        let line_width = ctx.setup_line(line, backend)?;
        match ctx.get_select() {
            Some(select) => {
                if line_width > self.content.len() {
                    self.rendered_at = 0;
                    build_line_select(
                        self.content.char_indices(),
                        &self.tokens,
                        select,
                        ctx.lexer().theme.selected,
                        backend,
                    )?;
                    inline_diagnostics(line_width - self.content.len(), &self.diagnostics, backend)
                } else {
                    let end_loc = line_width.saturating_sub(2);
                    build_line_select(
                        self.content.char_indices().take(end_loc),
                        &self.tokens,
                        select,
                        ctx.lexer().theme.selected,
                        backend,
                    )?;
                    backend.print_styled(">>", Style::reversed())
                }
            }
            None => {
                if line_width > self.content.len() {
                    build_line(&self.content, &self.tokens, backend)?;
                    inline_diagnostics(line_width - self.content.len(), &self.diagnostics, backend)
                } else {
                    let max_len = line_width.saturating_sub(2);
                    shrank_line(truncate_str(&self.content, max_len), &self.tokens, backend)
                }
            }
        }?;
        backend.flush()
    }

    #[inline]
    fn fast_render(&mut self, ctx: &mut impl Context, line: Line, writer: &mut Backend) -> std::io::Result<()> {
        if self.rendered_at != 1 && self.rendered_at == line.row {
            ctx.skip_line();
            return Ok(());
        };
        self.render(ctx, line, writer)
    }

    #[inline]
    fn clear_cache(&mut self) {
        self.rendered_at = 0;
    }
}

impl Into<String> for CodeLine {
    fn into(self) -> String {
        self.content
    }
}

pub struct CodeLineContext<'a> {
    lexer: &'a mut Lexer,
    line_number: usize,
    line: usize,
    char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
    select_buffer: Option<Range<usize>>,
}

impl<'a> CodeLineContext<'a> {
    pub fn new(cursor: &Cursor, lexer: &'a mut Lexer) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self { line: cursor.line - line_number, char: cursor.char, select, select_buffer: None, lexer, line_number }
    }
}

impl<'a> Context for CodeLineContext<'a> {
    #[inline]
    fn lexer(&self) -> &Lexer {
        &*self.lexer
    }

    #[inline]
    fn get_select(&mut self) -> Option<Range<usize>> {
        self.select_buffer.take()
    }

    #[inline]
    fn setup_line(&mut self, line: Line, writer: &mut Backend) -> std::io::Result<usize> {
        let line_number = self.line_number + 1;
        let text = format!("{: >1$} ", line_number, self.lexer.line_number_offset);
        let remaining_width = line.width - text.len();
        self.select_buffer = build_select_buffer(self.select, self.line_number, remaining_width);
        self.line_number = line_number;
        writer.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()))?;
        writer.clear_to_eol().map(|_| remaining_width)
    }

    #[inline]
    fn skip_line(&mut self) {
        self.line_number += 1;
    }

    #[inline]
    fn count_skipped_to_cursor(&mut self, wrap_len: usize, remaining_lines: usize) -> usize {
        let wraps = self.char / wrap_len + 1;
        let skip_lines = wraps.saturating_sub(remaining_lines);
        self.char = self.char % wrap_len;
        self.line += wraps.saturating_sub(skip_lines);
        skip_lines
    }

    #[inline]
    fn render_cursor(self, gs: &mut GlobalState) -> std::io::Result<()> {
        let row = gs.editor_area.row + self.line as u16;
        let col = gs.editor_area.col + (self.char + self.lexer.line_number_offset + 1) as u16;
        self.lexer.render_modal_if_exist(row, col, gs);
        gs.writer.render_cursor_at(row, col)
    }
}

pub fn build_select_buffer(
    select: Option<(CursorPosition, CursorPosition)>,
    at_line: usize,
    max_len: usize,
) -> Option<Range<usize>> {
    select.and_then(|(from, to)| match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
        (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
        (Ordering::Less, Ordering::Less) => Some(0..max_len),
        (Ordering::Equal, Ordering::Equal) => Some(from.char..to.char),
        (Ordering::Equal, ..) => Some(from.char..max_len),
        (.., Ordering::Equal) => Some(0..to.char),
    })
}

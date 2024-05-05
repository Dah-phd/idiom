use crate::{
    render::{
        backend::{color, Backend, Color, Style},
        layout::{Line, RectIter},
    },
    syntax::{DiagnosticLine, Lang, Lexer, Token},
    workspace::{cursor::Cursor, line::EditorLine, CursorPosition},
};
use std::{
    cmp::Ordering,
    fmt::Display,
    fs::write,
    io::Write,
    ops::{Index, Range, RangeBounds, RangeFrom, RangeFull, RangeTo},
    slice::SliceIndex,
};

use super::Context;

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
    fn string(&self) -> &String {
        &self.content
    }

    #[inline]
    fn string_mut(&mut self) -> &mut String {
        &mut self.content
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
        self.content.replace_range(range, string);
    }

    #[inline]
    fn as_str(&self) -> &str {
        &self.content
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
    fn remove(&mut self, idx: usize) -> char {
        self.content.remove(idx)
    }

    #[inline]
    fn clear(&mut self) {
        self.tokens.clear();
        self.content.clear();
        self.rendered_at = 0;
    }

    #[inline]
    fn split_off(&mut self, at: usize) -> String {
        self.content.split_off(at)
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
    fn render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend) -> std::io::Result<()> {
        if self.tokens.is_empty() {
            Token::parse(&ctx.lexer().lang, &ctx.lexer().theme, &self.content, &mut self.tokens);
        };
        self.rendered_at = line.row;
        let line_width = ctx.setup_line(line, backend)?;
        if line_width <= self.content.len() {
            let end_loc = line_width.saturating_sub(2);
            shrank_line(unsafe { self.content.get_unchecked(..end_loc) }, &self.tokens, backend)?;
        } else {
            match ctx.get_select() {
                Some(select) => {
                    build_line_select(&self.content, &self.tokens, select, ctx.lexer().theme.selected, backend)
                }
                None => build_line(&self.content, &self.tokens, backend),
            }?;
        };
        backend.flush()
    }

    #[inline]
    fn fast_render(&mut self, ctx: &mut impl Context, line: Line, writer: &mut Backend) -> std::io::Result<()> {
        if self.rendered_at != 1 && self.rendered_at == line.row {
            return Ok(());
        };
        self.render(ctx, line, writer)
    }

    #[inline]
    fn wrapped_render(
        &mut self,
        ctx: &mut impl Context,
        lines: &mut RectIter,
        writer: &mut Backend,
    ) -> std::io::Result<()> {
        wrapped_line(&self.content, &self.tokens, ctx, lines, writer)
    }
}

impl Into<String> for CodeLine {
    fn into(self) -> String {
        self.content
    }
}

#[inline]
fn build_line_(content: &str, tokens: &[Token], writer: &mut Backend) -> std::io::Result<()> {
    let mut iter_toknes = tokens.into_iter();
    let mut maybe_token = iter_toknes.next();
    for (idx, text) in content.char_indices() {
        if let Some(token) = maybe_token {
            if idx == token.to {
                writer.reset_style()?;
                maybe_token = iter_toknes.next();
            }
        }
        if let Some(token) = maybe_token {
            if idx == token.from {
                writer.set_style(token.style)?;
            }
        }
        writer.print(text)?;
    }
    Ok(())
}

#[inline]
fn build_line(content: &str, tokens: &[Token], backend: &mut Backend) -> std::io::Result<()> {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                backend.print(text)?;
            } else if let Some(text) = content.get(end..) {
                return backend.print(text);
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            backend.print_styled(text, token.style)?;
        } else if let Some(text) = content.get(token.from..) {
            return backend.print_styled(text, token.style);
        };
        end = token.to;
    }
    if let Some(text) = content.get(end..) {
        backend.print(text)?;
    }
    Ok(())
}

#[inline]
fn build_line_select(
    content: &str,
    tokens: &[Token],
    select: Range<usize>,
    select_color: Color,
    writer: &mut Backend,
) -> std::io::Result<()> {
    let mut iter_tokens = tokens.into_iter();
    let mut maybe_token = iter_tokens.next();
    for (idx, text) in content.char_indices() {
        if select.start == idx {
            writer.add_bg(select_color)?;
        }
        if select.end == idx {
            writer.drop_bg()?;
        }
        if let Some(token) = maybe_token {
            if token.from == idx {
                writer.update_style(token.style)?;
            } else if token.to == idx {
                if let Some(token) = iter_tokens.next() {
                    if token.from == idx {
                        writer.update_style(token.style)?;
                    } else {
                        let mut reset_style = writer.get_style();
                        reset_style.drop_fg();
                        reset_style.reset_mods();
                        writer.set_style(reset_style)?;
                    };
                    maybe_token.replace(token);
                } else {
                    let mut reset_style = writer.get_style();
                    reset_style.drop_fg();
                    reset_style.reset_mods();
                    writer.set_style(reset_style)?;
                    maybe_token = None;
                };
            };
        }
        writer.print(text)?;
    }
    writer.reset_style()
}

#[inline]
fn shrank_line(content: &str, tokens: &[Token], writer: &mut Backend) -> std::io::Result<()> {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                writer.print(text)?;
            } else if let Some(text) = content.get(end..) {
                writer.print(text)?;
                return writer.print_styled(">>", Style::reversed());
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            writer.print_styled(text, token.style)?;
        } else if let Some(text) = content.get(token.from..) {
            writer.print_styled(text, token.style)?;
            return writer.print_styled(">>", Style::reversed());
        };
        end = token.to;
    }
    writer.print_styled(">>", Style::reversed())
}

#[inline]
fn shrank_line_select(
    content: &str,
    tokens: &[Token],
    select: Range<usize>,
    select_color: Color,
    writer: &mut Backend,
) -> std::io::Result<()> {
    Ok(())
}

#[inline]
fn wrapped_line_select(
    content: &str,
    tokens: &[Token],
    ctx: &mut impl Context,
    lines: &mut RectIter,
    select: Range<usize>,
    select_color: Color,
    writer: &mut Backend,
) -> std::io::Result<()> {
    Ok(())
}

#[inline]
fn wrapped_line(
    content: &str,
    tokens: &[Token],
    ctx: &mut impl Context,
    lines: &mut RectIter,
    writer: &mut Backend,
) -> std::io::Result<()> {
    let wrap_len = lines.width() - (ctx.lexer().line_number_offset + 1);
    let wrapped_lines = content.len() / wrap_len;
    let wrap_number = format!("{:.<1$} ", "", ctx.lexer().line_number_offset);
    let mut line_end = wrap_len;
    if wrapped_lines > lines.len() {
    } else {
        match lines.next() {
            Some(line) => ctx.setup_line(line, writer)?,
            None => return Ok(()),
        };
        let mut iter_tokens = tokens.iter();
        let mut maybe_token = iter_tokens.next();
        for (idx, text) in content.char_indices() {
            if line_end == idx {
                let line = lines.next().unwrap();
                writer.print_styled_at(line.row, line.col, &wrap_number, Style::fg(color::dark_grey()))?;
                writer.clear_to_eol()?;
                line_end += wrap_len;
            }
            if let Some(token) = maybe_token {
                if token.to == idx {
                    writer.reset_style()?;
                    maybe_token = iter_tokens.next();
                };
            }
            if let Some(token) = maybe_token {
                if token.from == idx {
                    writer.set_style(token.style)?;
                };
            }
            writer.print(text)?;
        }
    }
    Ok(())
}

pub struct CodeLineContext<'a> {
    lexer: &'a Lexer,
    line_number: usize,
    select: Option<(CursorPosition, CursorPosition)>,
    select_buffer: Option<Range<usize>>,
}

impl<'a> CodeLineContext<'a> {
    pub fn new(cursor: &Cursor, lexer: &'a Lexer) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self { lexer, line_number, select, select_buffer: None }
    }
}

impl<'a> Context for CodeLineContext<'a> {
    #[inline]
    fn lexer(&self) -> &'a Lexer {
        self.lexer
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
    fn render_cursor(&self) -> std::io::Result<()> {
        Ok(())
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

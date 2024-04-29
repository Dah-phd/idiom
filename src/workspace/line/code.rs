use crate::{
    render::layout::Line as LineInfo,
    syntax::{DiagnosticLine, Lexer, Token},
    workspace::line::Line as LineInterface,
};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};
use std::{
    fmt::Display,
    ops::{Index, Range, RangeBounds, RangeFrom, RangeFull, RangeTo},
    slice::SliceIndex,
};

#[derive(Default)]
pub struct CodeLine {
    content: String,
    // syntax
    tokens: Vec<Token>,
    diagnostics: Option<DiagnosticLine>,
    // used for caching
    rendered_at: usize,
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

impl LineInterface for CodeLine {
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
                    diagnostic.check_token(token);
                }
            }
        };
    }

    #[inline]
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine) {
        self.rendered_at = 0;
        for diagnostic in diagnostics.data.iter() {
            for token in self.tokens.iter_mut() {
                diagnostic.check_token(token);
            }
        }
        self.diagnostics.replace(diagnostics);
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
    fn render(
        &mut self,
        idx: usize,
        line: LineInfo,
        lexer: &mut Lexer,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if self.tokens.is_empty() {
            Token::parse(&lexer.lang, &lexer.theme, &self.content, &mut self.tokens);
        };
        self.rendered_at = idx;
        let line_number = format!("{: >1$} ", self.rendered_at, lexer.line_number_offset);
        queue!(writer, MoveTo(line.col, line.row), PrintStyledContent(line_number.dark_grey()))?;
        queue!(writer, Clear(ClearType::UntilNewLine))?;
        if line.width <= self.content.len() + lexer.line_number_offset {
            let end_loc = line.width.saturating_sub(3 + lexer.line_number_offset);
            shrank_line(unsafe { self.content.get_unchecked(..end_loc) }, &self.tokens, writer)?;
        } else {
            build_line(&self.content, &self.tokens, writer)?;
        };
        writer.flush()
    }

    #[inline]
    fn fast_render(
        &mut self,
        idx: usize,
        line: LineInfo,
        lexer: &mut Lexer,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if self.rendered_at == idx {
            return Ok(());
        };
        self.render(idx, line, lexer, writer)
    }

    #[inline]
    fn wrapped_render(
        &mut self,
        idx: usize,
        line: LineInfo,
        limit: usize,
        lexer: &mut Lexer,
        writer: &mut impl std::io::prelude::Write,
    ) -> std::io::Result<usize> {
        wrapped_line()
    }
}

impl Into<String> for CodeLine {
    fn into(self) -> String {
        self.content
    }
}

#[inline]
fn build_line(content: &str, tokens: &[Token], writer: &mut impl std::io::Write) -> std::io::Result<()> {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                queue!(writer, Print(text))?;
            } else if let Some(text) = content.get(end..) {
                return queue!(writer, Print(text));
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            queue!(writer, PrintStyledContent(token.color.apply(text)))?;
        } else if let Some(text) = content.get(token.from..) {
            return queue!(writer, PrintStyledContent(token.color.apply(text)));
        };
        end = token.to;
    }
    if let Some(text) = content.get(end..) {
        queue!(writer, Print(text))?;
    }
    Ok(())
}

#[inline]
fn shrank_line(content: &str, tokens: &[Token], writer: &mut impl std::io::Write) -> std::io::Result<()> {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                queue!(writer, Print(text))?;
            } else if let Some(text) = content.get(end..) {
                queue!(writer, Print(text))?;
                return queue!(writer, PrintStyledContent(">>".reverse()));
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            queue!(writer, PrintStyledContent(token.color.apply(text)))?;
        } else if let Some(text) = content.get(token.from..) {
            queue!(writer, PrintStyledContent(token.color.apply(text)))?;
            return queue!(writer, PrintStyledContent(">>".reverse()));
        };
        end = token.to;
    }
    queue!(writer, PrintStyledContent(">>".reverse()))
}

#[inline]
fn wrapped_line() -> std::io::Result<usize> {
    todo!()
}

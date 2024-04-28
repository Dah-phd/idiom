use crate::{
    render::layout::Line as LineInfo,
    syntax::{line_builder::tokens::Token, Lexer},
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
    rendered_at: usize,
    tokens: Vec<Token>,
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
        Self { content, tokens: Vec::new(), rendered_at: 0 }
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
        self.content.insert(idx, ch);
    }

    #[inline]
    fn push(&mut self, ch: char) {
        self.content.push(ch);
    }

    #[inline]
    fn insert_str(&mut self, idx: usize, string: &str) {
        self.content.insert_str(idx, string);
    }

    #[inline]
    fn push_str(&mut self, string: &str) {
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
        self.tokens.push(token);
    }

    #[inline]
    fn replace_tokens(&mut self, tokens: Vec<Token>) {
        self.tokens = tokens;
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
        self.rendered_at = idx + 1;
        let line_number = format!("{: >1$} ", self.rendered_at, lexer.line_number_offset);
        let mut end = 0;
        queue!(writer, MoveTo(line.col, line.row), PrintStyledContent(line_number.dark_grey()))?;
        queue!(writer, Clear(ClearType::UntilNewLine))?;
        for token in self.tokens.iter() {
            if token.from > end {
                if let Some(text) = self.content.get(end..token.from) {
                    queue!(writer, Print(text))?;
                }
            };
            if let Some(text) = self.content.get(token.from..token.to).or(self.content.get(token.from..)) {
                queue!(writer, PrintStyledContent(token.color.apply(text)))?;
            };
            end = token.to;
        }
        if self.content.len() > end {
            if let Some(text) = self.content.get(end..) {
                queue!(writer, Print(text))?;
            }
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
        if self.rendered_at == idx + 1 {
            return Ok(());
        };
        self.render(idx, line, lexer, writer)
    }
}

impl Into<String> for CodeLine {
    fn into(self) -> String {
        self.content
    }
}

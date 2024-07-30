mod context;
mod render;
use render::{ascii_line, complex_line, RenderCache};
use unicode_width::UnicodeWidthChar;

use crate::{
    render::{
        backend::{Backend, BackendProtocol, Style},
        layout::{Line, RectIter},
        utils::UTF8SafeStringExt,
        UTF8Safe,
    },
    syntax::{DiagnosticLine, Lang, Lexer, Token},
    workspace::line::EditorLine,
};
pub use context::CodeLineContext;
use std::{
    fmt::Display,
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    path::Path,
};

/// Used to represent code, has simpler wrapping as cpde lines shoud be shorter than 120 chars in most cases
#[derive(Default)]
pub struct CodeLine {
    content: String,
    // keeps trach of utf8 char len
    char_len: usize,
    // syntax
    tokens: Vec<Token>,
    diagnostics: Option<DiagnosticLine>,
    // used for caching - 0 is reseved for file tabs and can be used to reset line
    pub cached: RenderCache,
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

impl From<&'static str> for CodeLine {
    fn from(value: &'static str) -> Self {
        value.to_owned().into()
    }
}

impl CodeLine {
    pub fn new(content: String) -> Self {
        Self { char_len: content.char_len(), content, ..Default::default() }
    }
}

impl Index<Range<usize>> for CodeLine {
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

impl Index<RangeTo<usize>> for CodeLine {
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

impl Index<RangeFrom<usize>> for CodeLine {
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

impl Index<RangeFull> for CodeLine {
    type Output = str;
    fn index(&self, _: RangeFull) -> &Self::Output {
        &self.content
    }
}

impl EditorLine for CodeLine {
    type Context<'a> = CodeLineContext<'a>;
    type Error = String;

    #[inline]
    fn parse_lines<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Self::Error> {
        Ok(std::fs::read_to_string(path)
            .map_err(|err| err.to_string())?
            .split('\n')
            .map(|line| CodeLine::new(line.to_owned()))
            .collect())
    }

    #[inline]
    fn is_simple(&self) -> bool {
        self.content.len() == self.char_len
    }

    #[inline]
    fn unwrap(self) -> String {
        self.content
    }

    #[inline]
    fn get(&self, from: usize, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..to);
        }
        self.content.utf8_get(from, to)
    }

    #[inline]
    fn get_from(&self, from: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(from..);
        }
        self.content.utf8_get_from(from)
    }

    #[inline]
    fn get_to(&self, to: usize) -> Option<&str> {
        if self.char_len == self.content.len() {
            return self.content.get(..to);
        }
        self.content.utf8_get_to(to)
    }

    #[inline]
    fn replace_till(&mut self, to: usize, string: &str) {
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
    fn replace_from(&mut self, from: usize, string: &str) {
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
    fn replace_range(&mut self, range: Range<usize>, string: &str) {
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
        self.cached.reset();
        if self.char_len == self.content.len() {
            self.char_len += 1;
            self.content.insert(idx, ch);
        } else {
            self.char_len += 1;
            self.content.utf8_insert(idx, ch);
        }
    }

    #[inline]
    fn push(&mut self, ch: char) {
        self.cached.reset();
        self.char_len += 1;
        self.content.push(ch);
    }

    #[inline]
    fn insert_str(&mut self, idx: usize, string: &str) {
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
    fn push_str(&mut self, string: &str) {
        self.cached.reset();
        self.char_len += string.char_len();
        self.content.push_str(string);
    }

    #[inline]
    fn push_line(&mut self, line: Self) {
        self.cached.reset();
        self.char_len += line.char_len;
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
        self.cached.reset();
        if self.content.len() == self.char_len {
            self.char_len -= 1;
            return self.content.remove(idx);
        }
        self.char_len -= 1;
        self.content.utf8_remove(idx)
    }

    #[inline]
    fn trim_start(&self) -> &str {
        self.content.trim_start()
    }

    #[inline]
    fn trim_end(&self) -> &str {
        self.content.trim_end()
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
        self.cached.reset();
    }

    #[inline]
    fn split_off(&mut self, at: usize) -> Self {
        self.cached.reset();
        if self.content.len() == self.char_len {
            let content = self.content.split_off(at);
            self.char_len = self.content.len();
            self.tokens.clear();
            return Self {
                char_len: content.len(),
                content,
                diagnostics: self.diagnostics.take(),
                ..Default::default()
            };
        }
        let content = self.content.utf8_split_off(at);
        self.char_len = self.content.char_len();
        self.tokens.clear();
        Self { char_len: content.char_len(), content, diagnostics: self.diagnostics.take(), ..Default::default() }
    }

    #[inline]
    fn split_at(&self, mid: usize) -> (&str, &str) {
        if self.content.len() == self.char_len {
            self.content.split_at(mid)
        } else {
            self.content.utf8_split_at(mid)
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.content.len()
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.char_len
    }

    #[inline]
    fn unsafe_utf8_idx_at(&self, char_idx: usize) -> usize {
        if char_idx > self.char_len {
            panic!("Index out of bounds! Index {} where max is {}", char_idx, self.char_len);
        }
        if self.char_len == self.content.len() {
            return char_idx;
        };
        self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf8())
    }

    #[inline]
    fn unsafe_utf16_idx_at(&self, char_idx: usize) -> usize {
        if char_idx > self.char_len {
            panic!("Index out of bounds! Index {} where max is {}", char_idx, self.char_len);
        }
        if self.is_simple() {
            return char_idx;
        }
        self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf16())
    }

    #[inline]
    fn unsafe_utf8_to_idx(&self, utf8_idx: usize) -> usize {
        for (idx, (byte_idx, ..)) in self.content.char_indices().enumerate() {
            if byte_idx == utf8_idx {
                return idx;
            }
        }
        panic!("Index out of bounds! Index {} where max is {}", utf8_idx, self.content.len());
    }

    #[inline]
    fn unsafe_utf16_to_idx(&self, utf16_idx: usize) -> usize {
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
    fn utf16_len(&self) -> usize {
        self.content.chars().fold(0, |sum, ch| sum + ch.len_utf16())
    }

    #[inline]
    fn iter_tokens(&self) -> impl Iterator<Item = &Token> {
        self.tokens.iter()
    }

    #[inline]
    fn push_token(&mut self, token: Token) {
        self.cached.reset();
        self.tokens.push(token);
    }

    #[inline]
    fn replace_tokens(&mut self, tokens: Vec<Token>) {
        self.cached.reset();
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
        self.cached.reset();
        self.tokens.clear();
        Token::parse_to_buf(&lexer.lang, &lexer.theme, &self.content, &mut self.tokens);
    }

    #[inline]
    fn set_diagnostics(&mut self, diagnostics: DiagnosticLine) {
        self.cached.reset();
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
            self.cached.reset();
        };
    }

    #[inline]
    fn cursor(&mut self, ctx: &mut CodeLineContext<'_>, lines: &mut RectIter, backend: &mut Backend) {
        if self.tokens.is_empty() {
            Token::parse_to_buf(&ctx.lexer.lang, &ctx.lexer.theme, &self.content, &mut self.tokens);
        };
        render::cursor(self, ctx, lines, backend);
    }

    #[inline]
    fn cursor_fast(&mut self, ctx: &mut CodeLineContext<'_>, lines: &mut RectIter, backend: &mut Backend) {
        if self.tokens.is_empty() {
            Token::parse_to_buf(&ctx.lexer.lang, &ctx.lexer.theme, &self.content, &mut self.tokens);
            if !self.tokens.is_empty() {
                self.cached.reset();
            }
        };
        render::cursor_fast(self, ctx, lines, backend);
    }

    #[inline]
    fn render(&mut self, ctx: &mut CodeLineContext<'_>, line: Line, backend: &mut Backend) {
        if self.tokens.is_empty() {
            Token::parse_to_buf(&ctx.lexer.lang, &ctx.lexer.theme, &self.content, &mut self.tokens);
        };
        let cache_line = line.row;
        let (line_width, select) = ctx.setup_line(line, backend);
        self.cached.line(cache_line, select.clone());
        match select {
            Some(select) => self.render_with_select(line_width, select, ctx, backend),
            None => self.render_no_select(line_width, ctx, backend),
        }
    }

    #[inline]
    fn fast_render(&mut self, ctx: &mut CodeLineContext, line: Line, backend: &mut Backend) {
        if self.cached.should_render_line(line.row, &ctx.get_select(line.width)) {
            self.render(ctx, line, backend);
        } else {
            ctx.skip_line();
        }
    }

    #[inline]
    fn clear_cache(&mut self) {
        self.cached.reset();
    }
}

impl CodeLine {
    #[inline(always)]
    fn render_with_select(
        &mut self,
        line_width: usize,
        select: Range<usize>,
        ctx: &mut CodeLineContext,
        backend: &mut impl BackendProtocol,
    ) {
        if self.char_len == 0 && select.end != 0 {
            backend.print_styled(" ", Style::bg(ctx.lexer.theme.selected));
            return;
        }
        if self.is_simple() {
            if line_width > self.char_len() {
                let content = self.content.char_indices();
                ascii_line::ascii_line_with_select(content, &self.tokens, select, ctx.lexer, backend);
                if let Some(diagnostic) = self.diagnostics.as_ref() {
                    diagnostic.inline_render(line_width - self.char_len, backend)
                }
            } else {
                let content = self.content.char_indices().take(line_width.saturating_sub(2));
                ascii_line::ascii_line_with_select(content, &self.tokens, select, ctx.lexer, backend);
                backend.print_styled(">>", Style::reversed());
            }
        // handles non ascii shrunk lines
        } else if let Ok(truncated) = self.content.truncate_if_wider(line_width) {
            let mut content = truncated.chars();
            if let Some(ch) = content.next_back() {
                if UnicodeWidthChar::width(ch).unwrap_or_default() <= 1 {
                    content.next_back();
                }
            };
            complex_line::complex_line_with_select(content, &self.tokens, select, ctx.lexer, backend);
            backend.print_styled(">>", Style::reversed());
        } else {
            complex_line::complex_line_with_select(self.content.chars(), &self.tokens, select, ctx.lexer, backend);
            if let Some(diagnostic) = self.diagnostics.as_ref() {
                diagnostic.inline_render(line_width - self.content.width(), backend)
            }
        }
    }

    #[inline(always)]
    fn render_no_select(&mut self, line_width: usize, ctx: &mut CodeLineContext, backend: &mut impl BackendProtocol) {
        if self.is_simple() {
            if line_width > self.content.len() {
                ascii_line::ascii_line(&self.content, &self.tokens, backend);
                if let Some(diagnostic) = self.diagnostics.as_ref() {
                    diagnostic.inline_render(line_width - self.char_len, backend)
                }
            } else {
                ascii_line::ascii_line(&self.content[..line_width.saturating_sub(2)], &self.tokens, backend);
                backend.print_styled(">>", Style::reversed());
            }
        // handles non ascii shrunk lines
        } else if let Ok(truncated) = self.content.truncate_if_wider(line_width) {
            let mut content = truncated.chars();
            if let Some(ch) = content.next_back() {
                if UnicodeWidthChar::width(ch).unwrap_or_default() <= 1 {
                    content.next_back();
                }
            };
            complex_line::complex_line(content, &self.tokens, ctx.lexer, backend);
            backend.print_styled(">>", Style::reversed());
        } else {
            complex_line::complex_line(self.content.chars(), &self.tokens, ctx.lexer, backend);
            if let Some(diagnostic) = self.diagnostics.as_ref() {
                diagnostic.inline_render(line_width - self.content.width(), backend)
            }
        }
    }
}

impl From<CodeLine> for String {
    fn from(val: CodeLine) -> Self {
        val.content
    }
}

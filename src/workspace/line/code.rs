use crate::{
    global_state::GlobalState,
    render::{
        backend::{color, Backend, BackendProtocol, Style},
        layout::{Line, RectIter},
        utils::UTF8SafeStringExt,
        UTF8Safe,
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
    ops::{Index, Range, RangeFrom, RangeTo},
};

use super::utils::inline_diagnostics;

#[derive(Default)]
pub struct CodeLine {
    content: String,
    // keeps trach of utf8 char len
    char_len: usize,
    // syntax
    tokens: Vec<Token>,
    diagnostics: Option<DiagnosticLine>,
    // used for caching - 0 is reseved for file tabs and can be used to reset line
    rendered_at: u16,
    select: Option<Range<usize>>,
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
        Self {
            char_len: content.char_len(),
            content,
            tokens: Vec::new(),
            diagnostics: None,
            rendered_at: 0,
            select: None,
        }
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

impl EditorLine for CodeLine {
    #[inline]
    fn is_ascii(&self) -> bool {
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
        self.rendered_at = 0;
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
        self.rendered_at = 0;
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
        self.rendered_at = 0;
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
        self.rendered_at = 0;
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
        self.rendered_at = 0;
        self.char_len += 1;
        self.content.push(ch);
    }

    #[inline]
    fn insert_str(&mut self, idx: usize, string: &str) {
        self.rendered_at = 0;
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
        self.rendered_at = 0;
        self.char_len += string.char_len();
        self.content.push_str(string);
    }

    #[inline]
    fn push_line(&mut self, line: Self) {
        self.rendered_at = 0;
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
        self.rendered_at = 0;
        if self.content.len() == self.char_len {
            self.char_len -= 1;
            return self.content.remove(idx);
        }
        self.char_len -= 1;
        self.content.utf8_remove(idx)
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
        if self.content.len() == self.char_len {
            let split_off_len = self.char_len - at;
            self.tokens.clear();
            return Self {
                content: self.content.split_off(at),
                char_len: split_off_len,
                tokens: Vec::new(),
                diagnostics: self.diagnostics.take(),
                rendered_at: 0,
                select: None,
            };
        } else {
            let (current, new) = self.content.utf8_split_at(at);
            let content = new.to_owned();
            self.content = current.to_owned();
            self.char_len = self.content.char_len();
            self.tokens.clear();
            Self {
                char_len: content.char_len(),
                content,
                tokens: Vec::new(),
                diagnostics: self.diagnostics.take(),
                select: None,
                rendered_at: 0,
            }
        }
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

    #[inline]
    fn char_len(&self) -> usize {
        self.char_len
    }

    #[inline]
    fn utf8_reposition(&self, char_idx: usize) -> Option<usize> {
        if char_idx < self.char_len {
            return Some(if self.char_len == self.content.len() {
                char_idx
            } else {
                self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf8())
            });
        }
        None
    }

    #[inline]
    fn utf16_len(&self) -> usize {
        self.content.chars().fold(0, |sum, ch| sum + ch.len_utf16())
    }

    #[inline]
    fn utf16_reposition(&self, char_idx: usize) -> Option<usize> {
        if char_idx < self.char_len {
            return Some(self.content.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf16()));
        }
        None
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
    fn wrapped_render(&mut self, ctx: &mut impl Context, lines: &mut RectIter, backend: &mut Backend) {
        let (wrap_len, select) = match lines.next() {
            Some(line) => ctx.setup_with_select(line, backend),
            None => return,
        };
        match select {
            Some(select) => wrapped_line_select(&self.content, &self.tokens, ctx, wrap_len, lines, select, backend),
            None => wrapped_line(&self.content, &self.tokens, ctx, wrap_len, lines, backend),
        };
        backend.reset_style();
    }

    #[inline]
    fn render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend) {
        if self.tokens.is_empty() {
            Token::parse(&ctx.lexer().lang, &ctx.lexer().theme, &self.content, &mut self.tokens);
        };
        self.rendered_at = line.row;
        let (line_width, select) = ctx.setup_with_select(line, backend);
        self.select = select;
        match self.select.clone() {
            Some(select) => {
                if line_width > self.content.len() {
                    self.rendered_at = 0;
                    build_line_select(
                        self.content.char_indices(),
                        &self.tokens,
                        select,
                        ctx.lexer().theme.selected,
                        backend,
                    );
                    inline_diagnostics(line_width - self.char_len, &self.diagnostics, backend);
                } else {
                    let end_loc = line_width.saturating_sub(2);
                    build_line_select(
                        self.content.char_indices().take(end_loc), // utf8 safe
                        &self.tokens,
                        select,
                        ctx.lexer().theme.selected,
                        backend,
                    );
                    backend.print_styled(">>", Style::reversed());
                }
            }
            None => {
                if line_width > self.content.len() {
                    build_line(&self.content, &self.tokens, backend);
                    inline_diagnostics(line_width - self.char_len, &self.diagnostics, backend);
                } else {
                    let max_len = line_width.saturating_sub(2);
                    shrank_line(&self.content.truncate_width(max_len), &self.tokens, backend);
                };
            }
        }
    }

    #[inline]
    fn fast_render(&mut self, ctx: &mut impl Context, line: Line, backend: &mut Backend) {
        if self.rendered_at != line.row || self.select != ctx.get_select(line.width) {
            return self.render(ctx, line, backend);
        }
        ctx.skip_line();
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
    line_number_offset: usize,
    line: usize,
    char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl<'a> CodeLineContext<'a> {
    pub fn collect_context(lexer: &'a mut Lexer, cursor: &Cursor, line_number_offset: usize) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self { line: cursor.line - line_number, char: cursor.char, select, lexer, line_number, line_number_offset }
    }

    pub fn correct_cursor(&mut self, code_line: &CodeLine) {
        if !code_line.is_ascii() {
            self.char = code_line.content.width_at(self.char);
        }
    }
}

impl<'a> Context for CodeLineContext<'a> {
    #[inline]
    fn lexer(&self) -> &Lexer {
        &*self.lexer
    }

    #[inline]
    fn setup_with_select(&mut self, line: Line, backend: &mut Backend) -> (usize, Option<Range<usize>>) {
        let line_number = self.line_number + 1;
        let text = format!("{: >1$} ", line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        let select_buffer = build_select_buffer(self.select, self.line_number, remaining_width);
        self.line_number = line_number;
        backend.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()));
        backend.clear_to_eol();
        (remaining_width, select_buffer)
    }

    #[inline]
    fn setup_line(&mut self, line: Line, backend: &mut Backend) -> usize {
        let line_number = self.line_number + 1;
        let text = format!("{: >1$} ", line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        self.line_number = line_number;
        backend.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()));
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    fn setup_wrap(&self) -> String {
        format!("{:.<1$} ", "", self.line_number_offset)
    }

    #[inline]
    fn get_select(&self, width: usize) -> Option<super::Select> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_offset + 1))
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
    fn render_cursor(self, gs: &mut GlobalState) {
        let row = gs.editor_area.row + self.line as u16;
        let col = gs.editor_area.col + (self.char + self.line_number_offset + 1) as u16;
        self.lexer.render_modal_if_exist(row, col, gs);
        gs.writer.render_cursor_at(row, col);
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

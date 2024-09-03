mod context;
mod render;
mod types;
use crate::render::backend::BackendProtocol;
use crate::render::backend::Style;
use context::Context;
use render::RenderStatus;
use unicode_width::UnicodeWidthChar;

use crate::{
    render::{
        backend::Backend,
        layout::RectIter,
        utils::{UTF8SafeStringExt, WriteChunks},
        UTF8Safe,
    },
    workspace::line::EditorLine,
};
use std::{
    fmt::Display,
    ops::{Index, Range, RangeFrom, RangeFull, RangeTo},
    path::Path,
};
pub use types::TextType;

#[derive(Debug, Default)]
pub struct TextLine {
    content: String,
    char_len: usize,
    cached: RenderStatus,
}

impl EditorLine for TextLine {
    type Context<'a> = Context<'a>;
    type Error = String;

    #[inline]
    fn parse_lines<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Self::Error> {
        Ok(std::fs::read_to_string(path)
            .map_err(|err| err.to_string())?
            .split('\n')
            .map(|line| TextLine::new(line.to_owned()))
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
        let add_char_len = string.char_len();
        self.char_len -= to;
        if self.content.len() == self.char_len {
            self.char_len += add_char_len;
            self.content.replace_range(..to, string);
        } else {
            self.char_len += add_char_len;
            self.content.utf8_replace_till(to, string);
        }
    }

    #[inline]
    fn replace_from(&mut self, from: usize, string: &str) {
        self.cached.reset();
        self.char_len = from + string.char_len();
        if self.content.len() == self.char_len {
            self.content.truncate(from);
            self.content.push_str(string);
        } else {
            self.content.utf8_replace_from(from, string);
        }
    }

    #[inline]
    fn replace_range(&mut self, range: Range<usize>, string: &str) {
        self.cached.reset();
        self.char_len -= range.len();
        self.char_len += string.char_len();
        if self.char_len == self.content.len() {
            self.content.replace_range(range, string);
        } else {
            self.content.utf8_replace_range(range, string);
        }
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
        self.char_len += 1;
        if self.char_len == self.content.len() {
            self.content.insert(idx, ch);
        } else {
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
        let TextLine { content, char_len, .. } = line;
        self.cached.reset();
        self.char_len += char_len;
        self.content.push_str(&content)
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
        let ch = self.content.utf8_remove(idx);
        self.char_len -= 1;
        ch
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
        self.content.clear();
        self.cached.reset();
        self.char_len = 0;
    }

    #[inline]
    fn split_off(&mut self, at: usize) -> Self {
        self.cached.reset();
        if self.content.len() == self.char_len {
            let content = self.content.split_off(at);
            self.char_len = self.content.len();
            return Self { char_len: content.len(), content, ..Default::default() };
        }
        let content = self.content.utf8_split_off(at);
        self.char_len = self.content.char_len();
        Self::new(content)
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
}

impl TextLine {
    pub fn new(content: String) -> Self {
        Self { char_len: content.char_len(), content, cached: RenderStatus::None }
    }

    pub fn render<'a>(
        self_iter: impl Iterator<Item = &'a mut Self>,
        mut lines: RectIter,
        mut ctx: Context,
        backend: &mut Backend,
    ) {
        for (idx, text_line) in self_iter.enumerate().skip(ctx.line_number) {
            if idx == ctx.line {
                text_line.cursor(&mut lines, &mut ctx, backend);
            } else {
                text_line.render_line(&mut lines, &mut ctx, backend);
            }
        }

        for remaining in lines {
            remaining.render_empty(backend);
        }
    }

    pub fn render_line(&mut self, lines: &mut RectIter, ctx: &mut Context, backend: &mut Backend) {
        let line = match lines.next() {
            Some(line) => line,
            None => return,
        };
        let select = ctx.get_select(line.width);
        let width = ctx.setup_line(line, backend);
        match select {
            Some(select) => {
                todo!()
            }
            None => {
                if self.is_simple() {
                    let mut start = 0;
                    loop {
                        let end = start + width;
                        if end >= self.content.len() {
                            backend.print(unsafe { self.content.get_unchecked(start..) });
                            let remaing_width = self.content.len() - start;
                            if remaing_width != 0 {
                                backend.pad(remaing_width);
                            }
                            return;
                        }
                        backend.print(unsafe { self.content.get_unchecked(start..end) });
                        if ctx.skip_line(lines, backend).is_none() {
                            return;
                        }
                        start = end;
                    }
                } else {
                    for (chunk_width, chunk) in WriteChunks::new(&self.content, width) {
                        backend.print(chunk);
                        let remaing_width = width - chunk_width;
                        if remaing_width != 0 {
                            backend.pad(remaing_width);
                        }
                        if ctx.skip_line(lines, backend).is_none() {
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn cursor(&mut self, lines: &mut RectIter, ctx: &mut Context, backend: &mut Backend) {
        let line = match lines.next() {
            Some(line) => line,
            None => return,
        };
        let select = ctx.get_select(line.width);
        let mut width = ctx.setup_line(line, backend);
        let char = ctx.get_char();
        match select {
            Some(select) => {
                if self.is_simple() {
                    for (idx, ch) in self.content.chars().enumerate() {
                        if width == 0 {
                            match ctx.skip_line(lines, backend) {
                                Some(new_width) => width = new_width,
                                None => return,
                            }
                        }
                        if select.start == idx {
                            backend.set_style(ctx.select_style());
                        }
                        if select.end == idx {
                            backend.reset_style();
                        }
                        width -= 1;
                        if idx == char {
                            backend.print_styled(ch, Style::reversed());
                        } else {
                            backend.print(ch);
                        }
                    }
                } else {
                    for (idx, ch) in self.content.chars().enumerate() {
                        if width == 0 {
                            match ctx.skip_line(lines, backend) {
                                Some(new_width) => width = new_width,
                                None => return,
                            }
                        }
                        if select.start == idx {
                            backend.set_style(ctx.select_style());
                        }
                        if select.end == idx {
                            backend.reset_style();
                        }
                        match UnicodeWidthChar::width(ch) {
                            Some(w) => width -= w,
                            None => continue,
                        }
                        if idx == char {
                            backend.print_styled(ch, Style::reversed());
                        } else {
                            backend.print(ch);
                        }
                    }
                }
            }
            None => {
                if self.is_simple() {
                    for (idx, ch) in self.content.chars().enumerate() {
                        if width == 0 {
                            match lines.move_cursor(backend) {
                                Some(new_width) => width = new_width,
                                None => return,
                            }
                        }
                        width -= 1;
                        if idx == char {
                            backend.print_styled(ch, Style::reversed());
                        } else {
                            backend.print(ch);
                        }
                    }
                } else {
                    for (idx, ch) in self.content.chars().enumerate() {
                        if width == 0 {
                            match lines.move_cursor(backend) {
                                Some(new_width) => width = new_width,
                                None => return,
                            }
                        }
                        match UnicodeWidthChar::width(ch) {
                            Some(w) => width -= w,
                            None => continue,
                        }
                        if idx == char {
                            backend.print_styled(ch, Style::reversed());
                        } else {
                            backend.print(ch);
                        }
                    }
                }
            }
        }
    }

    pub fn clear_cache(&mut self) {
        self.cached.reset();
    }
}

impl Display for TextLine {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.content.fmt(f)
    }
}

impl From<String> for TextLine {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl From<&'static str> for TextLine {
    fn from(value: &'static str) -> Self {
        value.to_owned().into()
    }
}

impl Index<Range<usize>> for TextLine {
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

impl Index<RangeTo<usize>> for TextLine {
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

impl Index<RangeFrom<usize>> for TextLine {
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

impl Index<RangeFull> for TextLine {
    type Output = str;
    fn index(&self, _: RangeFull) -> &Self::Output {
        &self.content
    }
}

impl From<TextLine> for String {
    fn from(val: TextLine) -> Self {
        val.content
    }
}
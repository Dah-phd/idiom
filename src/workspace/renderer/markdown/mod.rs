mod ascii;
mod complex;

use std::ops::Range;

use crossterm::style::{Attribute, Attributes, Color, ContentStyle, Stylize};
use markdown::{tokenize, Block, ListItem, Span};
use unicode_width::UnicodeWidthChar;

use crate::{
    render::{
        backend::{Backend, BackendProtocol},
        layout::{IterLines, RectIter},
    },
    syntax::tokens::{calc_wrap_line, calc_wrap_line_capped},
    workspace::{
        cursor::Cursor,
        line::{EditorLine, LineContext},
    },
};

const HEADING: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkRed),
    background_color: None,
    underline_color: Some(Color::DarkRed),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_2: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkBlue),
    background_color: None,
    underline_color: Some(Color::DarkBlue),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_3: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkGreen),
    background_color: None,
    underline_color: Some(Color::DarkGreen),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_NEXT: ContentStyle = ContentStyle {
    foreground_color: None,
    background_color: None,
    underline_color: None,
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

struct StyledParser<'a, 'b> {
    lines: &'a mut RectIter,
    ctx: &'a mut LineContext<'b>,
    line_width: usize,
    backend: &'a mut Backend,
    wrap_printer: fn(&str, usize, usize, &mut RectIter, &mut LineContext, &mut Backend) -> Option<usize>,
}

impl<'a, 'b> StyledParser<'a, 'b> {
    fn new_ascii(lines: &'a mut RectIter, ctx: &'a mut LineContext<'b>, backend: &'a mut Backend) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split })
    }

    fn new_complex(lines: &'a mut RectIter, ctx: &'a mut LineContext<'b>, backend: &'a mut Backend) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split_comp })
    }

    fn render(mut self, content: &str) {
        let mut limit = self.line_width;
        for block in tokenize(content) {
            match self.print_block(block, limit) {
                Some(remining) => limit = remining,
                None => return,
            }
        }
    }

    fn print_block(&mut self, block: Block, mut limit: usize) -> Option<usize> {
        match block {
            Block::Header(header, level) => {
                for span in header {
                    match level {
                        1 => self.backend.set_style(HEADING),
                        2 => self.backend.set_style(HEADING_2),
                        3 => self.backend.set_style(HEADING_3),
                        _ => self.backend.set_style(HEADING_NEXT),
                    }
                    limit = self.print_span(span, limit)?;
                }
            }
            Block::Paragraph(parag) => {
                for span in parag {
                    limit = self.print_span(span, limit)?;
                }
            }
            Block::Hr => {
                self.backend.print((0..limit).map(|_| '-').collect::<String>());
                limit = 0;
            }
            Block::CodeBlock(x, y) => {
                limit = (self.wrap_printer)(
                    &format!(" X X X {x:?} {y}"),
                    limit,
                    self.line_width,
                    self.lines,
                    self.ctx,
                    self.backend,
                )?;
            }
            Block::Raw(text) => {
                limit = (self.wrap_printer)(&text, limit, self.line_width, self.lines, self.ctx, self.backend)?;
            }
            Block::OrderedList(items, list_type) => {
                limit = (self.wrap_printer)(
                    &format!(" {}.", list_type.0),
                    limit,
                    self.line_width,
                    self.lines,
                    self.ctx,
                    self.backend,
                )?;
                for item in items {
                    limit = self.print_list_item(item, limit)?;
                }
            }
            Block::UnorderedList(items) => {
                limit = (self.wrap_printer)(" > ", limit, self.line_width, self.lines, self.ctx, self.backend)?;
                for item in items {
                    limit = self.print_list_item(item, limit)?;
                }
            }
            Block::Blockquote(blocks) => {
                for block in blocks {
                    limit = self.print_block(block, limit)?;
                }
            }
        }
        Some(limit)
    }

    fn print_list_item(&mut self, item: ListItem, mut limit: usize) -> Option<usize> {
        match item {
            ListItem::Simple(spans) => {
                for span in spans {
                    limit = self.print_span(span, limit)?;
                }
            }
            ListItem::Paragraph(parag) => {
                for block in parag {
                    limit = self.print_block(block, limit)?;
                }
            }
        }
        Some(limit)
    }

    fn print_span(&mut self, span: Span, mut limit: usize) -> Option<usize> {
        match span {
            Span::Emphasis(spans) => {
                let style = self.backend.get_style();
                self.backend.set_style(style.italic());
                for span in spans {
                    limit = self.print_span(span, limit)?;
                }
                self.backend.set_style(style);
            }
            Span::Text(text) => {
                limit = (self.wrap_printer)(&text, limit, self.line_width, self.lines, self.ctx, self.backend)?
            }
            Span::Strong(spans) => {
                let style = self.backend.get_style();
                self.backend.set_style(style.bold());
                for span in spans {
                    limit = self.print_span(span, limit)?;
                }
                self.backend.set_style(style);
            }
            Span::Code(text) => {
                limit = match text.as_str() {
                    "`" => (self.wrap_printer)(">>> ", limit, self.line_width, self.lines, self.ctx, self.backend)?,
                    _ => (self.wrap_printer)(&text, limit, self.line_width, self.lines, self.ctx, self.backend)?,
                }
            }
            Span::Image(name, path, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)("Image", limit, self.line_width, self.lines, self.ctx, self.backend)?,
                    false => (self.wrap_printer)(&name, limit, self.line_width, self.lines, self.ctx, self.backend)?,
                };
                limit = (self.wrap_printer)(" > ", limit, self.line_width, self.lines, self.ctx, self.backend)?;
                limit = (self.wrap_printer)(&path, limit, self.line_width, self.lines, self.ctx, self.backend)?;
            }
            Span::Link(name, link, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)("Link", limit, self.line_width, self.lines, self.ctx, self.backend)?,
                    false => (self.wrap_printer)(&name, limit, self.line_width, self.lines, self.ctx, self.backend)?,
                };
                limit = (self.wrap_printer)(" > ", limit, self.line_width, self.lines, self.ctx, self.backend)?;
                limit = (self.wrap_printer)(&link, limit, self.line_width, self.lines, self.ctx, self.backend)?;
            }
            Span::Break => {
                let line = self.lines.next()?;
                self.ctx.wrap_line(line, self.backend);
                limit = self.line_width;
            }
        }
        Some(limit)
    }
}

fn print_split(
    text: &str,
    limit: usize,
    line_width: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) -> Option<usize> {
    match text.len() > limit {
        true => {
            let (first, mut text) = text.split_at(limit);
            backend.print(first);
            loop {
                let next_line = lines.next()?;
                ctx.wrap_line(next_line, backend);
                match text.len() > line_width {
                    true => {
                        let (part, remaining) = text.split_at(line_width);
                        text = remaining;
                        backend.print(part);
                    }
                    false => {
                        backend.print(text);
                        return Some(line_width - text.len());
                    }
                }
            }
        }
        false => {
            backend.print(text);
            Some(limit - text.len())
        }
    }
}

pub fn print_split_comp(
    text: &str,
    mut limit: usize,
    line_width: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) -> Option<usize> {
    for ch in text.chars() {
        if let Some(ch_width) = ch.width() {
            match ch_width > limit {
                true => {
                    if ch_width > line_width {
                        continue; // ensure no strange chars are printed
                    }
                    let line = lines.next()?;
                    ctx.wrap_line(line, backend);
                    backend.print(ch);
                    limit = line_width - ch_width;
                }
                false => {
                    backend.print(ch);
                    limit -= ch_width;
                }
            }
        }
    }
    Some(limit)
}

pub fn repositioning(cursor: &mut Cursor, content: &mut [EditorLine]) -> Option<usize> {
    if let Some(skipped) = calc_wrap_line_capped(&mut content[cursor.line], cursor) {
        cursor.at_line = cursor.line;
        return Some(skipped);
    };
    if cursor.at_line > cursor.line {
        cursor.at_line = cursor.line;
        return None;
    }
    let mut row_sum = calc_rows(content, cursor);
    while row_sum > cursor.max_rows {
        if cursor.at_line == cursor.line {
            return None;
        }
        row_sum -= 1 + content[cursor.at_line].tokens.char_len();
        cursor.at_line += 1;
    }
    None
}

fn calc_rows(content: &mut [EditorLine], cursor: &Cursor) -> usize {
    let take = (cursor.line + 1) - cursor.at_line;
    let text_width = cursor.text_width;
    let mut buf = 0;
    for (idx, text) in content.iter_mut().enumerate().skip(cursor.at_line).take(take) {
        if idx != cursor.line {
            calc_wrap_line(text, text_width);
        }
        buf += 1 + text.tokens.char_len();
    }
    buf
}

#[inline(always)]
pub fn cursor(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    skip: usize,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    backend: &mut Backend,
) {
    text.cached.cursor(lines.next_line_idx(), ctx.cursor_char(), skip, select.clone());
    match text.is_simple() {
        true => ascii::cursor(text, select, skip, lines, ctx, backend),
        false => complex::cursor(text, select, skip, lines, ctx, backend),
    }
}

#[inline(always)]
pub fn line(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    backend: &mut Backend,
) {
    text.cached.line(lines.next_line_idx(), select.clone());
    match text.is_simple() {
        true => match select {
            Some(select) => ascii::line_with_select(text, select, lines, ctx, backend),
            None => ascii::line(text, lines, ctx, backend),
        },
        false => match select {
            Some(select) => complex::line_with_select(text, select, lines, ctx, backend),
            None => complex::line(text, lines, ctx, backend),
        },
    }
}

#[cfg(test)]
mod tests;

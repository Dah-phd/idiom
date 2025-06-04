mod ascii;
mod complex;
mod parser;

use std::ops::Range;

use crossterm::style::{Attribute, Attributes, Color, ContentStyle, Stylize};
use parser::{parse, Block, ListItem, Span};

use crate::{
    global_state::CrossTerm,
    syntax::tokens::{calc_wrap_line, calc_wrap_line_capped},
    workspace::{
        cursor::Cursor,
        line::{EditorLine, LineContext},
    },
};

use idiom_ui::{
    backend::Backend,
    layout::{IterLines, RectIter},
    utils::CharLimitedWidths,
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
    backend: &'a mut CrossTerm,
    wrap_printer: fn(&mut Self, &str, usize) -> Option<usize>, //,usize, &mut RectIter, &mut LineContext, &mut Backend) -> Option<usize>,
}

impl<'a, 'b> StyledParser<'a, 'b> {
    fn new_ascii(lines: &'a mut RectIter, ctx: &'a mut LineContext<'b>, backend: &'a mut CrossTerm) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split })
    }

    fn new_complex(lines: &'a mut RectIter, ctx: &'a mut LineContext<'b>, backend: &'a mut CrossTerm) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split_comp })
    }

    fn render(mut self, content: &str) {
        let mut limit = self.line_width;
        for block in parse(content) {
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
                limit = (self.wrap_printer)(self, &format!(" X X X {x:?} {y}"), limit)?;
            }
            Block::OrderedList(items, list_type) => {
                limit = (self.wrap_printer)(self, &format!(" {}.", list_type.0), limit)?;
                for item in items {
                    limit = self.print_list_item(item, limit)?;
                }
            }
            Block::UnorderedList(items) => {
                limit = (self.wrap_printer)(self, " > ", limit)?;
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
            Span::Text(text) => limit = (self.wrap_printer)(self, &text, limit)?,
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
                    "`" => (self.wrap_printer)(self, ">>> ", limit)?,
                    _ => (self.wrap_printer)(self, &text, limit)?,
                }
            }
            Span::Image(name, path, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)(self, "Image", limit)?,
                    false => (self.wrap_printer)(self, &name, limit)?,
                };
                limit = (self.wrap_printer)(self, " > ", limit)?;
                limit = (self.wrap_printer)(self, &path, limit)?;
            }
            Span::Link(name, link, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)(self, "Link", limit)?,
                    false => (self.wrap_printer)(self, &name, limit)?,
                };
                limit = (self.wrap_printer)(self, " > ", limit)?;
                limit = (self.wrap_printer)(self, &link, limit)?;
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

fn print_split(parser: &mut StyledParser, text: &str, limit: usize) -> Option<usize> {
    match text.len() > limit {
        true => {
            let (first, mut text) = text.split_at(limit);
            parser.backend.print(first);
            loop {
                let next_line = parser.lines.next()?;
                parser.ctx.wrap_line(next_line, parser.backend);
                match text.len() > parser.line_width {
                    true => {
                        let (part, remaining) = text.split_at(parser.line_width);
                        text = remaining;
                        parser.backend.print(part);
                    }
                    false => {
                        parser.backend.print(text);
                        return Some(parser.line_width - text.len());
                    }
                }
            }
        }
        false => {
            parser.backend.print(text);
            Some(limit - text.len())
        }
    }
}

fn print_split_comp(parser: &mut StyledParser, text: &str, mut limit: usize) -> Option<usize> {
    for (ch, ch_width) in CharLimitedWidths::new(text, 3) {
        match ch_width > limit {
            true => {
                let line = parser.lines.next()?;
                parser.ctx.wrap_line(line, parser.backend);
                parser.backend.print(ch);
                limit = parser.line_width - ch_width;
            }
            false => {
                parser.backend.print(ch);
                limit -= ch_width;
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
    backend: &mut impl Backend,
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
    backend: &mut CrossTerm,
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

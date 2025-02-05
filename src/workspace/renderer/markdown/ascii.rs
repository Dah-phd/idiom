use crate::{
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        layout::RectIter,
    },
    workspace::{
        line::{EditorLine, LineContext},
        renderer::markdown::{HEADING, HEADING_2, HEADING_3, HEADING_NEXT},
    },
};
use crossterm::style::{ContentStyle, Stylize};
use markdown::{tokenize, Block, ListItem, Span};
use std::ops::Range;

fn print_block(
    block: Block,
    lines: &mut RectIter,
    mut limit: usize,
    line_width: usize,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) -> Option<usize> {
    match block {
        Block::Header(header, level) => {
            for span in header {
                match level {
                    1 => backend.set_style(HEADING),
                    2 => backend.set_style(HEADING_2),
                    3 => backend.set_style(HEADING_3),
                    _ => backend.set_style(HEADING_NEXT),
                }
                limit = print_span(span, lines, limit, line_width, ctx, backend)?;
            }
        }
        Block::Paragraph(parag) => {
            for span in parag {
                limit = print_span(span, lines, limit, line_width, ctx, backend)?;
            }
        }
        Block::Hr => {
            backend.print((0..limit).map(|_| '-').collect::<String>());
            limit = 0;
        }
        Block::CodeBlock(x, y) => {
            limit = print_split(&format!(" X X X {x:?} {y}"), limit, line_width, lines, ctx, backend)?;
        }
        Block::Raw(text) => {
            limit = print_split(&text, limit, line_width, lines, ctx, backend)?;
        }
        Block::OrderedList(items, list_type) => {
            limit = print_split(&format!(" {}.", list_type.0), limit, line_width, lines, ctx, backend)?;
            for item in items {
                limit = print_list_item(item, lines, limit, line_width, ctx, backend)?;
            }
        }
        Block::UnorderedList(items) => {
            limit = print_split(" > ", limit, line_width, lines, ctx, backend)?;
            for item in items {
                limit = print_list_item(item, lines, limit, line_width, ctx, backend)?;
            }
        }
        Block::Blockquote(blocks) => {
            for block in blocks {
                limit = print_block(block, lines, limit, line_width, ctx, backend)?;
            }
        }
    }
    Some(limit)
}

fn print_list_item(
    item: ListItem,
    lines: &mut RectIter,
    mut limit: usize,
    line_width: usize,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) -> Option<usize> {
    match item {
        ListItem::Simple(spans) => {
            for span in spans {
                limit = print_span(span, lines, limit, line_width, ctx, backend)?;
            }
        }
        ListItem::Paragraph(parag) => {
            for block in parag {
                limit = print_block(block, lines, limit, line_width, ctx, backend)?;
            }
        }
    }
    Some(limit)
}

fn print_span(
    span: Span,
    lines: &mut RectIter,
    mut limit: usize,
    line_width: usize,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) -> Option<usize> {
    match span {
        Span::Emphasis(spans) => {
            let style = backend.get_style();
            backend.set_style(style.italic());
            for span in spans {
                limit = print_span(span, lines, limit, line_width, ctx, backend)?;
            }
            backend.set_style(style);
        }
        Span::Text(text) => limit = print_split(&text, limit, line_width, lines, ctx, backend)?,
        Span::Strong(spans) => {
            let style = backend.get_style();
            backend.set_style(style.bold());
            for span in spans {
                limit = print_span(span, lines, limit, line_width, ctx, backend)?;
            }
            backend.set_style(style);
        }
        Span::Code(text) => {
            limit = match text.as_str() {
                "`" => print_split(">>> ", limit, line_width, lines, ctx, backend)?,
                _ => print_split(&text, limit, line_width, lines, ctx, backend)?,
            }
        }
        Span::Image(name, path, _) => {
            limit = match name.is_empty() {
                true => print_split("Image", limit, line_width, lines, ctx, backend)?,
                false => print_split(&name, limit, line_width, lines, ctx, backend)?,
            };
            limit = print_split(" > ", limit, line_width, lines, ctx, backend)?;
            limit = print_split(&path, limit, line_width, lines, ctx, backend)?;
        }
        Span::Link(name, link, _) => {
            limit = match name.is_empty() {
                true => print_split("Link", limit, line_width, lines, ctx, backend)?,
                false => print_split(&name, limit, line_width, lines, ctx, backend)?,
            };
            limit = print_split(" > ", limit, line_width, lines, ctx, backend)?;
            limit = print_split(&link, limit, line_width, lines, ctx, backend)?;
        }
        Span::Break => {
            let line = lines.next()?;
            ctx.wrap_line(line, backend);
            limit = line_width;
        }
    }
    Some(limit)
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

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut impl BackendProtocol) {
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let mut limit = line_width;
    let blocks = tokenize(&text.content);
    for block in blocks {
        match print_block(block, lines, limit, line_width, ctx, backend) {
            None => {
                backend.reset_style();
                return;
            }
            Some(remainig_limit) => {
                limit = remainig_limit;
            }
        }
    }
    backend.reset_style();
}

pub fn line_with_select(
    text: &mut EditorLine,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) {
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    if text.char_len == 0 {
        backend.print_styled(" ", ContentStyle::bg(ctx.lexer.theme.selected));
        return;
    }
    let mut line_end = line_width;
    let select_color = ctx.lexer.theme.selected;
    for (idx, text) in text.content.chars().enumerate() {
        if idx == line_end {
            line_end += line_width;
            match lines.next() {
                Some(line) => {
                    let reset_style = backend.get_style();
                    backend.reset_style();
                    ctx.wrap_line(line, backend);
                    backend.set_style(reset_style)
                }
                None => return,
            }
        }
        if select.start == idx {
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.reset_style();
        }
        backend.print(text);
    }
    backend.reset_style();
}

pub fn cursor(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut Backend,
) {
    match select {
        Some(select) => self::select(text, skip, select, lines, ctx, backend),
        None => self::basic(text, skip, lines, ctx, backend),
    }
}

pub fn basic(text: &mut EditorLine, skip: usize, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut Backend) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;
    for text in text.content.chars().skip(idx) {
        if idx == line_end {
            line_end += line_width;
            match lines.next() {
                Some(line) => {
                    ctx.wrap_line(line, backend);
                }
                None => break,
            }
        }
        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    }
    backend.reset_style();
}

#[inline]
pub fn select(
    text: &mut EditorLine,
    skip: usize,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut Backend,
) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let select_color = ctx.lexer.theme.selected;
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;
    if select.start < idx && idx < select.end {
        backend.set_bg(Some(select_color));
    }
    for text in text.content.chars().skip(idx) {
        if idx == line_end {
            line_end += line_width;
            match lines.next() {
                Some(line) => {
                    let reset_style = backend.get_style();
                    backend.reset_style();
                    ctx.wrap_line(line, backend);
                    backend.set_style(reset_style)
                }
                None => break,
            }
        }
        if select.start == idx {
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    }
    backend.reset_style();
}

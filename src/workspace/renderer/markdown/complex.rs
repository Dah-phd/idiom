use super::{HEADING, HEADING_2, HEADING_3, HEADING_NEXT};
use crate::{
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        layout::RectIter,
    },
    workspace::line::{EditorLine, LineContext},
};
use markdown::{tokenize, Block, ListItem, Span};
use unicode_width::UnicodeWidthChar;

use crossterm::style::{ContentStyle, Stylize};
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

pub fn print_split(
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
    let mut remaining_width = line_width;
    let select_color = ctx.lexer.theme.selected;
    for (idx, text) in text.content.chars().enumerate() {
        let current_width = UnicodeWidthChar::width(text).unwrap_or_default();
        if remaining_width < current_width {
            remaining_width = line_width;
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
        remaining_width -= current_width;
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
        Some(select) => self::select(text, select, skip, lines, ctx, backend),
        None => self::basic(text, skip, lines, ctx, backend),
    }
}

pub fn basic(
    text: &mut EditorLine,
    mut skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut Backend,
) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let mut content = text.content.chars();
    let mut idx = 0;
    let mut remaining_width = line_width;

    if skip != 0 {
        for ch in content.by_ref() {
            idx += 1;
            let char_w = UnicodeWidthChar::width(ch).unwrap_or_default();
            if remaining_width < char_w {
                remaining_width = line_width - char_w;
                skip -= 1;
                if skip == 0 {
                    backend.print(ch);
                    break;
                }
            } else {
                remaining_width -= char_w;
            }
        }
    };

    for text in content {
        let current_width = UnicodeWidthChar::width(text).unwrap_or_default();
        if remaining_width < current_width {
            remaining_width = line_width;
            match lines.next() {
                Some(line) => ctx.wrap_line(line, backend),
                None => break,
            }
        }
        remaining_width -= current_width;
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
    select: Range<usize>,
    mut skip: usize,
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
    let mut content = text.content.chars();
    let mut idx = 0;
    let mut remaining_width = line_width;

    if skip != 0 {
        for ch in content.by_ref() {
            idx += 1;
            let char_w = UnicodeWidthChar::width(ch).unwrap_or_default();
            if remaining_width < char_w {
                remaining_width = line_width - char_w;
                skip -= 1;
                if skip == 0 {
                    if idx > select.start && select.end > idx {
                        backend.set_bg(Some(select_color));
                    }
                    backend.print(ch);
                    break;
                }
            } else {
                remaining_width -= char_w;
            }
        }
    }

    for text in content {
        let current_width = UnicodeWidthChar::width(text).unwrap_or_default();
        if remaining_width < current_width {
            remaining_width = line_width;
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
        remaining_width -= current_width;
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

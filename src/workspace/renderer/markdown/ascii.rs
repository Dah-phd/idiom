use super::StyledParser;
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::RectIter, Backend};
use std::ops::Range;

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if let Some(parser) = StyledParser::new_ascii(lines, ctx, backend) {
        parser.render(&text.content);
    }
    backend.reset_style();
}

pub fn line_with_select(
    text: &mut EditorLine,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut CrossTerm,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);

    if text.char_len == 0 {
        backend.print_styled(" ", ContentStyle::bg(ctx.lexer.theme.selected));
        return;
    }

    let mut line_end = line_width;
    let select_color = ctx.lexer.theme.selected;

    for (idx, text) in text.content.chars().enumerate() {
        if idx == line_end {
            let Some(line) = lines.next() else { return };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            line_end += line_width;
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
    backend: &mut CrossTerm,
) {
    match select {
        Some(select) => self::select(text, skip, select, lines, ctx, backend),
        None => self::basic(text, skip, lines, ctx, backend),
    }
}

pub fn basic(text: &mut EditorLine, skip: usize, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;

    for text in text.content.chars().skip(idx) {
        if idx == line_end {
            let Some(line) = lines.next() else { break };
            ctx.wrap_line(line, backend);
            line_end += line_width;
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
    backend: &mut CrossTerm,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let select_color = ctx.lexer.theme.selected;
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;

    if select.start < idx && idx < select.end {
        backend.set_bg(Some(select_color));
    }

    for text in text.content.chars().skip(idx) {
        if idx == line_end {
            let Some(line) = lines.next() else { break };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            line_end += line_width;
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

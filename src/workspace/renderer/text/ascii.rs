use crate::{
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        layout::RectIter,
        utils::ByteChunks,
    },
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::ContentStyle;
use std::ops::Range;

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut impl BackendProtocol) {
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let mut chunks = ByteChunks::new(&text.content, line_width);
    match chunks.next() {
        Some(chunk) => backend.print(chunk.text),
        None => return,
    }
    for chunk in chunks {
        match lines.next() {
            None => return,
            Some(line) => {
                ctx.wrap_line(line, backend);
            }
        }
        backend.print(chunk.text);
    }
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

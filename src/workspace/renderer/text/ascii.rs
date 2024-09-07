use crate::{
    render::{
        backend::{Backend, BackendProtocol, Style},
        layout::RectIter,
    },
    workspace::line::{EditorLine, LineContext},
};
use std::ops::Range;

pub fn ascii_line(
    text: &mut EditorLine,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) {
    let line_width = match lines.next() {
        Some(line) => {
            text.cached.line(line.row, None);
            ctx.setup_line(line, backend)
        }
        None => return,
    };
    let mut start = 0;
    while let Some(text_chunk) = text.content.get(start..start + line_width) {
        backend.print(text_chunk);
        start += line_width;
        match lines.next() {
            Some(line) => ctx.wrap_line(line, backend),
            None => return,
        }
    }
    if let Some(last_chunk) = text.content.get(start..) {
        backend.print(last_chunk);
    }
}

pub fn ascii_line_with_select(
    text: &mut EditorLine,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) {
    let line_width = match lines.next() {
        Some(line) => {
            text.cached.line(line.row, None);
            ctx.setup_line(line, backend)
        }
        None => return,
    };
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
            backend.set_bg(None);
        }
        backend.print(text);
    }
}

pub fn render(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut Backend) {
    match ctx.get_select(text.char_len()) {
        Some(select) => self::select(text, select, lines, ctx, backend),
        None => self::basic(text, lines, ctx, backend),
    }
}

pub fn basic(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut Backend) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => {
            text.cached.cursor(line.row, cursor_idx, 0, None);
            ctx.setup_line(line, backend)
        }
        None => return,
    };
    let mut line_end = line_width;
    let mut idx = 0;
    for text in text.chars() {
        if idx == line_end {
            line_end += line_width;
            match lines.next() {
                Some(line) => ctx.wrap_line(line, backend),
                None => return,
            }
        }
        if cursor_idx == idx {
            backend.print_styled(text, Style::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

#[inline]
pub fn select(
    text: &mut EditorLine,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut Backend,
) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => {
            text.cached.cursor(line.row, cursor_idx, 0, Some(select.clone()));
            ctx.setup_line(line, backend)
        }
        None => return,
    };
    let select_color = ctx.lexer.theme.selected;
    let mut line_end = line_width;
    let mut idx = 0;
    for text in text.chars() {
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
            backend.set_bg(None);
        }

        if cursor_idx == idx {
            backend.print_styled(text, Style::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

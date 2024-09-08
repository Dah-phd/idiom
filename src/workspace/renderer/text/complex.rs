use unicode_width::UnicodeWidthChar;

use crate::{
    render::{
        backend::{Backend, BackendProtocol, Style},
        layout::RectIter,
        utils::WriteChunks,
    },
    workspace::line::{EditorLine, LineContext},
};
use std::ops::Range;

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut impl BackendProtocol) {
    let line_width = match lines.next() {
        Some(line) => {
            text.cached.line(line.row, None);
            ctx.setup_line(line, backend)
        }
        None => return,
    };
    let mut chunks = WriteChunks::new(&text.content, line_width);
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
        Some(line) => {
            text.cached.line(line.row, None);
            ctx.setup_line(line, backend)
        }
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
            backend.set_bg(None);
        }
        backend.print(text);
    }
}

pub fn cursor(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut Backend) {
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
    let mut remaining_width = line_width;
    let mut idx = 0;
    for text in text.content.chars() {
        let current_width = UnicodeWidthChar::width(text).unwrap_or_default();
        if remaining_width < current_width {
            remaining_width = line_width;
            match lines.next() {
                Some(line) => ctx.wrap_line(line, backend),
                None => return,
            }
        }
        remaining_width -= current_width;
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
    let mut remaining_width = line_width;
    let mut idx = 0;
    for text in text.content.chars() {
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

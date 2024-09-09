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
        Some(line) => ctx.setup_line(line, backend),
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

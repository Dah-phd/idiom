use std::ops::Range;

use crate::{
    render::{
        backend::{color, Backend, BackendProtocol, Color, Style},
        layout::RectIter,
    },
    syntax::Token,
    workspace::line::{Context, EditorLine, WrappedCursor},
};

#[inline]
pub fn basic(line: &impl EditorLine, ctx: &impl Context, backend: &mut Backend) {
    let mut iter_tokens = line.iter_tokens();
    let mut maybe_token = iter_tokens.next();
    let mut idx = 0;
    let cursor_idx = ctx.cursor_char();
    for text in line.chars() {
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.set_style(token.style);
            } else if token.to == idx {
                if let Some(token) = iter_tokens.next() {
                    if token.from == idx {
                        backend.set_style(token.style);
                    } else {
                        backend.reset_style();
                    };
                    maybe_token.replace(token);
                } else {
                    backend.reset_style();
                    maybe_token = None;
                };
            };
        }
        if idx == cursor_idx {
            backend.print_styled(text, Style::reversed());
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
pub fn with_select(line: &impl EditorLine, ctx: &impl Context, select: Range<usize>, backend: &mut Backend) {
    let mut reset_style = Style::default();
    let mut iter_tokens = line.iter_tokens();
    let mut maybe_token = iter_tokens.next();
    let mut idx = 0;
    let select_color = ctx.lexer().theme.selected;
    let cursor_idx = ctx.cursor_char();
    for text in line.chars() {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.update_style(token.style);
            } else if token.to == idx {
                if let Some(token) = iter_tokens.next() {
                    if token.from == idx {
                        backend.update_style(token.style);
                    } else {
                        backend.set_style(reset_style);
                    };
                    maybe_token.replace(token);
                } else {
                    backend.set_style(reset_style);
                    maybe_token = None;
                };
            };
        }
        if idx == cursor_idx {
            backend.print_styled(text, Style::reversed());
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
pub fn wrap(
    line: &impl EditorLine,
    ctx: &mut impl Context,
    line_width: usize,
    lines: &mut RectIter,
    backend: &mut Backend,
) {
    let wrap_number = ctx.setup_wrap();
    let wrap_cursor = ctx.count_skipped_to_cursor(line_width, lines.len());
    if wrap_cursor.skip_lines != 0 {
        let mut wrap_text = format!("..{} hidden wrapped lines", wrap_cursor.skip_lines);
        wrap_text.truncate(line_width);
        backend.print_styled(wrap_text, Style::reversed());
        let line_end = wrap_cursor.skip_chars;
        let mut tokens = line.iter_tokens().skip_while(|token| token.to < line_end).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < line_end {
                backend.set_style(token.style);
            }
        };
        wrapping_loop(line.chars(), backend, tokens, &wrap_number, line_end, wrap_cursor, line_width, lines)
    } else {
        let tokens = line.iter_tokens();
        wrapping_loop(line.chars(), backend, tokens, &wrap_number, line_width, wrap_cursor, line_width, lines)
    };
}

#[inline]
pub fn wrap_select(
    line: &impl EditorLine,
    ctx: &mut impl Context,
    line_width: usize,
    lines: &mut RectIter,
    select: Range<usize>,
    backend: &mut Backend,
) {
    let wrap_number = ctx.setup_wrap();
    let wrap_cursor = ctx.count_skipped_to_cursor(line_width, lines.len());
    if wrap_cursor.skip_lines != 0 {
        let mut wrap_text = format!("..{} hidden wrapped lines", wrap_cursor.skip_lines);
        wrap_text.truncate(line_width);
        backend.print_styled(wrap_text, Style::reversed());
        let mut tokens = line.iter_tokens().skip_while(|token| token.to < wrap_cursor.skip_chars).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < wrap_cursor.skip_chars {
                backend.set_style(token.style);
            }
        };
        let reset_style = if select.start < wrap_cursor.skip_chars && select.end > wrap_cursor.skip_chars {
            backend.set_bg(Some(ctx.lexer().theme.selected));
            Style::bg(ctx.lexer().theme.selected)
        } else {
            Style::default()
        };
        wrapping_loop_select(
            line.char_indices().skip(wrap_cursor.skip_chars),
            backend,
            tokens,
            (&wrap_number, line_width),
            wrap_cursor.flat_char_idx,
            (select, ctx.lexer().theme.selected),
            lines,
            reset_style,
        )
    } else {
        wrapping_loop_select(
            line.char_indices(),
            backend,
            line.iter_tokens(),
            (&wrap_number, line_width),
            wrap_cursor.flat_char_idx,
            (select, ctx.lexer().theme.selected),
            lines,
            Style::default(),
        )
    };
}

#[inline(always)]
fn wrapping_loop<'a>(
    content: impl Iterator<Item = char>,
    backend: &mut Backend,
    mut tokens: impl Iterator<Item = &'a Token>,
    wrap_number: &str,
    mut line_end: usize,
    wrap_cursor: WrappedCursor,
    wrap_len: usize,
    lines: &mut RectIter,
) {
    let mut maybe_token = tokens.next();
    let cursor_idx = wrap_cursor.flat_char_idx;
    let mut idx = wrap_cursor.skip_chars;
    for text in content.skip(idx) {
        if line_end == idx {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            backend.print_styled_at(line.row, line.col, wrap_number, Style::fg(color::dark_grey()));
            backend.clear_to_eol();
            line_end += wrap_len;
        }
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.set_style(token.style);
            } else if token.to == idx {
                if let Some(token) = tokens.next() {
                    if token.from == idx {
                        backend.set_style(token.style);
                    } else {
                        backend.reset_style();
                    };
                    maybe_token.replace(token);
                } else {
                    backend.reset_style();
                    maybe_token = None;
                };
            };
        }
        if cursor_idx == idx {
            backend.print_styled(text, Style::reversed());
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
}

#[inline(always)]
fn wrapping_loop_select<'a>(
    content: impl Iterator<Item = (usize, char)>,
    backend: &mut Backend,
    mut tokens: impl Iterator<Item = &'a Token>,
    (wrap_number, wrap_len): (&str, usize),
    cursor_idx: usize,
    (select, select_color): (Range<usize>, Color),
    lines: &mut RectIter,
    mut reset_style: Style,
) {
    let mut maybe_token = tokens.next();
    let mut remaining = wrap_len;
    for (idx, text) in content {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
        if remaining == 0 {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            let current_style = backend.get_style();
            backend.reset_style();
            backend.print_styled_at(line.row, line.col, wrap_number, Style::fg(color::dark_grey()));
            backend.clear_to_eol();
            backend.set_style(current_style);
            remaining += wrap_len;
        }
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.update_style(token.style);
            } else if token.to == idx {
                if let Some(token) = tokens.next() {
                    if token.from == idx {
                        backend.update_style(token.style);
                    } else {
                        backend.set_style(reset_style);
                    };
                    maybe_token.replace(token);
                } else {
                    backend.set_style(reset_style);
                    maybe_token = None;
                };
            };
        }
        if cursor_idx == idx {
            backend.print_styled(text, Style::reversed());
        } else {
            backend.print(text);
        }
        remaining -= 1;
    }
}

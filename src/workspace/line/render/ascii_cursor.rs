use std::ops::Range;

use crate::{
    render::{
        backend::{color, Backend, BackendProtocol, Style},
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
        wrapping_loop(line.chars(), tokens, &wrap_number, (line_end, line_width), wrap_cursor, lines, backend)
    } else {
        let tokens = line.iter_tokens();
        wrapping_loop(line.chars(), tokens, &wrap_number, (line_width, line_width), wrap_cursor, lines, backend)
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
    let wrap_cursor = ctx.count_skipped_to_cursor(line_width, lines.len());
    if wrap_cursor.skip_lines != 0 {
        let mut wrap_text = format!("..{} hidden wrapped lines", wrap_cursor.skip_lines);
        wrap_text.truncate(line_width);
        backend.print_styled(wrap_text, Style::reversed());
        let line_end = wrap_cursor.skip_chars;
        let mut tokens = line.iter_tokens().skip_while(|token| token.to < line_end).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < wrap_cursor.skip_chars {
                backend.set_style(token.style);
            }
        };
        let reset_style = if select.start < wrap_cursor.skip_chars && select.end > line_end {
            backend.set_bg(Some(ctx.lexer().theme.selected));
            Style::bg(ctx.lexer().theme.selected)
        } else {
            Style::default()
        };
        let position_data = (line_end, line_width, select, wrap_cursor);
        wrapping_loop_select(line.chars(), tokens, ctx, position_data, reset_style, lines, backend)
    } else {
        let position_data = (line_width, line_width, select, wrap_cursor);
        wrapping_loop_select(line.chars(), line.iter_tokens(), ctx, position_data, Style::default(), lines, backend)
    };
}

#[inline(always)]
fn wrapping_loop<'a>(
    content: impl Iterator<Item = char>,
    mut tokens: impl Iterator<Item = &'a Token>,
    wrap_number: &str,
    (mut line_end, line_width): (usize, usize),
    wrap_cursor: WrappedCursor,
    lines: &mut RectIter,
    backend: &mut Backend,
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
            line_end += line_width;
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
    content: impl Iterator<Item = char>,
    mut tokens: impl Iterator<Item = &'a Token>,
    ctx: &impl Context,
    (mut line_end, line_width, select, wrap_cursor): (usize, usize, Range<usize>, WrappedCursor),
    mut reset_style: Style,
    lines: &mut RectIter,
    backend: &mut Backend,
) {
    let cursor_idx = wrap_cursor.flat_char_idx;
    let wrap_number = ctx.setup_wrap();
    let mut idx = wrap_cursor.skip_chars;
    let mut maybe_token = tokens.next();
    let select_color = ctx.lexer().theme.selected;
    for text in content.skip(idx) {
        if idx == line_end {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            let current_style = backend.get_style();
            backend.reset_style();
            backend.print_styled_at(line.row, line.col, &wrap_number, Style::fg(color::dark_grey()));
            backend.clear_to_eol();
            backend.set_style(current_style);
            line_end += line_width;
        }
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
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
}

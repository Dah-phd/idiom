use crate::{
    render::{
        backend::{color, Backend, BackendProtocol, Color, Style},
        layout::RectIter,
    },
    syntax::{DiagnosticLine, Lexer, Token},
    workspace::line::Context,
};
use std::ops::Range;

#[inline]
pub fn inline_diagnostics(max_len: usize, diagnostics: &Option<DiagnosticLine>, backend: &mut Backend) {
    if let Some(data) = diagnostics.as_ref().and_then(|d| d.data.first()) {
        backend.print_styled(data.truncated_inline(max_len), Style::fg(data.color));
    };
}

#[inline]
pub fn complex_line(content: impl Iterator<Item = char>, tokens: &[Token], lexer: &Lexer, backend: &mut Backend) {
    let mut iter_tokens = tokens.iter();
    let mut maybe_token = iter_tokens.next();
    let reset_style = Style::default();
    let mut idx = 0;
    for text in content {
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
        backend.print(text);
        idx += (lexer.char_lsp_pos)(text);
    }
    backend.reset_style();
}

#[inline]
pub fn complex_line_with_select(
    content: impl Iterator<Item = char>,
    tokens: &[Token],
    select: Range<usize>,
    lexer: &Lexer,
    backend: &mut Backend,
) {
    let mut iter_tokens = tokens.iter();
    let mut maybe_token = iter_tokens.next();
    let mut reset_style = Style::default();
    let select_color = lexer.theme.selected;
    let mut idx = 0;
    for text in content {
        if select.start == idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
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
        backend.print(text);
        idx += (lexer.char_lsp_pos)(text);
    }
    backend.reset_style();
}

#[inline]
pub fn ascii_line(content: &str, tokens: &[Token], backend: &mut Backend) {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                backend.print(text);
            } else if let Some(text) = content.get(end..) {
                return backend.print(text);
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            backend.print_styled(text, token.style);
        } else if let Some(text) = content.get(token.from..) {
            return backend.print_styled(text, token.style);
        };
        end = token.to;
    }
    match content.get(end..) {
        Some(text) if !text.is_empty() => {
            backend.print(text);
        }
        _ => (),
    }
}

#[inline]
pub fn ascii_line_with_select(
    content: impl Iterator<Item = (usize, char)>,
    tokens: &[Token],
    select: Range<usize>,
    lexer: &Lexer,
    backend: &mut Backend,
) {
    let select_color = lexer.theme.selected;
    let mut iter_tokens = tokens.iter();
    let mut maybe_token = iter_tokens.next();
    let mut reset_style = Style::default();
    for (idx, text) in content {
        if select.start == idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
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
        backend.print(text);
    }
    backend.reset_style();
}

#[inline]
pub fn shrank_line(content: &str, tokens: &[Token], writer: &mut Backend) {
    let mut end = 0;
    for token in tokens.iter() {
        if token.from > end {
            if let Some(text) = content.get(end..token.from) {
                writer.print(text);
            } else if let Some(text) = content.get(end..) {
                writer.print(text);
                break;
            };
        };
        if let Some(text) = content.get(token.from..token.to) {
            writer.print_styled(text, token.style);
        } else if let Some(text) = content.get(token.from..) {
            writer.print_styled(text, token.style);
            break;
        };
        end = token.to;
    }
    writer.print_styled(">>", Style::reversed());
}

/// WRAP

#[inline]
pub fn wrapped_line_select(
    content: &str,
    tokens: &[Token],
    ctx: &mut impl Context,
    wrap_len: usize,
    lines: &mut RectIter,
    select: Range<usize>,
    backend: &mut Backend,
) {
    let wrap_number = ctx.setup_wrap();
    let skip_lines = ctx.count_skipped_to_cursor(wrap_len, lines.len());
    if skip_lines != 0 {
        let mut wrap_text = format!("..{skip_lines} hidden wrapped lines");
        wrap_text.truncate(wrap_len);
        backend.print_styled(wrap_text, Style::reversed());
        let line_end = wrap_len * skip_lines;
        let mut tokens = tokens.iter().skip_while(|token| token.to < line_end).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < line_end {
                backend.set_style(token.style);
            }
        };
        let reset_style = if select.start < line_end && select.end > line_end {
            backend.set_bg(Some(ctx.lexer().theme.selected));
            Style::bg(ctx.lexer().theme.selected)
        } else {
            Style::default()
        };
        wrapping_loop_select(
            content.char_indices().skip(skip_lines * wrap_len),
            backend,
            tokens,
            &wrap_number,
            line_end,
            wrap_len,
            select,
            ctx.lexer().theme.selected,
            lines,
            reset_style,
        )
    } else {
        wrapping_loop_select(
            content.char_indices(),
            backend,
            tokens.iter(),
            &wrap_number,
            wrap_len,
            wrap_len,
            select,
            ctx.lexer().theme.selected,
            lines,
            Style::default(),
        )
    };
}

#[inline(always)]
fn wrapping_loop_select<'a>(
    content: impl Iterator<Item = (usize, char)>,
    backend: &mut Backend,
    mut tokens: impl Iterator<Item = &'a Token>,
    wrap_number: &str,
    mut line_end: usize,
    wrap_len: usize,
    select: Range<usize>,
    select_color: Color,
    lines: &mut RectIter,
    mut reset_style: Style,
) {
    let mut maybe_token = tokens.next();
    for (idx, text) in content {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
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
        backend.print(text);
    }
}

#[inline]
pub fn wrapped_line(
    content: &str,
    tokens: &[Token],
    ctx: &mut impl Context,
    wrap_len: usize,
    lines: &mut RectIter,
    backend: &mut Backend,
) {
    let wrap_number = ctx.setup_wrap();
    let skip_lines = ctx.count_skipped_to_cursor(wrap_len, lines.len());
    if skip_lines != 0 {
        let mut wrap_text = format!("..{skip_lines} hidden wrapped lines");
        wrap_text.truncate(wrap_len);
        backend.print_styled(wrap_text, Style::reversed());
        let line_end = wrap_len * skip_lines;
        let mut tokens = tokens.iter().skip_while(|token| token.to < line_end).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < line_end {
                backend.set_style(token.style);
            }
        };
        wrapping_loop(
            content.char_indices().skip(skip_lines * wrap_len),
            backend,
            tokens,
            &wrap_number,
            line_end,
            wrap_len,
            lines,
        )
    } else {
        wrapping_loop(
            content.char_indices(),
            backend,
            tokens.iter(),
            &wrap_number,
            wrap_len, // postion char where line ends
            wrap_len,
            lines,
        )
    };
}

#[inline(always)]
fn wrapping_loop<'a>(
    content: impl Iterator<Item = (usize, char)>,
    backend: &mut Backend,
    mut tokens: impl Iterator<Item = &'a Token>,
    wrap_number: &str,
    mut line_end: usize,
    wrap_len: usize,
    lines: &mut RectIter,
) {
    let mut maybe_token = tokens.next();
    for (idx, text) in content {
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
            if token.to == idx {
                backend.reset_style();
                maybe_token = tokens.next();
            };
        }
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.set_style(token.style);
            };
        }
        backend.print(text);
    }
}

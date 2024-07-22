use crate::{
    render::{
        backend::{color, BackendProtocol, Style},
        layout::RectIter,
    },
    syntax::Token,
    workspace::line::{CodeLineContext, EditorLine, WrappedCursor},
};
use std::ops::Range;
use unicode_width::UnicodeWidthChar;

use super::is_wider_complex;

#[inline(always)]
pub fn render(
    line: &impl EditorLine,
    ctx: &mut CodeLineContext,
    line_width: usize,
    select: Option<Range<usize>>,
    lines: &mut RectIter,
    backend: &mut impl BackendProtocol,
) {
    if is_wider_complex(line, line_width) {
        match select {
            Some(select) => wrap_select(line, ctx, line_width, lines, select, backend),
            None => wrap(line, ctx, line_width, lines, backend),
        }
    } else {
        match select {
            Some(select) => self::select(line, ctx, select, backend),
            None => self::basic(line, ctx, backend),
        }
    }
}

#[inline]
pub fn basic(line: &impl EditorLine, ctx: &CodeLineContext, backend: &mut impl BackendProtocol) {
    let mut tokens = line.iter_tokens();
    let mut maybe_token = tokens.next();
    let mut idx = 0;
    let mut lsp_idx = 0;
    let cursor_idx = ctx.cursor_char();
    let lexer = &ctx.lexer;
    for text in line.chars() {
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.set_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
                    if token.from == lsp_idx {
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
        lsp_idx += lexer.char_lsp_pos(text);
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

#[inline]
pub fn select(line: &impl EditorLine, ctx: &CodeLineContext, select: Range<usize>, backend: &mut impl BackendProtocol) {
    let lexer = &ctx.lexer;
    let cursor_idx = ctx.cursor_char();
    let select_color = lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut tokens = line.iter_tokens();
    let mut maybe_token = tokens.next();
    let mut idx = 0;
    let mut lsp_idx = 0;
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
            if token.from == lsp_idx {
                backend.update_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
                    if token.from == lsp_idx {
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
        lsp_idx += lexer.char_lsp_pos(text);
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

#[inline(always)]
pub fn wrap(
    line: &impl EditorLine,
    ctx: &mut CodeLineContext,
    wrap_len: usize,
    lines: &mut RectIter,
    backend: &mut impl BackendProtocol,
) {
    let (wrap_cursor, offset) = ctx.count_skipped_to_cursor_complex(line, wrap_len, lines.len());
    if wrap_cursor.skip_lines != 0 {
        let mut wrap_text = format!("..{} hidden wrapped lines", wrap_cursor.skip_lines);
        wrap_text.truncate(wrap_len);
        backend.print_styled(wrap_text, Style::reversed());
        let mut tokens = line.iter_tokens().skip_while(|token| token.to < offset).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < wrap_cursor.skip_chars {
                backend.set_style(token.style);
            }
        };
        wrapping_loop(line.chars(), tokens, wrap_len, lines, (wrap_cursor, offset, 0), ctx, backend)
    } else {
        wrapping_loop(line.chars(), line.iter_tokens(), wrap_len, lines, (wrap_cursor, offset, wrap_len), ctx, backend)
    };
}

#[inline(always)]
fn wrapping_loop<'a>(
    content: impl Iterator<Item = char>,
    mut tokens: impl Iterator<Item = &'a Token>,
    wrap_len: usize,
    lines: &mut RectIter,
    (wrap_cursor, mut lsp_idx, mut remaining): (WrappedCursor, usize, usize),
    ctx: &CodeLineContext,
    backend: &mut impl BackendProtocol,
) {
    let cursor_idx = wrap_cursor.flat_char_idx;
    let lexer = &ctx.lexer;
    let wrap_number = ctx.setup_wrap();
    let mut maybe_token = tokens.next();
    let mut idx = wrap_cursor.skip_chars;
    for text in content.skip(idx) {
        let text_width = match UnicodeWidthChar::width(text) {
            Some(ch_width) => ch_width,
            None => continue,
        };
        if text_width > remaining {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            backend.print_styled_at(line.row, line.col, &wrap_number, Style::fg(color::dark_grey()));
            backend.clear_to_eol();
            remaining = wrap_len.saturating_sub(text_width);
        } else {
            remaining -= text_width;
        }
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.set_style(token.style);
            };
            if token.to == lsp_idx {
                backend.reset_style();
                maybe_token = tokens.next();
            };
        }
        if cursor_idx == idx {
            backend.print_styled(text, Style::reversed());
        } else {
            backend.print(text);
        }
        idx += 1;
        lsp_idx += lexer.char_lsp_pos(text);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
}

#[inline]
pub fn wrap_select(
    line: &impl EditorLine,
    ctx: &mut CodeLineContext,
    wrap_len: usize,
    lines: &mut RectIter,
    select: Range<usize>,
    backend: &mut impl BackendProtocol,
) {
    let (wrap_cursor, offset) = ctx.count_skipped_to_cursor_complex(line, wrap_len, lines.len());
    if wrap_cursor.skip_lines != 0 {
        let mut wrap_text = format!("..{} hidden wrapped lines", wrap_cursor.skip_lines);
        wrap_text.truncate(wrap_len);
        backend.print_styled(wrap_text, Style::reversed());
        let line_end = wrap_cursor.skip_chars;
        let mut tokens = line.iter_tokens().skip_while(|token| token.to < line_end).peekable();
        if let Some(token) = tokens.peek() {
            if token.from < line_end {
                backend.set_style(token.style);
            }
        };
        let reset_style = if select.start < offset && select.end > offset {
            backend.set_bg(Some(ctx.lexer.theme.selected));
            Style::bg(ctx.lexer.theme.selected)
        } else {
            Style::default()
        };
        let style_data = (tokens, select, reset_style);
        let position_data = (wrap_cursor, offset, 0, wrap_len);
        wrapping_loop_select(line.chars(), style_data, lines, ctx, position_data, backend)
    } else {
        let style_data = (line.iter_tokens(), select, Style::default());
        let position_data = (wrap_cursor, offset, wrap_len, wrap_len);
        wrapping_loop_select(line.chars(), style_data, lines, ctx, position_data, backend)
    };
}

#[inline(always)]
fn wrapping_loop_select<'a>(
    content: impl Iterator<Item = char>,
    (mut tokens, select, mut reset_style): (impl Iterator<Item = &'a Token>, Range<usize>, Style),
    lines: &mut RectIter,
    ctx: &CodeLineContext,
    (wrap_cursor, mut lsp_idx, mut remaining, wrap_len): (WrappedCursor, usize, usize, usize),
    backend: &mut impl BackendProtocol,
) {
    let cursor_idx = wrap_cursor.flat_char_idx;
    let lexer = &ctx.lexer;
    let select_color = lexer.theme.selected;
    let mut maybe_token = tokens.next();
    let mut idx = wrap_cursor.skip_chars;
    let wrap_number = ctx.setup_wrap();
    for text in content.skip(idx) {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
        let text_width = match UnicodeWidthChar::width(text) {
            Some(ch_width) => ch_width,
            None => continue,
        };
        if text_width > remaining {
            let line = match lines.next() {
                Some(line) => line,
                None => return,
            };
            backend.print_styled_at(line.row, line.col, &wrap_number, Style::fg(color::dark_grey()));
            backend.clear_to_eol();
            remaining = wrap_len.saturating_sub(text_width);
        } else {
            remaining -= text_width;
        }
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.update_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
                    if token.from == lsp_idx {
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
        lsp_idx += lexer.char_lsp_pos(text);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
}

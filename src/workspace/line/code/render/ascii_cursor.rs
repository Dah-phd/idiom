use super::{WRAP_CLOSE, WRAP_OPEN};
use crate::{
    render::backend::{Backend, BackendProtocol, Style},
    workspace::line::{CodeLine, CodeLineContext, EditorLine},
};
use std::ops::Range;

#[inline]
pub fn render(
    line: &mut CodeLine,
    ctx: &mut CodeLineContext,
    line_width: usize,
    select: Option<Range<usize>>,
    backend: &mut Backend,
) {
    if line_width > line.char_len {
        match select {
            Some(select) => self::select(line, ctx, select, backend),
            None => self::basic(line, ctx, backend),
        }
        if let Some(diagnostics) = line.diagnostics.as_ref() {
            diagnostics.inline_render(line_width - line.char_len, backend);
        }
    } else {
        match select {
            Some(select) => self::partial_select(line, ctx, line_width, select, backend),
            None => self::partial(line, ctx, line_width, backend),
        }
    }
}

#[inline]
pub fn basic(line: &CodeLine, ctx: &CodeLineContext, backend: &mut Backend) {
    let mut iter_tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
    let cursor_idx = ctx.cursor_char();
    if let Some(token) = iter_tokens.next() {
        if token.delta_start == 0 {
            counter = token.len;
            backend.set_style(token.style);
        } else {
            lined_up.replace(token.style);
            counter = token.delta_start;
        }
        last_len = token.len;
    };
    for text in line.chars() {
        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len - 1;
                }
                None => match iter_tokens.next() {
                    None => {
                        backend.reset_style();
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - (last_len + 1);
                            lined_up.replace(token.style);
                            backend.reset_style();
                        } else {
                            counter = token.len - 1;
                            backend.set_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
            }
        } else {
            counter -= 1;
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
pub fn select(line: &CodeLine, ctx: &CodeLineContext, select: Range<usize>, backend: &mut Backend) {
    let select_color = ctx.lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut iter_tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
    let cursor_idx = ctx.cursor_char();
    if let Some(token) = iter_tokens.next() {
        if token.delta_start == 0 {
            backend.set_style(token.style);
            counter = token.len;
        } else {
            lined_up.replace(token.style);
            counter = token.delta_start;
        }
        last_len = token.len;
    };
    for text in line.chars() {
        if select.start == idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
        }
        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len - 1;
                }
                None => match iter_tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - (last_len + 1);
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len - 1;
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
            }
        } else {
            counter -= 1;
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

#[inline(always)]
pub fn partial(line: &mut CodeLine, ctx: &CodeLineContext, line_width: usize, backend: &mut Backend) {
    let cursor_idx = ctx.cursor_char();
    let (mut idx, reduction) = line.cached.generate_skipped_chars_simple(cursor_idx, line_width);
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
    }
    let expected_token_end = idx;
    let mut tokens = line.iter_tokens().skip_while(|token| token.to < expected_token_end).peekable();
    if let Some(token) = tokens.peek() {
        if token.from < expected_token_end {
            backend.set_style(token.style);
        }
    };
    let mut maybe_token = tokens.next();
    let content = unsafe { line.content.get_unchecked(idx..) };
    for text in content.chars().take(line_width.saturating_sub(reduction)) {
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
    } else if line.char_len() > idx {
        backend.reset_style();
        backend.print_styled(WRAP_CLOSE, Style::reversed());
    }
}

#[inline(always)]
pub fn partial_select(
    line: &mut CodeLine,
    ctx: &CodeLineContext,
    line_width: usize,
    select: Range<usize>,
    backend: &mut Backend,
) {
    let cursor_idx = ctx.cursor_char();
    let (mut idx, reduction) = line.cached.generate_skipped_chars_simple(cursor_idx, line_width);
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
    }
    let expected_token_end = idx;
    let mut tokens = line.iter_tokens().skip_while(|token| token.to < expected_token_end).peekable();
    if let Some(token) = tokens.peek() {
        if token.from < expected_token_end {
            backend.set_style(token.style);
        }
    };
    let mut maybe_token = tokens.next();
    let select_color = ctx.lexer.theme.selected;
    let mut reset_style = Style::default();
    if select.start <= idx && idx < select.end {
        reset_style.set_bg(Some(select_color));
        backend.set_bg(Some(select_color));
    }
    let content = unsafe { line.content.get_unchecked(idx..) };
    for text in content.chars().take(line_width.saturating_sub(reduction)) {
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
    } else if line.char_len() > idx {
        backend.reset_style();
        backend.print_styled(WRAP_CLOSE, Style::reversed());
    }
}

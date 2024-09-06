use unicode_width::UnicodeWidthChar;

use crate::{
    render::backend::{Backend, BackendProtocol, Style},
    workspace::line::{CodeLineContext, EditorLine},
};
use std::ops::Range;

use super::{width_remainder, WRAP_CLOSE, WRAP_OPEN};

pub fn render(
    line: &mut EditorLine,
    ctx: &mut CodeLineContext,
    line_width: usize,
    select: Option<Range<usize>>,
    backend: &mut Backend,
) {
    if let Some(remainder) = width_remainder(line, line_width) {
        match select {
            Some(select) => self::select(line, ctx, select, backend),
            None => self::basic(line, ctx, backend),
        }
        if let Some(diagnostic) = line.diagnostics.as_ref() {
            diagnostic.inline_render(remainder, backend);
        }
    } else {
        match select {
            Some(select) => partial_select(line, ctx, select, line_width, backend),
            None => partial(line, ctx, line_width, backend),
        }
    }
}

pub fn basic(line: &EditorLine, ctx: &CodeLineContext, backend: &mut Backend) {
    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
    let char_position = ctx.lexer.char_lsp_pos;
    let cursor_idx = ctx.cursor_char();
    if let Some(token) = tokens.next() {
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
                None => match tokens.next() {
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
            counter = counter.saturating_sub(char_position(text));
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

pub fn select(line: &EditorLine, ctx: &CodeLineContext, select: Range<usize>, backend: &mut Backend) {
    let char_position = ctx.lexer.char_lsp_pos;
    let select_color = ctx.lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
    let cursor_idx = ctx.cursor_char();
    if let Some(token) = tokens.next() {
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
                None => match tokens.next() {
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
            counter = counter.saturating_sub(char_position(text));
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

pub fn partial(line: &mut EditorLine, ctx: &mut CodeLineContext, mut line_width: usize, backend: &mut Backend) {
    line_width -= 2;

    let cursor_idx = ctx.cursor_char();
    let char_position = ctx.lexer.char_lsp_pos;
    let mut idx = line.cached.generate_skipped_chars_complex(cursor_idx, line_width, line.content.chars());
    let mut content = line.chars();

    let mut cursor = 0;
    let mut counter_to_idx = idx;
    while counter_to_idx != 0 {
        cursor += content.next().map(char_position).unwrap_or_default();
        counter_to_idx -= 1;
    }

    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;

    for token in tokens.by_ref() {
        if token.delta_start + token.len > cursor {
            last_len = token.len;
            if token.delta_start > cursor {
                counter = token.delta_start - cursor;
                lined_up.replace(token.style);
            } else {
                backend.set_style(token.style);
                counter = (token.delta_start + last_len) - cursor;
            }
            break;
        }
        cursor -= token.delta_start;
    }

    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
        line_width -= 2;
    }

    for text in content {
        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len - 1;
                }
                None => match tokens.next() {
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
            counter = counter.saturating_sub(char_position(text));
        }

        // handle width
        let char_width = UnicodeWidthChar::width(text).unwrap_or(1);

        if char_width > line_width {
            break;
        } else {
            line_width -= char_width;
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

pub fn partial_select(
    line: &mut EditorLine,
    ctx: &mut CodeLineContext,
    select: Range<usize>,
    mut line_width: usize,
    backend: &mut Backend,
) {
    line_width -= 2;

    let cursor_idx = ctx.cursor_char();
    let char_position = ctx.lexer.char_lsp_pos;
    let mut idx = line.cached.generate_skipped_chars_complex(cursor_idx, line_width, line.content.chars());
    let mut content = line.chars();

    let mut cursor = 0;
    let mut counter_to_idx = idx;
    while counter_to_idx != 0 {
        cursor += content.next().map(char_position).unwrap_or_default();
        counter_to_idx -= 1;
    }

    let select_color = ctx.lexer.theme.selected;
    let mut reset_style = Style::default();
    if select.start <= idx && idx < select.end {
        reset_style.set_bg(Some(select_color));
        backend.set_bg(Some(select_color));
    }

    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;

    for token in tokens.by_ref() {
        if token.delta_start + token.len > cursor {
            last_len = token.len;
            if token.delta_start > cursor {
                counter = token.delta_start - cursor;
                lined_up.replace(token.style);
            } else {
                backend.update_style(token.style);
                counter = (token.delta_start + last_len) - cursor;
            }
            break;
        }
        cursor -= token.delta_start;
    }

    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
        line_width -= 2;
    };

    for text in content {
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
                    counter = last_len.saturating_sub(char_position(text));
                }
                None => match tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start.saturating_sub(last_len + char_position(text));
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len.saturating_sub(char_position(text));
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
            }
        } else {
            counter = counter.saturating_sub(char_position(text));
        }

        // handle width
        let char_width = UnicodeWidthChar::width(text).unwrap_or(1);

        if char_width > line_width {
            break;
        } else {
            line_width -= char_width;
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

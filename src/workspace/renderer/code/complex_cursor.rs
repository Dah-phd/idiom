use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::{utils::CharLimitedWidths, Backend};
use std::ops::Range;

use super::{width_remainder, WRAP_CLOSE, WRAP_OPEN};

pub fn render(
    line: &mut EditorLine,
    ctx: &mut LineContext,
    line_width: usize,
    select: Option<Range<usize>>,
    gs: &mut GlobalState,
) {
    if let Some(remainder) = width_remainder(line, line_width) {
        match select {
            Some(select) => self::select(line, ctx, select, gs),
            None => self::basic(line, ctx, gs.backend()),
        }
        if let Some(diagnostic) = line.diagnostics() {
            diagnostic.render_pad_4(remainder - 1, gs.backend());
        }
    } else {
        match select {
            Some(select) => partial_select(line, ctx, select, line_width, gs),
            None => partial(line, ctx, line_width, gs.backend()),
        }
    }
}

pub fn basic(line: &EditorLine, ctx: &LineContext, backend: &mut CrossTerm) {
    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
    let char_position = ctx.char_lsp_pos;
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
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len - 1;
                }
            }
        } else {
            counter = counter.saturating_sub(char_position(text));
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
    } else {
        backend.print(" ");
    }
    backend.reset_style();
}

pub fn select(line: &EditorLine, ctx: &LineContext, select: Range<usize>, gs: &mut GlobalState) {
    let select_color = gs.theme.selected;
    let backend = gs.backend();
    let char_position = ctx.char_lsp_pos;
    let mut reset_style = ContentStyle::default();
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
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len - 1;
                }
            }
        } else {
            counter = counter.saturating_sub(char_position(text));
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
    } else {
        backend.print(" ");
    }
    backend.reset_style();
}

pub fn partial(code: &mut EditorLine, ctx: &mut LineContext, mut line_width: usize, backend: &mut CrossTerm) {
    let cursor_idx = ctx.cursor_char();
    let char_position = ctx.char_lsp_pos;
    let mut idx = code.generate_skipped_chars_complex(cursor_idx, line_width);
    let mut content = CharLimitedWidths::new(code.as_str(), 3);
    let mut cursor = 0;

    for _ in 0..idx {
        cursor += content.next().map(|(ch, ..)| char_position(ch)).unwrap_or_default();
    }

    let mut tokens = code.iter_tokens();
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
        backend.print_styled(WRAP_OPEN, ctx.accent_style.reverse());
        line_width -= 1;
    }

    for (text, char_width) in content {
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

        if char_width > line_width {
            break;
        } else {
            line_width -= char_width;
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed());
        } else {
            backend.print(text);
        }

        idx += 1;
    }

    backend.reset_style();
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else if code.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

pub fn partial_select(
    code: &mut EditorLine,
    ctx: &mut LineContext,
    select: Range<usize>,
    mut line_width: usize,
    gs: &mut GlobalState,
) {
    let backend = &mut gs.backend;
    let cursor_idx = ctx.cursor_char();
    let char_position = ctx.char_lsp_pos;
    let mut idx = code.generate_skipped_chars_complex(cursor_idx, line_width);
    let mut content = CharLimitedWidths::new(code.as_str(), 3);

    let mut cursor = 0;
    for _ in 0..idx {
        cursor += content.next().map(|(ch, ..)| char_position(ch)).unwrap_or_default();
    }

    let select_color = gs.theme.selected;
    let mut reset_style = ContentStyle::default();
    if select.start <= idx && idx < select.end {
        reset_style.set_bg(Some(select_color));
        backend.set_bg(Some(select_color));
    }

    let mut tokens = code.iter_tokens();
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
        backend.print_styled(WRAP_OPEN, ctx.accent_style.reverse());
        line_width -= 1;
    };

    for (text, char_width) in content {
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

        if char_width > line_width {
            break;
        } else {
            line_width -= char_width;
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed());
        } else {
            backend.print(text);
        }
        idx += 1;
    }

    backend.reset_style();
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else if code.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

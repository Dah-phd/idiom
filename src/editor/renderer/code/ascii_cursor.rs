use super::{SelectManager, WRAP_CLOSE, WRAP_OPEN};
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::Backend;

pub fn render(
    line: &mut EditorLine,
    ctx: &mut LineContext,
    line_width: usize,
    select: Option<SelectManager>,
    gs: &mut GlobalState,
) {
    if line_width > line.char_len() {
        match select {
            Some(select) => self::select(line, ctx, select, gs),
            None => self::basic(line, ctx, gs.backend()),
        }
        if let Some(diagnostics) = line.diagnostics() {
            let padded_len = line.char_len() + 1;
            let diagnostic_width = line_width - padded_len;
            diagnostics.render_pad_4(diagnostic_width, gs.backend());
        }
    } else {
        match select {
            Some(select) => self::partial_select(line, ctx, line_width, select, gs),
            None => self::partial(line, ctx, line_width, gs.backend()),
        }
    }
}

pub fn basic(line: &EditorLine, ctx: &LineContext, backend: &mut CrossTerm) {
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
                None => match iter_tokens.next() {
                    None => {
                        backend.reset_style();
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - last_len;
                            lined_up.replace(token.style);
                            backend.reset_style();
                        } else {
                            counter = token.len;
                            backend.set_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len;
                }
            }
        }
        counter = counter.saturating_sub(1);

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

#[inline]
pub fn select(line: &EditorLine, ctx: &LineContext, mut select: SelectManager, gs: &mut GlobalState) {
    let backend = gs.backend();
    let mut reset_style = ContentStyle::default();
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
        select.set_style(idx, &mut reset_style, backend);
        if counter == 0 {
            match lined_up.take() {
                None => match iter_tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - last_len;
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len;
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len;
                }
            }
        }
        counter = counter.saturating_sub(1);

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    backend.reset_style();
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else {
        select.pad(gs);
    }
}

#[inline(always)]
pub fn partial(line: &mut EditorLine, ctx: &LineContext, line_width: usize, backend: &mut CrossTerm) {
    let cursor_idx = ctx.cursor_char();
    let (mut idx, reduction) = line.generate_skipped_chars_simple(cursor_idx, line_width);
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, ctx.accent_style.reverse());
    }
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut tokens = line.iter_tokens();
    let mut cursor = idx;
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

    let content = unsafe { line.as_str().get_unchecked(idx..) };
    for text in content.chars().take(line_width.saturating_sub(reduction)) {
        if counter == 0 {
            match lined_up.take() {
                None => match tokens.next() {
                    None => {
                        backend.reset_style();
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - last_len;
                            lined_up.replace(token.style);
                            backend.reset_style();
                        } else {
                            counter = token.len;
                            backend.set_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len;
                }
            }
        }
        counter = counter.saturating_sub(1);

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }

    backend.reset_style();

    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else if line.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

pub fn partial_select(
    line: &mut EditorLine,
    ctx: &LineContext,
    line_width: usize,
    mut select: SelectManager,
    gs: &mut GlobalState,
) {
    let backend = &mut gs.backend;
    let cursor_idx = ctx.cursor_char();
    let (mut idx, reduction) = line.generate_skipped_chars_simple(cursor_idx, line_width);
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, ctx.accent_style.reverse());
    }
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut tokens = line.iter_tokens();
    let mut cursor = idx;
    let mut reset_style = ContentStyle::default();

    select.go_to_index(idx, &mut reset_style, backend);

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

    let content = unsafe { line.as_str().get_unchecked(idx..) };
    for text in content.chars().take(line_width.saturating_sub(reduction)) {
        select.set_style(idx, &mut reset_style, backend);
        if counter == 0 {
            match lined_up.take() {
                None => match tokens.next() {
                    None => {
                        counter = usize::MAX;
                        backend.set_style(reset_style);
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - last_len;
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len;
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len;
                }
            }
        }
        counter = counter.saturating_sub(1);

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
    } else if line.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

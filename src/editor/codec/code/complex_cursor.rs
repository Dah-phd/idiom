use super::{CodecContext, SelectManager, WRAP_CLOSE, WRAP_OPEN, width_remainder};
use crate::{
    editor::syntax::Encoding,
    editor_line::EditorLine,
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::{Backend, utils::CharLimitedWidths};

pub fn render(
    line: &mut EditorLine,
    line_width: usize,
    select: Option<SelectManager>,
    encoding: &Encoding,
    ctx: &mut CodecContext,
    gs: &mut GlobalState,
) {
    if let Some(remainder) = width_remainder(line, line_width) {
        match select {
            Some(select) => self::select(line, select, encoding, ctx, gs),
            None => self::basic(line, encoding, ctx, gs.backend()),
        }
        if let Some(diagnostic) = line.diagnostics() {
            diagnostic.render_pad_4(remainder - 1, gs.backend());
        }
    } else {
        match select {
            Some(select) => partial_select(line, select, line_width, encoding, ctx, gs),
            None => partial(line, line_width, encoding, ctx, gs.backend()),
        }
    }
}

pub fn basic(line: &EditorLine, encoding: &Encoding, ctx: &CodecContext, backend: &mut CrossTerm) {
    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;
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
            counter = counter.saturating_sub(encoding.char_len(text));
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(line.end_view(), ContentStyle::reversed());
    } else {
        backend.print(line.end_view());
    }
    backend.reset_style();
}

pub fn select(
    line: &EditorLine,
    mut select: SelectManager,
    encoding: &Encoding,
    ctx: &CodecContext,
    gs: &mut GlobalState,
) {
    let backend = gs.backend();
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
        select.set_style(idx, &mut reset_style, backend);
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
            counter = counter.saturating_sub(encoding.char_len(text));
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }

        idx += 1;
    }
    backend.reset_style();
    if idx <= cursor_idx {
        backend.print_styled(' ', ContentStyle::reversed());
    } else {
        select.pad(gs);
    }
}

pub fn partial(
    code: &mut EditorLine,
    mut line_width: usize,
    encoding: &Encoding,
    ctx: &mut CodecContext,
    backend: &mut CrossTerm,
) {
    let cursor_idx = ctx.cursor_char();
    let mut idx = code.generate_skipped_chars_complex(cursor_idx, line_width);
    let mut content = CharLimitedWidths::new(code.as_str(), 3);
    let mut cursor = 0;

    for _ in 0..idx {
        cursor += content.next().map(|(ch, ..)| encoding.char_len(ch)).unwrap_or_default();
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
            counter = counter.saturating_sub(encoding.char_len(text));
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
        backend.print_styled(code.end_view(), ContentStyle::reversed());
    } else if code.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    } else {
        backend.print(code.end_view());
    }
}

pub fn partial_select(
    code: &mut EditorLine,
    mut select: SelectManager,
    mut line_width: usize,
    encoding: &Encoding,
    ctx: &mut CodecContext,
    gs: &mut GlobalState,
) {
    let backend = &mut gs.backend;
    let cursor_idx = ctx.cursor_char();
    let mut idx = code.generate_skipped_chars_complex(cursor_idx, line_width);
    let mut content = CharLimitedWidths::new(code.as_str(), 3);

    let mut cursor = 0;
    for _ in 0..idx {
        cursor += content.next().map(|(ch, ..)| encoding.char_len(ch)).unwrap_or_default();
    }

    let mut reset_style = ContentStyle::default();
    select.go_to_index(idx, &mut reset_style, backend);

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
        select.set_style(idx, &mut reset_style, backend);
        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len.saturating_sub(encoding.char_len(text));
                }
                None => match tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start.saturating_sub(last_len + encoding.char_len(text));
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len.saturating_sub(encoding.char_len(text));
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
            }
        } else {
            counter = counter.saturating_sub(encoding.char_len(text));
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
        backend.print_styled(' ', ContentStyle::reversed());
    } else if code.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

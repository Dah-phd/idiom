use super::WRAP_CLOSE;
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::{utils::CharLimitedWidths, Backend};
use std::ops::Range;

pub fn complex_line(
    code: &EditorLine,
    mut line_width: usize,
    ctx: &mut LineContext,
    backend: &mut CrossTerm,
) -> Option<usize> {
    let mut iter_tokens = code.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let char_position = ctx.lexer.char_lsp_pos;

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

    for (text, width) in CharLimitedWidths::new(code.as_str(), 3) {
        if line_width <= width {
            backend.reset_style();
            backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
            return None;
        } else {
            line_width -= width;
        }
        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.set_style(style);
                    counter = last_len;
                }
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
            }
        }

        counter = counter.saturating_sub(char_position(text));
        backend.print(text);
    }
    backend.reset_style();
    Some(line_width)
}

pub fn complex_line_with_select(
    code: &EditorLine,
    mut line_width: usize,
    select: Range<usize>,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) -> Option<usize> {
    let char_position = ctx.lexer.char_lsp_pos;
    let select_color = gs.theme.selected;
    let backend = gs.backend();
    let mut reset_style = ContentStyle::default();
    let mut iter_tokens = code.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;

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

    for (idx, (text, width)) in CharLimitedWidths::new(code.as_str(), 3).enumerate() {
        if line_width <= width {
            backend.reset_style();
            backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
            return None;
        } else {
            line_width -= width;
        }
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
                None => match iter_tokens.next() {
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

        counter = counter.saturating_sub(char_position(text));
        backend.print(text);
    }

    backend.reset_style();
    Some(line_width)
}

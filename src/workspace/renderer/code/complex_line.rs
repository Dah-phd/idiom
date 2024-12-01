use std::ops::Range;

use crate::{
    render::backend::{BackendProtocol, Style},
    syntax::{tokens::TokenLine, Lexer},
};

pub fn complex_line(
    content: impl Iterator<Item = char>,
    tokens: &TokenLine,
    lexer: &Lexer,
    backend: &mut impl BackendProtocol,
) {
    let mut iter_tokens = tokens.iter();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let char_position = lexer.char_lsp_pos;
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
    for text in content {
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
}

pub fn complex_line_with_select(
    content: impl Iterator<Item = char>,
    tokens: &TokenLine,
    select: Range<usize>,
    lexer: &Lexer,
    backend: &mut impl BackendProtocol,
) {
    let select_color = lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut iter_tokens = tokens.iter();
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
    for (idx, text) in content.enumerate() {
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
                    counter = last_len;
                }
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
            }
        }
        counter = counter.saturating_sub(1);

        backend.print(text);
    }
    backend.reset_style();
}

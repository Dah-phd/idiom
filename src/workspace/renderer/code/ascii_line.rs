use std::ops::Range;

use crate::{
    render::backend::Style,
    syntax::{tokens::TokenLine, Lexer},
    BackendProtocol,
};

pub fn ascii_line(content: &str, tokens: &TokenLine, backend: &mut impl BackendProtocol) {
    let mut cursor = 0;
    let mut last_len = 0;
    for token in tokens.iter() {
        // handle tokne gap
        if token.delta_start > last_len {
            let gap_start = cursor + last_len;
            cursor += token.delta_start;
            match content.get(gap_start..cursor) {
                Some(text) => backend.print(text),
                None => {
                    if let Some(text) = content.get(gap_start..) {
                        backend.print(text);
                    }
                    return;
                }
            }
        } else {
            cursor += token.delta_start;
        }

        // print token
        last_len = token.len;
        match content.get(cursor..cursor + last_len) {
            Some(text) => backend.print_styled(text, token.style),
            None => {
                if let Some(text) = content.get(cursor..) {
                    backend.print_styled(text, token.style);
                }
                return;
            }
        }
    }
    match content.get(cursor + last_len..) {
        Some(text) if !text.is_empty() => backend.print(text),
        _ => (),
    }
}

pub fn ascii_line_with_select(
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

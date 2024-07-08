use std::ops::Range;

use crate::{
    render::backend::{Backend, Style},
    syntax::{Lexer, Token},
    BackendProtocol,
};

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
        Some(text) if !text.is_empty() => backend.print(text),
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

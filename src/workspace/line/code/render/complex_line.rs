use std::ops::Range;

use crate::{
    render::backend::{BackendProtocol, Style},
    syntax::{tokens::TokenLine, Lexer},
};

#[inline]
pub fn complex_line(
    content: impl Iterator<Item = char>,
    tokens: &TokenLine,
    lexer: &Lexer,
    backend: &mut impl BackendProtocol,
) {
    let mut iter_tokens = tokens.iter();
    let mut maybe_token = iter_tokens.next();
    let mut idx = 0;
    for text in content {
        if let Some(token) = maybe_token {
            if token.from == idx {
                backend.update_style(token.style);
            } else if token.to == idx {
                if let Some(token) = iter_tokens.next() {
                    if token.from == idx {
                        backend.update_style(token.style);
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
        backend.print(text);
        idx += lexer.char_lsp_pos(text);
    }
    backend.reset_style();
}

#[inline]
pub fn complex_line_with_select(
    content: impl Iterator<Item = char>,
    tokens: &TokenLine,
    select: Range<usize>,
    lexer: &Lexer,
    backend: &mut impl BackendProtocol,
) {
    let mut iter_tokens = tokens.iter();
    let mut maybe_token = iter_tokens.next();
    let mut reset_style = Style::default();
    let select_color = lexer.theme.selected;
    let mut lsp_idx = 0;
    for (char_idx, text) in content.enumerate() {
        if select.start == char_idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }
        if select.end == char_idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
        }
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.update_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = iter_tokens.next() {
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
        backend.print(text);
        lsp_idx += lexer.char_lsp_pos(text);
    }
    backend.reset_style();
}

use unicode_width::UnicodeWidthChar;

use crate::{
    render::backend::{color, BackendProtocol, Style},
    workspace::line::{CodeLine, CodeLineContext, EditorLine},
};
use std::ops::Range;

use super::{width_remainder, WRAP_CLOSE, WRAP_OPEN};

#[inline(always)]
pub fn render(
    line: &mut CodeLine,
    ctx: &mut CodeLineContext,
    line_width: usize,
    select: Option<Range<usize>>,
    backend: &mut impl BackendProtocol,
) {
    if let Some(remainder) = width_remainder(line, line_width) {
        match select {
            Some(select) => self::select(line, ctx, select, backend),
            None => self::basic(line, ctx, backend),
        }
        if let Some(diagnostic) = line.diagnostics.as_ref() {
            diagnostic.inline_render(remainder - 1, backend);
        }
    } else {
        match select {
            Some(select) => partial_select(line, ctx, select, line_width, backend),
            None => partial(line, ctx, line_width, backend),
        }
    }
}

#[inline]
pub fn basic(line: &CodeLine, ctx: &CodeLineContext, backend: &mut impl BackendProtocol) {
    let mut tokens = line.iter_tokens();
    let mut maybe_token = tokens.next();
    let mut idx = 0;
    let mut lsp_idx = 0;
    let cursor_idx = ctx.cursor_char();
    let lexer = &ctx.lexer;
    for text in line.chars() {
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.set_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
                    if token.from == lsp_idx {
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
        if idx == cursor_idx {
            backend.print_styled(text, Style::reversed());
        } else {
            backend.print(text);
        }
        lsp_idx += lexer.char_lsp_pos(text);
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

#[inline]
pub fn select(line: &CodeLine, ctx: &CodeLineContext, select: Range<usize>, backend: &mut impl BackendProtocol) {
    let lexer = &ctx.lexer;
    let cursor_idx = ctx.cursor_char();
    let select_color = lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut tokens = line.iter_tokens();
    let mut maybe_token = tokens.next();
    let mut idx = 0;
    let mut lsp_idx = 0;
    for text in line.chars() {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.update_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
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
        if idx == cursor_idx {
            backend.print_styled(text, Style::reversed());
        } else {
            backend.print(text);
        }
        lsp_idx += lexer.char_lsp_pos(text);
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    }
    backend.reset_style();
}

#[inline(always)]
pub fn partial(
    line: &mut CodeLine,
    ctx: &mut CodeLineContext,
    mut line_width: usize,
    backend: &mut impl BackendProtocol,
) {
    line_width -= 2;
    let cursor_idx = ctx.cursor_char();
    let lexer = &ctx.lexer;
    let mut idx = line.cached.generate_skipped_chars_complex(cursor_idx, line_width, line.content.chars());
    let mut content = line.chars();
    let mut counter_to_idx = idx;
    let mut lsp_idx = 0;
    while counter_to_idx != 0 {
        lsp_idx += content.next().map(|ch| lexer.char_lsp_pos(ch)).unwrap_or_default();
        counter_to_idx -= 1;
    }
    let expected_token_end = lsp_idx;
    let mut tokens = line.iter_tokens().skip_while(|token| token.to < expected_token_end).peekable();
    if let Some(token) = tokens.peek() {
        if token.from < expected_token_end {
            backend.set_style(token.style);
        }
    };
    let mut maybe_token = tokens.next();
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
        line_width -= 2;
    }
    for text in content {
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.set_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
                    if token.from == lsp_idx {
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
        // handle width
        let char_width = match UnicodeWidthChar::width(text) {
            Some(w) => w,
            None => {
                if idx == cursor_idx {
                    backend.print_styled("?", Style::reversed());
                } else {
                    backend.print_styled("?", Style::fg(color::red()));
                }
                idx += 1;
                lsp_idx += lexer.char_lsp_pos(text);
                continue;
            }
        };
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
        lsp_idx += lexer.char_lsp_pos(text);
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
    ctx: &mut CodeLineContext,
    select: Range<usize>,
    mut line_width: usize,
    backend: &mut impl BackendProtocol,
) {
    line_width -= 2;
    let cursor_idx = ctx.cursor_char();
    let lexer = &ctx.lexer;
    let mut idx = line.cached.generate_skipped_chars_complex(cursor_idx, line_width, line.content.chars());
    let mut content = line.chars();
    let mut counter_to_idx = idx;
    let mut lsp_idx = 0;
    while counter_to_idx != 0 {
        lsp_idx += content.next().map(|ch| lexer.char_lsp_pos(ch)).unwrap_or_default();
        counter_to_idx -= 1;
    }

    let expected_token_end = lsp_idx;
    let select_color = lexer.theme.selected;
    let mut reset_style = Style::default();
    let mut tokens = line.iter_tokens().skip_while(|token| token.to < expected_token_end).peekable();
    if let Some(token) = tokens.peek() {
        if token.from < expected_token_end {
            backend.update_style(token.style);
        }
    };
    let mut maybe_token = tokens.next();
    if idx != 0 {
        backend.print_styled(WRAP_OPEN, Style::reversed());
        line_width -= 2;
    };
    for text in content {
        if select.start == idx {
            reset_style.set_bg(Some(select_color));
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            reset_style.set_bg(None);
            backend.set_bg(None);
        }
        if let Some(token) = maybe_token {
            if token.from == lsp_idx {
                backend.update_style(token.style);
            } else if token.to == lsp_idx {
                if let Some(token) = tokens.next() {
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

        // handle width
        let char_width = match UnicodeWidthChar::width(text) {
            Some(w) => w,
            None => {
                if idx == cursor_idx {
                    backend.print_styled("?", Style::reversed());
                } else {
                    backend.print_styled("?", Style::fg(color::red()));
                }
                idx += 1;
                lsp_idx += lexer.char_lsp_pos(text);
                continue;
            }
        };
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
        lsp_idx += lexer.char_lsp_pos(text);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", Style::reversed());
    } else if line.char_len() > idx {
        backend.reset_style();
        backend.print_styled(WRAP_CLOSE, Style::reversed());
    }
}

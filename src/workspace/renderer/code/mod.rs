pub mod ascii_cursor;
pub mod ascii_line;
pub mod complex_cursor;
pub mod complex_line;

use crate::render::backend::Style;
use crate::render::{
    backend::{Backend, BackendProtocol},
    layout::Line,
    UTF8Safe,
};
use crate::workspace::{
    cursor::Cursor,
    line::{EditorLine, LineContext},
};
use std::ops::Range;
use unicode_width::UnicodeWidthChar;

const WRAP_OPEN: &str = "<<";
const WRAP_CLOSE: &str = ">>";

#[inline(always)]
pub fn width_remainder(line: &EditorLine, line_width: usize) -> Option<usize> {
    let mut current_with = 0;
    for ch in line.chars() {
        if let Some(char_width) = UnicodeWidthChar::width(ch) {
            current_with += char_width;
            if current_with >= line_width {
                return None;
            }
        }
    }
    Some(line_width - current_with)
}

#[inline(always)]
pub fn cursor(code: &mut EditorLine, ctx: &mut LineContext, line: Line, backend: &mut Backend) {
    let line_row = line.row;
    let select = ctx.get_select(line.width);
    let line_width = ctx.setup_cursor(line, backend);
    code.cached.cursor(line_row, ctx.cursor_char(), 0, select.clone());
    if code.is_simple() {
        ascii_cursor::render(code, ctx, line_width, select, backend);
    } else {
        complex_cursor::render(code, ctx, line_width, select, backend);
    }
    backend.reset_style();
}

#[inline(always)]
pub fn inner_render(
    code: &mut EditorLine,
    ctx: &mut LineContext<'_>,
    line: Line,
    select: Option<Range<usize>>,
    backend: &mut Backend,
) {
    let cache_line = line.row;
    let line_width = ctx.setup_line(line, backend);
    code.cached.line(cache_line, select.clone());
    match select {
        Some(select) => render_with_select(code, line_width, select, ctx, backend),
        None => render_no_select(code, line_width, ctx, backend),
    }
}

#[inline(always)]
fn render_with_select(
    code: &mut EditorLine,
    line_width: usize,
    select: Range<usize>,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) {
    if code.char_len == 0 && select.end != 0 {
        backend.print_styled(" ", Style::bg(ctx.lexer.theme.selected));
        return;
    }
    if code.is_simple() {
        if line_width > code.char_len() {
            let content = code.content.chars();
            ascii_line::ascii_line_with_select(content, &code.tokens, select, ctx.lexer, backend);
            if let Some(diagnostic) = code.diagnostics.as_ref() {
                diagnostic.inline_render(line_width - code.char_len, backend)
            }
        } else {
            let content = code.content.chars().take(line_width.saturating_sub(2));
            ascii_line::ascii_line_with_select(content, &code.tokens, select, ctx.lexer, backend);
            backend.print_styled(">>", Style::reversed());
        }
    // handles non ascii shrunk lines
    } else if let Ok(truncated) = code.content.truncate_if_wider(line_width) {
        let mut content = truncated.chars();
        if let Some(ch) = content.next_back() {
            if UnicodeWidthChar::width(ch).unwrap_or_default() <= 1 {
                content.next_back();
            }
        };
        complex_line::complex_line_with_select(content, &code.tokens, select, ctx.lexer, backend);
        backend.print_styled(">>", Style::reversed());
    } else {
        complex_line::complex_line_with_select(code.content.chars(), &code.tokens, select, ctx.lexer, backend);
        if let Some(diagnostic) = code.diagnostics.as_ref() {
            diagnostic.inline_render(line_width - code.content.width(), backend)
        }
    }
}

#[inline(always)]
fn render_no_select(
    code: &mut EditorLine,
    line_width: usize,
    ctx: &mut LineContext,
    backend: &mut impl BackendProtocol,
) {
    if code.is_simple() {
        if line_width > code.content.len() {
            ascii_line::ascii_line(&code.content, &code.tokens, backend);
            if let Some(diagnostic) = code.diagnostics.as_ref() {
                diagnostic.inline_render(line_width - code.char_len, backend)
            }
        } else {
            ascii_line::ascii_line(&code.content[..line_width.saturating_sub(2)], &code.tokens, backend);
            backend.print_styled(">>", Style::reversed());
        }
    // handles non ascii shrunk lines
    } else if let Ok(truncated) = code.content.truncate_if_wider(line_width) {
        let mut content = truncated.chars();
        if let Some(ch) = content.next_back() {
            if UnicodeWidthChar::width(ch).unwrap_or_default() <= 1 {
                content.next_back();
            }
        };
        complex_line::complex_line(content, &code.tokens, ctx.lexer, backend);
        backend.print_styled(">>", Style::reversed());
    } else {
        complex_line::complex_line(code.content.chars(), &code.tokens, ctx.lexer, backend);
        if let Some(diagnostic) = code.diagnostics.as_ref() {
            diagnostic.inline_render(line_width - code.content.width(), backend)
        }
    }
}

#[inline(always)]
pub fn cursor_fast(code: &mut EditorLine, ctx: &mut LineContext, line: Line, backend: &mut Backend) {
    let select = ctx.get_select(line.width);
    if !code.cached.should_render_cursor_or_update(line.row, ctx.cursor_char(), select.clone()) {
        ctx.skip_line();
        return;
    }
    let line_width = ctx.setup_cursor(line, backend);
    if code.is_simple() {
        ascii_cursor::render(code, ctx, line_width, select, backend);
    } else {
        complex_cursor::render(code, ctx, line_width, select, backend);
    }
    backend.reset_style();
}

pub fn repositioning(cursor: &mut Cursor) {
    if cursor.line < cursor.at_line {
        cursor.at_line = cursor.line;
    } else if cursor.line + 1 >= cursor.max_rows + cursor.at_line {
        cursor.at_line = cursor.line + 1 - cursor.max_rows;
    }
}

#[cfg(test)]
mod tests;

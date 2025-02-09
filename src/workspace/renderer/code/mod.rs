pub mod ascii_cursor;
pub mod ascii_line;
pub mod complex_cursor;
pub mod complex_line;

use crate::render::backend::StyleExt;
use crate::render::utils::CharLimitedWidths;
use crate::render::{
    backend::{Backend, BackendProtocol},
    layout::Line,
};
use crate::workspace::{
    cursor::Cursor,
    line::{EditorLine, LineContext},
};
use crossterm::style::{ContentStyle, Stylize};
use std::ops::Range;

const WRAP_OPEN: char = '<';
const WRAP_CLOSE: char = '>';

#[inline(always)]
pub fn width_remainder(line: &EditorLine, line_width: usize) -> Option<usize> {
    let mut current_with = 0;
    for (.., char_width) in CharLimitedWidths::new(&line.content, 3) {
        current_with += char_width;
        if current_with >= line_width {
            return None;
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
        backend.print_styled(" ", ContentStyle::bg(ctx.lexer.theme.selected));
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
            let content = code.content.chars().take(line_width.saturating_sub(1));
            ascii_line::ascii_line_with_select(content, &code.tokens, select, ctx.lexer, backend);
            backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
        }
        return;
    }

    let max_width = match complex_line::complex_line_with_select(code, line_width, select, ctx, backend) {
        Some(remaining) => remaining,
        None => return,
    };

    if let Some(diagnostics) = code.diagnostics.as_ref() {
        diagnostics.inline_render(max_width, backend);
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
        // ascii (byte idx based) render
        match line_width > code.content.len() {
            true => {
                ascii_line::ascii_line(&code.content, &code.tokens, backend);
                if let Some(diagnostic) = code.diagnostics.as_ref() {
                    diagnostic.inline_render(line_width - code.char_len, backend)
                }
            }
            false => {
                ascii_line::ascii_line(&code.content[..line_width.saturating_sub(1)], &code.tokens, backend);
                backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
            }
        }
        return;
    }

    let max_width = match complex_line::complex_line(code, line_width, ctx, backend) {
        Some(remaining) => remaining,
        None => return,
    };

    if let Some(diagnostics) = code.diagnostics.as_ref() {
        diagnostics.inline_render(max_width, backend);
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

    match code.is_simple() {
        true => ascii_cursor::render(code, ctx, line_width, select, backend),
        false => complex_cursor::render(code, ctx, line_width, select, backend),
    }
    backend.reset_style();
}

// ensures cursor is rendered
pub fn repositioning(cursor: &mut Cursor) {
    if cursor.line < cursor.at_line {
        cursor.at_line = cursor.line;
    } else if cursor.line + 1 >= cursor.max_rows + cursor.at_line {
        cursor.at_line = cursor.line + 1 - cursor.max_rows;
    }
}

#[cfg(test)]
mod tests;

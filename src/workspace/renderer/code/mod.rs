pub mod ascii_cursor;
pub mod ascii_line;
pub mod ascii_multi_cursor;
pub mod complex_cursor;
pub mod complex_line;
pub mod complex_multi_cursor;

use crate::ext_tui::CrossTerm;
use crate::global_state::GlobalState;
use crate::workspace::{
    cursor::{Cursor, CursorPosition},
    line::{EditorLine, LineContext},
};
use crossterm::style::Stylize;
use idiom_tui::{layout::Line, utils::CharLimitedWidths, Backend};
use std::ops::Range;

const WRAP_OPEN: char = '<';
const WRAP_CLOSE: char = '>';

#[inline(always)]
pub fn width_remainder(line: &EditorLine, line_width: usize) -> Option<usize> {
    let mut current_with = 0;
    for (.., char_width) in CharLimitedWidths::new(line.as_str(), 3) {
        current_with += char_width;
        if current_with >= line_width {
            return None;
        }
    }
    Some(line_width - current_with)
}

#[inline(always)]
pub fn cursor(code: &mut EditorLine, ctx: &mut LineContext, line: Line, gs: &mut GlobalState) {
    let line_row = line.row;
    let select = ctx.select_get(line.width);
    let line_width = ctx.setup_cursor(line, gs.backend());
    code.cached.cursor(line_row, ctx.cursor_char(), 0, select.clone());
    if code.is_simple() {
        ascii_cursor::render(code, ctx, line_width, select, gs);
    } else {
        complex_cursor::render(code, ctx, line_width, select, gs);
    }
    gs.backend.reset_style();
}

#[inline(always)]
pub fn inner_render(
    code: &mut EditorLine,
    ctx: &mut LineContext<'_>,
    line: Line,
    select: Option<Range<usize>>,
    gs: &mut GlobalState,
) {
    let cache_line = line.row;
    let line_width = ctx.setup_line(line, gs.backend());
    code.cached.line(cache_line, select.clone());
    match select {
        Some(select) => render_with_select(code, line_width, select, ctx, gs),
        None => render_no_select(code, line_width, ctx, gs.backend()),
    }
}

#[inline(always)]
fn render_with_select(
    code: &mut EditorLine,
    line_width: usize,
    select: Range<usize>,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    if code.char_len() == 0 && select.end != 0 {
        gs.backend.print_styled(" ", gs.get_select_style());
        return;
    }
    if code.is_simple() {
        if line_width > code.char_len() {
            let content = code.chars();
            ascii_line::ascii_line_with_select(content, &code.tokens, select, gs);
            if let Some(diagnostic) = code.diagnostics.as_ref() {
                diagnostic.inline_render(line_width - code.char_len(), gs.backend())
            }
        } else {
            let content = code.chars().take(line_width.saturating_sub(1));
            ascii_line::ascii_line_with_select(content, &code.tokens, select, gs);
            gs.backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
        }
        return;
    }

    let Some(max_width) = complex_line::complex_line_with_select(code, line_width, select, ctx, gs) else {
        return;
    };

    if let Some(diagnostics) = code.diagnostics.as_ref() {
        diagnostics.inline_render(max_width, gs.backend());
    }
}

#[inline(always)]
fn render_no_select(code: &mut EditorLine, line_width: usize, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if code.is_simple() {
        // ascii (byte idx based) render
        match line_width > code.len() {
            true => {
                ascii_line::ascii_line(code.as_str(), &code.tokens, backend);
                if let Some(diagnostic) = code.diagnostics.as_ref() {
                    diagnostic.inline_render(line_width - code.char_len(), backend)
                }
            }
            false => {
                ascii_line::ascii_line(&code.as_str()[..line_width.saturating_sub(1)], &code.tokens, backend);
                backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
            }
        }
        return;
    }

    let Some(max_width) = complex_line::complex_line(code, line_width, ctx, backend) else {
        return;
    };

    if let Some(diagnostics) = code.diagnostics.as_ref() {
        diagnostics.inline_render(max_width, backend);
    }
}

#[inline(always)]
pub fn cursor_fast(code: &mut EditorLine, ctx: &mut LineContext, line: Line, gs: &mut GlobalState) {
    let select = ctx.select_get(line.width);
    if !code.cached.should_render_cursor_or_update(line.row, ctx.cursor_char(), select.clone()) {
        ctx.skip_line();
        return;
    }

    let line_width = ctx.setup_cursor(line, gs.backend());

    match code.is_simple() {
        true => ascii_cursor::render(code, ctx, line_width, select, gs),
        false => complex_cursor::render(code, ctx, line_width, select, gs),
    }
    gs.backend.reset_style();
}

/// returns true if renders cursor
pub fn fast_render_is_cursor(
    text: &mut EditorLine,
    cursors: &[Cursor],
    line: Line,
    line_idx: usize,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) -> bool {
    if let Some((cursors, selects)) = ctx.multic_line_setup(cursors, line.width) {
        if !text.cached.should_render_multi_cursor(line.row, &cursors, &selects) {
            ctx.skip_line();
            return false;
        };
        multi_cursor(text, ctx, line, gs, cursors, selects);
    } else if ctx.has_cursor(line_idx) {
        let select = ctx.select_get(line.width);
        if !text.cached.should_render_cursor_or_update(line.row, ctx.cursor_char(), select.clone()) {
            ctx.skip_line();
            return false;
        }

        let line_width = ctx.setup_cursor(line, gs.backend());

        match text.is_simple() {
            true => ascii_cursor::render(text, ctx, line_width, select, gs),
            false => complex_cursor::render(text, ctx, line_width, select, gs),
        }
        gs.backend.reset_style();
    } else {
        let select = ctx.select_get(line.width);
        if text.cached.should_render_line(line.row, &select) {
            inner_render(text, ctx, line, select, gs);
        } else {
            ctx.skip_line();
        }
        return false;
    }
    true
}

#[inline(always)]
pub fn multi_cursor(
    code: &mut EditorLine,
    ctx: &mut LineContext,
    line: Line,
    gs: &mut GlobalState,
    cursors: Vec<CursorPosition>,
    selects: Vec<Range<usize>>,
) {
    let line_width = ctx.setup_cursor(line, gs.backend());
    match code.is_simple() {
        true => ascii_multi_cursor::render(code, ctx, line_width, cursors, selects, gs),
        false => complex_multi_cursor::render(code, ctx, line_width, cursors, selects, gs),
    }
}

// ensures cursor is rendered
#[inline]
pub fn repositioning(cursor: &mut Cursor) {
    if cursor.line < cursor.at_line {
        cursor.at_line = cursor.line;
    } else if cursor.line + 1 >= cursor.max_rows + cursor.at_line {
        cursor.at_line = cursor.line + 1 - cursor.max_rows;
    }
}

#[cfg(test)]
mod tests;

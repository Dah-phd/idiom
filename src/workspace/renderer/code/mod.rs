pub mod ascii_cursor;
pub mod ascii_line;
pub mod ascii_multi_cursor;
pub mod complex_cursor;
pub mod complex_line;
pub mod complex_multi_cursor;

use crate::ext_tui::StyleExt;
use crate::global_state::GlobalState;
use crate::workspace::{
    cursor::{CharRange, Cursor, CursorPosition},
    line::{EditorLine, LineContext},
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::{layout::Line, utils::CharLimitedWidths, Backend};

const WRAP_OPEN: char = '<';
const WRAP_CLOSE: char = '>';

/// if val is 0, it returns None
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


// ensures cursor is rendered
#[inline]
pub fn repositioning(cursor: &mut Cursor) {
    if cursor.line < cursor.at_line {
        cursor.at_line = cursor.line;
    } else if cursor.line + 1 >= cursor.max_rows + cursor.at_line {
        cursor.at_line = cursor.line + 1 - cursor.max_rows;
    }
}

#[inline]
pub fn cursor(code: &mut EditorLine, ctx: &mut LineContext, line: Line, gs: &mut GlobalState) {
    let line_row = line.row;
    let select = ctx.select_get(code.char_len());
    let line_width = ctx.setup_cursor(line, gs.backend());
    code.cached.cursor(line_row, ctx.cursor_char(), 0, select.clone());
    if code.is_simple() {
        ascii_cursor::render(code, ctx, line_width, select, gs);
    } else {
        complex_cursor::render(code, ctx, line_width, select, gs);
    }
    gs.backend.reset_style();
}

#[inline]
pub fn cursor_fast(code: &mut EditorLine, ctx: &mut LineContext, line: Line, gs: &mut GlobalState) {
    let select = ctx.select_get(code.char_len());
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


#[inline]
pub fn multi_cursor(
    code: &mut EditorLine,
    ctx: &mut LineContext,
    line: Line,
    gs: &mut GlobalState,
    cursors: Vec<CursorPosition>,
    selects: Vec<CharRange>,
) {
    let line_width = ctx.setup_cursor(line, gs.backend());
    match code.is_simple() {
        true => ascii_multi_cursor::render(code, ctx, line_width, cursors, selects, gs),
        false => complex_multi_cursor::render(code, ctx, line_width, cursors, selects, gs),
    }
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
        let select = ctx.select_get(text.char_len());
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
        let select = ctx.select_get(text.char_len());
        if text.cached.should_render_line(line.row, &select) {
            inner_render(text, ctx, line, select, gs);
        } else {
            ctx.skip_line();
        }
        return false;
    }
    true
}

#[inline]
pub fn inner_render(
    code: &mut EditorLine,
    ctx: &mut LineContext,
    line: Line,
    select: Option<CharRange>,
    gs: &mut GlobalState,
) {
    let cache_line = line.row;
    let line_width = ctx.setup_line(line, gs.backend());
    code.cached.line(cache_line, select.clone());
    match select {
        Some(select) => {
            if code.char_len() == 0 {
                gs.backend.print_styled(" ", gs.get_select_style());
            } else if code.is_simple() {
                render_select_ascii(code, line_width, select, ctx, gs);
            } else {
                render_select_complex(code, line_width, select, ctx, gs);
            }
        },
        None => {
            if code.is_simple() {
                // ascii (byte idx based) render
                if line_width > code.len() {
                    ascii_line::ascii_line(code.as_str(), code.tokens(), gs.backend());
                    if let Some(diagnostic) = code.diagnostics() {
                        let diagnosic_width = line_width - code.char_len();
                        diagnostic.render_pad_5(diagnosic_width, gs.backend())
                    }
                } else {
                    ascii_line::ascii_line(&code.as_str()[..line_width.saturating_sub(1)], code.tokens(), gs.backend());
                    gs.backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
                }
            } else {
                let Some(max_width) = complex_line::complex_line(code, line_width, ctx, gs.backend()) else {
                    return;
                };
        
                if let Some(diagnostics) = code.diagnostics() {
                    diagnostics.render_pad_5(max_width, gs.backend());
                }
            }
        },
    }
}

#[inline(always)]
fn render_select_ascii(
    code: &mut EditorLine,
    line_width: usize,
    select: CharRange,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    if line_width > code.char_len() {
        if select.from >= code.char_len() {
            ascii_line::ascii_line(code.as_str(), code.tokens(), gs.backend());
            let select_style = ContentStyle::bg(gs.theme.selected);
            gs.backend().print_styled(" ", select_style);
            if let Some(diagnostic) = code.diagnostics() {
                let diagnostic_width = line_width - code.char_len();
                diagnostic.render_pad_4(diagnostic_width, gs.backend())
            }
        } else if select.to >= code.char_len() {
            let content = code.chars();
            ascii_line::ascii_line_with_select(content, code.tokens(), select, gs);
            let select_style = ContentStyle::bg(gs.theme.selected);
            gs.backend().print_styled(" ", select_style);
            if let Some(diagnostic) = code.diagnostics() {
                let diagnostic_width = line_width - code.char_len();
                diagnostic.render_pad_4(diagnostic_width, gs.backend())
            }
        } else {
            let content = code.chars();
            ascii_line::ascii_line_with_select(content, code.tokens(), select, gs);
            if let Some(diagnostic) = code.diagnostics() {
                let diagnostic_width = line_width - code.char_len();
                diagnostic.render_pad_5(diagnostic_width, gs.backend())
            }
        }
    } else {
        let content = code.chars().take(line_width.saturating_sub(1));
        ascii_line::ascii_line_with_select(content, code.tokens(), select, gs);
        gs.backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}

#[inline(always)]
fn render_select_complex(
    code: &mut EditorLine,
    line_width: usize,
    select: CharRange,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    if select.from >= code.char_len() {
        let Some(max_width) = complex_line::complex_line(code, line_width, ctx, gs.backend()) else {
            return;
        };
        let select_style = ContentStyle::bg(gs.theme.selected);
        gs.backend().print_styled(" ", select_style);
        if let Some(diagnostics) = code.diagnostics() {
            diagnostics.render_pad_4(max_width, gs.backend());
        }
    } else if select.to >= code.char_len() {
        let Some(max_width) = complex_line::complex_line_with_select(code, line_width, select, ctx, gs) else {
            return;
        };
        let select_style = ContentStyle::bg(gs.theme.selected);
        gs.backend().print_styled(" ", select_style);
        if let Some(diagnostics) = code.diagnostics() {
            diagnostics.render_pad_4(max_width, gs.backend());
        }    
    } else {
        let Some(max_width) = complex_line::complex_line_with_select(code, line_width, select, ctx, gs) else {
            return;
        };

        if let Some(diagnostics) = code.diagnostics() {
            diagnostics.render_pad_5(max_width, gs.backend());
        }
    }
}

#[cfg(test)]
mod tests;

mod ascii;
mod complex;

use std::ops::Range;

use crate::{
    global_state::GlobalState,
    syntax::tokens::{calc_wrap_line, calc_wrap_line_capped},
    workspace::{
        cursor::Cursor,
        line::{EditorLine, LineContext},
    },
};
use idiom_tui::layout::{IterLines, RectIter};

#[inline]
pub fn repositioning(cursor: &mut Cursor, content: &mut [EditorLine]) -> Option<usize> {
    if let Some(skipped) = calc_wrap_line_capped(&mut content[cursor.line], cursor) {
        cursor.at_line = cursor.line;
        return Some(skipped);
    };
    if cursor.at_line > cursor.line {
        cursor.at_line = cursor.line;
        return None;
    }
    let mut row_sum = calc_rows(content, cursor);
    while row_sum > cursor.max_rows {
        if cursor.at_line == cursor.line {
            return None;
        }
        row_sum -= 1 + content[cursor.at_line].tokens().char_len();
        cursor.at_line += 1;
    }
    None
}

fn calc_rows(content: &mut [EditorLine], cursor: &Cursor) -> usize {
    // offset by 2 to ensure the line below is also recalculated
    // that will prevent artefacts when next line is saturated and cursor caused it to exapand
    let take = (cursor.line + 2) - cursor.at_line;
    let text_width = cursor.text_width;
    let mut buf = 0;
    for (idx, text) in content.iter_mut().enumerate().skip(cursor.at_line).take(take) {
        if idx != cursor.line {
            calc_wrap_line(text, text_width);
        }
        buf += 1 + text.tokens().char_len();
    }
    buf
}

#[inline(always)]
pub fn cursor(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    skip: usize,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    gs: &mut GlobalState,
) {
    text.cached.cursor(lines.next_line_idx(), ctx.cursor_char(), skip, select.clone());
    match text.is_simple() {
        true => ascii::cursor(text, select, skip, lines, ctx, gs),
        false => complex::cursor(text, select, skip, lines, ctx, gs),
    }
}

#[inline(always)]
pub fn line(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    gs: &mut GlobalState,
) {
    text.cached.line(lines.next_line_idx(), select.clone());
    match text.is_simple() {
        true => match select {
            Some(select) => ascii::line_with_select(text, select, lines, ctx, gs),
            None => ascii::line(text, lines, ctx, gs.backend()),
        },
        false => match select {
            Some(select) => complex::line_with_select(text, select, lines, ctx, gs),
            None => complex::line(text, lines, ctx, gs.backend()),
        },
    }
}

#[cfg(test)]
mod tests;

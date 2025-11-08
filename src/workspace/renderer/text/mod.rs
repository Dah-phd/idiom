mod ascii;
mod complex;

use super::utils::{pad_select, try_cache_wrap_data_from_lines, SelectManagerSimple};
use crate::{
    global_state::GlobalState,
    syntax::tokens::WrapData,
    workspace::{
        cursor::{CharRangeUnbound, Cursor},
        line::{EditorLine, LineContext},
    },
};
use idiom_tui::layout::{IterLines, RectIter};

pub fn reposition(cursor: &mut Cursor, content: &mut [EditorLine]) -> Option<usize> {
    let cursor_wraps = WrapData::calc_wraps_to_cursor(cursor, content);
    if cursor_wraps > cursor.max_rows {
        cursor.at_line = cursor.line;
        return Some(cursor_wraps - cursor.max_rows);
    }
    if cursor.at_line > cursor.line {
        cursor.at_line = cursor.line;
        return None;
    }
    let mut free_rows = cursor.max_rows - cursor_wraps;
    for (idx, text) in content.iter_mut().enumerate().skip(cursor.at_line).take(cursor.line - cursor.at_line).rev() {
        let wraps = WrapData::from_text_cached(text, cursor.text_width).count();
        if wraps > free_rows {
            cursor.at_line = idx + 1;
            break;
        }
        free_rows -= wraps;
    }
    None
}

#[inline(always)]
pub fn cursor(
    text: &mut EditorLine,
    select: Option<CharRangeUnbound>,
    skip: usize,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    gs: &mut GlobalState,
) {
    text.cached.cursor(lines.next_line_idx(), ctx.cursor_char(), skip, select.clone());
    match text.is_simple() {
        true => ascii::cursor(text, select, skip, lines, ctx, gs),
        false => {
            let len_pre_render = lines.len();
            complex::cursor(text, select, skip, lines, ctx, gs);
            try_cache_wrap_data_from_lines(text, len_pre_render, lines, ctx);
        }
    }
}

#[inline(always)]
pub fn line(
    text: &mut EditorLine,
    select: Option<CharRangeUnbound>,
    ctx: &mut LineContext,
    lines: &mut RectIter,
    gs: &mut GlobalState,
) {
    text.cached.line(lines.next_line_idx(), select.clone());
    match text.is_simple() {
        true => match select.and_then(|select| SelectManagerSimple::new(select, gs.theme.selected)) {
            Some(select) => ascii::line_with_select(text, select, lines, ctx, gs),
            None => ascii::line(text, lines, ctx, gs.backend()),
        },
        false => {
            let len_pre_render = lines.len();
            match select.and_then(|select| SelectManagerSimple::new(select, gs.theme.selected)) {
                Some(select) => complex::line_with_select(text, select, lines, ctx, gs),
                None => complex::line(text, lines, ctx, gs.backend()),
            }
            try_cache_wrap_data_from_lines(text, len_pre_render, lines, ctx);
        }
    }
}

#[cfg(test)]
mod tests;

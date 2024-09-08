mod ascii;
mod complex;

use crate::{
    render::{backend::Backend, layout::RectIter},
    syntax::tokens::calc_wrapse_line,
    workspace::{
        cursor::Cursor,
        line::{EditorLine, LineContext},
    },
};

pub fn repositioning(cursor: &mut Cursor, content: &mut [EditorLine]) {
    calc_wrapse_line(&mut content[cursor.line], cursor.text_width);
    if cursor.at_line > cursor.line {
        cursor.at_line = cursor.line;
        return;
    }
    while calc_rows(content, cursor) > cursor.max_rows {
        if cursor.at_line == cursor.line {
            return;
        }
        cursor.at_line += 1;
    }
}

fn calc_rows(content: &[EditorLine], cursor: &Cursor) -> usize {
    let take = (cursor.line + 1) - cursor.at_line;
    content.iter().skip(cursor.at_line).take(take).map(|eline| eline.tokens.char_len() + 1).sum()
}

#[inline(always)]
pub fn cursor(text: &mut EditorLine, ctx: &mut LineContext, lines: &mut RectIter, backend: &mut Backend) {
    match text.is_simple() {
        true => ascii::cursor(text, lines, ctx, backend),
        false => complex::cursor(text, lines, ctx, backend),
    }
}

#[inline(always)]
pub fn line(text: &mut EditorLine, ctx: &mut LineContext, lines: &mut RectIter, backend: &mut Backend) {
    let select = ctx.get_select(text.char_len());
    match text.is_simple() {
        true => match select {
            Some(select) => ascii::line_with_select(text, select, lines, ctx, backend),
            None => ascii::line(text, lines, ctx, backend),
        },
        false => match select {
            Some(select) => complex::line_with_select(text, select, lines, ctx, backend),
            None => complex::line(text, lines, ctx, backend),
        },
    }
}

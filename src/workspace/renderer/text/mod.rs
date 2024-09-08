mod ascii;
mod complex;
use crate::{
    render::{backend::Backend, layout::RectIter},
    workspace::line::{EditorLine, LineContext},
};

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

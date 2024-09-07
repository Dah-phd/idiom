mod ascii;
mod complex;
use crate::{
    render::{backend::Backend, layout::RectIter},
    workspace::line::{EditorLine, LineContext},
};

#[inline(always)]
pub fn cursor(text: &mut EditorLine, ctx: &mut LineContext, lines: &mut RectIter, backend: &mut Backend) {
    if text.is_simple() {
        ascii::render(text, lines, ctx, backend);
    } else {
        todo!()
    }
}

#[inline(always)]
pub fn line(text: &mut EditorLine, ctx: &mut LineContext, lines: &mut RectIter, backend: &mut Backend) {
    let select = ctx.get_select(text.char_len());
    if text.is_simple() {
        match select {
            Some(select) => ascii::ascii_line_with_select(text, select, lines, ctx, backend),
            None => ascii::ascii_line(text, lines, ctx, backend),
        }
    } else {
        todo!()
    }
}

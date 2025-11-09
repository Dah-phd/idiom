use super::{pad_select, SelectManagerSimple, StyledParser};
use crate::{
    ext_tui::CrossTerm,
    global_state::GlobalState,
    workspace::line::{EditorLine, LineContext},
};
use idiom_tui::{layout::RectIter, Backend};

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if let Some(parser) = StyledParser::new_ascii(lines, ctx, backend) {
        parser.render(text.as_str());
    }
    backend.reset_style();
}

pub fn line_with_select(
    text: &mut EditorLine,
    mut select: SelectManagerSimple,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    let Some(line) = lines.next() else { return };
    let backend = gs.backend();
    let line_width = ctx.setup_line(line, backend);

    if text.char_len() == 0 {
        pad_select(gs);
        return;
    }

    let mut line_end = line_width;

    let mut idx = 0;
    for text in text.chars() {
        if idx == line_end {
            let Some(line) = lines.next() else { return };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            line_end += line_width;
        }
        select.set_style(idx, backend);
        backend.print(text);
        idx += 1;
    }
    backend.reset_style();
    if idx >= line_end {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
    select.pad(gs);
}

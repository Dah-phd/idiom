use super::{SelectManagerSimple, StyledParser};
use crate::{
    ext_tui::CrossTerm,
    global_state::GlobalState,
    workspace::line::{EditorLine, LineContext},
};
use idiom_tui::{layout::RectIter, utils::CharLimitedWidths, Backend};

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if let Some(parser) = StyledParser::new_complex(lines, ctx, backend) {
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
    let backend = gs.backend();
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let mut remaining_width = line_width;

    for (idx, (text, current_width)) in CharLimitedWidths::new(text.as_str(), 3).enumerate() {
        if remaining_width < current_width {
            let Some(line) = lines.next() else { return };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            remaining_width = line_width;
        }
        remaining_width -= current_width;
        select.set_style(idx, backend);
        backend.print(text);
    }
    backend.reset_style();
    if remaining_width == 0 {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
    select.pad(gs);
}

use super::StyledParser;
use crate::{
    ext_tui::CrossTerm,
    workspace::line::{EditorLine, LineContext},
};
use idiom_tui::{layout::RectIter, Backend};

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if let Some(parser) = StyledParser::new_ascii(lines, ctx, backend) {
        parser.render(text.as_str());
    }
    backend.reset_style();
}

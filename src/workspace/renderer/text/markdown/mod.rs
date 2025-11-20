mod parser;
mod span;
mod tag;

use crate::{
    ext_tui::{iter::TakeLiens, CrossTerm},
    syntax::tokens::WrapData,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::{Attribute, Attributes, Color, ContentStyle};
use idiom_tui::{layout::RectIter, Backend};
pub use parser::{Span, Tag};
use tag::parse_tag;

const HEADING: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkRed),
    background_color: None,
    underline_color: Some(Color::DarkRed),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_2: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkBlue),
    background_color: None,
    underline_color: Some(Color::DarkBlue),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_3: ContentStyle = ContentStyle {
    foreground_color: Some(Color::DarkGreen),
    background_color: None,
    underline_color: Some(Color::DarkGreen),
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

const HEADING_NEXT: ContentStyle = ContentStyle {
    foreground_color: None,
    background_color: None,
    underline_color: None,
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Italic).with(Attribute::Underlined),
};

pub fn ascii_line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let text_width = ctx.setup_line(line, backend);
    parse_tag(text.as_str()).render_ascii(text_width, text_width, lines, ctx, backend);
    backend.reset_style();
}

pub fn ascii_line_exact(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let text_width = ctx.setup_line(line, backend);
    let wraps = WrapData::from_text_cached(text, text_width).count() - 1; // first in setup
    let mut take_lines = TakeLiens::new(lines, wraps);
    parse_tag(text.as_str()).render_ascii(text_width, text_width, &mut take_lines, ctx, backend);
    backend.reset_style();
    for remaining_line in take_lines {
        ctx.wrap_line(remaining_line, backend);
    }
}

pub fn complex_line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let text_width = ctx.setup_line(line, backend);
    parse_tag(text.as_str()).render(text_width, text_width, lines, ctx, backend);
    backend.reset_style();
}

pub fn complex_line_exact(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let text_width = ctx.setup_line(line, backend);
    let wraps = WrapData::from_text_cached(text, text_width).count() - 1; // first in setup
    let mut take_lines = TakeLiens::new(lines, wraps);
    parse_tag(text.as_str()).render(text_width, text_width, &mut take_lines, ctx, backend);
    backend.reset_style();
    for remaining_line in take_lines {
        ctx.wrap_line(remaining_line, backend);
    }
}

#[cfg(test)]
mod tests;

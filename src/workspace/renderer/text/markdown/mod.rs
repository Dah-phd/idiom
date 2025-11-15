pub mod ascii;
pub mod complex;
mod parser;
use crate::{ext_tui::CrossTerm, workspace::line::LineContext};
use crossterm::style::{Attribute, Attributes, Color, ContentStyle, Stylize};
use idiom_tui::{layout::RectIter, utils::CharLimitedWidths, Backend};
use parser::{parse, Block, Span};

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

struct StyledParser<'a, 'b> {
    lines: &'a mut RectIter,
    ctx: &'a mut LineContext,
    line_width: usize,
    backend: &'a mut CrossTerm,
    wrap_printer: fn(&mut Self, &str, usize) -> Option<usize>, //,usize, &mut RectIter, &mut LineContext, &mut Backend) -> Option<usize>,
}

impl<'a, 'b> StyledParser<'a, 'b> {
    fn new_ascii(lines: &'a mut RectIter, ctx: &'a mut LineContext, backend: &'a mut CrossTerm) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split })
    }

    fn new_complex(lines: &'a mut RectIter, ctx: &'a mut LineContext, backend: &'a mut CrossTerm) -> Option<Self> {
        let line = lines.next()?;
        let line_width = ctx.setup_line(line, backend);
        Some(Self { lines, ctx, line_width, backend, wrap_printer: print_split_comp })
    }

    fn render(mut self, content: &str) {
        let mut limit = self.line_width;
        let block = parse(content);
        match self.print_block(block, limit) {
            Some(new_limit) => limit = new_limit,
            None => return,
        };
        if limit == 0 {
            let Some(line) = self.lines.next() else { return };
            self.ctx.wrap_line(line, self.backend);
        }
    }

    fn print_block(&mut self, block: Block, mut limit: usize) -> Option<usize> {
        match block {
            Block::Header(header, level) => {
                for span in header {
                    match level {
                        1 => self.backend.set_style(HEADING),
                        2 => self.backend.set_style(HEADING_2),
                        3 => self.backend.set_style(HEADING_3),
                        _ => self.backend.set_style(HEADING_NEXT),
                    }
                    limit = self.print_span(span, limit)?;
                }
            }
            Block::Paragraph(parag) => {
                for span in parag {
                    limit = self.print_span(span, limit)?;
                }
            }
            Block::Blockquote(text, nesting) => {
                self.backend.set_style(self.ctx.accent_style);
                limit = (self.wrap_printer)(self, &text, limit)?;
            }
            Block::Hr => {
                self.backend.print((0..limit).map(|_| '-').collect::<String>());
                limit = 0;
            }
            Block::CodeBlock(Some(lang)) => {
                limit = (self.wrap_printer)(self, &format!(">>> {lang}"), limit)?;
            }
            Block::CodeBlock(None) => {
                limit = (self.wrap_printer)(self, "<<<", limit)?;
            }
        }
        Some(limit)
    }

    fn print_span(&mut self, span: Span, mut limit: usize) -> Option<usize> {
        match span {
            Span::Emphasis(spans) => {
                let style = self.backend.get_style();
                self.backend.set_style(style.italic());
                for span in spans {
                    limit = self.print_span(span, limit)?;
                }
                self.backend.set_style(style);
            }
            Span::Text(text) => limit = (self.wrap_printer)(self, &text, limit)?,
            Span::Strong(spans) => {
                let style = self.backend.get_style();
                self.backend.set_style(style.bold());
                for span in spans {
                    limit = self.print_span(span, limit)?;
                }
                self.backend.set_style(style);
            }
            Span::Image(name, path, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)(self, "Image", limit)?,
                    false => (self.wrap_printer)(self, &name, limit)?,
                };
                limit = (self.wrap_printer)(self, " > ", limit)?;
                limit = (self.wrap_printer)(self, &path, limit)?;
            }
            Span::Link(name, link, _) => {
                limit = match name.is_empty() {
                    true => (self.wrap_printer)(self, "Link", limit)?,
                    false => (self.wrap_printer)(self, &name, limit)?,
                };
                limit = (self.wrap_printer)(self, " > ", limit)?;
                limit = (self.wrap_printer)(self, &link, limit)?;
            }
            Span::Code(text) => limit = (self.wrap_printer)(self, &text, limit)?,
        }
        Some(limit)
    }
}

fn print_split(parser: &mut StyledParser, text: &str, limit: usize) -> Option<usize> {
    match text.len() > limit {
        true => {
            let (first, mut text) = text.split_at(limit);
            parser.backend.print(first);
            loop {
                let next_line = parser.lines.next()?;
                parser.ctx.wrap_line(next_line, parser.backend);
                match text.len() > parser.line_width {
                    true => {
                        let (part, remaining) = text.split_at(parser.line_width);
                        text = remaining;
                        parser.backend.print(part);
                    }
                    false => {
                        parser.backend.print(text);
                        return Some(parser.line_width - text.len());
                    }
                }
            }
        }
        false => {
            parser.backend.print(text);
            Some(limit - text.len())
        }
    }
}

fn print_split_comp(parser: &mut StyledParser, text: &str, mut limit: usize) -> Option<usize> {
    for (ch, ch_width) in CharLimitedWidths::new(text, 3) {
        match ch_width > limit {
            true => {
                let line = parser.lines.next()?;
                parser.ctx.wrap_line(line, parser.backend);
                parser.backend.print(ch);
                limit = parser.line_width - ch_width;
            }
            false => {
                parser.backend.print(ch);
                limit -= ch_width;
            }
        }
    }
    Some(limit)
}

#[cfg(test)]
mod tests;

use super::{HEADING, HEADING_2, HEADING_3, HEADING_NEXT};
use crate::{ext_tui::CrossTerm, workspace::line::LineContext};
use crossterm::style::Stylize;
use idiom_tui::{layout::Line, utils::CharLimitedWidths, Backend, UTFSafe};

#[derive(Debug, PartialEq, Clone)]
pub enum Tag<'a> {
    Header(Vec<Span<'a>>, usize),
    Paragraph(Vec<Span<'a>>),
    Blockquote(Vec<Span<'a>>, usize),
    Code(Option<String>),
    Hr,
}

impl<'a> Tag<'a> {
    pub fn render(
        &'a self,
        mut limit: usize,
        text_width: usize,
        lines: &mut impl Iterator<Item = Line>,
        ctx: &'a mut LineContext,
        backend: &'a mut CrossTerm,
    ) -> Option<usize> {
        match self {
            Tag::Header(header, level) => {
                match level {
                    1 => backend.set_style(HEADING),
                    2 => backend.set_style(HEADING_2),
                    3 => backend.set_style(HEADING_3),
                    _ => backend.set_style(HEADING_NEXT),
                }
                for span in header {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Paragraph(parag) => {
                for span in parag {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Blockquote(spans, nesting) => {
                backend.set_style(ctx.accent_style);
                limit = print_split_ascii(&format!("{:|>1$}", "", nesting), limit, text_width, lines, ctx, backend)?;
                for span in spans {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Hr => {
                backend.print(format!("{:->1$}", "", limit));
                limit = 0;
            }
            Tag::Code(Some(lang)) => {
                limit = print_split(&format!(">>> {lang}"), limit, text_width, lines, ctx, backend)?;
            }
            Tag::Code(None) => {
                limit = print_split_ascii("<<<", limit, text_width, lines, ctx, backend)?;
            }
        }
        Some(limit)
    }

    pub fn render_ascii(
        &'a self,
        mut limit: usize,
        text_width: usize,
        lines: &mut impl Iterator<Item = Line>,
        ctx: &'a mut LineContext,
        backend: &'a mut CrossTerm,
    ) -> Option<usize> {
        match self {
            Tag::Header(header, level) => {
                for span in header {
                    match level {
                        1 => backend.set_style(HEADING),
                        2 => backend.set_style(HEADING_2),
                        3 => backend.set_style(HEADING_3),
                        _ => backend.set_style(HEADING_NEXT),
                    }
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Paragraph(parag) => {
                for span in parag {
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Blockquote(spans, nesting) => {
                backend.set_style(ctx.accent_style);
                limit = print_split_ascii(&format!("{:|>1$}", "", nesting), limit, text_width, lines, ctx, backend)?;
                for span in spans {
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
            }
            Tag::Hr => {
                backend.print(format!("{:->1$}", "", limit));
                limit = 0;
            }
            Tag::Code(Some(lang)) => {
                limit = print_split_ascii(&format!(">>> {lang}"), limit, text_width, lines, ctx, backend)?;
            }
            Tag::Code(None) => {
                limit = print_split_ascii("<<<", limit, text_width, lines, ctx, backend)?;
            }
        }
        Some(limit)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Span<'a> {
    Text(&'a str),
    Link(String, String, Option<String>),
    Image(String, String, Option<String>),
    Emphasis(Vec<Span<'a>>),
    Strong(Vec<Span<'a>>),
    Code(&'a str),
}

impl<'a> Span<'a> {
    pub fn render(
        &'a self,
        mut limit: usize,
        text_width: usize,
        lines: &mut impl Iterator<Item = Line>,
        ctx: &'a mut LineContext,
        backend: &'a mut CrossTerm,
    ) -> Option<usize> {
        match self {
            Span::Emphasis(spans) => {
                let style = backend.get_style();
                backend.set_style(style.italic());
                for span in spans {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
                backend.set_style(style);
            }
            Span::Text(text) => limit = print_split(text, limit, text_width, lines, ctx, backend)?,
            Span::Strong(spans) => {
                let style = backend.get_style();
                backend.set_style(style.bold());
                for span in spans {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
                backend.set_style(style);
            }
            Span::Image(name, path, _) => {
                let style = backend.get_style();
                backend.set_style(style.underlined());
                limit = match name.is_empty() {
                    true => print_split_ascii("Image", limit, text_width, lines, ctx, backend)?,
                    false => print_split(name, limit, text_width, lines, ctx, backend)?,
                };
                backend.set_style(style);
                backend.pad(4);
                if limit > 6 {
                    backend.print_styled(path.truncate_width(limit - 4).1, ctx.accent_style.italic());
                }
                limit = 0;
            }
            Span::Link(name, link, _) => {
                let style = backend.get_style();
                backend.set_style(style.underlined());
                limit = match name.is_empty() {
                    true => print_split_ascii("Link", limit, text_width, lines, ctx, backend)?,
                    false => print_split(name, limit, text_width, lines, ctx, backend)?,
                };
                backend.set_style(style);
                backend.pad(4);
                if limit > 6 {
                    backend.print_styled(link.truncate_width(limit - 4).1, ctx.accent_style.italic());
                }
                limit = 0;
            }
            Span::Code(text) => limit = print_split(text, limit, text_width, lines, ctx, backend)?,
        }
        Some(limit)
    }

    pub fn render_ascii(
        &'a self,
        mut limit: usize,
        text_width: usize,
        lines: &mut impl Iterator<Item = Line>,
        ctx: &'a mut LineContext,
        backend: &'a mut CrossTerm,
    ) -> Option<usize> {
        match self {
            Span::Emphasis(spans) => {
                let style = backend.get_style();
                backend.set_style(style.italic());
                for span in spans {
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
                backend.set_style(style);
            }
            Span::Text(text) => limit = print_split_ascii(text, limit, text_width, lines, ctx, backend)?,
            Span::Strong(spans) => {
                let style = backend.get_style();
                backend.set_style(style.bold());
                for span in spans {
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
                backend.set_style(style);
            }
            Span::Image(name, path, _) => {
                let style = backend.get_style();
                backend.set_style(style.underlined());
                limit = match name.is_empty() {
                    true => print_split_ascii("Image", limit, text_width, lines, ctx, backend)?,
                    false => print_split_ascii(name, limit, text_width, lines, ctx, backend)?,
                };
                backend.set_style(style);
                backend.pad(4);
                if limit > 6 {
                    let end = std::cmp::min(limit - 4, path.len());
                    backend.print_styled(&path[..end], ctx.accent_style.italic());
                }
                limit = 0;
            }
            Span::Link(name, link, _) => {
                let style = backend.get_style();
                backend.set_style(style.underlined());
                limit = match name.is_empty() {
                    true => print_split_ascii("Link", limit, text_width, lines, ctx, backend)?,
                    false => print_split_ascii(name, limit, text_width, lines, ctx, backend)?,
                };
                backend.set_style(style);
                backend.pad(4);
                if limit > 6 {
                    let end = std::cmp::min(limit - 4, link.len());
                    backend.print_styled(&link[..end], ctx.accent_style.italic());
                }
                limit = 0;
            }
            Span::Code(text) => limit = print_split_ascii(text, limit, text_width, lines, ctx, backend)?,
        }
        Some(limit)
    }
}

fn print_split_ascii(
    text: &str,
    limit: usize,
    text_width: usize,
    lines: &mut impl Iterator<Item = Line>,
    ctx: &mut LineContext,
    backend: &mut CrossTerm,
) -> Option<usize> {
    match text.len() > limit {
        true => {
            let (first, mut text) = text.split_at(limit);
            backend.print(first);
            loop {
                let next_line = lines.next()?;
                ctx.wrap_line(next_line, backend);
                match text.len() > text_width {
                    true => {
                        let (part, remaining) = text.split_at(text_width);
                        text = remaining;
                        backend.print(part);
                    }
                    false => {
                        backend.print(text);
                        return Some(text_width - text.len());
                    }
                }
            }
        }
        false => {
            backend.print(text);
            Some(limit - text.len())
        }
    }
}

fn print_split(
    text: &str,
    mut limit: usize,
    text_width: usize,
    lines: &mut impl Iterator<Item = Line>,
    ctx: &mut LineContext,
    backend: &mut CrossTerm,
) -> Option<usize> {
    for (ch, ch_width) in CharLimitedWidths::new(text, 3) {
        match ch_width > limit {
            true => {
                let line = lines.next()?;
                ctx.wrap_line(line, backend);
                backend.print(ch);
                limit = text_width - ch_width;
            }
            false => {
                backend.print(ch);
                limit -= ch_width;
            }
        }
    }
    Some(limit)
}

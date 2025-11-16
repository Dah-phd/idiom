mod block;
mod span;
use super::{HEADING, HEADING_2, HEADING_3, HEADING_NEXT};
use crate::{ext_tui::CrossTerm, workspace::line::LineContext};
use crossterm::style::Stylize;
use idiom_tui::{layout::Line, utils::CharLimitedWidths, Backend};

pub fn parse<'a>(md: &'a str) -> Block<'a> {
    block::parse_blocks(md).unwrap_or(Block::Paragraph(span::parse_spans(md)))
}

#[derive(Debug, PartialEq, Clone)]
pub enum Block<'a> {
    Header(Vec<Span<'a>>, usize),
    Paragraph(Vec<Span<'a>>),
    Blockquote(String, usize),
    Code(Option<String>),
    Hr,
}

impl<'a> Block<'a> {
    pub fn render(
        &'a self,
        mut limit: usize,
        text_width: usize,
        lines: &mut impl Iterator<Item = Line>,
        ctx: &'a mut LineContext,
        backend: &'a mut CrossTerm,
    ) -> Option<usize> {
        match self {
            Block::Header(header, level) => {
                for span in header {
                    match level {
                        1 => backend.set_style(HEADING),
                        2 => backend.set_style(HEADING_2),
                        3 => backend.set_style(HEADING_3),
                        _ => backend.set_style(HEADING_NEXT),
                    }
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
            }
            Block::Paragraph(parag) => {
                for span in parag {
                    limit = span.render(limit, text_width, lines, ctx, backend)?;
                }
            }
            Block::Blockquote(text, nesting) => {
                backend.set_style(ctx.accent_style);
                limit = print_split(text, limit, text_width, lines, ctx, backend)?;
            }
            Block::Hr => {
                backend.print((0..limit).map(|_| '-').collect::<String>());
                limit = 0;
            }
            Block::Code(Some(lang)) => {
                limit = print_split(&format!(">>> {lang}"), limit, text_width, lines, ctx, backend)?;
            }
            Block::Code(None) => {
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
            Block::Header(header, level) => {
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
            Block::Paragraph(parag) => {
                for span in parag {
                    limit = span.render_ascii(limit, text_width, lines, ctx, backend)?;
                }
            }
            Block::Blockquote(text, nesting) => {
                backend.set_style(ctx.accent_style);
                limit = print_split_ascii(text, limit, text_width, lines, ctx, backend)?;
            }
            Block::Hr => {
                backend.print((0..limit).map(|_| '-').collect::<String>());
                limit = 0;
            }
            Block::Code(Some(lang)) => {
                limit = print_split_ascii(&format!(">>> {lang}"), limit, text_width, lines, ctx, backend)?;
            }
            Block::Code(None) => {
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
                limit = match name.is_empty() {
                    true => print_split_ascii("Image", limit, text_width, lines, ctx, backend)?,
                    false => print_split(name, limit, text_width, lines, ctx, backend)?,
                };
                limit = print_split_ascii(" > ", limit, text_width, lines, ctx, backend)?;
                limit = print_split(path, limit, text_width, lines, ctx, backend)?;
            }
            Span::Link(name, link, _) => {
                limit = match name.is_empty() {
                    true => print_split_ascii("Link", limit, text_width, lines, ctx, backend)?,
                    false => print_split(name, limit, text_width, lines, ctx, backend)?,
                };
                limit = print_split_ascii(" > ", limit, text_width, lines, ctx, backend)?;
                limit = print_split(link, limit, text_width, lines, ctx, backend)?;
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
                limit = match name.is_empty() {
                    true => print_split_ascii("Image", limit, text_width, lines, ctx, backend)?,
                    false => print_split_ascii(name, limit, text_width, lines, ctx, backend)?,
                };
                limit = print_split_ascii(" > ", limit, text_width, lines, ctx, backend)?;
                limit = print_split_ascii(path, limit, text_width, lines, ctx, backend)?;
            }
            Span::Link(name, link, _) => {
                limit = match name.is_empty() {
                    true => print_split_ascii("Link", limit, text_width, lines, ctx, backend)?,
                    false => print_split_ascii(name, limit, text_width, lines, ctx, backend)?,
                };
                limit = print_split_ascii(" > ", limit, text_width, lines, ctx, backend)?;
                limit = print_split_ascii(link, limit, text_width, lines, ctx, backend)?;
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

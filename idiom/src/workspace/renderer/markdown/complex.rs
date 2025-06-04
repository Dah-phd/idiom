use super::StyledParser;
use crate::global_state::CrossTerm;
use crate::workspace::line::{EditorLine, LineContext};
use crossterm::style::ContentStyle;
use std::ops::Range;

use idiom_ui::{
    backend::{Backend, StyleExt},
    layout::RectIter,
    utils::CharLimitedWidths,
};

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    if let Some(parser) = StyledParser::new_complex(lines, ctx, backend) {
        parser.render(&text.content);
    }
    backend.reset_style();
}

pub fn line_with_select(
    text: &mut EditorLine,
    select: Range<usize>,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl Backend,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let mut remaining_width = line_width;
    let select_color = ctx.lexer.theme.selected;

    for (idx, (text, current_width)) in CharLimitedWidths::new(&text.content, 3).enumerate() {
        if remaining_width < current_width {
            let Some(line) = lines.next() else { return };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            remaining_width = line_width;
        }
        remaining_width -= current_width;
        if select.start == idx {
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.reset_style();
        }
        backend.print(text);
    }
    backend.reset_style();
}

pub fn cursor(
    text: &mut EditorLine,
    select: Option<Range<usize>>,
    skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl Backend,
) {
    match select {
        Some(select) => self::select(text, select, skip, lines, ctx, backend),
        None => self::basic(text, skip, lines, ctx, backend),
    }
}

pub fn basic(
    text: &mut EditorLine,
    mut skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl Backend,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let mut content = CharLimitedWidths::new(&text.content, 3);
    let mut idx = 0;
    let mut remaining_width = line_width;

    if skip != 0 {
        for (ch, char_w) in content.by_ref() {
            idx += 1;
            if remaining_width < char_w {
                remaining_width = line_width - char_w;
                skip -= 1;
                if skip == 0 {
                    backend.print(ch);
                    break;
                }
            } else {
                remaining_width -= char_w;
            }
        }
    };

    for (text, current_width) in content {
        if remaining_width < current_width {
            let Some(line) = lines.next() else { break };
            ctx.wrap_line(line, backend);
            remaining_width = line_width;
        }
        remaining_width -= current_width;
        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    }
    backend.reset_style();
}

#[inline]
pub fn select(
    text: &mut EditorLine,
    select: Range<usize>,
    mut skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut impl Backend,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let select_color = ctx.lexer.theme.selected;
    let mut content = CharLimitedWidths::new(&text.content, 3);
    let mut idx = 0;
    let mut remaining_width = line_width;

    if skip != 0 {
        for (ch, char_w) in content.by_ref() {
            idx += 1;
            if remaining_width < char_w {
                remaining_width = line_width - char_w;
                skip -= 1;
                if skip == 0 {
                    if idx > select.start && select.end > idx {
                        backend.set_bg(Some(select_color));
                    }
                    backend.print(ch);
                    break;
                }
            } else {
                remaining_width -= char_w;
            }
        }
    }

    for (text, current_width) in content {
        if remaining_width < current_width {
            let Some(line) = lines.next() else { break };
            let reset_style = backend.get_style();
            backend.reset_style();
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            remaining_width = line_width;
        }

        remaining_width -= current_width;

        if select.start == idx {
            backend.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    }
    backend.reset_style();
}

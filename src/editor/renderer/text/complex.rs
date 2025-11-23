use super::SelectManagerSimple;
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    workspace::cursor::CharRangeUnbound,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::ContentStyle;
use idiom_tui::{
    layout::RectIter,
    utils::{CharLimitedWidths, WriteChunks},
    Backend,
};

pub fn line(text: &mut EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let mut chunks = WriteChunks::new(text.as_str(), line_width);

    let Some(chunk) = chunks.next() else { return };
    backend.print(chunk.text);
    let mut last_chunk_w = chunk.width;

    for chunk in chunks {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
        backend.print(chunk.text);
        last_chunk_w = chunk.width;
    }

    if last_chunk_w == line_width {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
}

pub fn line_with_select(
    text: &EditorLine,
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
            let reset_style = backend.get_style();
            backend.reset_style();
            let Some(line) = lines.next() else { return };
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

pub fn cursor(
    text: &mut EditorLine,
    select: Option<CharRangeUnbound>,
    skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    match select.and_then(|select| SelectManagerSimple::new(select, gs.theme.selected)) {
        Some(select) => self::select(text, select, skip, lines, ctx, gs),
        None => self::basic(text, skip, lines, ctx, gs.backend()),
    }
}

pub fn basic(
    text: &mut EditorLine,
    mut skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    backend: &mut CrossTerm,
) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let mut content = CharLimitedWidths::new(text.as_str(), 3);
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
            let Some(line) = lines.next() else { return };
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
    if remaining_width == 0 {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else {
        backend.print(" ");
    }
}

#[inline]
pub fn select(
    text: &mut EditorLine,
    mut select: SelectManagerSimple,
    mut skip: usize,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    let backend = gs.backend();
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let mut content = CharLimitedWidths::new(text.as_str(), 3);
    let mut idx = 0;
    let mut remaining_width = line_width;

    if skip != 0 {
        for (ch, char_w) in content.by_ref() {
            idx += 1;
            if remaining_width < char_w {
                remaining_width = line_width - char_w;
                skip -= 1;
                if skip == 0 {
                    select.go_to_index(idx, backend);
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
            let reset_style = backend.get_style();
            backend.reset_style();
            let Some(line) = lines.next() else { return };
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            remaining_width = line_width;
        }

        remaining_width -= current_width;
        select.set_style(idx, backend);
        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    backend.reset_style();
    if remaining_width == 0 {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else {
        select.pad(gs);
    }
}

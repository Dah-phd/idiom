use super::{pad_select, SelectManagerSimple};
use crate::{
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    workspace::cursor::CharRangeUnbound,
    workspace::line::{EditorLine, LineContext},
};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::RectIter, utils::ByteChunks, Backend};

pub fn line(text: &EditorLine, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let mut chunks = ByteChunks::new(text.as_str(), line_width);

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

    if text.char_len() == 0 {
        pad_select(gs);
        return;
    }

    let mut line_end = line_width;
    let mut idx = 0;

    for text in text.chars() {
        if idx == line_end {
            let reset_style = backend.get_style();
            backend.reset_style();
            let Some(line) = lines.next() else { return };
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            line_end += line_width;
        }
        select.set_style(idx, backend);
        backend.print(text);
        idx += 1;
    }
    backend.reset_style();
    if idx == line_end {
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
        Some(select) => self::select(text, skip, select, lines, ctx, gs),
        None => self::basic(text, skip, lines, ctx, gs.backend()),
    }
}

pub fn basic(text: &EditorLine, skip: usize, lines: &mut RectIter, ctx: &mut LineContext, backend: &mut CrossTerm) {
    let cursor_idx = ctx.cursor_char();
    let line_width = match lines.next() {
        Some(line) => ctx.setup_line(line, backend),
        None => return,
    };
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;
    for text in text.chars().skip(idx) {
        if idx == line_end {
            let Some(line) = lines.next() else { return };
            ctx.wrap_line(line, backend);
            line_end += line_width;
        }
        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    if idx == line_end {
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
    text: &EditorLine,
    skip: usize,
    mut select: SelectManagerSimple,
    lines: &mut RectIter,
    ctx: &mut LineContext,
    gs: &mut GlobalState,
) {
    let backend = gs.backend();

    let Some(line) = lines.next() else { return };
    let line_width = ctx.setup_line(line, backend);
    let cursor_idx = ctx.cursor_char();
    let mut idx = skip * line_width;
    let mut line_end = line_width + idx;

    select.go_to_index(idx, backend);

    for text in text.chars().skip(idx) {
        if idx == line_end {
            let reset_style = backend.get_style();
            backend.reset_style();
            let Some(line) = lines.next() else { return };
            ctx.wrap_line(line, backend);
            backend.set_style(reset_style);
            line_end += line_width;
        }
        select.set_style(idx, backend);
        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed())
        } else {
            backend.print(text);
        }
        idx += 1;
    }
    backend.reset_style();
    if idx == line_end {
        let Some(line) = lines.next() else { return };
        ctx.wrap_line(line, backend);
    }
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else {
        select.pad(gs);
    }
}

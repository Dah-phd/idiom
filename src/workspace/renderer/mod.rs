mod code;
mod markdown;
mod text;

use super::{line::LineContext, Editor};
use crate::{global_state::GlobalState, render::layout::IterLines, syntax::Lexer};

/// Component containing logic regarding rendering
/// In order to escape complicated state machines and any form on polymorphism,
/// it derives the correct function pointers on file opening.
pub struct Renderer {
    pub render: fn(&mut Editor, &mut GlobalState),
    pub fast_render: fn(&mut Editor, &mut GlobalState),
}

impl Renderer {
    pub fn code() -> Self {
        Self { render: code_render, fast_render: fast_code_render }
    }

    pub fn text() -> Self {
        Self { render: text_render, fast_render: fast_text_render }
    }

    pub fn markdown() -> Self {
        Self { render: md_render, fast_render: fast_md_render }
    }
}

// CODE

fn code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);
    code::repositioning(&mut editor.cursor);
    code_render_full(editor, gs);
}

fn fast_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);
    code::repositioning(&mut editor.cursor);
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return code_render_full(editor, gs);
    }
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    ctx.correct_last_line_match(&mut editor.content, lines.len());
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if editor.cursor.line == line_idx {
            code::cursor_fast(text, &mut ctx, line, backend);
        } else {
            let select = ctx.get_select(line.width);
            if text.cached.should_render_line(line.row, &select) {
                code::inner_render(text, &mut ctx, line, select, backend);
            } else {
                ctx.skip_line();
            }
        }
    }
    if !ctx.lexer.modal_is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.writer);
        }
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
    ctx.render_modal(gs);
}

#[inline(always)]
fn code_render_full(editor: &mut Editor, gs: &mut GlobalState) {
    editor.last_render_at_line.replace(editor.cursor.at_line);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if editor.cursor.line == line_idx {
            code::cursor(text, &mut ctx, line, backend);
        } else {
            let select = ctx.get_select(line.width);
            code::inner_render(text, &mut ctx, line, select, backend);
        }
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
    ctx.forced_modal_render(gs);
}

// TEXT

fn text_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = text::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    text_full_render(editor, gs, skip);
}

fn fast_text_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = text::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return text_full_render(editor, gs, skip);
    }
    editor.last_render_at_line.replace(editor.cursor.at_line);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.get_select_full_line(text.char_len());
        if editor.cursor.line == line_idx {
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                text::cursor(text, select, skip, &mut ctx, &mut lines, backend);
            } else {
                ctx.skip_line();
                lines.forward(1 + text.tokens.char_len());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            text::line(text, select, &mut ctx, &mut lines, backend)
        } else {
            ctx.skip_line();
            lines.forward(1 + text.tokens.char_len());
        }
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
}

#[inline(always)]
fn text_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    editor.last_render_at_line.replace(editor.cursor.at_line);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.get_select_full_line(text.char_len());
        if editor.cursor.line == line_idx {
            text::cursor(text, select, skip, &mut ctx, &mut lines, backend);
        } else {
            text::line(text, select, &mut ctx, &mut lines, backend)
        }
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
}

// MARKDOWN

fn md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = markdown::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    md_full_render(editor, gs, skip);
}

fn md_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    editor.last_render_at_line.replace(editor.cursor.at_line);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.get_select_full_line(text.char_len());
        if editor.cursor.line == line_idx {
            markdown::cursor(text, select, skip, &mut ctx, &mut lines, backend);
        } else {
            markdown::line(text, select, &mut ctx, &mut lines, backend)
        }
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
}

fn fast_md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = markdown::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return md_full_render(editor, gs, skip);
    }
    editor.last_render_at_line.replace(editor.cursor.at_line);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.get_select_full_line(text.char_len());
        if editor.cursor.line == line_idx {
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                markdown::cursor(text, select, skip, &mut ctx, &mut lines, backend);
            } else {
                ctx.skip_line();
                lines.forward(1 + text.tokens.char_len());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            markdown::line(text, select, &mut ctx, &mut lines, backend)
        } else {
            ctx.skip_line();
            lines.forward(1 + text.tokens.char_len());
        }
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
}

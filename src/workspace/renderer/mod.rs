mod code;
mod markdown;
mod text;

use super::{line::LineContext, Editor};
use crate::{global_state::GlobalState, syntax::Lexer};
use idiom_tui::layout::IterLines;

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

    pub fn multi_cursor(&mut self) {
        self.render = multi_code_render;
        self.fast_render = multi_fast_code_render;
    }

    pub fn single_cursor(&mut self) {
        self.render = code_render;
        self.fast_render = fast_code_render;
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

    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    ctx.correct_last_line_match(&mut editor.content, lines.len());
    let backend = &mut gs.backend;

    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor_fast(text, &mut ctx, line, backend);
        } else {
            let select = ctx.select_get(line.width);
            if text.cached.should_render_line(line.row, &select) {
                code::inner_render(text, &mut ctx, line, select, backend);
            } else {
                ctx.skip_line();
            }
        }
    }

    if !ctx.lexer.modal_is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
    ctx.render_modal(gs);
}

#[inline(always)]
fn code_render_full(editor: &mut Editor, gs: &mut GlobalState) {
    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;

    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, backend);
        } else {
            let select = ctx.select_get(line.width);
            code::inner_render(text, &mut ctx, line, select, backend);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
    ctx.forced_modal_render(gs);
}

// CODE RENDER MULTICURSOR

fn multi_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);
    code::repositioning(&mut editor.cursor);
    multi_code_render_full(editor, gs);
}

fn multi_fast_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);
    code::repositioning(&mut editor.cursor);
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return multi_code_render_full(editor, gs);
    }

    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    ctx.correct_last_line_match(&mut editor.content, lines.len());
    let backend = &mut gs.backend;

    ctx.init_multic_mod(&editor.controls.cursors);
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if let Some((cursors, selects)) = ctx.multic_line_setup(&editor.controls.cursors, line.width) {
            code::multi_cursor_fast(text, &mut ctx, line, backend, cursors, selects);
        } else if ctx.has_cursor(line_idx) {
            code::cursor_fast(text, &mut ctx, line, backend);
        } else {
            let select = ctx.select_get(line.width);
            if text.cached.should_render_line(line.row, &select) {
                code::inner_render(text, &mut ctx, line, select, backend);
            } else {
                ctx.skip_line();
            }
        }
    }

    if !ctx.lexer.modal_is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
    }

    gs.render_stats(editor.content.len(), editor.controls.cursors.len(), cursor.into());
    ctx.render_modal(gs);
}

#[inline(always)]
fn multi_code_render_full(editor: &mut Editor, gs: &mut GlobalState) {
    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;

    ctx.init_multic_mod(&editor.controls.cursors);
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if let Some((cursors, selects)) = ctx.multic_line_setup(&editor.controls.cursors, line.width) {
            code::multi_cursor(text, &mut ctx, line, backend, cursors, selects);
        } else if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, backend);
        } else {
            let select = ctx.select_get(line.width);
            code::inner_render(text, &mut ctx, line, select, backend);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), editor.controls.cursors.len(), cursor.into());
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

    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;

    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
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
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
}

#[inline(always)]
fn text_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            text::cursor(text, select, skip, &mut ctx, &mut lines, backend);
        } else {
            text::line(text, select, &mut ctx, &mut lines, backend)
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
}

// MARKDOWN

fn md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = markdown::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    md_full_render(editor, gs, skip);
}

fn md_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;

    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            markdown::cursor(text, select, skip, &mut ctx, &mut lines, backend);
        } else {
            markdown::line(text, select, &mut ctx, &mut lines, backend)
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
}

fn fast_md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = markdown::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return md_full_render(editor, gs, skip);
    }

    let cursor = &editor.cursor;
    let accent_style = gs.theme.accent_fg();
    let line_number_offset = editor.line_number_padding;

    editor.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(&mut editor.lexer, cursor, line_number_offset, accent_style);
    let backend = &mut gs.backend;

    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
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
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(editor.content.len(), cursor.select_len(&editor.content), cursor.into());
}

#[cfg(test)]
mod tests;

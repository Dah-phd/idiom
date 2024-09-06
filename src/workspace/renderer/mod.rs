mod code;
use crate::{global_state::GlobalState, workspace::Editor};

use super::line::CodeLineContext;

pub struct Renderer {
    pub render: fn(&mut Editor, &mut GlobalState),
    pub fast_render: fn(&mut Editor, &mut GlobalState),
}

impl Renderer {
    pub fn code() -> Self {
        Self { render: code_render, fast_render: fast_code_render }
    }
}

fn code_render(editor: &mut Editor, gs: &mut GlobalState) {
    editor.last_render_at_line.replace(editor.cursor.at_line);
    editor.sync(gs);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = CodeLineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if let Some(line) = lines.next() {
            if editor.cursor.line == line_idx {
                if text.tokens.is_empty() {
                    text.tokens.internal_rebase(&text.content, &ctx.lexer.lang, &ctx.lexer.theme);
                };
                code::cursor(text, &mut ctx, line, backend);
            } else {
                let select = ctx.get_select(line.width);
                code::inner_render(text, &mut ctx, line, select, backend);
            }
        } else {
            break;
        };
    }
    for line in lines {
        line.render_empty(&mut gs.writer);
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
    ctx.forced_modal_render(gs);
}

fn fast_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    if !matches!(editor.last_render_at_line, Some(idx) if idx == editor.cursor.at_line) {
        return code_render(editor, gs);
    }
    editor.sync(gs);
    let mut lines = gs.editor_area.into_iter();
    let mut ctx = CodeLineContext::collect_context(&mut editor.lexer, &editor.cursor, editor.line_number_offset);
    let backend = &mut gs.writer;
    for (line_idx, text) in editor.content.iter_mut().enumerate().skip(editor.cursor.at_line) {
        if let Some(line) = lines.next() {
            if editor.cursor.line == line_idx {
                if text.tokens.is_empty() {
                    text.tokens.internal_rebase(&text.content, &ctx.lexer.lang, &ctx.lexer.theme);
                    if !text.tokens.is_empty() {
                        text.cached.reset();
                    }
                };
                code::cursor_fast(text, &mut ctx, line, backend);
            } else {
                let select = ctx.get_select(line.width);
                if text.cached.should_render_line(line.row, &select) {
                    code::inner_render(text, &mut ctx, line, select, backend);
                } else {
                    ctx.skip_line();
                }
            }
        } else {
            break;
        };
    }
    if !ctx.lexer.modal_is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.writer);
        }
    }
    gs.render_stats(editor.content.len(), editor.cursor.select_len(&editor.content), (&editor.cursor).into());
    ctx.render_modal(gs);
}

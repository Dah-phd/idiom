mod code;
mod markdown;
mod text;

use super::{line::LineContext, Editor};
use crate::{
    configs::{FileFamily, FileType},
    global_state::GlobalState,
    syntax::Lexer,
};
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

    pub fn all_lines_cached(editor: &Editor) -> bool {
        match editor.file_type.is_code() {
            true => editor
                .content
                .iter()
                .skip(editor.cursor.at_line)
                .take(editor.cursor.max_rows)
                .all(|line| !line.cached.is_none()),
            false => {
                let mut rows = editor.cursor.max_rows;
                let idx = editor.cursor.at_line;
                while rows != 0 {
                    let Some(line) = editor.content.get(idx) else { break };
                    if line.cached.is_none() {
                        return false;
                    }
                    let estimated_wraps = line.len().div_ceil(editor.cursor.text_width);
                    rows = rows.saturating_sub(estimated_wraps);
                }
                true
            }
        }
    }

    pub fn try_multi_cursor(&mut self, file_type: FileType) -> bool {
        if !file_type.is_code() {
            return false;
        }
        self.render = multi_code_render;
        self.fast_render = multi_fast_code_render;
        true
    }

    pub fn single_cursor(&mut self, file_type: FileType) {
        match file_type.family() {
            FileFamily::Text => {
                self.render = text_render;
                self.fast_render = fast_text_render;
            }
            FileFamily::MarkDown => {
                self.render = md_render;
                self.fast_render = fast_md_render;
            }
            FileFamily::Code(..) => {
                self.render = code_render;
                self.fast_render = fast_code_render;
            }
        }
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

    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, modal, .. } = editor;

    code::repositioning(cursor);
    if !matches!(last_render_at_line, Some(idx) if *idx == cursor.at_line) {
        return code_render_full(editor, gs);
    }

    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);
    ctx.correct_last_line_match(content, lines.len());

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor_fast(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get(line.width);
            if text.cached.should_render_line(line.row, &select) {
                code::inner_render(text, &mut ctx, line, select, gs);
            } else {
                ctx.skip_line();
            }
        }
    }

    if !modal.is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
    let relative_pos = ctx.get_modal_relative_position();
    modal.render_if_exist(relative_pos, gs);
}

fn code_render_full(editor: &mut Editor, gs: &mut GlobalState) {
    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, modal, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get(line.width);
            code::inner_render(text, &mut ctx, line, select, gs);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
    let relative_pos = ctx.get_modal_relative_position();
    modal.forece_render_if_exists(relative_pos, gs);
}

// CODE RENDER MULTICURSOR

fn multi_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);
    code::repositioning(&mut editor.cursor);
    multi_code_render_full(editor, gs);
}

fn multi_fast_code_render(editor: &mut Editor, gs: &mut GlobalState) {
    Lexer::context(editor, gs);

    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, controls, modal, .. } = editor;

    code::repositioning(cursor);

    if !matches!(last_render_at_line, Some(idx) if *idx == cursor.at_line) {
        return multi_code_render_full(editor, gs);
    }

    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);
    ctx.correct_last_line_match(content, lines.len());

    let mut is_rendered_cursor = false;

    ctx.init_multic_mod(&controls.cursors);
    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        is_rendered_cursor |= code::fast_render_is_cursor(text, &controls.cursors, line, line_idx, &mut ctx, gs);
    }

    if is_rendered_cursor {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
        let relative_pos = ctx.get_modal_relative_position();
        modal.forece_render_if_exists(relative_pos, gs);
    } else {
        if !modal.is_rendered() {
            for line in lines {
                line.render_empty(&mut gs.backend);
            }
        }
        let relative_pos = ctx.get_modal_relative_position();
        modal.render_if_exist(relative_pos, gs);
    }
    gs.render_stats(content.len(), controls.cursors.len(), cursor.into());
}

fn multi_code_render_full(editor: &mut Editor, gs: &mut GlobalState) {
    let Editor { modal, lexer, cursor, content, line_number_padding, last_render_at_line, controls, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    ctx.init_multic_mod(&controls.cursors);
    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if let Some((cursors, selects)) = ctx.multic_line_setup(&controls.cursors, line.width) {
            code::multi_cursor(text, &mut ctx, line, gs, cursors, selects);
        } else if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get(line.width);
            code::inner_render(text, &mut ctx, line, select, gs);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), controls.cursors.len(), cursor.into());
    let relative_pos = ctx.get_modal_relative_position();
    modal.forece_render_if_exists(relative_pos, gs);
}

// TEXT

fn text_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = text::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    text_full_render(editor, gs, skip);
}

fn fast_text_render(editor: &mut Editor, gs: &mut GlobalState) {
    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, .. } = editor;

    let skip = text::repositioning(cursor, content).unwrap_or_default();
    if !matches!(last_render_at_line, Some(idx) if *idx == cursor.at_line) {
        return text_full_render(editor, gs, skip);
    }

    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
            } else {
                ctx.skip_line();
                lines.forward(1 + text.tokens().char_len());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            text::line(text, select, &mut ctx, &mut lines, gs)
        } else {
            ctx.skip_line();
            lines.forward(1 + text.tokens().char_len());
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
}

fn text_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
        } else {
            text::line(text, select, &mut ctx, &mut lines, gs)
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
}

// MARKDOWN

fn md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let skip = markdown::repositioning(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    md_full_render(editor, gs, skip);
}

fn fast_md_render(editor: &mut Editor, gs: &mut GlobalState) {
    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, .. } = editor;

    let skip = markdown::repositioning(cursor, content).unwrap_or_default();
    if !matches!(last_render_at_line, Some(idx) if *idx == cursor.at_line) {
        return md_full_render(editor, gs, skip);
    }

    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                markdown::cursor(text, select, skip, &mut ctx, &mut lines, gs);
            } else {
                ctx.skip_line();
                lines.forward(1 + text.tokens().char_len());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            markdown::line(text, select, &mut ctx, &mut lines, gs)
        } else {
            ctx.skip_line();
            lines.forward(1 + text.tokens().char_len());
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
}

fn md_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) {
    let Editor { lexer, cursor, content, line_number_padding, last_render_at_line, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = LineContext::collect_context(cursor, lexer.char_lsp_pos, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get_full_line(text.char_len());
        if ctx.has_cursor(line_idx) {
            markdown::cursor(text, select, skip, &mut ctx, &mut lines, gs);
        } else {
            markdown::line(text, select, &mut ctx, &mut lines, gs)
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    gs.render_stats(content.len(), cursor.select_len(content), cursor.into());
}

#[cfg(test)]
mod tests;

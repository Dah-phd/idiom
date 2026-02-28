mod code;
mod context;
mod text;
mod utils;

use crate::{
    configs::{FileFamily, FileType},
    editor::utils::EditorStats,
    editor::{
        syntax::{tokens::WrapData, Lexer},
        Editor,
    },
    global_state::GlobalState,
};
use context::CodecContext;
use idiom_tui::layout::IterLines;

/// Component containing logic regarding rendering
/// In order to escape complicated state machines and any form on polymorphism,
/// it derives the correct function pointers on file opening.
pub struct TuiCodec {
    last_render_len: usize,
    last_render_at_line: Option<usize>,
    render: fn(&mut Editor, &mut GlobalState) -> EditorStats,
    fast_render: fn(&mut Editor, &mut GlobalState) -> EditorStats,
}

impl TuiCodec {
    #[inline]
    pub fn code() -> Self {
        Self { render: code_render, fast_render: fast_code_render, last_render_at_line: None, last_render_len: 0 }
    }

    #[inline]
    pub fn text() -> Self {
        Self { render: text_render, fast_render: fast_text_render, last_render_at_line: None, last_render_len: 0 }
    }

    #[inline]
    pub fn markdown() -> Self {
        Self { render: md_render, fast_render: fast_md_render, last_render_at_line: None, last_render_len: 0 }
    }

    #[inline]
    pub fn render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
        editor.codec.last_render_len = editor.content.len();
        (editor.codec.render)(editor, gs)
    }

    #[inline]
    pub fn fast_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
        (editor.codec.fast_render)(editor, gs)
    }

    #[inline]
    pub fn clear_cache(&mut self) {
        self.last_render_at_line = None
    }

    #[inline]
    pub fn is_cached_at_line(&self, at_line: usize) -> bool {
        matches!(self.last_render_at_line, Some(idx) if idx == at_line)
    }

    pub fn all_lines_cached(editor: &mut Editor) -> bool {
        match editor.file_type.is_code() {
            true => editor
                .content
                .iter()
                .skip(editor.cursor.at_line)
                .take(editor.cursor.max_rows)
                .all(|line| !line.cached.is_none()),
            false => {
                let mut rows = editor.cursor.max_rows;
                let mut idx = editor.cursor.at_line;
                while rows != 0 {
                    let Some(line) = editor.content.get_mut(idx) else { break };
                    if line.cached.is_none() {
                        return false;
                    }
                    let wraps = WrapData::from_text_cached(line, editor.cursor.text_width).count();
                    rows = rows.saturating_sub(wraps);
                    idx += 1;
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

    fn is_full_render_needed(&mut self, content_len: usize, at_line: usize) -> bool {
        let is_multiline_reduction = self.last_render_len > content_len + 1;
        self.last_render_len = content_len; // reduction of 1 is allwed with context.correct_last_line_match
        is_multiline_reduction || !self.is_cached_at_line(at_line)
    }
}

// CODE

fn code_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    Lexer::context(editor, gs);
    code::reposition(&mut editor.cursor);
    code_render_full(editor, gs)
}

fn fast_code_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    Lexer::context(editor, gs);

    let Editor { lexer, cursor, content, line_number_padding, modal, codec, .. } = editor;

    code::reposition(cursor);
    if codec.is_full_render_needed(content.len(), cursor.at_line) {
        return code_render_full(editor, gs);
    }
    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);
    ctx.correct_last_line_match(content, lines.len());

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor_fast(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get();
            if text.cached.should_render_line(line.row, &select) {
                code::line_render(text, &mut ctx, line, select, gs);
            } else {
                ctx.skip_line();
            }
        }
    }

    if !modal.is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
        let relative_pos = ctx.get_modal_relative_position();
        modal.render_if_exists(relative_pos, gs);
    }

    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

fn code_render_full(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let Editor { lexer, cursor, content, line_number_padding, codec, modal, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    codec.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get();
            code::line_render(text, &mut ctx, line, select, gs);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    let relative_pos = ctx.get_modal_relative_position();
    modal.render_if_exists(relative_pos, gs);
    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

// CODE RENDER MULTICURSOR

fn multi_code_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    Lexer::context(editor, gs);
    code::reposition(&mut editor.cursor);
    multi_code_render_full(editor, gs)
}

fn multi_fast_code_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    Lexer::context(editor, gs);

    let Editor { lexer, cursor, content, line_number_padding, codec, controls, modal, .. } = editor;

    code::reposition(cursor);

    if codec.is_full_render_needed(content.len(), cursor.at_line) {
        return multi_code_render_full(editor, gs);
    }

    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);
    ctx.correct_last_line_match(content, lines.len());

    let mut is_rendered_cursor = false;

    ctx.init_multic_mod(controls.cursors());
    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        is_rendered_cursor |= code::fast_render_is_cursor(text, controls.cursors(), line, line_idx, &mut ctx, gs);
    }

    if is_rendered_cursor || !modal.is_rendered() {
        for line in lines {
            line.render_empty(&mut gs.backend);
        }
        let relative_pos = ctx.get_modal_relative_position();
        modal.render_if_exists(relative_pos, gs);
    }

    EditorStats { len: content.len(), select_len: controls.cursors_count(), position: cursor.into() }
}

fn multi_code_render_full(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let Editor { modal, lexer, cursor, content, line_number_padding, codec, controls, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    codec.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);

    ctx.init_multic_mod(controls.cursors());
    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        let line = match lines.next() {
            None => break,
            Some(line) => line,
        };
        if let Some((cursors, selects)) = ctx.multic_line_setup(controls.cursors(), line.width) {
            code::multi_cursor(text, &mut ctx, line, gs, cursors, selects);
        } else if ctx.has_cursor(line_idx) {
            code::cursor(text, &mut ctx, line, gs);
        } else {
            let select = ctx.select_get();
            code::line_render(text, &mut ctx, line, select, gs);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    let relative_pos = ctx.get_modal_relative_position();
    modal.render_if_exists(relative_pos, gs);
    EditorStats { len: content.len(), select_len: controls.cursors_count(), position: cursor.into() }
}

// TEXT

fn text_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let skip = text::reposition(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    text_full_render(editor, gs, skip)
}

fn fast_text_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let Editor { lexer, cursor, content, line_number_padding, codec, .. } = editor;

    let skip = text::reposition(cursor, content).unwrap_or_default();
    if codec.is_full_render_needed(content.len(), cursor.at_line) {
        return text_full_render(editor, gs, skip);
    }
    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);
    ctx.correct_last_line_match(content, lines.len());

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get();
        if ctx.has_cursor(line_idx) {
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
            } else {
                ctx.skip_line();
                lines.forward(WrapData::from_text_cached(text, cursor.text_width).count());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            text::line(text, select, &mut ctx, &mut lines, gs);
        } else {
            ctx.skip_line();
            lines.forward(WrapData::from_text_cached(text, cursor.text_width).count());
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

fn text_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) -> EditorStats {
    let Editor { lexer, cursor, content, line_number_padding, codec, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    codec.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get();
        if ctx.has_cursor(line_idx) {
            text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
        } else {
            text::line(text, select, &mut ctx, &mut lines, gs)
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

// MARKDOWN

fn md_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let skip = text::reposition(&mut editor.cursor, &mut editor.content).unwrap_or_default();
    md_full_render(editor, gs, skip)
}

fn fast_md_render(editor: &mut Editor, gs: &mut GlobalState) -> EditorStats {
    let Editor { lexer, cursor, content, line_number_padding, codec, .. } = editor;

    let skip = text::reposition(cursor, content).unwrap_or_default();
    if codec.is_full_render_needed(content.len(), cursor.at_line) {
        return md_full_render(editor, gs, skip);
    }
    let accent_style = gs.ui_theme.accent_fg();

    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);
    let mut cursor_rendered = false;

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get();
        if ctx.has_cursor(line_idx) {
            cursor_rendered = true;
            if text.cached.should_render_cursor(lines.next_line_idx(), ctx.cursor_char(), &select)
                || text.cached.skipped_chars() != skip
            {
                text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
            } else {
                ctx.skip_line();
                lines.forward(WrapData::from_text_cached(text, cursor.text_width).count());
            }
        } else if text.cached.should_render_line(lines.next_line_idx(), &select) {
            if cursor_rendered {
                text::md_line(text, select, &mut ctx, &mut lines, gs);
            } else {
                text::md_line_exact_styled_wraps(text, select, &mut ctx, &mut lines, gs);
            }
        } else {
            ctx.skip_line();
            lines.forward(WrapData::from_text_cached(text, cursor.text_width).count());
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

fn md_full_render(editor: &mut Editor, gs: &mut GlobalState, skip: usize) -> EditorStats {
    let Editor { lexer, cursor, content, line_number_padding, codec, .. } = editor;

    let accent_style = gs.ui_theme.accent_fg();

    codec.last_render_at_line.replace(cursor.at_line);
    let mut lines = gs.editor_area().into_iter();
    let mut ctx = CodecContext::collect_context(cursor, lexer.encoding().char_len, *line_number_padding, accent_style);
    let mut cursor_rendered = false;

    for (line_idx, text) in content.iter_mut().enumerate().skip(cursor.at_line) {
        if lines.is_finished() {
            break;
        }
        let select = ctx.select_get();
        if ctx.has_cursor(line_idx) {
            cursor_rendered = true;
            text::cursor(text, select, skip, &mut ctx, &mut lines, gs);
        } else if cursor_rendered {
            text::md_line(text, select, &mut ctx, &mut lines, gs);
        } else {
            text::md_line_exact_styled_wraps(text, select, &mut ctx, &mut lines, gs);
        }
    }

    for line in lines {
        line.render_empty(&mut gs.backend);
    }

    EditorStats { len: content.len(), select_len: cursor.select_len(content), position: cursor.into() }
}

#[cfg(test)]
mod tests;

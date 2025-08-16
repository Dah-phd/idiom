use super::{
    calc_wraps, controls, Actions, Cursor, Editor, EditorConfigs, EditorLine, FileType, GlobalState, Lexer, Renderer,
};
use crate::error::{IdiomError, IdiomResult};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

pub enum FileUpdate {
    None,
    Updated,
    Deny,
}

impl FileUpdate {
    pub fn deny(&mut self) {
        *self = Self::Deny
    }

    pub fn collect(&mut self) -> bool {
        match self {
            Self::Updated => {
                *self = Self::None;
                true
            }
            _ => false,
        }
    }

    pub fn mark_updated(&mut self) {
        match self {
            Self::None => *self = Self::Updated,
            Self::Deny => *self = Self::None,
            _ => (),
        }
    }
}

pub fn build_display(path: &Path) -> String {
    let mut buffer = Vec::new();
    let mut text_path = path.display().to_string();
    if let Ok(base_path) = PathBuf::from("./").canonicalize().map(|p| p.display().to_string()) {
        if text_path.starts_with(&base_path) {
            text_path.replace_range(..base_path.len(), "");
        }
    }
    for part in text_path.split(MAIN_SEPARATOR).rev().take(2) {
        buffer.insert(0, part);
    }
    buffer.join(MAIN_SEPARATOR_STR)
}

/// reject files over 50mb
pub fn big_file_protection(path: &Path) -> IdiomResult<()> {
    let meta = std::fs::metadata(path)?;
    if meta.size() > 50 * 1024 * 1024 {
        return Err(IdiomError::io_other("File over 50MB"));
    }
    Ok(())
}

/// calculates the max digits for line number
#[inline(always)]
pub const fn calc_line_number_offset(len: usize) -> usize {
    if len == 0 {
        1
    } else {
        (len.ilog10() + 1) as usize
    }
}

/// This is not a normal constructor for Editor
/// it should be used in cases where the content is present
/// or real file does not exists
pub fn text_editor_from_data(
    path: PathBuf,
    content: Vec<EditorLine>,
    cursor: Option<Cursor>,
    cfg: &EditorConfigs,
    gs: &mut GlobalState,
) -> Editor {
    let display = build_display(&path);
    let line_number_offset = calc_line_number_offset(content.len());

    let cursor = match cursor {
        Some(cursor) if cursor.matches_content(&content) => cursor,
        Some(..) | None => Cursor::default(),
    };

    let mut editor = Editor {
        actions: Actions::new(cfg.default_indent_cfg()),
        action_map: controls::single_cursor_map,
        update_status: FileUpdate::None,
        renderer: Renderer::text(),
        last_render_at_line: None,
        cursor,
        multi_positions: Vec::new(),
        line_number_offset,
        lexer: Lexer::text_lexer(&path, gs),
        content,
        file_type: FileType::Ignored,
        display,
        path,
    };
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    calc_wraps(&mut editor.content, editor.cursor.text_width);
    editor
}

pub fn editor_from_data(
    path: PathBuf,
    file_type: FileType,
    content: Vec<EditorLine>,
    cursor: Option<Cursor>,
    cfg: &EditorConfigs,
    gs: &mut GlobalState,
) -> Editor {
    if matches!(file_type, FileType::Ignored) {
        return text_editor_from_data(path, content, cursor, cfg, gs);
    };
    let display = build_display(&path);
    let line_number_offset = calc_line_number_offset(content.len());

    let cursor = match cursor {
        Some(cursor) if cursor.matches_content(&content) => cursor,
        Some(..) | None => Cursor::default(),
    };

    let mut editor = Editor {
        actions: Actions::new(cfg.get_indent_cfg(&file_type)),
        action_map: controls::single_cursor_map,
        update_status: FileUpdate::None,
        renderer: Renderer::code(),
        last_render_at_line: None,
        cursor,
        multi_positions: Vec::new(),
        line_number_offset,
        lexer: Lexer::with_context(file_type, &path, gs),
        content,
        file_type,
        display,
        path,
    };
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    editor
}

use super::{
    calc_wraps, controls::ControlMap, Actions, Cursor, Editor, EditorConfigs, EditorLine, EditorModal, FileFamily,
    FileType, GlobalState, Lexer, Renderer,
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

/// builds editor from provided data
pub fn editor_from_data(
    path: PathBuf,
    file_type: FileType,
    content: Vec<EditorLine>,
    cursor: Option<Cursor>,
    cfg: &EditorConfigs,
    gs: &mut GlobalState,
) -> Editor {
    let (renderer, lexer) = match file_type.family() {
        FileFamily::Text => (Renderer::text(), Lexer::text_lexer(&path)),
        FileFamily::MarkDown => (Renderer::markdown(), Lexer::md_lexer(&path)),
        FileFamily::Code(file_type) => (Renderer::code(), Lexer::with_context(file_type, &path)),
    };

    let display = build_display(&path);
    let line_number_offset = calc_line_number_offset(content.len());

    let cursor = match cursor {
        Some(cursor) if cursor.matches_content(&content) => cursor,
        Some(..) | None => Cursor::default(),
    };

    let mut editor = Editor {
        actions: Actions::new(cfg.get_indent_cfg(file_type)),
        controls: ControlMap::default(),
        update_status: FileUpdate::None,
        renderer,
        last_render_at_line: None,
        cursor,
        line_number_padding: line_number_offset,
        lexer,
        content,
        file_type,
        display,
        modal: EditorModal::default(),
        path,
    };
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    // precal wraps for redndering of non code
    if !editor.file_type.is_code() {
        calc_wraps(&mut editor.content, editor.cursor.text_width);
    }

    editor
}

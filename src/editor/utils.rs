use super::{
    controls::ControlMap, Actions, Cursor, CursorPosition, Editor, EditorConfigs, EditorLine, EditorModal, FileFamily,
    FileType, GlobalState, Lexer, TuiCodec,
};
use crate::error::{IdiomError, IdiomResult};
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

#[derive(Debug, PartialEq)]
pub struct EditorStats {
    pub len: usize,
    pub select_len: usize,
    pub position: CursorPosition,
}

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

pub fn select_indent(editor: &Editor) -> Option<(CursorPosition, CursorPosition)> {
    let start_line = &editor.content[editor.cursor.line];
    let expect_indent = start_line.as_str().chars().take_while(|c| c.is_whitespace()).collect::<String>();
    if expect_indent.is_empty() {
        return None;
    }
    let mut from = CursorPosition { line: editor.cursor.line, char: 0 };
    let mut to = CursorPosition { line: editor.cursor.line, char: start_line.char_len() };
    for (idx, line) in editor.content.iter().enumerate().take(editor.cursor.line).rev() {
        if line.as_str().chars().all(|c| c.is_whitespace()) {
            continue;
        }
        // bigger indents are also included
        if !line.starts_with(&expect_indent) {
            break;
        }
        from.line = idx;
    }
    for (idx, line) in editor.content.iter().enumerate().skip(editor.cursor.line + 1) {
        if line.as_str().chars().all(|c| c.is_whitespace()) {
            continue;
        }
        // bigger indents are also included
        if !line.starts_with(&expect_indent) {
            break;
        }
        to.line = idx;
        to.char = line.char_len();
    }
    Some((from, to))
}

pub fn select_between_chars(editor: &Editor, open: char, close: char) -> Option<(CursorPosition, CursorPosition)> {
    let start_line = &editor.content[editor.cursor.line];
    let (start, end) = start_line.split_at(editor.cursor.char);
    let mut maybe_from = None;
    let mut maybe_to = None;
    let mut idx = editor.cursor.char;
    let mut counter_from = 0;
    let mut counter_to = 0;
    for ch in start.chars().rev() {
        if ch == open {
            if counter_from > 0 {
                counter_from -= 1;
            } else {
                maybe_from = Some(CursorPosition { line: editor.cursor.line, char: idx });
                break;
            }
        } else if ch == close {
            counter_from += 1;
        }
        idx -= 1;
    }
    idx = editor.cursor.char;
    for ch in end.chars() {
        // do stuff
        if ch == close {
            if counter_to > 0 {
                counter_to -= 1;
            } else {
                maybe_to = Some(CursorPosition { line: editor.cursor.line, char: idx });
                break;
            }
        } else if ch == open {
            counter_to += 1;
        }
        idx += 1;
    }
    if maybe_from.is_none() {
        for (line_idx, line) in editor.content.iter().enumerate().take(editor.cursor.line).rev() {
            idx = line.char_len();
            for ch in line.chars().rev() {
                if ch == open {
                    if counter_from > 0 {
                        counter_from -= 1;
                    } else {
                        maybe_from = Some(CursorPosition { line: line_idx, char: idx });
                        break;
                    }
                } else if ch == close {
                    counter_from += 1;
                }
                idx -= 1;
            }
            if maybe_from.is_some() {
                break;
            }
        }
    }
    let from = maybe_from?;
    if maybe_to.is_none() {
        for (line_idx, line) in editor.content.iter().enumerate().skip(editor.cursor.line + 1) {
            idx = 0;
            for ch in line.chars() {
                if ch == close {
                    if counter_to > 0 {
                        counter_to -= 1;
                    } else {
                        maybe_to = Some(CursorPosition { line: line_idx, char: idx });
                        break;
                    }
                } else if ch == open {
                    counter_to += 1;
                }
                idx += 1;
            }
            if maybe_to.is_some() {
                break;
            }
        }
    }
    Some((from, maybe_to?))
}

pub fn select_between_chars_inc(editor: &Editor, open: char, close: char) -> Option<(CursorPosition, CursorPosition)> {
    let (from, to) = select_between_chars(editor, open, close)?;
    let inc_from = from.prev(&editor.content)?;
    let inc_to = to.next(&editor.content)?;
    Some((inc_from, inc_to))
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
        FileFamily::Text => (TuiCodec::text(), Lexer::text_lexer(&path)),
        FileFamily::MarkDown => (TuiCodec::markdown(), Lexer::md_lexer(&path)),
        FileFamily::Code(file_type) => (TuiCodec::code(), Lexer::with_context(file_type, &path)),
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
    editor
}

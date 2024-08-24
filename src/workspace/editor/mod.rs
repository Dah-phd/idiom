mod code;
mod plain;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::{
    configs::{EditorAction, EditorConfigs, FileType},
    global_state::GlobalState,
    lsp::LSPResult,
    workspace::CursorPosition,
};
pub use code::CodeEditor;
pub use plain::TextEditor;

type DocLen = usize;
type SelectLen = usize;
pub type DocStats<'a> = (DocLen, SelectLen, CursorPosition);

pub enum Editor {
    Code(CodeEditor),
    Text(TextEditor),
}

impl Editor {
    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> bool {
        match self {
            Self::Code(editor) => editor.map(action, gs),
            Self::Text(editor) => editor.map(action, gs),
        }
    }

    pub fn go_to(&mut self, line: usize) {
        match self {
            Self::Code(editor) => editor.go_to(line),
            Self::Text(editor) => editor.go_to(line),
        }
    }

    pub fn find(&self, pat: &str, buffer: &mut Vec<(CursorPosition, CursorPosition)>) {
        match self {
            Self::Code(editor) => editor.find(pat, buffer),
            Self::Text(editor) => editor.find(pat, buffer),
        }
    }

    pub fn file_type(&self) -> FileType {
        match self {
            Self::Code(editor) => editor.file_type,
            _ => FileType::Unknown,
        }
    }

    pub fn update_path(&mut self, new_path: PathBuf) -> LSPResult<()> {
        match self {
            Self::Code(editor) => editor.update_path(new_path),
            _ => Ok(()),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        match self {
            Self::Code(editor) => editor.resize(width, height),
            Self::Text(editor) => todo!(),
        }
    }

    pub fn get_stats(&self) -> DocStats {
        match self {
            Self::Code(editor) => editor.get_stats(),
            Self::Text(editor) => editor.get_stats(),
        }
    }

    pub fn clear_screen_cache(&mut self) {
        match self {
            Self::Code(editor) => editor.clear_screen_cache(),
            Self::Text(editor) => editor.clear_screen_cache(),
        }
    }

    pub fn display(&self) -> &str {
        match self {
            Self::Code(editor) => &editor.display,
            Self::Text(editor) => &editor.display,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Code(editor) => &editor.path,
            Self::Text(editor) => &editor.path,
        }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        match self {
            Self::Code(editor) => editor.render(gs),
            Self::Text(editor) => editor.render(gs),
        }
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        match self {
            Self::Code(editor) => editor.fast_render(gs),
            Self::Text(editor) => editor.fast_render(gs),
        }
    }

    pub fn refresh_cfg(&mut self, new_cfg: &EditorConfigs) {
        match self {
            Self::Code(editor) => editor.refresh_cfg(new_cfg),
            Self::Text(editor) => editor.refresh_cfg(new_cfg),
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

#[cfg(test)]
pub mod code_tests;

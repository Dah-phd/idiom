mod code;
mod plain;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::{
    configs::{EditorAction, EditorConfigs},
    global_state::GlobalState,
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

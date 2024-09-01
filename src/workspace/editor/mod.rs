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

#[allow(dead_code)]
pub enum Editor {
    Code(CodeEditor),
    Text(TextEditor),
}

fn build_display(path: &Path) -> String {
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

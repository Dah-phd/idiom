mod code;
// mod plain;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::{configs::EditorAction, global_state::GlobalState, workspace::CursorPosition};
pub use code::CodeEditor;
// pub use plain::TextEditor;

type DocLen = usize;
type SelectLen = usize;
pub type DocStats<'a> = (DocLen, SelectLen, CursorPosition);

#[allow(dead_code)]
pub trait Editor {
    fn render(&mut self, gs: &mut GlobalState);
    fn fast_render(&mut self, gs: &mut GlobalState);
    fn map(&mut self, action: EditorAction, gs: &mut GlobalState);
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

use std::path::PathBuf;
use crate::utils::{trim_start_inplace, Offset};

use super::types::FileType;
use super::{load_or_create_config, EDITOR_CFG_FILE};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigs {
    pub indent: String,
    #[serde(skip, default = "get_indent_after")]
    pub indent_after: String,
    #[serde(skip, default = "get_unident_before")]
    pub unindent_before: String,
    pub format_on_save: bool,
    pub rust_lsp: String,
    #[serde(skip, default = "get_python_lsp")]
    pub python_lsp: String,
}

impl Default for EditorConfigs {
    fn default() -> Self {
        Self {
            indent: "    ".to_owned(),
            indent_after: get_indent_after(),
            unindent_before: get_unident_before(),
            rust_lsp: get_rust_lsp(),
            python_lsp: get_python_lsp(),
            format_on_save: true,
        }
    }
}

impl EditorConfigs {
    pub fn new() -> Self {
        load_or_create_config(EDITOR_CFG_FILE)
    }

    pub fn derive_file_type(&mut self, path: &PathBuf) -> FileType {
        let ft = FileType::derive_type(path);
        self.update_by_file_type(&ft);
        ft
    }

    pub fn update_by_file_type(&mut self, file_type: &FileType) {
        #[allow(clippy::single_match)]
        match file_type {
            FileType::Python => self.indent_after.push(':'),
            _ => (),
        }
    }

    pub fn derive_lsp(&self, file_type: &FileType) -> Option<String> {
        match file_type {
            FileType::Rust => Some(self.rust_lsp.to_owned()),
            FileType::Python => Some(self.python_lsp.to_owned()),
            _ => None
        }
    }

    pub fn unindent_if_before_base_pattern(&self, line: &mut String) -> usize {
        if line.starts_with(&self.indent) {
            if let Some(first) = line.trim_start().chars().next() {
                if self.unindent_before.contains(first) {
                    line.replace_range(..self.indent.len(), "");
                    return self.indent.len();
                }
            }
        }
        0
    }

    pub fn derive_indent_from(&self, prev_line: &str) -> String {
        let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace() || c == '\t').collect::<String>();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.indent_after.contains(last) {
                indent.insert_str(0, &self.indent);
            }
        };
        indent
    }

    pub fn indent_line(&self, line_idx: usize, content: &mut [String]) -> Offset {
        if line_idx > 0 {
            let (prev_split, current_split) = content.split_at_mut(line_idx);
            let prev_line = &prev_split[line_idx - 1];
            if prev_line.chars().all(|c| c.is_whitespace()) {
                return Offset::Pos(0);
            }
            let line = &mut current_split[0];
            let indent = self.derive_indent_from(prev_line);
            let offset = Offset::Pos(indent.len()) - trim_start_inplace(line);
            line.insert_str(0, &indent);
            offset - self.unindent_if_before_base_pattern(line)
        } else {
            let line = &mut content[line_idx];
            Offset::Neg(trim_start_inplace(line))
        }
    }

    pub fn refresh(&mut self) {
        (*self) = Self::new()
    }
}

fn get_indent_after() -> String {
    String::from("({[")
}

fn get_unident_before() -> String {
    String::from("]})")
}

fn get_rust_lsp() -> String {
    String::from("${cfg_dir}/rust-analyzer")
}

fn get_python_lsp() -> String {
    String::from("python3 -m pylsp")
}

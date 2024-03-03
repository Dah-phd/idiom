use super::types::FileType;
use super::{load_or_create_config, EDITOR_CFG_FILE};
use crate::utils::{trim_start_inplace, Offset};

use serde::{Deserialize, Serialize};

pub struct IndentConfigs {
    pub indent: String,
    pub indent_after: String,
    pub unindent_before: String,
}

impl Default for IndentConfigs {
    fn default() -> Self {
        Self { indent: "    ".to_owned(), unindent_before: get_unident_before(), indent_after: get_indent_after() }
    }
}

impl IndentConfigs {
    pub fn update_by_file_type(mut self, file_type: &FileType) -> Self {
        #[allow(clippy::single_match)]
        match file_type {
            FileType::Python => self.indent_after.push(':'),
            _ => (),
        }
        self
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigs {
    pub format_on_save: bool,
    pub indent: String,
    #[serde(skip, default = "get_indent_after")]
    pub indent_after: String,
    #[serde(skip, default = "get_unident_before")]
    pub unindent_before: String,
    rust_lsp: Option<String>,
    rust_lsp_preload_if_present: Option<Vec<String>>,
    python_lsp: Option<String>,
    python_lsp_preload_if_present: Option<Vec<String>>,
    c_lsp: Option<String>,
    c_lsp_preload_if_present: Option<Vec<String>>,
    cpp_lsp: Option<String>,
    cpp_preload_if_present: Option<Vec<String>>,
    type_script_lsp: Option<String>,
    type_script_preload_if_present: Option<Vec<String>>,
    java_script_lsp: Option<String>,
    java_script_preload_if_present: Option<Vec<String>>,
    html_lsp: Option<String>,
    html_preload_if_present: Option<Vec<String>>,
    toml_lsp: Option<String>,
    toml_preload_if_present: Option<Vec<String>>,
    yaml_lsp: Option<String>,
    yaml_preload_if_present: Option<Vec<String>>,
}

impl Default for EditorConfigs {
    fn default() -> Self {
        Self {
            format_on_save: true,
            indent: "    ".to_owned(),
            indent_after: get_indent_after(),
            unindent_before: get_unident_before(),
            rust_lsp: Some(String::from("${cfg_dir}/rust-analyzer")),
            rust_lsp_preload_if_present: Some(vec!["Cargo.toml".to_owned(), "Cargo.lock".to_owned()]),
            python_lsp: Some(String::from("jedi-language-server")),
            python_lsp_preload_if_present: Some(vec!["pyproject.toml".to_owned(), "pytest.init".to_owned()]),
            c_lsp: None,
            c_lsp_preload_if_present: None,
            cpp_lsp: None,
            cpp_preload_if_present: None,
            type_script_lsp: None,
            type_script_preload_if_present: None,
            java_script_lsp: None,
            java_script_preload_if_present: None,
            html_lsp: None,
            html_preload_if_present: None,
            toml_lsp: None,
            toml_preload_if_present: None,
            yaml_lsp: None,
            yaml_preload_if_present: None,
        }
    }
}

impl EditorConfigs {
    pub fn new() -> Self {
        load_or_create_config(EDITOR_CFG_FILE)
    }

    pub fn get_indent_cfg(&self, file_type: &FileType) -> IndentConfigs {
        let indent_cfg = IndentConfigs {
            indent: self.indent.to_owned(),
            indent_after: self.indent_after.to_owned(),
            unindent_before: self.unindent_before.to_owned(),
        };
        indent_cfg.update_by_file_type(file_type)
    }

    pub fn derive_lsp(&self, file_type: &FileType) -> Option<String> {
        match file_type {
            FileType::Rust => self.rust_lsp.to_owned(),
            FileType::Python => self.python_lsp.to_owned(),
            FileType::C => self.c_lsp.to_owned(),
            FileType::Cpp => self.cpp_lsp.to_owned(),
            FileType::JavaScript => self.java_script_lsp.to_owned(),
            FileType::TypeScript => self.type_script_lsp.to_owned(),
            FileType::Html => self.html_lsp.to_owned(),
            FileType::Yml => self.yaml_lsp.to_owned(),
            FileType::Toml => self.toml_lsp.to_owned(),
            FileType::MarkDown => None,
            FileType::Unknown => None,
        }
    }

    pub fn derive_lsp_preloads(&mut self, base_tree_paths: Vec<String>) -> Vec<(FileType, String)> {
        [
            (FileType::Rust, self.rust_lsp_preload_if_present.take(), self.rust_lsp.as_ref()),
            (FileType::Python, self.python_lsp_preload_if_present.take(), self.python_lsp.as_ref()),
            (FileType::C, self.c_lsp_preload_if_present.take(), self.c_lsp.as_ref()),
            (FileType::Cpp, self.cpp_preload_if_present.take(), self.cpp_lsp.as_ref()),
            (FileType::JavaScript, self.java_script_preload_if_present.take(), self.java_script_lsp.as_ref()),
            (FileType::TypeScript, self.type_script_preload_if_present.take(), self.type_script_lsp.as_ref()),
            (FileType::Html, self.html_preload_if_present.take(), self.html_lsp.as_ref()),
            (FileType::Yml, self.yaml_preload_if_present.take(), self.yaml_lsp.as_ref()),
            (FileType::Toml, self.toml_preload_if_present.take(), self.toml_lsp.as_ref()),
        ]
        .into_iter()
        .flat_map(|(ft, expected, cmd)| Some((ft, map_preload(&base_tree_paths, expected, cmd)?)))
        .collect()
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

fn map_preload(base_tree_paths: &Vec<String>, expected: Option<Vec<String>>, cmd: Option<&String>) -> Option<String> {
    if let Some(cmd) = cmd {
        for expected_file in expected? {
            if base_tree_paths.contains(&expected_file) {
                return Some(cmd.to_owned());
            }
        }
    }
    None
}

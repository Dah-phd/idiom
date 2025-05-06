use std::collections::HashMap;

use super::{
    defaults::{get_indent_after, get_indent_spaces, get_unident_before},
    load_or_create_config,
    types::FileType,
    EDITOR_CFG_FILE,
};
use crate::utils::{trim_start_inplace, Offset};
use crate::workspace::line::EditorLine;
use crate::{global_state::GlobalState, lsp::LSP};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditorConfigs {
    #[serde(default, skip)]
    pub format_on_save: bool,
    #[serde(default = "get_indent_spaces")]
    pub indent_spaces: usize,
    #[serde(default = "get_indent_after")]
    pub indent_after: String,
    #[serde(default = "get_unident_before")]
    pub unindent_before: String,
    /// SHELL
    pub shell: Option<String>,
    pub git_tui: Option<String>,
    /// LSP
    rust_lsp: Option<String>,
    rust_lsp_preload_if_present: Option<Vec<String>>,
    zig_lsp: Option<String>,
    zig_lsp_preload_if_present: Option<Vec<String>>,
    python_lsp: Option<String>,
    python_lsp_preload_if_present: Option<Vec<String>>,
    nim_lsp: Option<String>,
    nim_lsp_preload_if_present: Option<Vec<String>>,
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
            indent_spaces: get_indent_spaces(),
            indent_after: String::from("({["),
            unindent_before: String::from("]})"),
            // shell
            shell: None,
            git_tui: None,
            // lsp
            rust_lsp: Some(String::from("rust-analyzer")),
            rust_lsp_preload_if_present: Some(vec!["Cargo.toml".to_owned(), "Cargo.lock".to_owned()]),
            zig_lsp: None,
            zig_lsp_preload_if_present: Some(vec!["build.zig".to_owned()]),
            python_lsp: Some(String::from("jedi-language-server")),
            python_lsp_preload_if_present: Some(vec!["pyproject.toml".to_owned(), "pytest.init".to_owned()]),
            nim_lsp: Some(String::from("nimlsp")),
            nim_lsp_preload_if_present: Some(vec![r".*\.nimble".to_owned()]),
            c_lsp: None,
            c_lsp_preload_if_present: None,
            cpp_lsp: None,
            cpp_preload_if_present: None,
            type_script_lsp: Some(String::from("vtsls --stdio")),
            type_script_preload_if_present: None,
            java_script_lsp: Some(String::from("vtsls --stdio")),
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
    pub fn new() -> Result<Self, toml::de::Error> {
        load_or_create_config(EDITOR_CFG_FILE)
    }

    pub fn get_indent_cfg(&self, file_type: &FileType) -> IndentConfigs {
        let indent_cfg = self.default_indent_cfg();
        indent_cfg.update_by_file_type(file_type)
    }

    pub fn default_indent_cfg(&self) -> IndentConfigs {
        IndentConfigs {
            indent: (0..self.indent_spaces).map(|_| ' ').collect(),
            indent_after: self.indent_after.to_owned(),
            unindent_before: self.unindent_before.to_owned(),
        }
    }

    pub fn derive_lsp(&self, file_type: &FileType) -> Option<String> {
        match file_type {
            FileType::Ignored | FileType::Lobster | FileType::Json | FileType::Shell => None,
            FileType::Rust => self.rust_lsp.to_owned(),
            FileType::Zig => self.zig_lsp.to_owned(),
            FileType::Python => self.python_lsp.to_owned(),
            FileType::Nim => self.nim_lsp.to_owned(),
            FileType::C => self.c_lsp.to_owned(),
            FileType::Cpp => self.cpp_lsp.to_owned(),
            FileType::JavaScript => self.java_script_lsp.to_owned(),
            FileType::TypeScript => self.type_script_lsp.to_owned(),
            FileType::Html => self.html_lsp.to_owned(),
            FileType::Yml => self.yaml_lsp.to_owned(),
            FileType::Toml => self.toml_lsp.to_owned(),
        }
    }

    pub async fn init_preloaded_lsp_servers(
        &mut self,
        base_tree_paths: Vec<String>,
        gs: &mut GlobalState,
    ) -> HashMap<FileType, LSP> {
        let mut lsp_servers = HashMap::new();
        for (ft, lsp_cmd) in self.derive_lsp_preloads(base_tree_paths, gs) {
            gs.success(format!("Preloading {lsp_cmd}"));
            match LSP::new(lsp_cmd, ft).await {
                Ok(lsp) => {
                    lsp_servers.insert(ft, lsp);
                }
                Err(err) => gs.error(format!("Preload filed: {err}")),
            }
        }
        lsp_servers
    }

    fn derive_lsp_preloads(&mut self, base_tree: Vec<String>, gs: &mut GlobalState) -> Vec<(FileType, String)> {
        [
            (FileType::Rust, self.rust_lsp_preload_if_present.take(), self.rust_lsp.as_ref()),
            (FileType::Zig, self.zig_lsp_preload_if_present.take(), self.zig_lsp.as_ref()),
            (FileType::Python, self.python_lsp_preload_if_present.take(), self.python_lsp.as_ref()),
            (FileType::C, self.c_lsp_preload_if_present.take(), self.c_lsp.as_ref()),
            (FileType::Cpp, self.cpp_preload_if_present.take(), self.cpp_lsp.as_ref()),
            (FileType::JavaScript, self.java_script_preload_if_present.take(), self.java_script_lsp.as_ref()),
            (FileType::TypeScript, self.type_script_preload_if_present.take(), self.type_script_lsp.as_ref()),
            (FileType::Html, self.html_preload_if_present.take(), self.html_lsp.as_ref()),
            (FileType::Yml, self.yaml_preload_if_present.take(), self.yaml_lsp.as_ref()),
            (FileType::Toml, self.toml_preload_if_present.take(), self.toml_lsp.as_ref()),
            (FileType::Nim, self.nim_lsp_preload_if_present.take(), self.nim_lsp.as_ref()),
        ]
        .into_iter()
        .flat_map(|(ft, expected, cmd)| Some((ft, map_preload(&base_tree, expected, cmd, gs)?)))
        .collect()
    }

    pub fn refresh(&mut self) -> Result<(), toml::de::Error> {
        (*self) = Self::new()?;
        Ok(())
    }
}

fn map_preload(
    base_tree: &[String],
    expected: Option<Vec<String>>,
    cmd: Option<&String>,
    gs: &mut GlobalState,
) -> Option<String> {
    let cmd = cmd?;
    for try_re in expected?.iter().map(|re| Regex::new(re)) {
        match try_re {
            Ok(file_re) => {
                if base_tree.iter().any(|path| file_re.is_match(path)) {
                    return Some(cmd.to_owned());
                }
            }
            Err(error) => gs.error(error),
        }
    }
    None
}

pub struct IndentConfigs {
    pub indent: String,
    pub indent_after: String,
    pub unindent_before: String,
}

impl Default for IndentConfigs {
    fn default() -> Self {
        Self { indent: "    ".to_owned(), unindent_before: String::from("]})"), indent_after: String::from("({[") }
    }
}

impl IndentConfigs {
    pub fn update_by_file_type(mut self, file_type: &FileType) -> Self {
        #[allow(clippy::single_match)]
        match file_type {
            FileType::Python | FileType::Nim | FileType::Lobster => self.indent_after.push(':'),
            _ => (),
        }
        self
    }

    pub fn unindent_if_before_base_pattern(&self, line: &mut EditorLine) -> usize {
        if line.starts_with(&self.indent)
            && matches!(line.trim_start().chars().next(), Some(first) if self.unindent_before.contains(first))
        {
            line.replace_till(self.indent.len(), "");
            return self.indent.len();
        }
        0
    }

    pub fn derive_indent_from(&self, prev_line: &EditorLine) -> String {
        let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
        if let Some(last) = prev_line.trim_end().chars().last() {
            if self.indent_after.contains(last) {
                indent.insert_str(0, &self.indent);
            }
        };
        indent
    }

    pub fn derive_indent_from_lines(&self, prev_lines: &[EditorLine]) -> String {
        for prev_line in prev_lines.iter().rev() {
            if !prev_line.chars().all(|c| c.is_whitespace()) {
                return self.derive_indent_from(prev_line);
            }
        }
        String::new()
    }

    pub fn indent_line(&self, line_idx: usize, content: &mut [EditorLine]) -> Offset {
        if line_idx > 0 {
            let indent = self.derive_indent_from_lines(&content[..line_idx]);
            if indent.is_empty() {
                return Offset::Pos(0);
            }
            let line = &mut content[line_idx];
            let offset = Offset::Pos(indent.len()) - trim_start_inplace(line);
            line.insert_str(0, &indent);
            offset - self.unindent_if_before_base_pattern(line)
        } else {
            let line = &mut content[line_idx];
            Offset::Neg(trim_start_inplace(line))
        }
    }
}

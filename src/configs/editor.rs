use super::{
    defaults::{get_indent_after, get_indent_spaces, get_unident_before},
    load_or_create_config,
    types::FileType,
    write_config_file, EDITOR_CFG_FILE,
};
use crate::{
    configs::lsp_cfg::LSPConfig,
    editor_line::EditorLine,
    global_state::GlobalState,
    lsp::servers::InitCfg,
    utils::{trim_start_inplace, Offset},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
pub struct EditorConfigs {
    #[serde(default = "get_indent_spaces")]
    pub indent_spaces: usize,
    #[serde(default = "get_indent_after")]
    pub indent_after: String,
    #[serde(default = "get_unident_before")]
    pub unindent_before: String,
    /// SHELL
    pub shell: Option<String>,
    pub git_tui: Option<String>,
    /// GENERAL
    #[serde(default)]
    pub format_on_save: bool,
    pub max_sessions: Option<usize>,
    /// LSP
    LSP: Option<HashMap<String, LSPConfig>>,

    /// Deprecated
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
        let mut lsp_configs = HashMap::new();
        lsp_configs.insert(
            FileType::Rust.as_str().into(),
            LSPConfig::new("rust-analyzer", Some(vec!["Cargo.toml".to_owned(), "Cargo.lock".to_owned()]), None),
        );
        lsp_configs.insert(
            FileType::Python.as_str().into(),
            LSPConfig::new_no_semantic_tokens(
                "uv tool run ty server",
                Some(vec!["pyproject.toml".to_owned(), "pytest.init".to_owned()]),
            ),
        );
        lsp_configs.insert(
            FileType::Nim.as_str().into(),
            LSPConfig::new("nimlsp", Some(vec![r".*\.nimble".to_owned()]), None),
        );
        lsp_configs
            .insert(FileType::TypeScript.as_str().into(), LSPConfig::new_no_semantic_tokens("vtsls --stdio", None));
        lsp_configs
            .insert(FileType::JavaScript.as_str().into(), LSPConfig::new_no_semantic_tokens("vtsls --stdio", None));
        Self {
            format_on_save: false,
            indent_spaces: get_indent_spaces(),
            indent_after: String::from("({["),
            unindent_before: String::from("]})"),
            // shell
            shell: None,
            git_tui: Some("gitui".to_owned()),
            // general
            max_sessions: Some(10),
            // lsp
            LSP: Some(lsp_configs),
            // deprecated
            rust_lsp: None,
            rust_lsp_preload_if_present: None,
            zig_lsp: None,
            zig_lsp_preload_if_present: None,
            python_lsp: None,
            python_lsp_preload_if_present: None,
            nim_lsp: None,
            nim_lsp_preload_if_present: None,
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
    pub fn new() -> Result<Self, toml::de::Error> {
        let mut configs: Self = load_or_create_config(EDITOR_CFG_FILE)?;
        let lsp_cfgs = configs.LSP.get_or_insert_default();

        let mut backward_comp = false;
        if let Some(rust) = configs.rust_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Rust.as_str().into(),
                LSPConfig::new(rust, configs.rust_lsp_preload_if_present.take(), None),
            );
        }
        if let Some(zig) = configs.zig_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Zig.as_str().into(),
                LSPConfig::new(zig, configs.zig_lsp_preload_if_present.take(), None),
            );
        }
        if let Some(py) = configs.python_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Python.as_str().into(),
                LSPConfig::new(py, configs.python_lsp_preload_if_present.take(), None),
            );
        }
        if let Some(nim) = configs.nim_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Nim.as_str().into(),
                LSPConfig::new(nim, configs.nim_lsp_preload_if_present.take(), None),
            );
        }
        if let Some(c_lsp) = configs.c_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::C.as_str().into(),
                LSPConfig::new(c_lsp, configs.c_lsp_preload_if_present.take(), None),
            );
        }
        if let Some(cpp) = configs.cpp_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Cpp.as_str().into(),
                LSPConfig::new(cpp, configs.cpp_preload_if_present.take(), None),
            );
        }
        if let Some(ts) = configs.type_script_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::TypeScript.as_str().into(),
                LSPConfig::new(ts, configs.type_script_preload_if_present.take(), None),
            );
        }
        if let Some(js) = configs.java_script_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::JavaScript.as_str().into(),
                LSPConfig::new(js, configs.java_script_preload_if_present.take(), None),
            );
        }
        if let Some(html) = configs.html_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Html.as_str().into(),
                LSPConfig::new(html, configs.html_preload_if_present.take(), None),
            );
        }
        if let Some(toml) = configs.toml_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Toml.as_str().into(),
                LSPConfig::new(toml, configs.toml_preload_if_present.take(), None),
            );
        }
        if let Some(yaml) = configs.yaml_lsp.take() {
            backward_comp = true;
            lsp_cfgs.insert(
                FileType::Yml.as_str().into(),
                LSPConfig::new(yaml, configs.yaml_preload_if_present.take(), None),
            );
        }

        if backward_comp {
            _ = write_config_file(EDITOR_CFG_FILE, &configs)
        }
        Ok(configs)
    }

    pub fn get_indent_cfg(&self, file_type: FileType) -> IndentConfigs {
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

    pub fn derive_lsp(&self, file_type: &FileType) -> Option<(String, InitCfg)> {
        let lsp_configs = self.LSP.as_ref()?;
        lsp_configs.get(file_type.as_str()).map(LSPConfig::get_cmd_with_configs)
    }

    /// consumes preload info - should be invoked only once
    pub fn derive_lsp_preloads(
        &mut self,
        base_tree: Vec<String>,
        gs: &mut GlobalState,
    ) -> Vec<(FileType, String, InitCfg)> {
        let Some(lsp_configs) = self.LSP.as_mut() else {
            return Default::default();
        };
        FileType::iter_langs()
            .into_iter()
            .flat_map(|ft| {
                let lsp = lsp_configs.get_mut(ft.as_str())?;
                let (cmd, init_cfg) = flat_map_preload(lsp, &base_tree, gs)?;
                Some((ft, cmd, init_cfg))
            })
            .collect()
    }
}

fn flat_map_preload(lsp: &mut LSPConfig, base_tree: &[String], gs: &mut GlobalState) -> Option<(String, InitCfg)> {
    for try_re in lsp.take_preloads_markers()?.iter().map(|re| Regex::new(re)) {
        match try_re {
            Ok(file_re) => {
                if base_tree.iter().any(|path| file_re.is_match(path)) {
                    return Some(lsp.get_cmd_with_configs());
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
    pub fn update_by_file_type(mut self, file_type: FileType) -> Self {
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

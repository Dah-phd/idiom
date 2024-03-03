use crate::configs::FileType;
use crate::syntax::line_builder::Action;
use crate::syntax::GlobalState;
use crate::syntax::WorkspaceEvent;
use lsp_types::DiagnosticRelatedInformation;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Lang {
    pub file_type: FileType,
    pub comment_start: Vec<&'static str>,
    pub declaration: Vec<&'static str>,
    pub key_words: Vec<&'static str>,
    pub frow_control: Vec<&'static str>,
    pub mod_import: Vec<&'static str>,
    pub completion_data_handler: Option<fn(&Self, Value, gs: &mut GlobalState)>,
    pub diagnostic_handler: Option<fn(&Self, &Vec<DiagnosticRelatedInformation>) -> Option<Vec<Action>>>,
}

impl Lang {
    pub fn is_keyword(&self, token: &str) -> bool {
        self.declaration.contains(&token) || self.key_words.contains(&token)
    }

    pub fn is_import(&self, token: &str) -> bool {
        self.mod_import.contains(&token)
    }

    pub fn is_comment(&self, line: &str) -> bool {
        for start in self.comment_start.iter() {
            if line.trim_start().starts_with(start) {
                return true;
            }
        }
        false
    }

    pub fn handle_completion_data(&self, data: Value, gs: &mut GlobalState) {
        self.completion_data_handler.as_ref().inspect(|cb| (cb)(self, data, gs));
    }

    pub fn derive_diagnostic_actions(&self, info: Option<&Vec<DiagnosticRelatedInformation>>) -> Option<Vec<Action>> {
        self.diagnostic_handler.as_ref().and_then(|cb| (cb)(self, info?))
    }

    pub fn completelable(&self, line: &str, idx: usize) -> bool {
        let mut curr_token = String::new();
        let mut prev_token = String::new();
        let mut trigger = false;
        for (char_idx, ch) in line.char_indices() {
            if ch.is_alphabetic() || ch == '_' {
                if char_idx + 1 == idx {
                    return trigger
                        || prev_token.is_empty()
                            && curr_token.len() < 4
                            && !self.declaration.contains(&prev_token.as_str());
                }
                curr_token.push(ch);
            } else {
                if " (.".contains(ch) {
                    trigger = true;
                }
                prev_token = std::mem::take(&mut curr_token);
            }
        }
        false
    }
}

impl From<FileType> for Lang {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Rust => Self {
                file_type,
                comment_start: vec!["//", "///"],
                declaration: vec!["fn", "struct", "enum", "type", "const"],
                key_words: vec!["pub", "let", "self", "mut", "crate", "async", "super", "impl", "Self"],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use"],
                completion_data_handler: Some(|_lang: &Self, data: Value, gs: &mut GlobalState| {
                    if let Value::Object(map) = data {
                        if let Some(Value::Object(import_map)) = map.get("imports").and_then(|arr| arr.get(0)) {
                            if let Some(Value::String(import)) = import_map.get("full_import_path") {
                                gs.workspace.push(WorkspaceEvent::InsertText(format!("use {import};\n")));
                            }
                        };
                    }
                }),
                diagnostic_handler: Some(rust_process_related_info),
                ..Default::default()
            },
            FileType::Python => Self {
                comment_start: vec!["#"],
                file_type,
                declaration: vec!["def", "class"],
                key_words: vec!["self"],
                frow_control: vec![
                    "if", "else", "elif", "for", "while", "break", "continue", "try", "except", "raise", "with",
                    "match",
                ],
                mod_import: vec!["import", "from", "as"],
                ..Default::default()
            },
            FileType::MarkDown => Self {
                file_type,
                comment_start: vec![],
                declaration: vec![],
                key_words: vec![],
                frow_control: vec![],
                mod_import: vec![],
                ..Default::default()
            },
            _ => Self {
                file_type,
                comment_start: vec!["#", "//"],
                declaration: vec![
                    "fn", "struct", "enum", "type", "const", "def", "class", "var", "function",
                ],
                key_words: vec![
                    "pub", "use", "mod", "let", "self", "mut", "crate", "async", "super", "impl", "Self",
                ],
                frow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use", "from", "import"],
                ..Default::default()
            },
        }
    }
}

fn rust_process_related_info(_lang: &Lang, related_info: &Vec<DiagnosticRelatedInformation>) -> Option<Vec<Action>> {
    let mut buffer = Vec::new();
    for info in related_info {
        if info.message.starts_with("consider importing") {
            if let Some(imports) = rust_derive_import(&info.message) {
                buffer.extend(imports.into_iter().map(Action::Import))
            }
        }
    }
    if !buffer.is_empty() {
        return Some(buffer);
    }
    None
}

fn rust_derive_import(message: &str) -> Option<Vec<String>> {
    let matches: Vec<_> = message.match_indices("\n`").map(|(idx, _)| idx).collect();
    let mut buffer = Vec::new();
    let mut end_idx = 0;
    for match_idx in matches {
        let substr = &message[end_idx..match_idx + 1];
        end_idx = match_idx + 2;
        for (current_idx, c) in substr.char_indices().rev() {
            if c == '`' {
                buffer.push(String::from(&substr[current_idx + 1..]));
                break;
            }
        }
    }
    if !buffer.is_empty() {
        return Some(buffer);
    }
    None
}

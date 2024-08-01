mod rust;
use crate::render::backend::Style;
use crate::render::widgets::{StyledLine, Text};
use crate::syntax::{theme::Theme, Action, GlobalState, WorkspaceEvent};
use crate::workspace::line::EditorLine;
use crate::{configs::FileType, render::backend::Color};
use lsp_types::DiagnosticRelatedInformation;
use rust::{rust_process_related_info, rust_specific_handler};
use serde_json::Value;

type LangSpecificHandler = Option<fn(char_idx: usize, word: &str, full_line: &str, theme: &Theme) -> Option<Color>>;
type DiagnosticHandler = Option<fn(&Lang, &Vec<DiagnosticRelatedInformation>) -> Option<Vec<Action>>>;

#[derive(Debug, Clone, Default)]
pub struct Lang {
    pub file_type: FileType,
    comment_start: Vec<&'static str>,
    declaration: Vec<&'static str>,
    key_words: Vec<&'static str>,
    flow_control: Vec<&'static str>,
    mod_import: Vec<&'static str>,
    string_markers: &'static str,
    escape_chars: &'static str,
    completion_data_handler: Option<fn(&Self, Value, gs: &mut GlobalState)>,
    diagnostic_handler: DiagnosticHandler,
    lang_specific_handler: LangSpecificHandler,
}

impl Lang {
    pub fn is_code(&self) -> bool {
        !matches!(self.file_type, FileType::Unknown)
    }

    pub fn is_keyword(&self, token: &str) -> bool {
        self.declaration.contains(&token) || self.key_words.contains(&token)
    }

    pub fn is_flow(&self, token: &str) -> bool {
        self.flow_control.contains(&token)
    }

    pub fn is_import(&self, token: &str) -> bool {
        self.mod_import.contains(&token)
    }

    pub fn is_string_mark(&self, ch: char, prev_ch: Option<char>) -> bool {
        if self.string_markers.contains(ch) {
            if let Some(prev_ch) = prev_ch {
                return !self.escape_chars.contains(prev_ch);
            };
            return true;
        };
        false
    }

    pub fn is_import_start(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        self.mod_import.iter().any(|pat| trimmed.starts_with(pat))
    }

    pub fn is_comment(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        self.comment_start.iter().any(|pat| trimmed.starts_with(pat))
    }

    pub fn lang_specific_handler(&self, char_idx: usize, word: &str, full_line: &str, theme: &Theme) -> Option<Color> {
        (self.lang_specific_handler?)(char_idx, word, full_line, theme)
    }

    pub fn handle_completion_data(&self, data: Value, gs: &mut GlobalState) {
        self.completion_data_handler.as_ref().inspect(|cb| (cb)(self, data, gs));
    }

    pub fn derive_diagnostic_actions(&self, info: Option<&Vec<DiagnosticRelatedInformation>>) -> Option<Vec<Action>> {
        self.diagnostic_handler.as_ref().and_then(|cb| (cb)(self, info?))
    }

    pub fn completable(&self, line: &impl EditorLine, idx: usize) -> bool {
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

    pub fn stylize(&self, text_line: String, theme: &Theme) -> StyledLine {
        if self.is_comment(&text_line) {
            return vec![Text::new(text_line, Some(Style::fg(theme.comment)))].into();
        }
        if self.is_import_start(&text_line) {
            return vec![Text::new(text_line, Some(Style::fg(theme.imports)))].into();
        }
        let mut buffer = vec![];
        let mut word = String::new();
        let mut iter = text_line.chars();
        for ch in iter.by_ref() {
            match ch {
                ' ' | ':' => {
                    if let Some(text) = self.format(&mut word, theme) {
                        buffer.push(text);
                    }
                    buffer.push(ch.into());
                }
                '(' => {
                    if !word.is_empty() {
                        buffer.push(Text::new(std::mem::take(&mut word), Some(Style::fg(theme.functions))));
                    }
                    buffer.push(Text::new(ch.to_string(), None));
                }
                '.' => {
                    if !word.is_empty() {
                        buffer.push(Text::new(std::mem::take(&mut word), Some(Style::fg(theme.default))));
                    }
                    buffer.push(ch.into());
                }
                ',' => {
                    if let Some(text) = self.format(&mut word, theme) {
                        buffer.push(text);
                    }
                    buffer.push(ch.into());
                }
                _ => {
                    word.push(ch);
                }
            }
        }
        if !word.is_empty() {
            let style = self.map_style(&word, theme);
            buffer.push(Text::new(word, Some(style)));
        }
        buffer.into()
    }

    #[inline(always)]
    fn format(&self, word_buf: &mut String, theme: &Theme) -> Option<Text> {
        if word_buf.is_empty() {
            return None;
        }
        let text = std::mem::take(word_buf);
        let style = Some(self.map_style(&text, theme));
        Some(Text::new(text, style))
    }

    #[inline(always)]
    fn map_style(&self, token: &str, theme: &Theme) -> Style {
        if self.is_flow(token) {
            Style::fg(theme.flow_control)
        } else if self.is_keyword(token) {
            Style::fg(theme.key_words)
        } else if token.chars().next().map(|f| f.is_uppercase()).unwrap_or_default() {
            Style::fg(theme.class_or_struct)
        } else {
            Style::fg(theme.default)
        }
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
                flow_control: vec![
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
                lang_specific_handler: Some(rust_specific_handler),
                string_markers: "\"",
                ..Default::default()
            },
            FileType::Python => Self {
                comment_start: vec!["#"],
                file_type,
                declaration: vec!["def", "class"],
                key_words: vec!["self"],
                flow_control: vec![
                    "if", "else", "elif", "for", "while", "break", "continue", "try", "except", "raise", "with",
                    "match",
                ],
                mod_import: vec!["import", "from", "as"],
                string_markers: "\"'",
                ..Default::default()
            },
            FileType::MarkDown => Self {
                file_type,
                comment_start: vec![],
                declaration: vec![],
                key_words: vec![],
                flow_control: vec![],
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
                flow_control: vec![
                    "if", "loop", "for", "in", "while", "break", "continue", "await", "return", "match", "else",
                ],
                mod_import: vec!["mod", "use", "from", "import"],
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests;

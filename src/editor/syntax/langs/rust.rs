use super::{Fix, Lang};
use crate::configs::Theme;
use crossterm::style::Color;
use lsp_types::DiagnosticRelatedInformation;

pub fn rust_process_related_info(_lang: &Lang, related_info: &Vec<DiagnosticRelatedInformation>) -> Option<Vec<Fix>> {
    let mut buffer = Vec::new();
    for info in related_info {
        if info.message.starts_with("consider importing") {
            if let Some(imports) = rust_derive_import(&info.message) {
                buffer.extend(imports.into_iter().map(Fix::Import))
            }
        }
    }
    if !buffer.is_empty() {
        return Some(buffer);
    }
    None
}

pub fn rust_specific_handler(_char_idx: usize, word: &str, _full_line: &str, theme: &Theme) -> Option<Color> {
    if matches!(word.chars().next(), Some('\'')) {
        return Some(theme.key_words);
    };
    None
}

pub fn rust_derive_import(message: &str) -> Option<Vec<String>> {
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

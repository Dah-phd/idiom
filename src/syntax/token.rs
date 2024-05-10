use std::io::Write;

use crate::{
    lsp::{LSPClient, LSPError, LSPResult},
    render::{
        backend::{Backend, Color, Style},
        layout::RectIter,
    },
    syntax::{theme::Theme, Lang},
    workspace::{actions::EditMetaData, line::EditorLine},
};
use lsp_types::{SemanticToken, TextDocumentContentChangeEvent};

use super::legend::Legend;
use super::modal::LSPResponseType;

pub struct Token {
    pub from: usize,
    pub to: usize,
    pub len: usize,
    pub style: Style,
}

impl Token {
    pub fn new(from: usize, to: usize, len: usize, color: Color) -> Self {
        Self { from, to, len, style: Style::fg(color) }
    }

    pub fn enrich(mut char_idx: usize, lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        let mut last_word = String::new();
        for ch in snippet.chars() {
            if ch.is_alphabetic() || "_\"'\\".contains(ch) {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                char_idx += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, style: Style::fg(theme.key_words) });
            };
            char_idx += len;
        }
    }

    pub fn drop_diagstic(&mut self) {
        self.style.reset_mods();
    }

    pub fn parse(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        if lang.is_comment(snippet) {
            buf.push(Token { to: snippet.len(), from: 0, len: snippet.len(), style: Style::fg(theme.comment) });
            return;
        };
        let mut last_word = String::new();
        let mut from = 0;
        let mut is_import = false;
        for ch in snippet.chars() {
            if ch.is_alphabetic() || ch == '_' {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                from += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if is_import {
                buf.push(Token { to: from + len, from, len, style: Style::fg(theme.class_or_struct) });
            } else if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, style: Style::fg(theme.key_words) });
            } else if lang.is_flow(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, style: Style::fg(theme.flow_control) });
            } else if lang.is_import(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, style: Style::fg(theme.key_words) });
                is_import = true;
            } else if let Some(color) = lang.lang_specific_handler(from, token_base.as_str(), snippet, theme) {
                buf.push(Token { to: from + len, from, len, style: Style::fg(color) })
            } else {
                if ch == '(' {
                    buf.push(Token { to: from + len, from, len, style: Style::fg(theme.functions) });
                } else if matches!(token_base.chars().next(), Some(f) if f.is_uppercase()) {
                    buf.push(Token { to: from + len, from, len, style: Style::fg(theme.class_or_struct) });
                } else {
                    buf.push(Token { to: from + len, from, len, style: Style::fg(theme.default) });
                }
            };
            from += len + 1;
        }
        let len = last_word.len();
        if is_import {
            buf.push(Token { to: from + len, from, len, style: Style::fg(theme.class_or_struct) });
        } else if lang.is_keyword(last_word.as_str()) {
            buf.push(Token { to: from + len, from, len, style: Style::fg(theme.key_words) });
        } else if lang.is_flow(last_word.as_str()) {
            buf.push(Token { to: from + len, from, len, style: Style::fg(theme.flow_control) });
        } else if let Some(color) = lang.lang_specific_handler(from, last_word.as_str(), snippet, theme) {
            buf.push(Token { to: from + len, from, len, style: Style::fg(color) })
        } else {
            buf.push(Token { to: from + len, from, len, style: Style::fg(theme.default) });
        };
    }

    pub fn render_code(
        text: &str,
        lang: &Lang,
        theme: &Theme,
        lines: &mut RectIter,
        backend: &mut Backend,
    ) -> std::io::Result<()> {
        if let Some(line) = lines.next() {
            let mut tokens = Vec::new();
            Token::parse(lang, theme, text, &mut tokens);
            backend.flush()?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub enum TokensType {
    LSP,
    #[default]
    Internal,
}

#[inline]
pub fn set_tokens(
    tokens: Vec<SemanticToken>,
    legend: &Legend,
    lang: &Lang,
    theme: &Theme,
    content: &mut Vec<impl EditorLine>,
) {
    let mut line_idx = 0;
    let mut char_idx = 0;
    let mut len = 0;
    let mut token_line = Vec::new();
    for token in tokens {
        if token.delta_line != 0 {
            len = 0;
            char_idx = 0;
            content[line_idx].replace_tokens(std::mem::take(&mut token_line));
            line_idx += token.delta_line as usize;
        };
        let from = char_idx + token.delta_start as usize;
        let to = from + token.length as usize;
        // enriches the tokens with additinal highlights
        if from.saturating_sub(char_idx + len) > 3 {
            content[line_idx].get(char_idx + len..from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = match content[line_idx].get(from..from + len) {
            Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
            None => theme.default,
        };
        token_line.push(Token::new(from, to, len, token_type));
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}

#[inline]
pub fn set_tokens_partial(
    tokens: Vec<SemanticToken>,
    max_lines: usize,
    legend: &Legend,
    lang: &Lang,
    theme: &Theme,
    content: &mut Vec<impl EditorLine>,
) {
    let mut line_idx = 0;
    let mut char_idx = 0;
    let mut len = 0;
    let mut token_line = Vec::new();
    for token in tokens {
        if token.delta_line != 0 {
            len = 0;
            char_idx = 0;
            content[line_idx].replace_tokens(std::mem::take(&mut token_line));
            line_idx += token.delta_line as usize;
            if line_idx > max_lines {
                return;
            }
        };
        let from = char_idx + token.delta_start as usize;
        let to = from + token.length as usize;
        // enriches the tokens with additinal highlights
        if from.saturating_sub(char_idx + len) > 3 {
            content[line_idx].get(char_idx + len..from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = match content[line_idx].get(from..from + len) {
            Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
            None => theme.default,
        };
        token_line.push(Token::new(from, to, len, token_type));
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}

#[inline]
pub fn collect_changes(
    path: &std::path::Path,
    version: i32,
    events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
    content: &[impl EditorLine],
    client: &mut LSPClient,
) -> LSPResult<LSPResponseType> {
    if events.is_empty() {
        return Err(LSPError::Null);
    }
    let mut change_events = Vec::new();
    let meta = events
        .drain(..)
        .map(|(meta, edit)| {
            change_events.push(edit);
            meta
        })
        .reduce(|em1, em2| em1 + em2)
        .expect("Value is checked");
    client.file_did_change(path, version, change_events)?;
    let max_lines = meta.start_line + meta.to;
    let range = meta.build_range(content);
    client.request_partial_tokens(path, range).map(|id| LSPResponseType::TokensPartial { id, max_lines })
}

use crate::{
    render::backend::{Color, Style},
    syntax::{theme::Theme, Lang, Legend},
    workspace::line::{CodeLine, EditorLine},
};
use lsp_types::SemanticToken;

#[derive(Debug)]
pub struct Token {
    pub from: usize,
    pub to: usize,
    pub len: usize,
    pub delta_start: usize,
    pub style: Style,
}

impl Token {
    pub fn new(from: usize, to: usize, len: usize, color: Color) -> Self {
        Self { from, to, len, style: Style::fg(color), delta_start: 0 }
    }

    pub fn with_delta(from: usize, to: usize, len: usize, delta_start: usize, color: Color) -> Self {
        Self { from, to, len, delta_start, style: Style::fg(color) }
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
                buf.push(Token::new(char_idx, char_idx + len, len, theme.key_words));
            };
            char_idx += len;
        }
    }

    pub fn drop_diagstic(&mut self) {
        self.style.reset_mods();
    }

    pub fn parse_to_buf(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        if !lang.is_code() {
            return;
        };
        if lang.is_comment(snippet) {
            buf.push(Token::new(0, snippet.len(), snippet.len(), theme.comment));
            return;
        };
        let mut last_word = String::new();
        let mut from = 0;
        let mut is_import = false;
        let mut delta_start = 0;
        for ch in snippet.chars() {
            if ch.is_alphabetic() || ch == '_' {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                from += 1;
                delta_start += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if is_import {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.class_or_struct));
                delta_start = len;
            } else if lang.is_keyword(token_base.as_str()) {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.key_words));
                delta_start = len;
            } else if lang.is_flow(token_base.as_str()) {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.flow_control));
                delta_start = len;
            } else if lang.is_import(token_base.as_str()) {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.key_words));
                delta_start = len;
                is_import = true;
            } else if let Some(color) = lang.lang_specific_handler(from, token_base.as_str(), snippet, theme) {
                buf.push(Token::with_delta(from, from + len, len, delta_start, color));
                delta_start = len;
            } else if ch == '(' {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.functions));
                delta_start = len;
            } else if matches!(token_base.chars().next(), Some(f) if f.is_uppercase()) {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.class_or_struct));
                delta_start = len;
            } else {
                buf.push(Token::with_delta(from, from + len, len, delta_start, theme.default));
                delta_start = len;
            };
            from += len + 1;
            delta_start += 1;
        }
        let len = last_word.len();
        if is_import {
            buf.push(Token::with_delta(from, from + len, len, delta_start, theme.class_or_struct));
        } else if lang.is_keyword(last_word.as_str()) {
            buf.push(Token::with_delta(from, from + len, len, delta_start, theme.key_words));
        } else if lang.is_flow(last_word.as_str()) {
            buf.push(Token::with_delta(from, from + len, len, delta_start, theme.flow_control));
        } else if let Some(color) = lang.lang_specific_handler(from, last_word.as_str(), snippet, theme) {
            buf.push(Token::with_delta(from, from + len, len, delta_start, color));
        } else {
            buf.push(Token::with_delta(from, from + len, len, delta_start, theme.default));
        };
    }
}

#[derive(Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum TokensType {
    LSP,
    #[default]
    Internal,
}

#[inline]
pub fn set_tokens(tokens: Vec<SemanticToken>, legend: &Legend, lang: &Lang, theme: &Theme, content: &mut [CodeLine]) {
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
            content[line_idx].get(char_idx + len, from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = legend.parse_to_color(token.token_type as usize, token.token_modifiers_bitset, theme);
        token_line.push(Token { from, to, len, delta_start: token.delta_start as usize, style: Style::fg(token_type) });
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
    content: &mut [CodeLine],
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
            content[line_idx].get(char_idx + len, from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = legend.parse_to_color(token.token_type as usize, token.token_modifiers_bitset, theme);
        token_line.push(Token { from, to, len, delta_start: token.delta_start as usize, style: Style::fg(token_type) });
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}

#[inline]
pub fn set_tokens_partial__(
    tokens: Vec<SemanticToken>,
    max_lines: usize,
    legend: &Legend,
    lang: &Lang,
    theme: &Theme,
    content: &mut [CodeLine],
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
        // enriches the tokens with additinal highlights
        if from.saturating_sub(char_idx + len) > 3 {
            content[line_idx].get(char_idx + len, from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = legend.parse_to_color(token.token_type as usize, token.token_modifiers_bitset, theme);
        token_line.push(Token {
            from: 0,
            to: 0,
            len,
            delta_start: token.delta_start as usize,
            style: Style::fg(token_type),
        });
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}

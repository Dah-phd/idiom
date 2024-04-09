use crate::syntax::line_builder::Lang;
use crate::syntax::theme::Theme;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub struct Token {
    pub from: usize,
    pub to: usize,
    pub len: usize,
    pub color: Color,
}

impl Token {
    pub fn push_span(&self, text: &str, buf: &mut Vec<Span<'static>>) -> usize {
        if let Some(content) = text.get(self.from..self.to) {
            buf.push(Span::styled(content.to_owned(), Style::new().fg(self.color)));
        };
        self.to
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
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.key_words });
            };
            char_idx += len;
        }
    }

    pub fn parse(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        if lang.is_comment(snippet) {
            buf.push(Token { to: snippet.len(), from: 0, len: snippet.len(), color: theme.comment });
            return;
        };
        let mut last_word = String::new();
        let mut char_idx = 0;
        let mut is_import = false;
        for ch in snippet.chars() {
            if ch.is_alphabetic() || ch == '_' {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                char_idx += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if is_import {
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.class_or_struct });
            } else if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.key_words });
            } else if lang.is_flow(token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.flow_control });
            } else if lang.is_import(token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.key_words });
                is_import = true;
            } else if let Some(color) = lang.lang_specific_handler(char_idx, token_base.as_str(), snippet, theme) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, color })
            } else {
                if ch == '(' {
                    buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.functions });
                } else if matches!(token_base.chars().next(), Some(f) if f.is_uppercase()) {
                    buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.class_or_struct });
                } else {
                    buf.push(Token { to: char_idx + len, from: char_idx, len, color: theme.default });
                }
            };
            char_idx += len + 1;
        }
    }
}

use std::ops::Range;

use crate::syntax::line_builder::Lang;
use crate::syntax::theme::Theme;
use crate::workspace::line::Line;
use crossterm::style::ContentStyle;
use ratatui::style::Color;

pub struct Token {
    pub from: usize,
    pub to: usize,
    pub len: usize,
    pub color: ContentStyle,
}

impl Token {
    pub fn new(from: usize, to: usize, len: usize, color: Color) -> Self {
        Self { from, to, len, color: from_color(color) }
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
                buf.push(Token { to: char_idx + len, from: char_idx, len, color: from_color(theme.key_words) });
            };
            char_idx += len;
        }
    }

    pub fn parse(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        if lang.is_comment(snippet) {
            buf.push(Token { to: snippet.len(), from: 0, len: snippet.len(), color: from_color(theme.comment) });
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
                buf.push(Token { to: from + len, from, len, color: from_color(theme.class_or_struct) });
            } else if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, color: from_color(theme.key_words) });
            } else if lang.is_flow(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, color: from_color(theme.flow_control) });
            } else if lang.is_import(token_base.as_str()) {
                buf.push(Token { to: from + len, from, len, color: from_color(theme.key_words) });
                is_import = true;
            } else if let Some(color) = lang.lang_specific_handler(from, token_base.as_str(), snippet, theme) {
                buf.push(Token { to: from + len, from, len, color: from_color(color) })
            } else {
                if ch == '(' {
                    buf.push(Token { to: from + len, from, len, color: from_color(theme.functions) });
                } else if matches!(token_base.chars().next(), Some(f) if f.is_uppercase()) {
                    buf.push(Token { to: from + len, from, len, color: from_color(theme.class_or_struct) });
                } else {
                    buf.push(Token { to: from + len, from, len, color: from_color(theme.default) });
                }
            };
            from += len + 1;
        }
        let len = last_word.len();
        if is_import {
            buf.push(Token { to: from + len, from, len, color: from_color(theme.class_or_struct) });
        } else if lang.is_keyword(last_word.as_str()) {
            buf.push(Token { to: from + len, from, len, color: from_color(theme.key_words) });
        } else if lang.is_flow(last_word.as_str()) {
            buf.push(Token { to: from + len, from, len, color: from_color(theme.flow_control) });
        } else if let Some(color) = lang.lang_specific_handler(from, last_word.as_str(), snippet, theme) {
            buf.push(Token { to: from + len, from, len, color: from_color(color) })
        } else {
            buf.push(Token { to: from + len, from, len, color: from_color(theme.default) });
        };
    }
}

fn from_color(c: Color) -> ContentStyle {
    let mut style = ContentStyle::new();
    style.foreground_color = Some(c.into());
    style
}

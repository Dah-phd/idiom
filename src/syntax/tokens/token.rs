use lsp_types::SemanticToken;
use unicode_width::UnicodeWidthChar;

use crate::{
    render::backend::Style,
    syntax::{diagnostics::DiagnosticData, theme::Theme, Lang, Legend},
    workspace::{cursor::Cursor, line::EditorLine},
};

#[derive(Default)]
pub struct TokenLine {
    inner: Vec<Token>,
    char_len: usize,
}

impl TokenLine {
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
        self.char_len = 0;
    }

    #[inline]
    pub fn internal_rebase(&mut self, code: &str, lang: &Lang, theme: &Theme) {
        self.clear();
        Token::parse_to_buf(lang, theme, code, self);
    }

    #[inline]
    pub fn char_len(&self) -> usize {
        self.char_len
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn increment_end(&mut self) {
        if let Some(last) = self.inner.last_mut() {
            last.len += 1;
            self.char_len += 1;
        }
    }

    pub fn increment_at(&mut self, mut idx: usize) {
        let mut increment = 1;
        for token in self.inner.iter_mut() {
            if idx < token.delta_start {
                token.delta_start += 1;
                self.char_len += increment;
                break;
            } else if idx <= token.delta_start + token.len {
                token.len += 1;
                self.char_len += increment;
                increment = 0;
            }
            idx -= token.delta_start;
        }
    }

    pub fn decrement_at(&mut self, mut idx: usize) {
        let mut decrement = 1;
        for token in self.inner.iter_mut() {
            if idx < token.delta_start {
                token.delta_start += 1;
                self.char_len -= decrement;
                break;
            } else if idx <= token.delta_start + token.len {
                token.len += 1;
                self.char_len += decrement;
                decrement = 0;
            }
            idx -= token.delta_start;
        }
    }

    #[inline]
    pub fn mark_diagnostics(&mut self, diagnostic: &DiagnosticData) {
        let mut cursor = 0;

        for token in self.inner.iter_mut() {
            cursor += token.delta_start;
            match diagnostic.end {
                Some(end) if diagnostic.start <= cursor && token.len + cursor <= end => {
                    token.style.undercurle(Some(diagnostic.color));
                }
                None if diagnostic.start <= cursor => {
                    token.style.undercurle(Some(diagnostic.color));
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn drop_diagnostics(&mut self) {
        for token in self.inner.iter_mut() {
            token.drop_diagstic();
        }
    }

    pub fn push(&mut self, token: Token) {
        if self.char_len == 0 && !self.is_empty() {
            self.calc_char_len();
        }
        self.char_len += token.delta_start + token.len;
        self.inner.push(token);
    }

    pub fn insert(&mut self, index: usize, token: Token) {
        self.inner.insert(index, token);
        self.calc_char_len();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Token> {
        self.inner.iter()
    }

    #[inline]
    fn calc_char_len(&mut self) {
        self.char_len = self.inner.iter().map(|token| token.delta_start).sum();
        if let Some(last_len) = self.inner.last().map(|t| t.len) {
            self.char_len += last_len;
        }
    }
}

pub struct Token {
    pub len: usize,
    pub delta_start: usize,
    pub style: Style,
}

impl Token {
    pub fn parse(token: SemanticToken, legend: &Legend, theme: &Theme) -> Self {
        let SemanticToken { delta_start, length, token_type, token_modifiers_bitset, .. } = token;
        let style = Style::fg(legend.parse_to_color(token_type as usize, token_modifiers_bitset, theme));
        Self { delta_start: delta_start as usize, len: length as usize, style }
    }

    pub fn enrich(mut delta_start: usize, lang: &Lang, theme: &Theme, snippet: &str, buf: &mut TokenLine) {
        let mut last_word = String::new();
        for ch in snippet.chars() {
            if ch.is_alphabetic() || "_\"'\\".contains(ch) {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                delta_start += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.key_words) });
                delta_start = len;
            } else if lang.is_string(token_base.as_str()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.string) });
                delta_start = len;
            } else {
                buf.push(Token { len, delta_start, style: Style::fg(theme.default) });
                delta_start = len;
            };
        }
    }

    pub fn drop_diagstic(&mut self) {
        self.style.reset_mods();
    }

    fn parse_to_buf(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut TokenLine) {
        if lang.is_comment(snippet) {
            buf.push(Token { len: snippet.len(), delta_start: 0, style: Style::fg(theme.comment) });
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
                buf.push(Token { len, delta_start, style: Style::fg(theme.class_or_struct) });
                delta_start = len;
            } else if lang.is_keyword(token_base.as_str()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.key_words) });
                delta_start = len;
            } else if lang.is_flow(token_base.as_str()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.flow_control) });
                delta_start = len;
            } else if lang.is_import(token_base.as_str()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.key_words) });
                delta_start = len;
                is_import = true;
            } else if let Some(color) = lang.lang_specific_handler(from, token_base.as_str(), snippet, theme) {
                buf.push(Token { len, delta_start, style: Style::fg(color) });
                delta_start = len;
            } else if ch == '(' {
                buf.push(Token { len, delta_start, style: Style::fg(theme.functions) });
                delta_start = len;
            } else if matches!(token_base.chars().next(), Some(f) if f.is_uppercase()) {
                buf.push(Token { len, delta_start, style: Style::fg(theme.class_or_struct) });
                delta_start = len;
            } else {
                buf.push(Token { len, delta_start, style: Style::fg(theme.default) });
                delta_start = len;
            };
            from += len + 1;
            delta_start += 1;
        }
        let len = last_word.len();
        if is_import {
            buf.push(Token { len, delta_start, style: Style::fg(theme.class_or_struct) });
        } else if lang.is_keyword(last_word.as_str()) {
            buf.push(Token { len, delta_start, style: Style::fg(theme.key_words) });
        } else if lang.is_flow(last_word.as_str()) {
            buf.push(Token { len, delta_start, style: Style::fg(theme.flow_control) });
        } else if let Some(color) = lang.lang_specific_handler(from, last_word.as_str(), snippet, theme) {
            buf.push(Token { len, delta_start, style: Style::fg(color) });
        } else {
            buf.push(Token { len, delta_start, style: Style::fg(theme.default) });
        };
    }
}

/// In plain text condition TokenLine is used to store wrapped lines, without affecting the code editing
pub fn calc_wraps(content: &mut [EditorLine], text_width: usize) {
    for text in content.iter_mut() {
        calc_wrap_line(text, text_width);
    }
}

pub fn calc_wrap_line(text: &mut EditorLine, text_width: usize) -> usize {
    if text.is_simple() {
        text.tokens.char_len = text.content.len() / text_width;
    } else {
        complex_wrap_calc(text, text_width);
    }
    text.tokens.char_len
}

pub fn complex_wrap_calc(text: &mut EditorLine, text_width: usize) {
    text.tokens.char_len = 0;
    let mut counter = text_width;
    for ch in text.content.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or_default();
        if w > counter {
            counter = text_width;
            text.tokens.char_len += 1;
        }
        counter -= w;
    }
}

pub fn calc_wrap_line_capped(text: &mut EditorLine, cursor: &Cursor) -> Option<usize> {
    let text_width = cursor.text_width;
    let cursor_char = cursor.char;
    let max_rows = cursor.max_rows;
    if text.is_simple() {
        text.tokens.char_len = text.content.len() / text_width;
        let cursor_at_row = 2 + cursor_char / text_width;
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    } else {
        text.tokens.char_len = 0;
        let mut counter = text_width;
        let mut cursor_at_row = 1;
        let mut prev_idx_break = 0;
        for (idx, ch) in text.content.chars().enumerate() {
            let w = UnicodeWidthChar::width(ch).unwrap_or_default();
            if w > counter {
                counter = text_width;
                text.tokens.char_len += 1;
                if prev_idx_break < cursor_char {
                    cursor_at_row += 1;
                }
                prev_idx_break = idx;
            }
            counter -= w;
        }
        if prev_idx_break < cursor_char {
            cursor_at_row += 1;
        }
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    }
    None
}

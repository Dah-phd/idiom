use super::legend::Legend;
use super::Lang;
use crate::syntax::theme::Theme;
use crate::workspace::actions::EditMetaData;
use lsp_types::SemanticToken;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::text::Span;
use std::cmp::Ordering;

pub struct TokenLine {
    pub tokens: Vec<Token>,
    pub cache: Line<'static>,
}

impl TokenLine {
    fn new(line: &str, tokens: Vec<Token>) -> Self {
        Self { tokens, cache: Line::from(line.to_owned()) }
    }

    fn rebuild(&self, line: &str) -> Line<'static> {
        todo!()
    }
}

pub struct Token {
    pub from: usize,
    pub len: usize,
    pub token_type: Color,
}

impl Token {
    fn enrich(mut char_idx: usize, lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        let mut last_word = String::new();
        for ch in snippet.chars() {
            if ch.is_alphabetic() || ch == '_' {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                char_idx += 1;
                continue;
            }
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.declaration.contains(&token_base.as_str()) {
                buf.push(Token { from: char_idx, len, token_type: theme.key_words });
            };
            char_idx += len;
        }
    }
}

#[derive(Default)]
pub struct Tokens {
    inner: Vec<Vec<Token>>,
}

impl Tokens {
    /// set full token request
    pub fn tokens_reset(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        self.inner.clear();
        let mut line_idx = 0;
        let mut char_idx = 0;
        let mut token_line = Vec::new();
        let mut len = 0;
        for token in tokens {
            if token.delta_line != 0 {
                char_idx = 0;
                len = 0;
                self.insert_line(line_idx, std::mem::take(&mut token_line));
                line_idx += token.delta_line as usize;
            };
            let from = char_idx + token.delta_start as usize;
            if from.saturating_sub(char_idx + len) > 3 {
                content.get(line_idx).and_then(|line| line.get(char_idx + len..from)).inspect(|snippet| {
                    Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
                });
            };
            len = token.length as usize;
            let color = match content[line_idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            token_line.push(Token { from, len, token_type: color });
            char_idx = from;
        }
        if !token_line.is_empty() {
            self.insert_line(line_idx, token_line);
        };
    }

    /// process token range
    pub fn tokens_set(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        let mut line_idx = 0;
        let mut char_idx = 0;
        let mut len = 0;
        for token in tokens {
            if token.delta_line != 0 {
                len = 0;
                char_idx = 0;
                line_idx += token.delta_line as usize;
            };
            let mut token_line = self.get_line(line_idx);
            let from = char_idx + token.delta_start as usize;
            // enriches the tokens with additinal highlights
            if from.saturating_sub(char_idx + len) > 3 {
                content.get(line_idx).and_then(|line| line.get(char_idx + len..from)).inspect(|snippet| {
                    Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
                });
            };
            len = token.length as usize;
            let token_type = match content[line_idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            token_line.push(Token { from, len, token_type });
            char_idx = from;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// drop tokens in range
    pub fn clear_lines(&mut self, from: usize, count: usize) {
        for token_line in self.inner.iter_mut().skip(from).take(count) {
            token_line.clear();
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.inner.len() {
            self.inner.remove(index);
        }
    }

    /// handle EditMetaData when using tokens from LSP
    pub fn map_meta_data(&mut self, meta: EditMetaData) {
        match meta.from.cmp(&meta.to) {
            Ordering::Equal => {}
            Ordering::Greater => {
                let mut lines_to_remove = meta.from - meta.to;
                while lines_to_remove != 0 {
                    self.remove(meta.start_line);
                    lines_to_remove -= 1;
                }
            }
            Ordering::Less => {
                let mut lines_to_add = meta.to - meta.from;
                while lines_to_add != 0 {
                    self.insert_empty(meta.start_line);
                    lines_to_add -= 1;
                }
            }
        }
        self.clear_lines(meta.start_line, meta.to);
    }

    /// getter for lsp line (Vec)
    pub fn get(&self, index: usize) -> Option<&Vec<Token>> {
        let tokens = self.inner.get(index)?;
        if tokens.is_empty() {
            return None;
        };
        Some(tokens)
    }

    fn insert_empty(&mut self, idx: usize) {
        while idx > self.inner.len() {
            self.inner.push(Vec::new());
        }
        self.inner.insert(idx, Vec::new());
    }

    fn get_line(&mut self, idx: usize) -> &mut Vec<Token> {
        while idx + 1 > self.inner.len() {
            self.inner.push(Vec::new());
        }
        &mut self.inner[idx]
    }

    fn insert_line(&mut self, idx: usize, tokens: Vec<Token>) {
        while idx > self.inner.len() {
            self.inner.push(Vec::new());
        }
        self.inner.insert(idx, tokens);
    }
}

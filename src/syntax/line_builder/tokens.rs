use super::legend::Legend;
use super::Lang;
use crate::syntax::theme::Theme;
use crate::workspace::actions::EditMetaData;
use lsp_types::SemanticToken;
use ratatui::style::Color;
use std::cmp::Ordering;

pub struct Token {
    pub from: usize,
    pub len: usize,
    pub token_type: Color,
}

impl Token {
    /// create pseudo token from existing code, currently checking only for declarations
    fn try_token(lang: &Lang, theme: &Theme, word: &str) -> Option<Self> {
        for dec in lang.declaration.iter() {
            if let Some(from) = word.find(dec) {
                return Some(Token { from, len: dec.len(), token_type: theme.key_words });
            }
        }
        None
    }

    fn end(&self) -> usize {
        self.from + self.len
    }

    fn enrich(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        let mut token_start = 0;
        let mut last_word = String::new();
        for ch in snippet.chars() {
            if ch.is_alphabetic() || ch == '_' {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                token_start += 1;
                continue;
            }
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.declaration.contains(&token_base.as_str()) {
                buf.push(Token { from: token_start, len, token_type: theme.key_words })
            };
            token_start += len;
        }
    }
}

#[derive(Default)]
pub struct Tokens {
    inner: Vec<Vec<Token>>,
}

impl Tokens {
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

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

    fn insert(&mut self, idx: usize, token: Token) {
        while idx + 1 > self.inner.len() {
            self.inner.push(Vec::new());
        }
        self.inner[idx].push(token);
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

    pub fn tokens_reset(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        self.inner.clear();
        let mut idx = 0;
        let mut token_line = Vec::new();
        let mut start_idx = 0;
        for token in tokens {
            if token.delta_line != 0 {
                start_idx = 0;
                self.insert_line(idx, std::mem::take(&mut token_line));
                idx += token.delta_line as usize;
            };
            let from = start_idx + token.delta_start as usize;
            let len = token.length as usize;
            let token_type = match content[idx].get(start_idx..start_idx + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            if from.saturating_sub(start_idx) > 3 {
                content.get(idx).and_then(|line| line.get(start_idx..from)).inspect(|snippet| {
                    Token::enrich(lang, theme, snippet, &mut token_line);
                });
            };
            token_line.push(Token { from, len, token_type });
            start_idx = from;
        }
        if !token_line.is_empty() {
            self.insert_line(idx, token_line);
        };
    }

    pub fn tokens_set(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        let mut idx = 0;
        let mut start_idx = 0;
        for token in tokens {
            if token.delta_line != 0 {
                start_idx = 0;
                idx += token.delta_line as usize;
            };
            let from = start_idx + token.delta_start as usize;
            let len = token.length as usize;
            let token_type = match content[idx].get(start_idx..start_idx + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            let mut token_line = self.get_line(idx);
            if from.saturating_sub(start_idx) > 3 {
                content.get(idx).and_then(|line| line.get(start_idx..from)).inspect(|snippet| {
                    Token::enrich(lang, theme, snippet, &mut token_line);
                });
            };
            token_line.push(Token { from, len, token_type });
            start_idx = from;
        }
    }
}

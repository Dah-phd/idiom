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
    fn try_token(lang: &Lang, theme: &Theme, word: &str) -> Option<Self> {
        for dec in lang.declaration.iter() {
            if let Some(from) = word.find(dec) {
                return Some(Token { from, len: dec.len(), token_type: theme.key_words });
            }
        }
        None
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

    fn insert_empty(&mut self, index: usize) {
        while index > self.inner.len() {
            self.inner.push(Vec::new());
        }
        self.inner.insert(index, Vec::new());
    }

    fn insert(&mut self, index: usize, token: Token) {
        while index > self.inner.len() {
            self.inner.push(Vec::new());
        }
        match self.inner.get_mut(index) {
            Some(line) => line.push(token),
            None => self.inner.insert(index, vec![token]),
        }
    }

    fn insert_line(&mut self, index: usize, tokens: Vec<Token>) {
        while index > self.inner.len() {
            self.inner.push(Vec::new());
        }
        self.inner.insert(index, tokens);
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
        let mut from = 0;
        for token in tokens {
            if token.delta_line != 0 {
                from = 0;
                self.insert_line(idx, std::mem::take(&mut token_line));
                idx += token.delta_line as usize;
                if token.delta_start > 0 {
                    if let Some(token) = content[idx]
                        .get(..token.delta_start as usize)
                        .and_then(|word| Token::try_token(lang, theme, word))
                    {
                        token_line.push(token);
                    };
                };
            };
            from += token.delta_start as usize;
            let len = token.length as usize;
            let token_type = match content[idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            token_line.push(Token { from, len, token_type });
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
        let mut line_idx = 0;
        let mut from = 0;
        for token in tokens {
            if token.delta_line != 0 {
                from = 0;
                line_idx += token.delta_line as usize;
                if token.delta_start as usize > 0 {
                    if let Some(token) = content[line_idx]
                        .get(..token.delta_start as usize)
                        .and_then(|word| Token::try_token(lang, theme, word))
                    {
                        self.insert(line_idx, token);
                    };
                };
            };
            from += token.delta_start as usize;
            let len = token.length as usize;
            let token_type = match content[line_idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            self.insert(line_idx, Token { from, len, token_type });
        }
    }
}

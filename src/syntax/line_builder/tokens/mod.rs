use super::DiagnosticInfo;
use super::DiagnosticLine;
use crate::syntax::line_builder::{Lang, Legend};
use ratatui::buffer::Buffer;
use ratatui::style::Style;
mod line;
mod token;
use crate::syntax::theme::Theme;
use crate::workspace::actions::EditMetaData;
use line::TokenLine;
use lsp_types::SemanticToken;
use std::cmp::Ordering;
pub use token::Token;

#[derive(Default)]
enum TokensType {
    LSP,
    #[default]
    Internal,
}

#[derive(Default)]
pub struct Tokens {
    inner: Vec<TokenLine>,
    producer: TokensType,
}

impl Tokens {
    pub fn new(content: &[String], lang: &Lang, theme: &Theme) -> Self {
        let mut new = Self::default();
        for snippet in content.iter() {
            let mut token_buf = Vec::new();
            Token::parse(lang, theme, snippet, &mut token_buf);
            new.inner.push(TokenLine::new(token_buf, snippet));
        }
        new
    }

    pub fn rebuild_internals(&mut self, content: &[String], lang: &Lang, theme: &Theme) {
        self.inner.clear();
        for snippet in content.iter() {
            let mut token_buf = Vec::new();
            Token::parse(lang, theme, snippet, &mut token_buf);
            self.inner.push(TokenLine::new(token_buf, snippet));
        }
    }

    pub fn cached_render(
        &mut self,
        line_idx: usize,
        max_digits: usize,
        content: &str,
        buf: &mut Buffer,
        area: ratatui::prelude::Rect,
    ) -> bool {
        self.inner
            .get_mut(line_idx)
            .map(|token_line| token_line.render_ref(content, line_idx, max_digits, area, buf))
            .is_some()
    }

    pub fn tokens_reset_(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        if tokens.is_empty() {
            return;
        };
        let old_diagnostics: Vec<_> = std::mem::take(&mut self.inner)
            .into_iter()
            .enumerate()
            .flat_map(|(idx, token_line)| {
                if token_line.diagnosics.is_empty() {
                    return None;
                };
                Some((idx, token_line.diagnosics))
            })
            .collect();
        self.tokens_reset(tokens, legend, lang, theme, content);
        for (idx, diagnostics) in old_diagnostics.into_iter() {
            self.get_or_create_line(idx).set_diagnostics(diagnostics);
        }
    }

    /// set full token request
    pub fn tokens_reset(
        &mut self,
        tokens: Vec<SemanticToken>,
        legend: &Legend,
        lang: &Lang,
        theme: &Theme,
        content: &[String],
    ) {
        if tokens.is_empty() {
            return;
        }
        self.producer = TokensType::LSP;
        self.inner.clear();
        let mut line_idx = 0;
        let mut char_idx = 0;
        let mut token_line = Vec::new();
        let mut len = 0;
        for token in tokens {
            if token.delta_line != 0 {
                char_idx = 0;
                len = 0;
                self.insert_line(line_idx, std::mem::take(&mut token_line), &content[line_idx]);
                line_idx += token.delta_line as usize;
            };
            let from = char_idx + token.delta_start as usize;
            let to = from + token.length as usize;
            if from - to > 3 {
                content.get(line_idx).and_then(|line| line.get(char_idx + len..from)).inspect(|snippet| {
                    Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
                });
            };
            len = token.length as usize;
            let color = match content[line_idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            token_line.push(Token { from, to, len, color: Style::new().fg(color) });
            char_idx = from;
        }
        if !token_line.is_empty() {
            self.insert_line(line_idx, token_line, &content[line_idx]);
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
            let token_line = self.get_or_create_line(line_idx);
            let from = char_idx + token.delta_start as usize;
            let to = from + token.length as usize;
            // enriches the tokens with additinal highlights
            if from.saturating_sub(char_idx + len) > 3 {
                content[line_idx].get(char_idx + len..from).inspect(|snippet| {
                    Token::enrich(char_idx, lang, theme, snippet, &mut token_line.tokens);
                });
            };
            len = token.length as usize;
            let token_type = match content[line_idx].get(from..from + len) {
                Some(word) => legend.parse_to_color(token.token_type as usize, theme, lang, word),
                None => theme.default,
            };
            token_line.tokens.push(Token { from, to, len, color: Style::new().fg(token_type) });
            token_line.build_cache(&content[line_idx]);
            char_idx = from;
        }
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<(usize, DiagnosticLine)>) {
        for token_line in self.inner.iter_mut() {
            token_line.clear_diagnostic();
        }
        for (line_idx, diagnostics) in diagnostics.into_iter() {
            self.get_or_create_line(line_idx).set_diagnostics(diagnostics.data);
        }
    }

    pub fn set_diagnositc_errors(&mut self, diagnostics: Vec<(usize, DiagnosticLine)>) {
        for token_line in self.inner.iter_mut() {
            token_line.diagnosics.clear();
        }
        for (line_idx, mut diagnostic) in diagnostics.into_iter() {
            diagnostic.drop_non_errs();
            if diagnostic.data.is_empty() {
                continue;
            }
            self.get_or_create_line(line_idx).diagnosics.extend(diagnostic.data.into_iter());
        }
    }

    pub fn diagnostic_info(&self, line_idx: usize, lang: &Lang) -> DiagnosticInfo {
        let mut info = DiagnosticInfo::default();
        if let Some(token_line) = self.get(line_idx) {
            let mut buffer = Vec::new();
            for diagnostic in token_line.diagnosics.iter() {
                info.messages.push(diagnostic.message.clone());
                if let Some(actions) = lang.derive_diagnostic_actions(diagnostic.info.as_ref()) {
                    for action in actions {
                        buffer.push(action.clone());
                    }
                }
            }
            if !buffer.is_empty() {
                info.actions.replace(buffer);
            }
        }
        info
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

    pub fn map_meta_internal(&mut self, meta: EditMetaData, content: &[String], lang: &Lang, theme: &Theme) {
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
        self.rebuild_lines(meta.start_line, meta.to, lang, theme, content);
    }

    pub fn are_from_lsp(&self) -> bool {
        matches!(self.producer, TokensType::LSP)
    }

    pub fn get(&self, index: usize) -> Option<&TokenLine> {
        let tokens = self.inner.get(index)?;
        if tokens.is_empty() {
            return None;
        };
        Some(tokens)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut TokenLine> {
        let tokens = self.inner.get_mut(index)?;
        if tokens.is_empty() {
            return None;
        };
        Some(tokens)
    }

    fn get_or_create_line(&mut self, idx: usize) -> &mut TokenLine {
        while idx + 1 > self.inner.len() {
            self.inner.push(TokenLine::default());
        }
        &mut self.inner[idx]
    }

    fn insert_empty(&mut self, idx: usize) {
        while idx > self.inner.len() {
            self.inner.push(TokenLine::default());
        }
        self.inner.insert(idx, TokenLine::default());
    }

    fn insert_line(&mut self, line_idx: usize, tokens: Vec<Token>, content: &str) {
        while line_idx > self.inner.len() {
            self.inner.push(TokenLine::default());
        }
        self.inner.insert(line_idx, TokenLine::new(tokens, content));
    }

    /// drop tokens in range
    pub fn clear_lines(&mut self, from: usize, count: usize) {
        for token_line in self.inner.iter_mut().skip(from).take(count) {
            token_line.clear();
        }
    }

    pub fn rebuild_lines(&mut self, from: usize, count: usize, lang: &Lang, theme: &Theme, content: &[String]) {
        for (line_idx, token_line) in self.inner.iter_mut().enumerate().skip(from).take(count) {
            token_line.clear();
            let code_line = &content[line_idx];
            Token::parse(lang, theme, code_line, &mut token_line.tokens);
            token_line.build_cache(code_line);
        }
    }

    pub fn remove(&mut self, line_idx: usize) {
        if line_idx < self.inner.len() {
            self.inner.remove(line_idx);
        }
    }
}

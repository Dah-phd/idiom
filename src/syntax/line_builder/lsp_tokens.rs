use lsp_types::SemanticTokensResult;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use std::ops::Range;

use crate::syntax::{langs::Lang, Theme};

#[derive(Debug, Default)]
pub struct LineBuilder {
    tokens: Vec<Vec<Token>>,
    legend: Vec<Color>,
    pub eror: Option<Range<usize>>,
    pub warn: Option<Range<usize>>,
    pub info: Option<Range<usize>>,
    pub select_range: Option<Range<usize>>,
}

impl LineBuilder {
    pub fn new(tokens_res: SemanticTokensResult) -> Self {
        let mut tokens = Vec::new();
        let mut inner_token = Vec::new();
        let mut from = 0;
        if let SemanticTokensResult::Tokens(tkns) = tokens_res {
            for tkn in tkns.data {
                for _ in 0..tkn.delta_line {
                    from = 0;
                    tokens.push(std::mem::take(&mut inner_token));
                }
                from += tkn.delta_start as usize;
                inner_token.push(Token { from, len: tkn.length, token_type: tkn.token_type as usize });
            }
        }
        Self { tokens, legend: Vec::new(), eror: None, warn: None, info: None, select_range: None }
    }

    pub fn build_line<'a>(&self, line_idx: usize, content: &'a str, theme: &Theme) -> Option<Vec<Span<'a>>> {
        if self.tokens.is_empty() || self.legend.is_empty() {
            return None;
        }
        let mut spans = Vec::new();
        let mut style = Style { fg: Some(Color::White), ..Default::default() };
        let mut len = 0;
        let mut token_num = 0;
        let token_line = self.tokens.get(line_idx);
        for (idx, ch) in content.char_indices() {
            if len == 0 {
                if let Some(syntax_line) = token_line {
                    if let Some(t) = syntax_line.get(token_num) {
                        if t.from == idx {
                            style.fg = Some(self.legend.get(t.token_type).copied().unwrap_or(Color::White));
                            len = t.len;
                            token_num += 1;
                        } else {
                            style.fg.replace(Color::White);
                        }
                    } else {
                        style.fg.replace(Color::White);
                    }
                }
            } else {
                len -= 1;
            }
            let mut span = Span::styled(ch.to_string(), style);
            if let Some(range) = &self.eror {
                if range.contains(&idx) {
                    span.style = span.style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Red);
                }
            } else if let Some(range) = &self.warn {
                if range.contains(&idx) {
                    span.style = span.style.add_modifier(Modifier::UNDERLINED).underline_color(Color::LightYellow);
                }
            } else if let Some(range) = &self.info {
                if range.contains(&idx) {
                    span.style = span.style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Gray);
                }
            }
            if let Some(range) = &self.select_range {
                if range.contains(&idx) {
                    span.style.bg.replace(theme.selected);
                }
            }
            spans.push(span);
        }
        Some(spans)
    }

    pub fn get_color(&self, token_type: usize, theme: &Theme) -> Color {
        if let Some(color) = self.legend.get(token_type) {
            return *color;
        }
        theme.default
    }

    pub fn map_styles(&mut self, lang: Lang, theme: &Theme) {}

    pub fn new_line(&mut self, index: usize) {
        self.tokens.insert(index, Vec::new());
    }
}

#[derive(Debug)]
struct Token {
    from: usize,
    len: u32,
    token_type: usize,
}

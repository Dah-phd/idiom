use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use super::get_line_num;

#[derive(Debug, Default)]
pub struct LSPLinter {
    tokens: Vec<Vec<Token>>,
    style_map: Vec<Style>,
    pub is_set: bool,
}

impl LSPLinter {
    pub fn map_styles(&mut self) {}

    pub fn set_tokens(&mut self) {
        self.is_set = true;
    }

    pub fn new_line(&mut self, index: usize) {
        self.tokens.insert(index, Vec::new());
    }

    pub fn highlited_line<'a>(&self, idx: usize, max_digits: usize, content: &'a str) -> Option<ListItem<'a>> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, max_digits),
            Style::default().fg(Color::Gray),
        )];
        let tokens = self.tokens.get(idx)?;
        let mut last_idx = 0;
        for token in tokens {
            if last_idx != token.from {
                spans.push(Span::raw(&content[0..token.from]));
            }
            spans.push(Span::styled(&content[token.from..token.to], self.style_map[token.token_type]));
            last_idx = token.to;
        }
        if last_idx < content.len() {
            spans.push(Span::raw(&content[last_idx..]));
        }
        Some(ListItem::new(Line::from(spans)))
    }
}

#[derive(Debug)]
struct Token {
    from: usize,
    to: usize,
    token_type: usize,
}

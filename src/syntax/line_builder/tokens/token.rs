use crate::syntax::line_builder::Lang;
use crate::syntax::theme::Theme;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

pub struct Token {
    pub from: usize,
    pub to: usize,
    pub len: usize,
    pub token_type: Color,
}

impl Token {
    pub fn push_span(&self, text: &str, buf: &mut Vec<Span<'static>>) -> usize {
        if let Some(content) = text.get(self.from..self.to) {
            buf.push(Span::styled(content.to_owned(), Style::new().fg(self.token_type)));
        };
        self.to
    }

    pub fn enrich(mut char_idx: usize, lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        let mut last_word = String::new();
        for ch in snippet.chars() {
            if ch.is_alphabetic() || "_\"'".contains(ch) {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                char_idx += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.declaration.contains(&token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, token_type: theme.key_words });
            } else if token_base.starts_with('"') && token_base.ends_with('"')
                || token_base.starts_with('\'') && token_base.ends_with('\'')
            {
                buf.push(Token { to: char_idx + len, from: char_idx, len, token_type: theme.string });
            };
            char_idx += len;
        }
    }

    pub fn parse(lang: &Lang, theme: &Theme, snippet: &str, buf: &mut Vec<Token>) {
        let mut last_word = String::new();
        let mut char_idx = 0;
        for ch in snippet.chars() {
            if ch.is_alphabetic() || "_\"'".contains(ch) {
                last_word.push(ch);
                continue;
            };
            if last_word.is_empty() {
                char_idx += 1;
                continue;
            };
            let token_base = std::mem::take(&mut last_word);
            let len = token_base.len();
            if lang.declaration.contains(&token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, token_type: theme.key_words });
            } else if lang.frow_control.contains(&token_base.as_str()) {
                buf.push(Token { to: char_idx + len, from: char_idx, len, token_type: theme.flow_control });
            } else if token_base.starts_with('"') && token_base.ends_with('"')
                || token_base.starts_with('\'') && token_base.ends_with('\'')
            {
                buf.push(Token { to: char_idx + len, from: char_idx, len, token_type: theme.string });
            } else {
                // starts with capital letter -> class
                // ends with bracket -> function
            };
            char_idx += len;
        }
    }
}

use crate::syntax::line_builder::diagnostics::DiagnosticData;
use crate::syntax::line_builder::tokens::Token;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::WidgetRef;

const DIGIT_STYLE: Style = Style::new().fg(Color::DarkGray);

#[derive(Default)]
pub struct TokenLine {
    pub tokens: Vec<Token>,
    pub cache: Vec<Span<'static>>,
    pub diagnosics: Vec<DiagnosticData>,
}

impl TokenLine {
    pub fn new(tokens: Vec<Token>, content: &str) -> Self {
        let mut line = Self { tokens, cache: Vec::new(), diagnosics: Vec::new() };
        line.build_cache(content);
        line
    }

    pub fn render_ref(&mut self, content: &str, line_idx: usize, max_digits: usize, area: Rect, buf: &mut Buffer) {
        if self.cache.is_empty() {
            self.build_cache(content);
        }
        self.cached_render(area, buf, line_idx, max_digits);
    }

    pub fn cached_render(&self, area: Rect, buf: &mut Buffer, line_idx: usize, max_digits: usize) {
        let mut x = area.left();
        let line_num = Span::styled(format!("{: >1$} ", line_idx + 1, max_digits), DIGIT_STYLE);
        let span_width = line_num.width() as u16;
        let span_area = Rect { x, width: span_width.min(area.right() - x), ..area };
        line_num.render_ref(span_area, buf);
        x = x.saturating_add(span_width);
        for span in self.cache.iter().chain(self.diagnosics.iter().map(|data| &data.inline_span)) {
            let span_width = span.width() as u16;
            let span_area = Rect { x, width: span_width.min(area.right() - x), ..area };
            span.render_ref(span_area, buf);
            x = x.saturating_add(span_width);
            if x >= area.right() {
                break;
            };
        }
    }

    pub fn build_cache(&mut self, content: &str) {
        self.cache.clear();
        let mut end = 0;
        for token in self.tokens.iter() {
            if token.from > end {
                if let Some(text) = content.get(end..token.from) {
                    self.cache.push(Span::raw(text.to_owned()));
                }
            };
            end = token.push_span(content, &mut self.cache);
        }
        if content.len() > end {
            if let Some(text) = content.get(end..) {
                self.cache.push(Span::raw(text.to_owned()));
            }
        };
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.cache.clear();
    }
}

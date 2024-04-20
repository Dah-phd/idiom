use crate::syntax::line_builder::{diagnostics::DiagnosticData, tokens::Token};
use crate::syntax::{LineBuilderContext, Theme};
use crate::widgests::LINE_CONTINIUES;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::WidgetRef;

const DIGIT_STYLE: Style = Style::new().fg(Color::DarkGray);

#[derive(Default)]
pub struct TokenLine {
    pub tokens: Vec<Token>,
    pub cache: Vec<Span<'static>>,
    pub diagnosics: Vec<DiagnosticData>,
    is_rend: bool,
}

impl TokenLine {
    pub fn new(tokens: Vec<Token>, content: &str) -> Self {
        let mut line = Self { tokens, cache: Vec::new(), diagnosics: Vec::new(), is_rend: false };
        line.build_cache(content);
        line
    }

    pub fn render_ref(&mut self, content: &str, line_idx: usize, max_digits: usize, area: Rect, buf: &mut Buffer) {
        if self.is_rend {
            return;
        }
        if self.cache.is_empty() {
            self.build_cache(content);
        }
        self.cached_render(area, buf, line_idx, max_digits);
        self.is_rend = true;
    }

    pub fn render_shrinked_ref(
        &mut self,
        content: &str,
        line_idx: usize,
        max_digits: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        if self.cache.is_empty() {
            self.build_cache(content);
        };
        let area_right = area.right();
        let mut x = area.left();
        let line_num = Span::styled(format!("{: >1$} ", line_idx + 1, max_digits), DIGIT_STYLE);
        let span_width = line_num.width() as u16;
        let span_area = Rect { x, width: span_width.min(area_right - x), ..area };
        line_num.render_ref(span_area, buf);
        x = x.saturating_add(span_width);
        for span in self.cache.iter() {
            let span_width = span.width() as u16;
            let next_x = x.saturating_add(span_width);
            if next_x >= area_right {
                let span_width = LINE_CONTINIUES.width() as u16;
                let span_area = Rect { x, width: span_width.min(area_right - x), ..area };
                LINE_CONTINIUES.render_ref(span_area, buf);
                break;
            };
            let span_area = Rect { x, width: span_width.min(area_right - x), ..area };
            span.render_ref(span_area, buf);
            x = next_x;
        }
    }

    pub fn cached_render(&self, area: Rect, buf: &mut Buffer, line_idx: usize, max_digits: usize) {
        let mut x = area.left();
        let area_right = area.right();
        let line_num = Span::styled(format!("{: >1$} ", line_idx + 1, max_digits), DIGIT_STYLE);
        let span_width = line_num.width() as u16;
        let span_area = Rect { x, width: span_width.min(area_right - x), ..area };
        line_num.render_ref(span_area, buf);
        x = x.saturating_add(span_width);
        for span in self.cache.iter().chain(self.diagnosics.iter().map(|data| &data.inline_span)) {
            let span_width = span.width() as u16;
            let span_area = Rect { x, width: span_width.min(area_right - x), ..area };
            span.render_ref(span_area, buf);
            x = x.saturating_add(span_width);
            if x >= area_right {
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

    pub fn build_spans(
        &self,
        content: &str,
        theme: &Theme,
        mut buf: Vec<Span<'static>>,
        ctx: &mut LineBuilderContext,
    ) -> Vec<Span<'static>> {
        let mut style = Style::new();
        let mut remaining_word_len: usize = 0;
        let mut token_num = 0;
        for (char_idx, ch) in content.char_indices() {
            remaining_word_len = remaining_word_len.saturating_sub(1);
            if remaining_word_len == 0 {
                match self.tokens.get(token_num) {
                    Some(token) if token.from == char_idx => {
                        remaining_word_len = token.len;
                        style.fg = token.color.fg;
                        token_num += 1;
                    }
                    _ => style.fg = None,
                }
            }
            if matches!(&ctx.select_range, Some(range) if range.contains(&char_idx)) {
                style.bg.replace(theme.selected);
            }
            buf.push(Span::styled(ch.to_string(), ctx.brackets.map_style(ch, style)));
            style.add_modifier = Modifier::empty();
            style.bg = None;
        }
        buf
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<DiagnosticData>) {
        self.diagnosics.extend(diagnostics.into_iter());
        for dianostic in self.diagnosics.iter().rev() {
            for token in self.tokens.iter_mut() {
                dianostic.check_token(token);
            }
        }
        self.cache.clear();
    }

    pub fn clear_diagnostic(&mut self) {
        if self.diagnosics.is_empty() {
            return;
        };
        self.diagnosics.clear();
        for token in self.tokens.iter_mut() {
            token.color.add_modifier = Modifier::empty();
        }
        self.cache.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.cache.clear();
    }
}

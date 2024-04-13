use crate::syntax::line_builder::{diagnostics::DiagnosticData, tokens::Token};
use crate::syntax::LineBuilderContext;
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

    pub fn wrap(&mut self, ctx: &mut LineBuilderContext, content: &str) -> WrappedLine<'_> {
        if self.cache.is_empty() {
            self.build_cache(content);
        }
        let mut len = 1;
        let mut current_width = 0;
        let mut relative_cursor = 0;
        let mut cursor_char = ctx.cursor.char;
        for span in self.cache.iter() {
            current_width += span.content.len();
            if current_width > ctx.text_width {
                current_width = 0;
                len += 1;
            };
        }
        WrappedLine { len, relative_cursor, cursor_char, width: ctx.text_width, at_wrap: 0, inner: &self.cache }
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

pub struct WrappedLine<'a> {
    pub len: usize,
    relative_cursor: usize,
    cursor_char: usize,
    width: usize,
    at_wrap: usize,
    inner: &'a [Span<'static>],
}

use crate::syntax::line_builder::{diagnostics::DiagnosticData, tokens::Token};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};
use ratatui::layout::Rect;
use std::io::{Result, Write};

#[derive(Default)]
pub struct TokenLine {
    pub tokens: Vec<Token>,
    pub diagnosics: Vec<DiagnosticData>,
    rendered_at: usize,
}

impl TokenLine {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, diagnosics: Vec::new(), rendered_at: 0 }
    }

    pub fn render(
        &mut self,
        idx: usize,
        max_digits: usize,
        content: &str,
        area: Rect,
        writer: &mut impl Write,
    ) -> Result<()> {
        self.rendered_at = 0;
        let line_number = format!("{: >1$} ", idx + 1, max_digits);
        let mut end = 0;
        queue!(writer, MoveTo(area.x, area.y), PrintStyledContent(line_number.dark_grey()))?;
        queue!(writer, Clear(ClearType::UntilNewLine))?;
        for token in self.tokens.iter() {
            if token.from > end {
                if let Some(text) = content.get(end..token.from) {
                    queue!(writer, Print(text))?;
                }
            };
            if let Some(text) = content.get(token.from..token.to).or(content.get(token.from..)) {
                queue!(writer, PrintStyledContent(token.color.apply(text)))?;
            };
            end = token.to;
        }
        if content.len() > end {
            if let Some(text) = content.get(end..) {
                queue!(writer, Print(text))?;
            }
        };
        writer.flush()
    }

    pub fn render_select(
        &mut self,
        idx: usize,
        max_digits: usize,
        content: &str,
        area: Rect,
        writer: &mut impl Write,
    ) -> Result<()> {
        self.rendered_at = 0;
        let line_number = format!("{: >1$} ", idx + 1, max_digits);
        let mut end = 0;
        queue!(writer, MoveTo(area.x, area.y), PrintStyledContent(line_number.dark_grey()))?;
        queue!(writer, Clear(ClearType::UntilNewLine))?;
        for token in self.tokens.iter() {
            if token.from > end {
                if let Some(text) = content.get(end..token.from) {
                    queue!(writer, Print(text))?;
                }
            };
            if let Some(text) = content.get(token.from..token.to).or(content.get(token.from..)) {
                queue!(writer, PrintStyledContent(token.color.apply(text)))?;
            }
            end = token.to;
        }
        if content.len() > end {
            if let Some(text) = content.get(end..) {
                queue!(writer, Print(text))?;
            }
        };
        writer.flush()
    }

    pub fn fast_render(
        &mut self,
        idx: usize,
        max_digits: usize,
        content: &str,
        area: Rect,
        writer: &mut impl Write,
    ) -> Result<()> {
        let line_idx = idx + 1; // transform to line number
        if self.rendered_at == line_idx {
            return Ok(());
        };
        self.rendered_at = line_idx;
        let line_number = format!("{: >1$} ", line_idx, max_digits);
        let mut end = 0;
        queue!(writer, MoveTo(area.x, area.y), PrintStyledContent(line_number.dark_grey()))?;
        queue!(writer, Clear(ClearType::UntilNewLine))?;
        for token in self.tokens.iter() {
            if token.from > end {
                if let Some(text) = content.get(end..token.from) {
                    queue!(writer, Print(text))?;
                }
            };
            if let Some(text) = content.get(token.from..token.to).or(content.get(token.from..)) {
                queue!(writer, PrintStyledContent(token.color.apply(text)))?;
            }
            end = token.to;
        }
        if content.len() > end {
            if let Some(text) = content.get(end..) {
                queue!(writer, Print(text))?;
            }
        };
        writer.flush()
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<DiagnosticData>) {
        self.rendered_at = 0;
        self.diagnosics.extend(diagnostics.into_iter());
        for dianostic in self.diagnosics.iter().rev() {
            for token in self.tokens.iter_mut() {
                dianostic.check_token(token);
            }
        }
    }

    pub fn clear_diagnostic(&mut self) {
        if self.diagnosics.is_empty() {
            return;
        };
        for token in self.tokens.iter_mut() {
            token.color.attributes.unset(Attribute::Underlined);
        }
        self.diagnosics.clear();
        self.rendered_at = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.rendered_at = 0;
    }
}

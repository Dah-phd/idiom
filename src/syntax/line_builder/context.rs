use std::{cmp::Ordering, ops::Range};

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::workspace::{cursor::Cursor, CursorPosition};

use super::{DiagnosticLine, INIT_BUF_SIZE};

pub const COLORS: [Color; 3] = [Color::LightMagenta, Color::LightYellow, Color::Blue];

#[derive(Default)]
pub struct LineBuilderContext {
    select: Option<(CursorPosition, CursorPosition)>,
    pub select_range: Option<Range<usize>>,
    cursor: CursorPosition,
    pub brackets: BracketColors,
}

impl LineBuilderContext {
    pub fn build_select_buffer(&mut self, at_line: usize, max_len: usize) {
        self.select_range = self.select.and_then(|(from, to)| match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
            (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
            (Ordering::Less, Ordering::Less) => Some(0..max_len),
            (Ordering::Equal, Ordering::Equal) => Some(from.char..to.char),
            (Ordering::Equal, ..) => Some(from.char..max_len),
            (.., Ordering::Equal) => Some(0..to.char),
        });
    }

    pub fn format_with_info(
        &self,
        line_idx: usize,
        diagnostic: Option<&DiagnosticLine>,
        mut buffer: Vec<Span<'static>>,
    ) -> Vec<Span<'static>> {
        // set cursor without the normal API
        if line_idx == self.cursor.line {
            let expected = self.cursor.char + INIT_BUF_SIZE;
            if buffer.len() > expected {
                buffer[self.cursor.char + INIT_BUF_SIZE].style.add_modifier = Modifier::REVERSED;
            } else {
                buffer.push(Span::styled(" ", Style { add_modifier: Modifier::REVERSED, ..Default::default() }))
            }
        };
        if let Some(diagnostic) = diagnostic {
            buffer.extend(diagnostic.data.iter().map(|d| d.inline_span.clone()));
        }
        buffer
    }

    pub fn set_select(&self, style: &mut Style, idx: &usize, color: Color) {
        if matches!(&self.select_range, Some(range) if range.contains(idx)) {
            style.bg.replace(color);
        }
    }
}

#[derive(Default)]
pub struct BracketColors {
    round: Option<usize>,
    curly: Option<usize>,
    square: Option<usize>,
}

impl BracketColors {
    pub fn map_style(&mut self, ch: char, style: &mut Style) {
        match ch {
            '(' => {
                style.fg.replace(self.open_round());
            }
            ')' => {
                style.fg.replace(self.close_round());
            }
            '[' => {
                style.fg.replace(self.open_square());
            }
            ']' => {
                style.fg.replace(self.close_square());
            }
            '{' => {
                style.fg.replace(self.open_curly());
            }
            '}' => {
                style.fg.replace(self.close_curly());
            }
            _ => (),
        };
    }

    pub fn open_round(&mut self) -> Color {
        Self::open(&mut self.round)
    }

    pub fn close_round(&mut self) -> Color {
        Self::close(&mut self.round)
    }

    pub fn open_curly(&mut self) -> Color {
        Self::open(&mut self.curly)
    }

    pub fn close_curly(&mut self) -> Color {
        Self::close(&mut self.curly)
    }

    pub fn open_square(&mut self) -> Color {
        Self::open(&mut self.square)
    }

    pub fn close_square(&mut self) -> Color {
        Self::close(&mut self.square)
    }

    fn close(bracket: &mut Option<usize>) -> Color {
        match bracket.take() {
            Some(idx) => {
                if idx != 0 {
                    bracket.replace(idx - 1);
                }
                COLORS[idx % COLORS.len()]
            }
            None => COLORS[COLORS.len() - 1],
        }
    }

    fn open(bracket: &mut Option<usize>) -> Color {
        let (color, idx) = match bracket.take() {
            Some(idx) => (COLORS[(idx + 1) % COLORS.len()], idx + 1),
            None => (COLORS[0 % COLORS.len()], 0),
        };
        bracket.replace(idx);
        color
    }
}

impl From<&Cursor> for LineBuilderContext {
    fn from(cursor: &Cursor) -> Self {
        Self {
            select: cursor.select_get(),
            select_range: None,
            cursor: cursor.into(),
            brackets: BracketColors::default(),
        }
    }
}

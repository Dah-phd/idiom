use crate::workspace::{cursor::Cursor, CursorPosition};
use ratatui::style::{Color, Style};
use std::{cmp::Ordering, ops::Range};
pub const COLORS: [Color; 3] = [Color::LightMagenta, Color::LightYellow, Color::Blue];

#[derive(Default)]
pub struct LineBuilderContext {
    pub text_width: usize,
    pub select_range: Option<Range<usize>>,
    pub brackets: BracketColors,
    select: Option<(CursorPosition, CursorPosition)>,
    pub cursor: CursorPosition,
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
    pub fn map_style(&mut self, ch: char, style: Style) -> Style {
        match ch {
            '(' => style.fg(self.open_round()),
            ')' => style.fg(self.close_round()),
            '[' => style.fg(self.open_square()),
            ']' => style.fg(self.close_square()),
            '{' => style.fg(self.open_curly()),
            '}' => style.fg(self.close_curly()),
            _ => style,
        }
    }

    pub fn open_round(&mut self) -> Color {
        open_bracket(&mut self.round)
    }

    pub fn close_round(&mut self) -> Color {
        close_bracket(&mut self.round)
    }

    pub fn open_curly(&mut self) -> Color {
        open_bracket(&mut self.curly)
    }

    pub fn close_curly(&mut self) -> Color {
        close_bracket(&mut self.curly)
    }

    pub fn open_square(&mut self) -> Color {
        open_bracket(&mut self.square)
    }

    pub fn close_square(&mut self) -> Color {
        close_bracket(&mut self.square)
    }
}

impl From<&Cursor> for LineBuilderContext {
    fn from(cursor: &Cursor) -> Self {
        Self {
            text_width: cursor.text_width,
            select: cursor.select_get(),
            select_range: None,
            cursor: cursor.into(),
            brackets: BracketColors::default(),
        }
    }
}

#[inline]
fn open_bracket(bracket: &mut Option<usize>) -> Color {
    let (color, idx) = match bracket.take() {
        Some(idx) => (COLORS[(idx + 1) % COLORS.len()], idx + 1),
        None => (COLORS[0 % COLORS.len()], 0),
    };
    bracket.replace(idx);
    color
}

#[inline]
fn close_bracket(bracket: &mut Option<usize>) -> Color {
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

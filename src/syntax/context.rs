use crate::{
    render::backend::{color, Color, Style},
    workspace::{cursor::Cursor, CursorPosition},
};
use std::{cmp::Ordering, ops::Range};
pub const COLORS: [Color; 3] = [color::magenta(), color::yellow(), color::blue()];

#[derive(Default)]
pub struct BracketColors {
    round: Option<usize>,
    curly: Option<usize>,
    square: Option<usize>,
}

impl BracketColors {
    pub fn map_style(&mut self, ch: char, style: &mut Style) {
        match ch {
            '(' => style.set_fg(Some(self.open_round())),
            ')' => style.set_fg(Some(self.close_round())),
            '[' => style.set_fg(Some(self.open_square())),
            ']' => style.set_fg(Some(self.close_square())),
            '{' => style.set_fg(Some(self.open_curly())),
            '}' => style.set_fg(Some(self.close_curly())),
            _ => (),
        };
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

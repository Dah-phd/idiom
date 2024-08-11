use std::{ops::Range, str::Chars};
use unicode_width::UnicodeWidthChar;

#[derive(Default, Debug)]
pub enum RenderStatus {
    Cursor {
        line: u16,
        char: usize,
        at_line: usize,
        select: Option<Range<usize>>,
    },
    Line {
        line: u16,
        at_line: usize,
        select: Option<Range<usize>>,
    },
    #[default]
    None,
}

impl RenderStatus {
    #[inline(always)]
    pub fn update_at_line(&mut self) {}

    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::None;
    }

    #[inline(always)]
    pub fn line(&mut self, line: u16, at_line: usize, select: Option<Range<usize>>) {
        *self = Self::Line { line, select, at_line }
    }

    #[inline(always)]
    pub fn cursor(&mut self, line: u16, char: usize, at_line: usize, select: Option<Range<usize>>) {
        *self = Self::Cursor { line, char, at_line, select };
    }
}

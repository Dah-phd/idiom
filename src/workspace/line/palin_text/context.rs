use std::{cmp::Ordering, ops::Range};

use crate::{
    render::backend::Style,
    syntax::theme::{self, Theme},
    workspace::{cursor::Cursor, CursorPosition},
};

use super::TextLine;

pub struct Context<'a> {
    at_line: usize,
    pub line: usize,
    pub char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
    line_number_offset: usize,
    theme: &'a Theme,
}

impl<'a> Context<'a> {
    fn collect(cursor: &'a mut Cursor, line_number_offset: usize, theme: &'a Theme) -> Self {
        Self {
            at_line: cursor.at_line,
            line: cursor.line,
            char: cursor.char,
            select: cursor.select_get(),
            line_number_offset,
            theme,
        }
    }

    pub fn get_select(&self, at: usize, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, at, width - (self.line_number_offset + 1))
    }

    pub fn select_style(&self) -> Style {
        Style::bg(self.theme.selected)
    }

    pub fn get_char(&self) -> usize {
        self.char
    }
}

pub fn build_select_buffer(
    select: Option<(CursorPosition, CursorPosition)>,
    at_line: usize,
    max_len: usize,
) -> Option<Range<usize>> {
    select.and_then(|(from, to)| match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
        (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
        (Ordering::Less, Ordering::Less) => Some(0..max_len),
        (Ordering::Equal, Ordering::Equal) => Some(from.char..to.char),
        (Ordering::Equal, ..) => Some(from.char..max_len),
        (.., Ordering::Equal) => Some(0..to.char),
    })
}
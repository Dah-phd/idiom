use std::{cmp::Ordering, fmt::format, ops::Range};

use crate::{
    render::{
        backend::{color, Backend, BackendProtocol, Style},
        layout::{Line, RectIter},
    },
    syntax::theme::{self, Theme},
    workspace::{cursor::Cursor, CursorPosition},
};

use super::TextLine;

pub struct Context<'a> {
    pub line_number: usize,
    pub line: usize,
    pub char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
    line_number_offset: usize,
    theme: &'a Theme,
}

impl<'a> Context<'a> {
    fn collect(cursor: &'a mut Cursor, line_number_offset: usize, theme: &'a Theme) -> Self {
        Self {
            line_number: cursor.at_line,
            line: cursor.line,
            char: cursor.char,
            select: cursor.select_get(),
            line_number_offset,
            theme,
        }
    }

    pub fn get_select(&self, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_offset + 1))
    }

    #[inline]
    pub fn setup_line(&mut self, line: Line, backend: &mut Backend) -> usize {
        self.line_number += 1;
        let text = format!("{: >1$} ", self.line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        backend.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()));
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    pub fn skip_line(&self, lines: &mut RectIter, backend: &mut Backend) -> Option<usize> {
        lines.move_cursor(backend).map(|mut width| {
            let txt = (0..self.line_number_offset + 1).map(|_| ".").collect::<String>();
            width -= txt.len();
            backend.print(txt);
            width
        })
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

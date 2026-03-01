use crate::{
    cursor::{CharRange, CharRangeUnbound, Cursor, CursorPosition},
    ext_tui::CrossTerm,
};
use crossterm::style::ContentStyle;
use idiom_tui::Position;
use idiom_tui::{layout::Line, Backend};
use std::cmp::Ordering;

pub struct CodecContext {
    pub accent_style: ContentStyle,
    line_number: usize,
    line_number_padding: usize,
    line: usize,
    cursor_line: usize,
    cursor_char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl CodecContext {
    pub fn collect_context(cursor: &Cursor, line_number_padding: usize, accent_style: ContentStyle) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self {
            line: cursor.line - line_number,
            cursor_line: cursor.line,
            cursor_char: cursor.char,
            select,
            line_number,
            line_number_padding,
            accent_style,
        }
    }

    #[inline(always)]
    pub fn has_cursor(&self, line_idx: usize) -> bool {
        self.cursor_line == line_idx
    }

    #[inline(always)]
    pub fn cursor_char(&self) -> usize {
        self.cursor_char
    }

    #[inline(always)]
    pub fn line_prefix_len(&self) -> usize {
        self.line_number_padding + 1
    }

    #[inline]
    pub fn setup_cursor(&mut self, line: Line, backend: &mut CrossTerm) -> usize {
        self.line_number += 1;
        let text = format!("{: >1$} ", self.line_number, self.line_number_padding);
        let remaining_width = line.width - text.len();
        backend.print_at(line.row, line.col, text);
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    pub fn setup_line(&mut self, line: Line, backend: &mut CrossTerm) -> usize {
        self.line_number += 1;
        let text = format!("{: >1$} ", self.line_number, self.line_number_padding);
        let remaining_width = line.width - text.len();
        backend.print_styled_at(line.row, line.col, text, self.accent_style);
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    pub fn wrap_line(&mut self, line: Line, backend: &mut CrossTerm) {
        let text = format!("{: >1$} ", "", self.line_number_padding);
        backend.print_styled_at(line.row, line.col, text, self.accent_style);
        backend.clear_to_eol();
    }

    #[inline(always)]
    pub fn select_get(&self) -> Option<CharRangeUnbound> {
        build_select_buffer(self.select, self.line_number)
    }

    #[inline(always)]
    pub fn skip_line(&mut self) {
        self.line_number += 1;
    }

    pub fn get_modal_relative_position(&self) -> Position {
        let row = self.line as u16;
        let col = (self.cursor_char + self.line_number_padding + 1) as u16;
        Position { row, col }
    }

    pub fn init_multic_mod(&mut self, cursors: &[Cursor]) {
        let Some(cursor) = cursors.last() else { return };
        self.cursor_line = cursor.line;
        self.cursor_char = cursor.char;
        self.select = cursor.select_get();
    }

    pub fn multic_line_setup(
        &mut self,
        cursors: &[Cursor],
        width: usize,
    ) -> Option<(Vec<CursorPosition>, Vec<CharRange>)> {
        let mut positions = vec![];
        let mut selects = vec![];
        for cursor in cursors.iter().rev() {
            if self.line_number == cursor.line {
                positions.push(cursor.get_position());
                if let Some(pos) = cursor.select_get() {
                    selects.push(pos);
                }
            } else if let Some((from, to)) = cursor.select_get() {
                if from.line <= self.line_number && self.line_number <= to.line {
                    selects.push((from, to));
                }
            }
            if cursor.line > self.line_number {
                break;
            }
        }
        if positions.len() > 1 || selects.len() > 1 {
            let max_len = width - (self.line_number_padding + 1);
            let select_ranges = selects
                .into_iter()
                .flat_map(|select| {
                    build_select_buffer(Some(select), self.line_number).map(|range| range.bound(max_len))
                })
                .collect();
            if let Some(last) = positions.last() {
                self.cursor_line = last.line;
                self.cursor_char = last.char;
            }
            return Some((positions, select_ranges));
        }
        if !positions.is_empty() {
            self.cursor_line = positions[0].line;
            self.cursor_char = positions[0].char;
        }
        self.select = selects.pop();
        None
    }
}

pub fn build_select_buffer(
    select: Option<(CursorPosition, CursorPosition)>,
    at_line: usize,
) -> Option<CharRangeUnbound> {
    select.and_then(|(from, to)| match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
        (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
        (Ordering::Less, Ordering::Less) => Some(CharRangeUnbound { from: None, to: None }),
        (Ordering::Equal, Ordering::Equal) => Some(CharRangeUnbound { from: Some(from.char), to: Some(to.char) }),
        (Ordering::Equal, ..) => Some(CharRangeUnbound { from: Some(from.char), to: None }),
        (.., Ordering::Equal) => Some(CharRangeUnbound { from: None, to: Some(to.char) }),
    })
}

use crate::{
    global_state::GlobalState,
    render::{
        backend::{color, BackendProtocol, Style},
        layout::Line,
    },
    syntax::Lexer,
    workspace::{cursor::Cursor, CursorPosition},
};
use std::{cmp::Ordering, ops::Range};

pub struct CodeLineContext<'a> {
    pub lexer: &'a mut Lexer,
    line_number: usize,
    line_number_offset: usize,
    line: usize,
    char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl<'a> CodeLineContext<'a> {
    pub fn collect_context(lexer: &'a mut Lexer, cursor: &Cursor, line_number_offset: usize) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self { line: cursor.line - line_number, char: cursor.char, select, lexer, line_number, line_number_offset }
    }

    #[inline(always)]
    pub fn cursor_char(&self) -> usize {
        self.char
    }

    #[inline]
    pub fn setup_line(&mut self, line: Line, backend: &mut impl BackendProtocol) -> (usize, Option<Range<usize>>) {
        let line_number = self.line_number + 1;
        let text = format!("{: >1$} ", line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        let select_buffer = build_select_buffer(self.select, self.line_number, remaining_width);
        self.line_number = line_number;
        backend.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()));
        backend.clear_to_eol();
        (remaining_width, select_buffer)
    }

    #[inline]
    pub fn get_select(&self, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_offset + 1))
    }

    pub fn skip_line(&mut self) {
        self.line_number += 1;
    }

    #[inline]
    pub fn render_cursor(self, gs: &mut GlobalState) {
        let row = gs.editor_area.row + self.line as u16;
        let col = gs.editor_area.col + (self.char + self.line_number_offset + 1) as u16;
        self.lexer.render_modal_if_exist(row, col, gs);
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

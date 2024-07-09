use unicode_width::UnicodeWidthChar;

use crate::{
    global_state::GlobalState,
    render::{
        backend::{color, Backend, BackendProtocol, Style},
        layout::Line,
    },
    syntax::Lexer,
    workspace::{
        cursor::Cursor,
        line::{Context, EditorLine, WrappedCursor},
        CursorPosition,
    },
};
use std::{cmp::Ordering, ops::Range};

pub struct CodeLineContext<'a> {
    lexer: &'a mut Lexer,
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
}

impl<'a> Context for CodeLineContext<'a> {
    #[inline]
    fn lexer(&self) -> &Lexer {
        &*self.lexer
    }

    #[inline]
    fn cursor_char(&self) -> usize {
        self.char
    }

    #[inline]
    fn setup_with_select(&mut self, line: Line, backend: &mut Backend) -> (usize, Option<Range<usize>>) {
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
    fn setup_line(&mut self, line: Line, backend: &mut Backend) -> usize {
        let line_number = self.line_number + 1;
        let text = format!("{: >1$} ", line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        self.line_number = line_number;
        backend.print_styled_at(line.row, line.col, text, Style::fg(color::dark_grey()));
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    fn setup_wrap(&self) -> String {
        format!("{:.<1$} ", "", self.line_number_offset)
    }

    #[inline]
    fn get_select(&self, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_offset + 1))
    }

    #[inline]
    fn skip_line(&mut self) {
        self.line_number += 1;
    }

    #[inline]
    fn count_skipped_to_cursor(&mut self, line_width: usize, remaining_lines: usize) -> WrappedCursor {
        let wraps = self.char / line_width + 1;
        let skip_lines = wraps.saturating_sub(remaining_lines);
        let flat_char_idx = self.char;
        self.char %= line_width;
        self.line += wraps.saturating_sub(skip_lines);
        if skip_lines > 1 {
            let skip_chars = skip_lines * line_width;
            return WrappedCursor { skip_chars, flat_char_idx, skip_lines: skip_lines - 1 };
        }
        WrappedCursor { skip_chars: 0, flat_char_idx, skip_lines: 0 }
    }

    /// operation is complex due to variability in position encoding and variable char width
    #[inline]
    fn count_skipped_to_cursor_complex(
        &mut self,
        content: &impl EditorLine,
        line_width: usize,
        remaining_lines: usize,
    ) -> (WrappedCursor, usize) {
        let mut wraps: usize = 0;
        let mut remaining_width = line_width;
        let mut lsp_enc = 0;
        let mut chars_per_line = Vec::new();
        for ch in content.chars().take(self.char) {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if remaining_width < ch_w {
                wraps += 1;
                chars_per_line.push(line_width - remaining_width);
                remaining_width = line_width;
            }
            lsp_enc += self.lexer.char_lsp_pos(ch);
            remaining_width -= ch_w;
        }
        let mut skip_lines = wraps.saturating_sub(remaining_lines);
        let flat_char_idx = self.char;
        if skip_lines == 0 {
            return (WrappedCursor { skip_chars: 0, skip_lines, flat_char_idx }, 0);
        }
        skip_lines += 1;
        self.line += wraps.saturating_sub(skip_lines);
        self.char = line_width - remaining_width;
        (WrappedCursor { skip_chars: chars_per_line.iter().take(skip_lines).sum(), flat_char_idx, skip_lines }, lsp_enc)
    }

    #[inline]
    fn render_cursor(self, gs: &mut GlobalState) {
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

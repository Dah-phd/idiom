use super::status::RenderStatus;
use super::EditorLine;
use crate::{
    ext_tui::CrossTerm,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{cursor::Cursor, CursorPosition},
};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Line, Backend};
use std::{cmp::Ordering, ops::Range};

pub struct LineContext<'a> {
    pub lexer: &'a mut Lexer,
    pub accent_style: ContentStyle,
    line_number: usize,
    line_number_offset: usize,
    line: usize,
    char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl<'a> LineContext<'a> {
    pub fn collect_context(
        lexer: &'a mut Lexer,
        cursor: &Cursor,
        line_number_offset: usize,
        accent_style: ContentStyle,
    ) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self {
            line: cursor.line - line_number,
            char: cursor.char,
            select,
            lexer,
            line_number,
            line_number_offset,
            accent_style,
        }
    }

    /// Ensures during deletion of lines, if scrolling has happened that last line will be rendered
    /// not the most elegant solution - probably should revisit at some point, but good enough
    /// it does not poison other parts of the logic, except fast render
    pub fn correct_last_line_match(&self, content: &mut [EditorLine], screen_hight: usize) {
        let last_line = self.line_number + screen_hight;
        if last_line < 1 {
            return;
        }
        let dissallowed_rendered_line = match content.get(last_line - 1).map(|el| &el.cached) {
            Some(RenderStatus::Line { line, .. }) => *line,
            _ => return,
        };
        if let Some(last_line) = content.get_mut(last_line) {
            if matches!(last_line.cached, RenderStatus::Line { line, .. } if line == dissallowed_rendered_line) {
                last_line.cached.reset();
            }
        }
    }

    #[inline(always)]
    pub fn cursor_char(&self) -> usize {
        self.char
    }

    #[inline]
    pub fn setup_cursor(&mut self, line: Line, backend: &mut CrossTerm) -> usize {
        self.line_number += 1;
        let text = format!("{: >1$} ", self.line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        backend.print_at(line.row, line.col, text);
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    pub fn setup_line(&mut self, line: Line, backend: &mut CrossTerm) -> usize {
        self.line_number += 1;
        let text = format!("{: >1$} ", self.line_number, self.line_number_offset);
        let remaining_width = line.width - text.len();
        backend.print_styled_at(line.row, line.col, text, self.accent_style);
        backend.clear_to_eol();
        remaining_width
    }

    #[inline]
    pub fn wrap_line(&mut self, line: Line, backend: &mut CrossTerm) {
        let text = format!("{: >1$} ", "", self.line_number_offset);
        backend.print_styled_at(line.row, line.col, text, self.accent_style);
        backend.clear_to_eol();
    }

    #[inline]
    pub fn get_select(&self, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_offset + 1))
    }

    #[inline]
    pub fn get_select_full_line(&self, char_len: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, char_len)
    }

    #[inline]
    pub fn skip_line(&mut self) {
        self.line_number += 1;
    }

    #[inline]
    pub fn forced_modal_render(self, gs: &mut GlobalState) {
        let row = self.line as u16;
        let col = (self.char + self.line_number_offset + 1) as u16;
        self.lexer.forece_modal_render_if_exists(row, col, gs);
    }

    #[inline]
    pub fn render_modal(self, gs: &mut GlobalState) {
        let row = self.line as u16;
        let col = (self.char + self.line_number_offset + 1) as u16;
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

use super::status::RenderStatus;
use super::EditorLine;
use crate::{
    ext_tui::CrossTerm,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{cursor::Cursor, editor::EditorModal, CursorPosition},
};
use crossterm::style::ContentStyle;
use idiom_tui::{layout::Line, Backend};
use std::{cmp::Ordering, ops::Range};

pub struct LineContext<'a> {
    pub lexer: &'a mut Lexer,
    pub accent_style: ContentStyle,
    line_number: usize,
    line_number_padding: usize,
    line: usize,
    cursor_line: usize,
    cursor_char: usize,
    select: Option<(CursorPosition, CursorPosition)>,
}

impl<'a> LineContext<'a> {
    pub fn collect_context(
        lexer: &'a mut Lexer,
        cursor: &Cursor,
        line_number_padding: usize,
        accent_style: ContentStyle,
    ) -> Self {
        let line_number = cursor.at_line;
        let select = cursor.select_get();
        Self {
            line: cursor.line - line_number,
            cursor_line: cursor.line,
            cursor_char: cursor.char,
            select,
            lexer,
            line_number,
            line_number_padding,
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
    pub fn has_cursor(&self, line_idx: usize) -> bool {
        self.cursor_line == line_idx
    }

    #[inline(always)]
    pub fn cursor_char(&self) -> usize {
        self.cursor_char
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

    #[inline]
    pub fn select_get(&self, width: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, width - (self.line_number_padding + 1))
    }

    #[inline(always)]
    pub fn select_get_full_line(&self, char_len: usize) -> Option<Range<usize>> {
        build_select_buffer(self.select, self.line_number, char_len)
    }

    #[inline(always)]
    pub fn skip_line(&mut self) {
        self.line_number += 1;
    }

    #[inline]
    pub fn forced_modal_render(self, modal: &mut EditorModal, gs: &mut GlobalState) {
        let row = self.line as u16;
        let col = (self.cursor_char + self.line_number_padding + 1) as u16;
        modal.forece_modal_render_if_exists(row, col, gs);
    }

    #[inline]
    pub fn render_modal(self, modal: &mut EditorModal, gs: &mut GlobalState) {
        let row = self.line as u16;
        let col = (self.cursor_char + self.line_number_padding + 1) as u16;
        modal.render_modal_if_exist(row, col, gs);
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
    ) -> Option<(Vec<CursorPosition>, Vec<Range<usize>>)> {
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
                .flat_map(|select| build_select_buffer(Some(select), self.line_number, max_len))
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

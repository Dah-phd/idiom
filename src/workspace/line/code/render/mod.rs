pub mod ascii_cursor;
pub mod ascii_line;
pub mod complex_cursor;
pub mod complex_line;

use super::{CodeLine, CodeLineContext, EditorLine};
use crate::render::{
    backend::{Backend, BackendProtocol},
    layout::RectIter,
};
use std::{ops::Range, str::Chars};
use unicode_width::UnicodeWidthChar;

const WRAP_OPEN: &str = "<<";
const WRAP_CLOSE: &str = ">>";

#[derive(Default)]
pub enum RenderStatus {
    Cursor {
        line: u16,
        char: usize,
        skipped_chars: usize,
        select: Option<Range<usize>>,
    },
    Line {
        line: u16,
        select: Option<Range<usize>>,
    },
    #[default]
    None,
}

impl RenderStatus {
    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::None;
    }

    #[inline(always)]
    pub fn line(&mut self, line: u16, select: Option<Range<usize>>) {
        *self = Self::Line { line, select }
    }

    #[inline(always)]
    pub fn cursor(&mut self, line: u16, char: usize, skipped_chars: usize, select: Option<Range<usize>>) {
        *self = Self::Cursor { line, char, skipped_chars, select };
    }

    #[inline(always)]
    pub fn should_render_line(&self, new_line: u16, new_select: &Option<Range<usize>>) -> bool {
        !matches!(self, Self::Line { line, select } if *line == new_line && select == new_select )
    }

    #[inline(always)]
    pub fn should_render_cursor(&self, new_line: u16, new_char: usize, new_select: &Option<Range<usize>>) -> bool {
        !matches!(
            self,
            Self::Cursor { line, char, skipped_chars: _, select }
            if *line == new_line
                && *char == new_char
                && select == new_select
        )
    }

    #[inline(always)]
    pub fn should_render_cursor_or_update(
        &mut self,
        new_line: u16,
        new_char: usize,
        new_select: Option<Range<usize>>,
    ) -> bool {
        if let Self::Cursor { line, char, skipped_chars, select } = self {
            if *char == new_char && *line == new_line && select == &new_select {
                false
            } else {
                if *line != new_line {
                    *skipped_chars = 0;
                }
                *select = new_select;
                *char = new_char;
                *line = new_line;
                true
            }
        } else {
            self.cursor(new_line, new_char, 0, new_select);
            true
        }
    }

    pub fn generate_skipped_chars_simple(&mut self, cursor_idx: usize, line_width: usize) -> (usize, usize) {
        let mut idx = self.skipped_chars();
        let mut reduction = if idx == 0 { 2 } else { 4 };
        if cursor_idx > idx + line_width.saturating_sub(reduction + 1) {
            if idx == 0 {
                reduction += 2;
            }
            idx = cursor_idx - line_width.saturating_sub(reduction + 1);
            self.set_skipped_chars(idx);
        } else if idx > cursor_idx {
            if cursor_idx == 2 {
                idx = 0;
            } else {
                idx = cursor_idx;
            }
            if idx == 0 {
                reduction -= 2;
            }
            self.set_skipped_chars(idx);
        }
        (idx, reduction)
    }

    pub fn generate_skipped_chars_complex(
        &mut self,
        cursor_idx: usize,
        mut line_width: usize,
        content: Chars<'_>,
    ) -> usize {
        line_width -= 3;
        let mut idx = self.skipped_chars();
        if idx == cursor_idx {
            return idx;
        }
        if idx > cursor_idx {
            if cursor_idx < 3 {
                self.set_skipped_chars(0);
                return 0;
            };
            self.set_skipped_chars(cursor_idx);
            return cursor_idx;
        }
        let widths =
            content.take(cursor_idx).skip(idx).map(|ch| UnicodeWidthChar::width(ch).unwrap_or(1)).collect::<Vec<_>>();
        for ch_width in widths.into_iter().rev() {
            if ch_width > line_width {
                idx += 1;
                line_width = 0;
            } else {
                line_width -= ch_width;
            }
        }
        if idx < 3 {
            self.set_skipped_chars(0);
            return 0;
        }
        self.set_skipped_chars(idx);
        idx
    }

    #[inline(always)]
    pub fn set_skipped_chars(&mut self, skipped: usize) {
        if let Self::Cursor { line: _, char: _, skipped_chars, .. } = self {
            *skipped_chars = skipped;
        }
    }

    #[inline(always)]
    pub fn skipped_chars(&self) -> usize {
        if let Self::Cursor { line: _, char: _, skipped_chars, .. } = self {
            *skipped_chars
        } else {
            0
        }
    }
}

#[inline(always)]
pub fn is_wider_complex(line: &impl EditorLine, line_width: usize) -> bool {
    let mut current_with = 0;
    for ch in line.chars() {
        if let Some(char_width) = UnicodeWidthChar::width(ch) {
            current_with += char_width;
            if current_with > line_width {
                return true;
            }
        }
    }
    false
}

#[inline(always)]
pub fn cursor(line: &mut CodeLine, ctx: &mut CodeLineContext, lines: &mut RectIter, backend: &mut Backend) {
    let (line_width, select) = match lines.next() {
        Some(rend_line) => {
            let line_row = rend_line.row;
            let select = ctx.get_select(rend_line.width);
            let line_width = ctx.setup_line(rend_line, backend);
            line.cached.cursor(line_row, ctx.cursor_char(), 0, select.clone());
            (line_width, select)
        }
        None => return,
    };
    if line.is_simple() {
        ascii_cursor::render(line, ctx, line_width, select, backend);
    } else {
        complex_cursor::render(line, ctx, line_width, select, backend);
    }
    backend.reset_style();
}

#[inline(always)]
pub fn cursor_fast(line: &mut CodeLine, ctx: &mut CodeLineContext, lines: &mut RectIter, backend: &mut Backend) {
    let (line_width, select) = match lines.next() {
        Some(rend_line) => {
            let select = ctx.get_select(rend_line.width);
            if !line.cached.should_render_cursor_or_update(rend_line.row, ctx.cursor_char(), select.clone()) {
                ctx.skip_line();
                return;
            }
            let line_width = ctx.setup_line(rend_line, backend);
            (line_width, select)
        }
        None => return,
    };
    if line.is_simple() {
        ascii_cursor::render(line, ctx, line_width, select, backend);
    } else {
        complex_cursor::render(line, ctx, line_width, select, backend);
    }
    backend.reset_style();
}

#[cfg(test)]
mod test {
    use super::RenderStatus;

    #[test]
    fn test_cache() {
        let mut cached = RenderStatus::default();
        cached.cursor(3, 0, 0, None);
        assert!(!cached.should_render_cursor(3, 0, &None));
        assert!(cached.should_render_cursor(3, 1, &None));
        assert_eq!(cached.skipped_chars(), 0);
        cached.set_skipped_chars(7);
        assert_eq!(cached.skipped_chars(), 7);
        assert!(cached.should_render_line(3, &None));
    }
}

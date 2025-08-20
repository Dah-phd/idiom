use crate::workspace::CursorPosition;
use idiom_tui::utils::CharLimitedWidths;
use std::ops::Range;

type Reduction = usize;

#[derive(Default, PartialEq)]
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

    /// handles caching on multicursor
    pub fn should_render_multi_cursor(
        &mut self,
        new_line: u16,
        cursors: &[CursorPosition],
        selects: &[Range<usize>],
    ) -> bool {
        let multi_cursor = Self::Cursor {
            // line stores number of cursors
            line: new_line * cursors.len() as u16,
            // cursor sum
            char: cursors.iter().fold(0, |s, c| s + c.char),
            // last char
            skipped_chars: cursors.last().map(|c| c.char).unwrap_or_default(),
            // select sum
            select: selects.iter().cloned().reduce(|r, s| (r.start + s.start)..(r.end + s.end)),
        };
        if self == &multi_cursor {
            return false;
        };
        *self = multi_cursor;
        true
    }

    pub fn generate_skipped_chars_simple(&mut self, cursor_idx: usize, line_width: usize) -> (usize, Reduction) {
        let mut idx = self.skipped_chars();
        let mut reduction = if idx == 0 { 1 } else { 2 };
        if cursor_idx > idx + line_width.saturating_sub(reduction + 1) {
            if idx == 0 {
                reduction += 1;
            }
            idx = cursor_idx - line_width.saturating_sub(reduction + 1);
        } else if idx > cursor_idx {
            if cursor_idx < 2 {
                idx = 0;
            } else {
                idx = cursor_idx;
            }
            if idx == 0 {
                reduction -= 1;
            }
        }
        self.set_skipped_chars(idx);
        (idx, reduction)
    }

    pub fn generate_skipped_chars_complex(
        &mut self,
        text: &str,
        char_len: usize,
        cursor_idx: usize,
        mut line_width: usize,
    ) -> usize {
        let mut idx = self.skipped_chars();

        // edge case if cursor == skipped just return
        if idx == cursor_idx {
            return idx;
        }

        // cursor is within skipped chars
        if idx > cursor_idx {
            if cursor_idx < 2 {
                self.set_skipped_chars(0);
                return 0;
            };
            self.set_skipped_chars(cursor_idx);
            return cursor_idx;
        }

        // setting up offsets and idx
        let skip = char_len.saturating_sub(cursor_idx + 1);
        let mut new_idx = cursor_idx + 1;
        line_width -= 2;

        let widths = CharLimitedWidths::new(text, 3).rev().map(|(.., w)| w).skip(skip);

        for ch_width in widths {
            if ch_width > line_width {
                break;
            }
            line_width -= ch_width;
            new_idx -= 1;
        }

        idx = std::cmp::max(idx, new_idx);

        if idx < 2 {
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

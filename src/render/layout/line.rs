use crate::render::{
    backend::{Backend, BackendProtocol},
    utils::UTF8Safe,
    widgets::Writable,
};
use crossterm::style::ContentStyle;
use std::ops::{AddAssign, SubAssign};

#[derive(Debug, Default, Clone)]
pub struct Line {
    pub row: u16,
    pub col: u16,
    pub width: usize,
}

impl Line {
    pub const fn empty() -> Self {
        Line { row: 0, col: 0, width: 0 }
    }

    #[inline]
    pub fn fill(self, symbol: char, backend: &mut Backend) {
        let text = (0..self.width).map(|_| symbol).collect::<String>();
        backend.print_at(self.row, self.col, text)
    }

    #[inline]
    pub fn fill_styled(self, symbol: char, style: ContentStyle, backend: &mut Backend) {
        let text = (0..self.width).map(|_| symbol).collect::<String>();
        backend.print_styled_at(self.row, self.col, text, style)
    }

    #[inline]
    pub fn render_centered(self, text: &str, backend: &mut Backend) {
        let (remaining_width, text) = text.truncate_width(self.width);
        backend.go_to(self.row, self.col);
        match remaining_width {
            0 => backend.print(text),
            1 => {
                backend.print(text);
                backend.pad(1);
            }
            pad => {
                let right_pad = pad / 2;
                backend.pad(right_pad + (pad % 2));
                backend.print(text);
                backend.pad(right_pad);
            }
        }
    }

    #[inline]
    pub fn render_centered_styled(self, text: &str, style: ContentStyle, backend: &mut Backend) {
        let (remaining_width, text) = text.truncate_width(self.width);
        let restore_style = backend.get_style();
        backend.set_style(style);
        backend.go_to(self.row, self.col);
        match remaining_width {
            0 => backend.print(text),
            1 => {
                backend.print(text);
                backend.pad(1);
            }
            pad => {
                let right_pad = pad / 2;
                backend.pad(right_pad + (pad % 2));
                backend.print(text);
                backend.pad(right_pad);
            }
        }
        backend.set_style(restore_style);
    }

    #[inline]
    pub fn render_left(self, text: &str, backend: &mut Backend) {
        let (pad_width, text) = text.truncate_width_start(self.width);
        backend.go_to(self.row, self.col);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
        backend.print(text);
    }

    #[inline]
    pub fn render_left_styled(self, text: &str, style: ContentStyle, backend: &mut Backend) {
        let (pad_width, text) = text.truncate_width_start(self.width);
        backend.go_to(self.row, self.col);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
        backend.print_styled(text, style);
    }

    #[inline]
    pub fn render_empty(self, backend: &mut Backend) {
        backend.go_to(self.row, self.col);
        backend.pad(self.width);
    }

    #[inline]
    pub fn render(self, text: &str, backend: &mut Backend) {
        let Line { width, row, col } = self;
        let (pad_width, text) = text.truncate_width(width);
        backend.go_to(row, col);
        backend.print(text);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
    }

    #[inline]
    pub fn render_styled(self, text: &str, style: ContentStyle, backend: &mut Backend) {
        let Line { width, row, col } = self;
        let (pad_width, text) = text.truncate_width(width);
        let reset_style = backend.get_style();
        backend.set_style(style);
        backend.go_to(row, col);
        backend.print(text);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
        backend.set_style(reset_style);
    }

    pub const fn split_at(mut self, idx: usize) -> (Self, Self) {
        let new = match idx < self.width {
            true => {
                let remaining_width = self.width - idx;
                self.width = idx;
                Self { row: self.row, col: self.width as u16 + self.col, width: remaining_width }
            }
            false => Self { row: self.row, col: self.col + self.width as u16, width: 0 },
        };
        (self, new)
    }

    /// creates line builder from Line
    /// push/push_styled can be used to add to line
    /// on drop pads the line to end
    #[inline]
    pub fn unsafe_builder(self, backend: &mut Backend) -> LineBuilder {
        backend.go_to(self.row, self.col);
        LineBuilder { row: self.row, col: self.col, remaining: self.width, backend }
    }

    /// creates reverse builder from Line
    /// push/push_styled can be used to add to line
    /// on drop pads the line to end
    #[inline]
    pub fn unsafe_builder_rev(self, backend: &mut Backend) -> LineBuilderRev {
        let remaining = self.width;
        let col = self.col;
        let row = self.row;
        self.render_empty(backend);
        LineBuilderRev { remaining, backend, row, col }
    }
}

impl AddAssign<usize> for Line {
    fn add_assign(&mut self, rhs: usize) {
        let offset = std::cmp::min(rhs, self.width);
        self.width -= offset;
        self.col += offset as u16;
    }
}

impl AddAssign<u16> for Line {
    fn add_assign(&mut self, rhs: u16) {
        let offset = std::cmp::min(rhs, self.width as u16);
        self.width -= offset as usize;
        self.col += offset;
    }
}

impl SubAssign<usize> for Line {
    fn sub_assign(&mut self, rhs: usize) {
        let offset = std::cmp::min(rhs, self.col as usize);
        self.width += offset;
        self.col -= offset as u16;
    }
}

impl SubAssign<u16> for Line {
    fn sub_assign(&mut self, rhs: u16) {
        let offset = std::cmp::min(rhs, self.col);
        self.width += offset as usize;
        self.col -= offset;
    }
}

pub struct LineBuilder<'a> {
    row: u16,
    col: u16,
    remaining: usize,
    backend: &'a mut Backend,
}

impl<'a> LineBuilder<'a> {
    /// returns Ok(bool) -> if true line is not full, false the line is finished
    pub fn push(&mut self, text: &str) -> bool {
        match text.truncate_if_wider(self.remaining) {
            Ok(truncated_text) => {
                self.backend.print(truncated_text);
                self.remaining = 0;
                false
            }
            Err(width) => {
                self.remaining -= width;
                self.backend.print(text);
                true
            }
        }
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: ContentStyle) -> bool {
        match text.truncate_if_wider(self.remaining) {
            Ok(truncated_text) => {
                self.backend.print_styled(truncated_text, style);
                self.remaining = 0;
                false
            }
            Err(width) => {
                self.remaining -= width;
                self.backend.print_styled(text, style);
                true
            }
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.remaining
    }

    pub fn into_line(self) -> Line {
        Line { row: self.row, col: self.col, width: self.remaining }
    }
}

impl Drop for LineBuilder<'_> {
    /// ensure line is rendered and padded till end;
    fn drop(&mut self) {
        if self.remaining != 0 {
            self.backend.pad(self.remaining);
        }
    }
}

pub struct LineBuilderRev<'a> {
    row: u16,
    col: u16,
    remaining: usize,
    backend: &'a mut Backend,
}

impl<'a> LineBuilderRev<'a> {
    /// returns Ok(bool) -> if true line is not full, false the line is finished
    pub fn push(&mut self, text: &str) -> bool {
        match text.truncate_if_wider_start(self.remaining) {
            Ok(truncated_text) => {
                self.remaining = 0;
                self.backend.print_at(self.row, self.col, truncated_text);
                false
            }
            Err(width) => {
                self.remaining -= width;
                self.backend.print_at(self.row, self.col + self.remaining as u16, text);
                true
            }
        }
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: ContentStyle) -> bool {
        match text.truncate_if_wider_start(self.remaining) {
            Ok(truncated_text) => {
                self.remaining = 0;
                self.backend.print_styled_at(self.row, self.col, truncated_text, style);
                false
            }
            Err(width) => {
                self.remaining -= width;
                self.backend.print_styled_at(self.row, self.col + self.remaining as u16, text, style);
                true
            }
        }
    }

    pub fn push_text(&mut self, text: impl Writable) -> Option<usize> {
        if self.remaining >= text.width() {
            self.remaining -= text.width();
            self.backend.go_to(self.row, self.col + self.remaining as u16);
            text.print(self.backend);
            None
        } else {
            // checked that truncated pring is safe
            self.backend.go_to(self.row, self.col);
            unsafe { text.print_truncated_start(self.remaining, self.backend) }
            let skipped = text.width() - self.remaining;
            self.remaining = 0;
            Some(skipped)
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.remaining
    }

    pub fn into_line(self) -> Line {
        Line { row: self.row, col: self.col, width: self.remaining }
    }
}

impl Drop for LineBuilderRev<'_> {
    /// ensure line is rendered and padded till end;
    fn drop(&mut self) {
        if self.remaining != 0 {
            self.backend.go_to(self.row, self.col);
            self.backend.pad(self.remaining);
        }
    }
}

use unicode_width::UnicodeWidthChar;

use crate::render::{
    backend::{Backend, BackendProtocol, Style},
    utils::UTF8Safe,
};
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
    pub fn fill_styled(self, symbol: char, style: Style, backend: &mut Backend) {
        let text = (0..self.width).map(|_| symbol).collect::<String>();
        backend.print_styled_at(self.row, self.col, text, style)
    }

    #[inline]
    pub fn render_centered(self, text: &str, backend: &mut Backend) {
        let text = text.truncate_width(self.width);
        backend.print_at(self.row, self.col, format!("{text:^width$}", width = self.width))
    }

    #[inline]
    pub fn render_centered_styled(self, text: &str, style: Style, backend: &mut Backend) {
        let text = text.truncate_width(self.width);
        backend.print_styled_at(self.row, self.col, format!("{text:>width$}", width = self.width), style);
    }

    #[inline]
    pub fn render_left(self, text: &str, backend: &mut Backend) {
        let text = text.truncate_width_start(self.width);
        backend.print_at(self.row, self.col, format!("{text:>width$}", width = self.width));
    }

    #[inline]
    pub fn render_left_styled(self, text: &str, style: Style, backend: &mut Backend) {
        let text = text.truncate_width_start(self.width);
        backend.print_styled_at(self.row, self.col, format!("{text:^width$}", width = self.width), style);
    }

    #[inline]
    pub fn render_empty(self, backend: &mut Backend) {
        backend.print_at(self.row, self.col, format!("{:width$}", "", width = self.width));
    }

    #[inline]
    pub fn render(self, text: &str, backend: &mut Backend) {
        let Line { mut width, row, col } = self;
        backend.go_to(row, col);
        for ch in text.chars() {
            match UnicodeWidthChar::width(ch) {
                Some(w) => {
                    if w > width {
                        return;
                    }
                    width -= w;
                }
                None => continue,
            }
            backend.print(ch);
        }
        if width != 0 {
            backend.print(format!("{:width$}", ""))
        }
    }

    #[inline]
    pub fn render_styled(self, text: &str, style: Style, backend: &mut Backend) {
        let Line { mut width, row, col } = self;
        let reset_style = backend.get_style();
        backend.go_to(row, col);
        backend.update_style(style);
        for ch in text.chars() {
            match UnicodeWidthChar::width(ch) {
                Some(w) => {
                    if w > width {
                        backend.set_style(reset_style);
                        return;
                    }
                    width -= w;
                }
                None => continue,
            }
            backend.print(ch);
        }
        if width != 0 {
            backend.print(format!("{:width$}", ""))
        }
        backend.set_style(reset_style);
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
        if text.len() > self.remaining {
            self.backend.print(text.truncate_width(self.remaining));
            self.remaining = 0;
            return false;
        }
        self.remaining -= text.len();
        self.backend.print(text);
        true
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: Style) -> bool {
        if text.len() > self.remaining {
            self.backend.print_styled(text.truncate_width(self.remaining), style);
            self.remaining = 0;
            return false;
        }
        self.remaining -= text.len();
        self.backend.print_styled(text, style);
        true
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
            self.push(format!("{:width$}", "", width = self.remaining).as_str());
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
        if text.len() > self.remaining {
            self.backend.print_at(self.row, self.col, text.truncate_width_start(self.remaining));
            self.remaining = 0;
            return false;
        }
        self.remaining -= text.len();
        self.backend.print_at(self.row, self.col + self.remaining as u16, text);
        true
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: Style) -> bool {
        if text.len() > self.remaining {
            self.backend.print_styled_at(self.row, self.col, text.truncate_width_start(self.remaining), style);
            self.remaining = 0;
            return false;
        }
        self.remaining -= text.len();
        self.backend.print_styled_at(self.row, self.col + self.remaining as u16, text, style);
        true
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
            self.push(format!("{:width$}", "", width = self.remaining).as_str());
        }
    }
}

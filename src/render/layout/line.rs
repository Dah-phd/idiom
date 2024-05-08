use crate::render::{
    backend::{Backend, Style},
    utils::{truncate_str, truncate_str_start},
};
use std::{
    cmp::Ordering,
    io::{Result, Write},
    ops::{AddAssign, SubAssign},
};

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
    pub fn fill(self, symbol: char, backend: &mut Backend) -> std::io::Result<()> {
        let text = (0..self.width).map(|_| symbol).collect::<String>();
        backend.print_at(self.row, self.col, text)
    }

    #[inline]
    pub fn fill_styled(self, symbol: char, style: Style, backend: &mut Backend) -> std::io::Result<()> {
        let text = (0..self.width).map(|_| symbol).collect::<String>();
        backend.print_styled_at(self.row, self.col, text, style)
    }

    #[inline]
    pub fn render_centered(self, text: &str, backend: &mut Backend) -> std::io::Result<()> {
        let text = truncate_str(text, self.width);
        backend.print_at(self.row, self.col, format!("{text:^width$}", width = self.width))
    }

    #[inline]
    pub fn render_centered_styled(self, text: &str, style: Style, backend: &mut Backend) -> std::io::Result<()> {
        let text = truncate_str(text, self.width);
        backend.print_styled_at(self.row, self.col, format!("{text:>width$}", width = self.width), style)
    }

    #[inline]
    pub fn render_left(self, text: &str, backend: &mut Backend) -> std::io::Result<()> {
        let text = truncate_str_start(text, self.width);
        backend.print_at(self.row, self.col, format!("{text:>width$}", width = self.width))
    }

    #[inline]
    pub fn render_left_styled(self, text: &str, style: Style, backend: &mut Backend) -> std::io::Result<()> {
        let text = truncate_str_start(text, self.width);
        backend.print_styled_at(self.row, self.col, format!("{text:^width$}", width = self.width), style)
    }

    #[inline]
    pub fn render_empty(self, backend: &mut Backend) -> std::io::Result<()> {
        backend.print_at(self.row, self.col, format!("{:width$}", "", width = self.width))
    }

    #[inline]
    pub fn render(self, text: &str, backend: &mut Backend) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => backend.print_at(self.row, self.col, truncate_str(text, self.width)),
            Ordering::Equal => backend.print_at(self.row, self.col, text),
            Ordering::Less => backend.print_at(self.row, self.col, format!("{text:width$}", width = self.width)),
        }
    }

    #[inline]
    pub fn render_styled(self, text: &str, style: Style, backend: &mut Backend) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => backend.print_styled_at(self.row, self.col, truncate_str(text, self.width), style),
            Ordering::Equal => backend.print_styled_at(self.row, self.col, text, style),
            Ordering::Less => {
                backend.print_styled_at(self.row, self.col, format!("{text:width$}", width = self.width), style)
            }
        }
    }

    /// creates line builder from Line
    /// push/push_styled can be used to add to line
    /// on drop pads the line to end
    #[inline]
    pub fn unsafe_builder<'a>(self, backend: &'a mut Backend) -> std::io::Result<LineBuilder<'a>> {
        backend.go_to(self.row, self.col).map(|_| LineBuilder {
            row: self.row,
            col: self.col,
            remaining: self.width,
            backend,
        })
    }

    /// creates reverse builder from Line
    /// push/push_styled can be used to add to line
    /// on drop pads the line to end
    #[inline]
    pub fn unsafe_builder_rev<'a>(self, backend: &'a mut Backend) -> std::io::Result<LineBuilderRev<'a>> {
        let remaining = self.width;
        let col = self.col;
        let row = self.row;
        self.render_empty(backend)?;
        Ok(LineBuilderRev { remaining, backend, row, col })
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
    pub fn push(&mut self, text: &str) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            self.backend.print(truncate_str(text, self.remaining))?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print(text)?;
        Ok(true)
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: Style) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            self.backend.print_styled(truncate_str(text, self.remaining), style)?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print_styled(text, style)?;
        Ok(true)
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.remaining
    }

    pub fn to_line(self) -> Line {
        Line { row: self.row, col: self.col, width: self.remaining }
    }
}

impl Drop for LineBuilder<'_> {
    /// ensure line is rendered and padded till end;
    fn drop(&mut self) {
        if self.remaining != 0 {
            self.push(format!("{:width$}", "", width = self.remaining).as_str()).unwrap();
        }
        self.backend.flush().unwrap();
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
    pub fn push(&mut self, text: &str) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            self.backend.print_at(self.row, self.col, truncate_str_start(text, self.remaining))?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print_at(self.row, self.col + self.remaining as u16, text)?;
        Ok(true)
    }

    /// push with style
    pub fn push_styled(&mut self, text: &str, style: Style) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            self.backend.print_styled_at(self.row, self.col, truncate_str_start(text, self.remaining), style)?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print_styled_at(self.row, self.col + self.remaining as u16, text, style)?;
        Ok(true)
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.remaining
    }

    pub fn to_line(self) -> Line {
        Line { row: self.row, col: self.col, width: self.remaining }
    }
}

impl Drop for LineBuilderRev<'_> {
    /// ensure line is rendered and padded till end;
    fn drop(&mut self) {
        if self.remaining != 0 {
            self.push(format!("{:width$}", "", width = self.remaining).as_str()).unwrap();
        }
        self.backend.flush().unwrap();
    }
}

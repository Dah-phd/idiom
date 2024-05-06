use crate::render::backend::{Backend, Color, Style};
use bitflags::bitflags;
use std::{
    cmp::Ordering,
    io::{Result, Write},
    ops::{AddAssign, Range, SubAssign},
};

pub const BORDERS: BorderSet = BorderSet {
    top_left_qorner: "┌",
    top_right_qorner: "┐",
    bot_left_qorner: "└",
    bot_right_qorner: "┘",
    vertical: "│",
    horizontal: "─",
};

pub const DOUBLE_BORDERS: BorderSet = BorderSet {
    top_left_qorner: "╔",
    top_right_qorner: "╗",
    bot_left_qorner: "╚",
    bot_right_qorner: "╝",
    vertical: "║",
    horizontal: "═",
};

bitflags! {
    /// Bitflags that can be composed to set the visible borders essentially on the block widget.
    #[derive(Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
    pub struct Borders: u8 {
        /// Show no border (default)
        const NONE   = 0b0000;
        /// Show the top border
        const TOP    = 0b0001;
        /// Show the right border
        const RIGHT  = 0b0010;
        /// Show the bottom border
        const BOTTOM = 0b0100;
        /// Show the left border
        const LEFT   = 0b1000;
        /// Show all borders
        const ALL = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
    }
}

pub struct BorderSet {
    pub top_left_qorner: &'static str,
    pub top_right_qorner: &'static str,
    pub bot_left_qorner: &'static str,
    pub bot_right_qorner: &'static str,
    pub vertical: &'static str,
    pub horizontal: &'static str,
}

impl BorderSet {
    pub const fn double() -> Self {
        DOUBLE_BORDERS
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Rect {
    pub row: u16,
    pub col: u16,
    pub width: usize,
    pub height: u16,
    borders: Borders,
}

impl Rect {
    pub const fn new(row: u16, col: u16, width: usize, height: u16) -> Self {
        Self { row, col, width, height, borders: Borders::NONE }
    }

    pub const fn new_bordered(mut row: u16, mut col: u16, mut width: usize, mut height: u16) -> Self {
        row -= 1;
        col -= 1;
        width -= 2;
        height -= 2;
        Self { row, col, width, height, borders: Borders::all() }
    }

    /// Creates floating modal around position (the row within it);
    /// Modal will float around the row (above or below - below is preffered) within Rect;
    /// Minimum height is 3 otherwise the modal will appear above the location;
    /// Minumum width is 40 otherwise the modal will appear before the location;
    /// If there is not enough space the rect will be without space height/width = 0;
    #[inline]
    pub fn modal_relative(&self, row_offset: u16, col_offset: u16, mut width: usize, mut height: u16) -> Self {
        let mut row = self.row + row_offset + 1; // goes to the row below it
        let mut col = self.col + col_offset; // goes behind col
        if self.height + self.row < height + row {
            if self.height > 3 + row {
                height = self.height - row;
            } else if row_offset > 3 {
                // goes above and finishes before the row;
                height = std::cmp::min(height, row_offset - 1);
                row -= height + 1;
            } else {
                width = 0;
                height = 0;
            };
        };
        if (self.width + self.col as usize) < (width + col as usize) {
            if self.width > 40 + col as usize {
                width = self.width - col as usize;
            } else if self.width > 40 {
                col = (self.col + self.width as u16) - 40;
                width = 40;
            } else {
                width = 0;
                height = 0;
            };
        };
        Rect::new(row, col, width, height)
    }

    /// Splitoff rows into Rect from current Rect - mutating it in place
    pub fn splitoff_rows(&mut self, rows: u16) -> Self {
        let old_height = self.height;
        self.height = self.height.saturating_sub(rows);
        Self {
            row: self.row + self.height,
            col: self.col,
            height: old_height - self.height,
            width: self.width,
            borders: self.borders,
        }
    }

    /// Splitoff cols into Rect from current Rect - mutating it in place
    pub fn splitoff_cols(&mut self, cols: usize) -> Self {
        let old_width = self.width;
        self.width = self.width.saturating_sub(cols);
        Self {
            row: self.row,
            col: self.width as u16 + self.col,
            height: self.height,
            width: old_width - self.width,
            borders: self.borders,
        }
    }

    /// Keep rows splitting the remaining into Rect
    pub fn keep_rows(&mut self, rows: u16) -> Self {
        let remaining_height = self.height.saturating_sub(rows);
        self.height -= remaining_height;
        Self {
            row: self.row + self.height,
            col: self.col,
            height: remaining_height,
            width: self.width,
            borders: self.borders,
        }
    }

    /// Keep cols splitting the remaining into Rect
    pub fn keep_col(&mut self, cols: usize) -> Self {
        let remaining_width = self.width.saturating_sub(cols);
        self.width -= remaining_width;
        Self {
            row: self.row,
            col: self.col + self.width as u16,
            height: self.height,
            width: remaining_width,
            borders: self.borders,
        }
    }

    pub fn get_line(&self, rel_idx: u16) -> Option<Line> {
        if rel_idx >= self.height {
            return None;
        }
        Some(Line { row: self.row + rel_idx, col: self.col, width: self.width })
    }

    pub fn center(&self, mut height: u16, mut width: usize) -> Self {
        height = std::cmp::min(self.height, height);
        let row = self.row + ((self.height - height) / 2);
        width = std::cmp::min(self.width, width);
        let col = self.col + ((self.width - width) / 2) as u16;
        Self { row, col, width, height, ..Default::default() }
    }

    pub fn right_top_corner(&self, mut height: u16, mut width: usize) -> Self {
        height = std::cmp::min(self.height, height);
        width = std::cmp::min(self.width, width);
        let col = self.col + (self.width - width) as u16;
        Self { row: self.row, col, width, height, ..Default::default() }
    }

    pub fn left_top_corner(&self, mut height: u16, mut width: usize) -> Self {
        height = std::cmp::min(self.height, height);
        width = std::cmp::min(self.width, width);
        Self { row: self.row, col: self.col, width, height, ..Default::default() }
    }

    pub fn right_bot_corner(&self, mut height: u16, mut width: usize) -> Self {
        height = std::cmp::min(self.height, height);
        width = std::cmp::min(self.width, width);
        let row = self.row + (self.height - height);
        let col = self.col + (self.width - width) as u16;
        Self { row, col, width, height, ..Default::default() }
    }

    pub fn left_bot_corner(&self, mut height: u16, mut width: usize) -> Self {
        height = std::cmp::min(self.height, height);
        width = std::cmp::min(self.width, width);
        let col = self.col + (self.width - width) as u16;
        Self { row: self.row, col, width, height, ..Default::default() }
    }

    pub fn rataui(rect: ratatui::layout::Rect) -> Self {
        Self::new(rect.y, rect.x, rect.width as usize, rect.height)
    }

    pub fn bordered(&mut self) {
        self.col += 1;
        self.row += 1;
        self.height -= 2;
        self.width -= 2;
        self.borders = Borders::all();
    }

    pub fn top_border(&mut self) -> &mut Self {
        self.row += 1;
        self.height -= 1;
        self.borders.insert(Borders::TOP);
        self
    }

    pub fn bot_border(&mut self) -> &mut Self {
        self.height -= 1;
        self.borders.insert(Borders::BOTTOM);
        self
    }

    pub fn right_border(&mut self) -> &mut Self {
        self.width -= 1;
        self.borders.insert(Borders::RIGHT);
        self
    }

    pub fn left_border(&mut self) -> &mut Self {
        self.col += 1;
        self.width -= 1;
        self.borders.insert(Borders::LEFT);
        self
    }

    pub fn absoute_diffs(&self) -> (u16, u16, usize) {
        (self.row, self.col, self.height as usize)
    }

    pub fn clear(&self, writer: &mut Backend) -> std::io::Result<()> {
        for line in self.into_iter() {
            line.render_empty(writer)?;
        }
        writer.flush()
    }

    pub fn draw_borders(&self, set: Option<BorderSet>, fg: Color, writer: &mut Backend) -> std::io::Result<()> {
        let top = self.borders.contains(Borders::TOP);
        let bot = self.borders.contains(Borders::BOTTOM);
        let left = self.borders.contains(Borders::LEFT);
        let right = self.borders.contains(Borders::RIGHT);

        let mut row = self.row;
        let mut col = self.col;
        let last_row = self.row + self.height;
        let last_col = self.col + self.width as u16;

        if top {
            row -= 1;
        };
        if left {
            col -= 1;
        };

        let set = set.unwrap_or(BORDERS);
        writer.save_cursor()?;
        writer.set_style(Style::fg(fg))?;
        if top {
            for col_idx in col..last_col {
                writer.go_to(row, col_idx)?;
                writer.print(set.horizontal)?;
            }
        }
        if bot {
            for col_idx in col..last_col {
                writer.go_to(last_row, col_idx)?;
                writer.print(set.horizontal)?;
            }
        }
        if left {
            for row_idx in row..last_row {
                writer.go_to(row_idx, col)?;
                writer.print(set.vertical)?;
            }
        }
        if right {
            for row_idx in row..last_row {
                writer.go_to(row_idx, last_col)?;
                writer.print(set.vertical)?;
            }
        }
        if self.borders.contains(Borders::TOP | Borders::LEFT) {
            writer.go_to(row, col)?;
            writer.print(set.top_left_qorner)?;
        }
        if self.borders.contains(Borders::TOP | Borders::RIGHT) {
            writer.go_to(row, last_col)?;
            writer.print(set.top_right_qorner)?;
        }
        if self.borders.contains(Borders::BOTTOM | Borders::LEFT) {
            writer.go_to(last_row, col)?;
            writer.print(set.bot_left_qorner)?;
        }
        if self.borders.contains(Borders::BOTTOM | Borders::RIGHT) {
            writer.go_to(last_row, last_col)?;
            writer.print(set.bot_right_qorner)?;
        }
        writer.reset_style()?;
        writer.restore_cursor()?;
        writer.flush()
    }
}

impl From<(u16, u16)> for Rect {
    fn from((width, height): (u16, u16)) -> Self {
        Self { row: 0, col: 0, width: width as usize, height, borders: Borders::empty() }
    }
}

pub struct RectIter<'a> {
    rect: &'a Rect,
    row_range: Range<u16>,
}

impl RectIter<'_> {
    /// return the number of lines remaining
    pub fn len(&self) -> usize {
        self.row_range.len()
    }

    /// returns the text width within the lines
    pub fn width(&self) -> usize {
        self.rect.width
    }
}

impl<'a> Iterator for RectIter<'a> {
    type Item = Line;
    fn next(&mut self) -> Option<Self::Item> {
        self.row_range.next().map(|row| Line { col: self.rect.col, row, width: self.rect.width })
    }
}

impl<'a> IntoIterator for &'a Rect {
    type IntoIter = RectIter<'a>;
    type Item = Line;
    fn into_iter(self) -> Self::IntoIter {
        RectIter { rect: self, row_range: self.row..self.row + self.height }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Line {
    pub row: u16,
    pub col: u16,
    pub width: usize,
}

impl Line {
    #[inline]
    pub fn render_centered(self, mut text: &str, backend: &mut Backend) -> std::io::Result<()> {
        if text.len() > self.width {
            text = unsafe { text.get_unchecked(..self.width) };
        }
        backend.print_at(self.row, self.col, format!("{text:^width$}", width = self.width))
    }

    #[inline]
    pub fn render_centered_styled(self, mut text: &str, style: Style, backend: &mut Backend) -> std::io::Result<()> {
        if text.len() > self.width {
            text = unsafe { text.get_unchecked(..self.width) };
        }
        backend.print_styled_at(self.row, self.col, format!("{text:>width$}", width = self.width), style)
    }

    #[inline]
    pub fn render_left(self, mut text: &str, backend: &mut Backend) -> std::io::Result<()> {
        if text.len() > self.width {
            text = unsafe { text.get_unchecked(..self.width) };
        }
        backend.print_at(self.row, self.col, format!("{text:>width$}", width = self.width))
    }

    #[inline]
    pub fn render_left_styled(self, mut text: &str, style: Style, backend: &mut Backend) -> std::io::Result<()> {
        if text.len() > self.width {
            text = unsafe { text.get_unchecked(..self.width) };
        }
        backend.print_styled_at(self.row, self.col, format!("{text:^width$}", width = self.width), style)
    }

    #[inline]
    pub fn render_empty(self, backend: &mut Backend) -> std::io::Result<()> {
        backend.print_at(self.row, self.col, format!("{:width$}", "", width = self.width))
    }

    #[inline]
    pub fn render(self, text: &str, backend: &mut Backend) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => backend.print_at(self.row, self.col, unsafe { text.get_unchecked(..self.width) }),
            Ordering::Equal => backend.print_at(self.row, self.col, text),
            Ordering::Less => backend.print_at(self.row, self.col, format!("{text:width$}", width = self.width)),
        }
    }

    #[inline]
    pub fn render_styled(self, text: &str, style: Style, backend: &mut Backend) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => {
                backend.print_styled_at(self.row, self.col, unsafe { text.get_unchecked(..self.width) }, style)
            }
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
            self.backend.print(unsafe { text.get_unchecked(..self.remaining) })?;
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
            self.backend.print_styled(unsafe { text.get_unchecked(..self.remaining) }, style)?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print_styled(text, style)?;
        Ok(true)
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
            self.backend.print_at(self.row, self.col, unsafe { text.get_unchecked(text.len() - self.remaining..) })?;
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
            self.backend.print_styled_at(
                self.row,
                self.col,
                unsafe { text.get_unchecked(text.len() - self.remaining..) },
                style,
            )?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        self.backend.print_styled_at(self.row, self.col + self.remaining as u16, text, style)?;
        Ok(true)
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

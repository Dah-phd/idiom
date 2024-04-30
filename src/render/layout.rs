use bitflags::bitflags;
use std::cmp::Ordering;
use std::io::{Result, Write};
use std::ops::Range;

use crossterm::cursor::{RestorePosition, SavePosition};
use crossterm::style::{ResetColor, SetForegroundColor};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, ContentStyle, Print, PrintStyledContent, StyledContent},
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
        let mut col = self.col + col_offset + 1; // goes behind col
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

    pub fn center(&self, rows: u16, cols: u16) -> Self {
        todo!()
    }

    pub fn right_corner(&self, rows: u16, cols: usize) -> Self {
        todo!()
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

    pub fn clear(&self, writer: &mut impl Write) -> std::io::Result<()> {
        for line in self.into_iter() {
            line.render_empty(writer)?;
        }
        writer.flush()
    }

    pub fn draw_borders(&self, set: Option<BorderSet>, fg: Color, writer: &mut impl Write) -> std::io::Result<()> {
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
        queue!(writer, SavePosition, SetForegroundColor(fg))?;
        if top {
            for col_idx in col..last_col {
                queue!(writer, MoveTo(col_idx, row), Print(set.horizontal))?;
            }
        }
        if bot {
            for col_idx in col..last_col {
                queue!(writer, MoveTo(col_idx, last_row), Print(set.horizontal))?;
            }
        }
        if left {
            for row_idx in row..last_row {
                queue!(writer, MoveTo(col, row_idx), Print(set.vertical))?;
            }
        }
        if right {
            for row_idx in row..last_row {
                queue!(writer, MoveTo(last_col, row_idx), Print(set.vertical))?;
            }
        }
        if self.borders.contains(Borders::TOP | Borders::LEFT) {
            queue!(writer, MoveTo(col, row), Print(set.top_left_qorner))?;
        }
        if self.borders.contains(Borders::TOP | Borders::RIGHT) {
            queue!(writer, MoveTo(last_col, row), Print(set.top_right_qorner))?;
        }
        if self.borders.contains(Borders::BOTTOM | Borders::LEFT) {
            queue!(writer, MoveTo(col, last_row), Print(set.bot_left_qorner))?;
        }
        if self.borders.contains(Borders::BOTTOM | Borders::RIGHT) {
            queue!(writer, MoveTo(last_col, last_row), Print(set.bot_right_qorner))?;
        }
        queue!(writer, RestorePosition, ResetColor)?;
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

#[derive(Debug)]
pub struct Line {
    pub row: u16,
    pub col: u16,
    pub width: usize,
}

impl Line {
    #[inline]
    pub fn render_empty(self, writer: &mut impl Write) -> std::io::Result<()> {
        queue!(writer, MoveTo(self.col, self.row), Print(format!("{:width$}", "", width = self.width)))
    }

    #[inline]
    pub fn render(self, text: &str, writer: &mut impl Write) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => {
                queue!(writer, MoveTo(self.col, self.row), Print(unsafe { text.get_unchecked(..self.width) }))
            }
            Ordering::Equal => {
                queue!(writer, MoveTo(self.col, self.row), Print(text))
            }
            Ordering::Less => {
                queue!(writer, MoveTo(self.col, self.row), Print(format!("{text:width$}", width = self.width)))
            }
        }
    }

    #[inline]
    pub fn render_styled(self, text: &str, style: ContentStyle, writer: &mut impl Write) -> Result<()> {
        match text.len().cmp(&self.width) {
            Ordering::Greater => {
                queue!(
                    writer,
                    MoveTo(self.col, self.row),
                    PrintStyledContent(StyledContent::new(style, unsafe { text.get_unchecked(..self.width) }))
                )
            }
            Ordering::Equal => {
                queue!(writer, MoveTo(self.col, self.row), PrintStyledContent(StyledContent::new(style, text)))
            }
            Ordering::Less => {
                queue!(
                    writer,
                    MoveTo(self.col, self.row),
                    PrintStyledContent(StyledContent::new(style, format!("{text:width$}", width = self.width)))
                )
            }
        }
    }

    pub fn builder(self, writer: &mut impl Write) -> std::io::Result<LineBuilder> {
        queue!(writer, MoveTo(self.col, self.row)).map(|_| LineBuilder { remaining: self.width })
    }
}

pub struct LineBuilder {
    remaining: usize,
}

impl LineBuilder {
    pub fn push(&mut self, text: &str, writer: &mut impl Write) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            queue!(writer, Print(unsafe { text.get_unchecked(..self.remaining) }))?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        queue!(writer, Print(text))?;
        Ok(true)
    }
    pub fn push_styled(&mut self, text: &str, style: ContentStyle, writer: &mut impl Write) -> std::io::Result<bool> {
        if text.len() > self.remaining {
            queue!(
                writer,
                PrintStyledContent(StyledContent::new(style, unsafe { text.get_unchecked(..self.remaining) }))
            )?;
            self.remaining = 0;
            return Ok(false);
        }
        self.remaining -= text.len();
        queue!(writer, PrintStyledContent(StyledContent::new(style, text)))?;
        Ok(true)
    }
}

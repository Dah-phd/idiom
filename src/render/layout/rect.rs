use crate::render::{
    backend::{Backend, BackendProtocol, Color, Style},
    layout::{BorderSet, Borders, Line, BORDERS},
    utils::UTF8Safe,
};
use std::ops::Range;

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
        let taken_width = self.width.saturating_sub(cols);
        self.width -= taken_width;
        Self {
            row: self.row,
            col: self.col + self.width as u16,
            height: self.height,
            width: taken_width,
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

    pub fn left(&self, cols: usize) -> Self {
        let width = std::cmp::min(cols, self.width);
        Rect { row: self.row, col: self.col, height: self.height, width, ..Default::default() }
    }

    pub fn right(&self, cols: usize) -> Self {
        let width = std::cmp::min(cols, self.width);
        let col = self.col + (self.width - width) as u16;
        Rect { row: self.row, col, height: self.height, width, ..Default::default() }
    }

    pub fn top(&self, rows: u16) -> Self {
        let height = std::cmp::min(rows, self.height);
        Rect { row: self.row, col: self.col, height, width: self.width, ..Default::default() }
    }

    pub fn bot(&self, rows: u16) -> Self {
        let height = std::cmp::min(rows, self.height);
        let row = self.row + (self.height - height);
        Rect { row, col: self.col, height, width: self.width, ..Default::default() }
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

    #[inline]
    pub fn bordered(&mut self) {
        self.col += 1;
        self.row += 1;
        self.height -= 2;
        self.width -= 2;
        self.borders = Borders::all();
    }

    #[inline]
    pub fn top_border(&mut self) -> &mut Self {
        self.row += 1;
        self.height -= 1;
        self.borders.insert(Borders::TOP);
        self
    }

    #[inline]
    pub fn bot_border(&mut self) -> &mut Self {
        self.height -= 1;
        self.borders.insert(Borders::BOTTOM);
        self
    }

    #[inline]
    pub fn right_border(&mut self) -> &mut Self {
        self.width -= 1;
        self.borders.insert(Borders::RIGHT);
        self
    }

    #[inline]
    pub fn left_border(&mut self) -> &mut Self {
        self.col += 1;
        self.width -= 1;
        self.borders.insert(Borders::LEFT);
        self
    }

    pub fn clear(&self, writer: &mut Backend) {
        for line in self.into_iter() {
            line.render_empty(writer);
        }
    }

    /// renders title if top border exists
    /// !!! this needs to happen after border rendering
    #[inline]
    pub fn border_title(&self, text: &str, backend: &mut Backend) {
        if self.borders.contains(Borders::TOP) {
            backend.print_at(self.row - 1, self.col, text.truncate_width(self.width).1);
        };
    }

    /// border_title with style
    #[inline]
    pub fn border_title_styled(&self, text: &str, style: Style, backend: &mut Backend) {
        if self.borders.contains(Borders::TOP) {
            backend.print_styled_at(self.row - 1, self.col, text.truncate_width(self.width).1, style);
        };
    }

    /// renders title if bottom border exists
    /// !!! this needs to happen after border rendering
    #[inline]
    pub fn border_title_bot(&self, text: &str, backend: &mut Backend) {
        if self.borders.contains(Borders::BOTTOM) {
            backend.print_at(self.row + self.height + 1, self.col, text.truncate_width(self.width).1);
        };
    }

    /// border_title_bot with style
    #[inline]
    pub fn border_title_bot_styled(&self, text: &str, style: Style, backend: &mut Backend) {
        if self.borders.contains(Borders::BOTTOM) {
            return backend.print_styled_at(
                self.row + self.height + 1,
                self.col,
                text.truncate_width(self.width).1,
                style,
            );
        }
    }

    pub fn draw_borders(&self, set: Option<BorderSet>, fg: Option<Color>, writer: &mut Backend) {
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
        writer.save_cursor();
        if let Some(color) = fg {
            writer.set_style(Style::fg(color));
        };
        if top {
            for col_idx in col..last_col {
                writer.go_to(row, col_idx);
                writer.print(set.horizontal);
            }
        }
        if bot {
            for col_idx in col..last_col {
                writer.go_to(last_row, col_idx);
                writer.print(set.horizontal);
            }
        }
        if left {
            for row_idx in row..last_row {
                writer.go_to(row_idx, col);
                writer.print(set.vertical);
            }
        }
        if right {
            for row_idx in row..last_row {
                writer.go_to(row_idx, last_col);
                writer.print(set.vertical);
            }
        }
        if self.borders.contains(Borders::TOP | Borders::LEFT) {
            writer.go_to(row, col);
            writer.print(set.top_left_qorner);
        }
        if self.borders.contains(Borders::TOP | Borders::RIGHT) {
            writer.go_to(row, last_col);
            writer.print(set.top_right_qorner);
        }
        if self.borders.contains(Borders::BOTTOM | Borders::LEFT) {
            writer.go_to(last_row, col);
            writer.print(set.bot_left_qorner);
        }
        if self.borders.contains(Borders::BOTTOM | Borders::RIGHT) {
            writer.go_to(last_row, last_col);
            writer.print(set.bot_right_qorner);
        }
        if fg.is_some() {
            writer.reset_style();
        }
    }
}

impl From<(u16, u16)> for Rect {
    fn from((width, height): (u16, u16)) -> Self {
        Self { row: 0, col: 0, width: width as usize, height, borders: Borders::empty() }
    }
}

pub struct RectIter {
    rect: Rect,
    row_range: Range<u16>,
}

impl RectIter {
    /// return the number of lines remaining
    #[inline]
    pub fn len(&self) -> usize {
        self.row_range.len()
    }

    /// returns the text width within the lines
    #[inline]
    pub fn width(&self) -> usize {
        self.rect.width
    }

    /// moves to next line and returns width if success
    #[inline]
    pub fn move_cursor(&mut self, backend: &mut Backend) -> Option<usize> {
        self.next().map(|Line { row, col, width }| {
            backend.go_to(row, col);
            width
        })
    }

    /// returns the remaining lines as rect (None if all lines are used)
    #[inline]
    pub fn into_rect(mut self) -> Option<Rect> {
        let height = self.row_range.len() as u16;
        self.row_range.next().map(|row| Rect {
            row,
            col: self.rect.col,
            width: self.rect.width,
            height,
            ..Default::default()
        })
    }

    #[inline]
    pub fn forward(&mut self, mut steps: usize) {
        while steps != 0 {
            steps -= 1;
            self.row_range.next();
        }
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.row_range.is_empty()
    }

    #[inline]
    pub fn next_line_idx(&self) -> u16 {
        self.row_range.start
    }
}

impl Iterator for RectIter {
    type Item = Line;
    fn next(&mut self) -> Option<Self::Item> {
        self.row_range.next().map(|row| Line { col: self.rect.col, row, width: self.rect.width })
    }
}

impl IntoIterator for Rect {
    type IntoIter = RectIter;
    type Item = Line;
    fn into_iter(self) -> Self::IntoIter {
        RectIter { row_range: self.row..self.row + self.height, rect: self }
    }
}

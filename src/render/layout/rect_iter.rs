use crate::render::{
    backend::{Backend, BackendProtocol},
    layout::{Line, Rect},
};
use std::ops::Range;

pub trait IterLines: Iterator<Item = Line> {
    fn len(&self) -> usize;
    fn width(&self) -> usize;
    fn move_cursor(&mut self, backend: &mut Backend) -> Option<usize>;
    fn into_rect(self) -> Option<Rect>;
    fn forward(&mut self, steps: usize);
    fn is_finished(&self) -> bool;
    fn next_line_idx(&self) -> u16;
    fn clear_to_end(&mut self, backend: &mut Backend);
}

pub struct RectIter {
    rect: Rect,
    row_range: Range<u16>,
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

impl IterLines for RectIter {
    /// return the number of lines remaining
    #[inline]
    fn len(&self) -> usize {
        self.row_range.len()
    }

    /// returns the text width within the lines
    #[inline]
    fn width(&self) -> usize {
        self.rect.width
    }

    /// moves to next line and returns width if success
    #[inline]
    fn move_cursor(&mut self, backend: &mut Backend) -> Option<usize> {
        self.next().map(|Line { row, col, width }| {
            backend.go_to(row, col);
            width
        })
    }

    /// returns the remaining lines as rect (None if all lines are used)
    #[inline]
    fn into_rect(mut self) -> Option<Rect> {
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
    fn forward(&mut self, mut steps: usize) {
        while steps != 0 {
            steps -= 1;
            self.row_range.next();
        }
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.row_range.is_empty()
    }

    #[inline]
    fn next_line_idx(&self) -> u16 {
        self.row_range.start
    }

    #[inline]
    fn clear_to_end(&mut self, backend: &mut Backend) {
        for remaining_line in self {
            remaining_line.render_empty(backend);
        }
    }
}

pub struct DoublePaddedRectIter {
    rect: Rect,
    row_range: Range<u16>,
    padding: usize,
    padded_col: u16,
    padded_width: usize,
}

impl Iterator for DoublePaddedRectIter {
    type Item = Line;
    // Implementation will not apply padding automatically, to get full line call unpadded_line to clear line before invoking next
    // or directly invoke, next_padded
    // or move_cursor <- best practice
    fn next(&mut self) -> Option<Self::Item> {
        self.row_range.next().map(|row| Line { col: self.padded_col, row, width: self.padded_width })
    }
}

impl IterLines for DoublePaddedRectIter {
    #[inline]
    fn len(&self) -> usize {
        self.row_range.len()
    }

    #[inline]
    fn width(&self) -> usize {
        self.padded_width
    }

    #[inline]
    fn forward(&mut self, mut steps: usize) {
        while steps != 0 {
            steps -= 1;
            self.row_range.next();
        }
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.row_range.is_empty()
    }

    #[inline]
    fn next_line_idx(&self) -> u16 {
        self.row_range.start
    }

    #[inline]
    fn move_cursor(&mut self, backend: &mut Backend) -> Option<usize> {
        let row = self.row_range.next()?;
        backend.go_to(row, self.padded_col + self.padded_width as u16);
        backend.pad(self.padding);
        backend.go_to(row, self.rect.col);
        backend.pad(self.padding);
        Some(self.padded_width)
    }

    #[inline]
    fn into_rect(mut self) -> Option<Rect> {
        let height = self.row_range.len() as u16;
        self.row_range.next().map(|row| Rect {
            row,
            col: self.padded_col,
            width: self.padded_width,
            height,
            ..Default::default()
        })
    }

    #[inline]
    fn clear_to_end(&mut self, backend: &mut Backend) {
        for row in self.row_range.by_ref() {
            Line { row, col: self.rect.col, width: self.rect.width }.render_empty(backend);
        }
    }
}

impl DoublePaddedRectIter {
    fn new(rect: Rect, padding: usize) -> Self {
        let two_way_pad = padding * 2;
        if rect.width <= two_way_pad {
            return Self { row_range: rect.row..rect.row, padded_col: rect.col, padded_width: 0, padding, rect };
        }
        Self {
            row_range: rect.row..rect.row + rect.height,
            padded_col: rect.col + padding as u16,
            padded_width: rect.width - two_way_pad,
            padding,
            rect,
        }
    }

    pub fn next_padded(&mut self, backend: &mut Backend) -> Option<Line> {
        let row = self.row_range.next()?;
        backend.go_to(row, self.padded_col + self.padded_width as u16);
        backend.pad(self.padding);
        backend.go_to(row, self.rect.col);
        backend.pad(self.padding);
        Some(Line { row, col: self.padded_col, width: self.padded_width })
    }
}

impl Rect {
    pub fn iter_padded(self, padding: usize) -> DoublePaddedRectIter {
        DoublePaddedRectIter::new(self, padding)
    }
}

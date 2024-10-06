use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Add, AddAssign},
};

#[derive(Clone, Copy, PartialEq)]
pub struct EditMetaData {
    pub start_line: usize,
    pub from: usize, // ignored after Add - is set to 0;
    pub to: usize,
}

impl Add for EditMetaData {
    type Output = Self;

    fn add(self, othr: Self) -> Self::Output {
        match self.start_line.cmp(&othr.start_line) {
            Ordering::Equal => {
                let start_line = self.start_line;
                if self.to > othr.from {
                    EditMetaData { start_line, from: self.from, to: othr.to + (self.to - othr.from) }
                } else {
                    EditMetaData { start_line, from: self.from + (othr.from - self.to), to: othr.to }
                }
            }
            Ordering::Greater => {
                let start_line = othr.start_line;
                let self_end = self.start_line + self.to;
                let othr_start = othr.start_line + othr.from;
                let from_base = (self.start_line + self.from) - start_line;
                let to_base = othr.to;
                if self_end > othr_start {
                    EditMetaData { start_line, from: from_base, to: to_base + (self_end - othr_start) }
                } else {
                    EditMetaData { start_line, from: from_base + (othr_start - self_end), to: to_base }
                }
            }
            Ordering::Less => {
                let start_line = self.start_line;
                let self_end = self.start_line + self.to;
                let othr_start = othr.start_line + othr.from;
                let from_base = self.from;
                let to_base = (othr.start_line + othr.to) - start_line;
                if self_end > othr_start {
                    EditMetaData { start_line, from: from_base, to: to_base + (self_end - othr_start) }
                } else {
                    EditMetaData { start_line, from: from_base + (othr_start - self_end), to: to_base }
                }
            }
        }
    }
}

impl AddAssign for EditMetaData {
    fn add_assign(&mut self, othr: Self) {
        match self.start_line.cmp(&othr.start_line) {
            Ordering::Equal => {
                if self.to > othr.from {
                    self.to = othr.to + (self.to - othr.from);
                } else {
                    self.from = self.from + (othr.from - self.to);
                    self.to = othr.to;
                }
            }
            Ordering::Greater => {
                let start_line = othr.start_line;
                let self_end = self.start_line + self.to;
                let othr_start = othr.start_line + othr.from;
                let from_base = (self.start_line + self.from) - start_line;
                let to_base = othr.to;
                if self_end > othr_start {
                    self.start_line = start_line;
                    self.from = from_base;
                    self.to = to_base + (self_end - othr_start);
                } else {
                    self.start_line = start_line;
                    self.from = from_base + (othr_start - self_end);
                    self.to = to_base;
                }
            }
            Ordering::Less => {
                let self_end = self.start_line + self.to;
                let othr_start = othr.start_line + othr.from;
                let from_base = self.from;
                let to_base = (othr.start_line + othr.to) - self.start_line;
                if self_end > othr_start {
                    self.from = from_base;
                    self.to = to_base + (self_end - othr_start);
                } else {
                    self.from = from_base + (othr_start - self_end);
                    self.to = to_base;
                }
            }
        }
    }
}

impl EditMetaData {
    #[inline]
    pub const fn line_changed(start_line: usize) -> Self {
        Self { start_line, from: 1, to: 1 }
    }

    #[inline]
    pub const fn end_line(&self) -> usize {
        self.start_line + self.to - 1
    }

    #[inline]
    pub const fn rev(&self) -> Self {
        EditMetaData { start_line: self.start_line, from: self.to, to: self.from }
    }
}

impl Debug for EditMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} >> {}", self.from, self.to))
    }
}

impl From<EditMetaData> for lsp_types::Range {
    fn from(meta: EditMetaData) -> Self {
        let start = lsp_types::Position::new(meta.start_line as u32, 0);
        let end = lsp_types::Position::new((meta.start_line + meta.to) as u32, 0);
        lsp_types::Range::new(start, end)
    }
}
use crate::{cursor::CursorPosition, ext_tui::CrossTerm};
use idiom_tui::{layout::Rect, Backend};
use std::cmp::Ordering;
use vt100::Screen;

pub struct CursorState {
    hidden: bool,
    row: u16,
    col: u16,
}

impl CursorState {
    pub fn apply(&mut self, screen: &Screen, backend: &mut CrossTerm) {
        if screen.hide_cursor() || screen.scrollback() != 0 {
            if self.hidden {
                return;
            }
            self.hidden = true;
            backend.hide_cursor();
        } else {
            if !self.hidden {
                return;
            }
            let (row, col) = screen.cursor_position();
            backend.go_to(self.row + row, self.col + col);
            backend.show_cursor();
        }
    }

    pub fn resize(&mut self, rect: Rect) {
        self.row = rect.row;
        self.col = rect.col;
    }
}

impl From<Rect> for CursorState {
    fn from(rect: Rect) -> Self {
        Self { row: rect.row, col: rect.col, hidden: true }
    }
}

#[derive(Default, Debug)]
pub struct Select {
    finished: bool,
    updated: bool,
    start: Option<Position>,
    end: Option<Position>,
}

impl Select {
    pub fn mouse_down(&mut self, row: u16, col: u16) {
        self.updated = true;
        self.finished = false;
        self.end = None;
        self.start = Some(Position { row, col });
    }

    pub fn mouse_drag(&mut self, row: u16, col: u16) {
        self.updated = true;
        if self.start.is_none() || self.finished {
            self.start = Some(Position { row, col });
        }
        self.finished = false;
        self.end = Some(Position { row, col });
    }

    pub fn mouse_up(&mut self, row: u16, col: u16) {
        self.updated = true;
        if self.start.is_some() && !self.finished {
            self.end = Some(Position { row, col });
            self.finished = true;
        }
    }

    pub fn clear(&mut self) {
        if self.start.is_none() {
            self.updated = true;
            return;
        }
        *self = Self::default();
        self.updated = true;
    }

    pub fn get(&self) -> Option<(Position, Position)> {
        self.start.zip(self.end).and_then(|(x, y)| match x.cmp(&y) {
            Ordering::Equal => None,
            Ordering::Greater => Some((y, x)),
            Ordering::Less => Some((x, y)),
        })
    }

    pub fn copy_clip(&self, screen: &Screen) -> Option<String> {
        let (start, end) = self.get()?;
        let clip = screen.contents_between(start.row, start.col, end.row, end.col);
        if clip.is_empty() {
            return None;
        }
        Some(clip)
    }

    pub fn collect_update(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Position {
    pub row: u16,
    pub col: u16,
}

impl From<Position> for CursorPosition {
    fn from(value: Position) -> Self {
        Self { line: value.row as usize, char: value.col as usize }
    }
}

#[cfg(test)]
mod test {
    use super::{Position, Select};

    #[test]
    fn test_select_get() {
        let s = Select {
            finished: false,
            updated: false,
            start: Some(Position { row: 0, col: 1 }),
            end: Some(Position { row: 1, col: 0 }),
        };

        assert_eq!(Some((Position { row: 0, col: 1 }, Position { row: 1, col: 0 })), s.get());

        let s = Select {
            finished: false,
            updated: false,
            start: Some(Position { row: 0, col: 11 }),
            end: Some(Position { row: 0, col: 1 }),
        };

        assert_eq!(Some((Position { row: 0, col: 1 }, Position { row: 0, col: 11 })), s.get());
    }
}

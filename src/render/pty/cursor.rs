use std::cmp::Ordering;

use crate::render::{
    backend::{Backend, BackendProtocol},
    layout::Rect,
};
use vt100::Screen;

pub struct CursorState {
    hidden: bool,
    row: u16,
    col: u16,
}

impl CursorState {
    pub fn apply(&mut self, screen: &Screen, backend: &mut Backend) {
        if screen.hide_cursor() {
            if self.hidden {
                return;
            }
            self.hidden = true;
            Backend::hide_cursor();
        } else {
            if !self.hidden {
                return;
            }
            let (row, col) = screen.cursor_position();
            backend.go_to(self.row + row, self.col + col);
            Backend::show_cursor();
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
    start: Option<Position>,
    end: Option<Position>,
}

impl Select {
    pub fn mouse_down(&mut self, row: u16, col: u16) {
        self.finished = false;
        self.end = None;
        self.start = Some(Position { row, col });
    }

    pub fn mouse_drag(&mut self, row: u16, col: u16) {
        if self.start.is_none() || self.finished {
            self.start = Some(Position { row, col });
        }
        self.finished = false;
        self.end = Some(Position { row, col });
    }

    pub fn mouse_up(&mut self, row: u16, col: u16) {
        if self.start.is_some() && !self.finished {
            self.end = Some(Position { row, col });
            self.finished = true;
        }
    }

    pub fn clear(&mut self) {
        if self.start.is_none() {
            return;
        }
        *self = Self::default();
    }

    pub fn get(&self) -> Option<(Position, Position)> {
        self.start.zip(self.end).and_then(|(x, y)| match x.cmp(&y) {
            Ordering::Equal => None,
            Ordering::Greater => Some((y, x)),
            Ordering::Less => Some((x, y)),
        })
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Position {
    pub row: u16,
    pub col: u16,
}

#[cfg(test)]
mod test {
    use super::{Position, Select};

    #[test]
    fn test_select_get() {
        let s = Select {
            finished: false,
            start: Some(Position { row: 0, col: 1 }),
            end: Some(Position { row: 1, col: 0 }),
        };

        assert_eq!(Some((Position { row: 0, col: 1 }, Position { row: 1, col: 0 })), s.get());

        let s = Select {
            finished: false,
            start: Some(Position { row: 0, col: 11 }),
            end: Some(Position { row: 0, col: 1 }),
        };

        assert_eq!(Some((Position { row: 0, col: 1 }, Position { row: 0, col: 11 })), s.get());
    }
}

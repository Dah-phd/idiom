use std::sync::{Arc, Mutex};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::ContentStyle,
};
use vt100::{Cell, Color, Parser, Screen};

use crate::ext_tui::StyleExt;

pub struct TrackedParser {
    buffers: Arc<Mutex<Vec<u8>>>,
    scrollback: usize,
    inner: Parser,
}

impl TrackedParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        let scrollback = 2000;
        Self { inner: Parser::new(rows, cols, scrollback), buffers: Arc::default(), scrollback }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.inner.screen_mut().set_size(rows, cols);
    }

    pub fn scroll_up(&mut self) {
        let cur_scroll = self.inner.screen().scrollback();
        self.inner.screen_mut().set_scrollback(cur_scroll + 1);
    }

    pub fn scroll_down(&mut self) {
        let cur_scroll = self.inner.screen().scrollback();
        self.inner.screen_mut().set_scrollback(cur_scroll.checked_sub(1).unwrap_or_default());
    }

    pub fn scroll_to_end(&mut self) {
        self.inner.screen_mut().set_scrollback(0);
    }

    pub fn buffer_access(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.buffers)
    }

    pub fn try_parse(&mut self) -> bool {
        let Ok(mut lock) = self.buffers.try_lock() else {
            return false;
        };
        if lock.is_empty() {
            return false;
        }
        let bytes = lock.drain(..).collect::<Vec<u8>>();
        drop(lock);
        self.inner.process(&bytes);
        true
    }

    /// Get full content of the pty
    /// that will drain the inner lock and collect all data
    #[must_use]
    pub fn full_content(&mut self) -> String {
        let mut lock = match self.buffers.lock() {
            Ok(lock) => lock,
            Err(err) => err.into_inner(),
        };
        let bytes = lock.drain(..).collect::<Vec<u8>>();
        drop(lock);
        self.inner.process(&bytes);
        let screen = self.inner.screen_mut();
        // do not resize for partial content (screen)
        let (rows, cols) = screen.size();
        screen.set_size(rows + self.scrollback as u16, cols);
        screen.set_scrollback(self.scrollback + rows as usize);
        screen.contents()
    }

    pub fn screen(&self) -> &Screen {
        self.inner.screen()
    }
}

pub fn get_ctrl_char(key: &KeyEvent) -> Option<u8> {
    if let KeyEvent { code: KeyCode::Char(ch), modifiers: KeyModifiers::CONTROL, .. } = key {
        let ctrl_char = match ch {
            '@' => 0x0,
            'a' => 0x1,
            'b' => 0x2,
            'c' => 0x3,
            'd' => 0x4,
            'e' => 0x5,
            'f' => 0x6,
            'g' => 0x7,
            'h' => 0x8,
            'i' => 0x9,
            'j' => 0x10,
            'k' => 0x11,
            'l' => 0x12,
            'm' => 0x13,
            'n' => 0x14,
            'o' => 0x15,
            'p' => 0x16,
            'q' => 0x17,
            'r' => 0x18,
            's' => 0x19,
            't' => 0x20,
            'u' => 0x21,
            'v' => 0x22,
            'w' => 0x23,
            'x' => 0x24,
            'y' => 0x25,
            'z' => 0x26,
            '[' => 0x27,
            '\\' => 0x28,
            ']' => 0x29,
            '^' => 0x30,
            '_' => 0x30,
            _ => return None,
        };
        return Some(ctrl_char);
    };
    None
}

pub fn parse_cell_style(cell: &Cell) -> ContentStyle {
    let mut style = ContentStyle::default();
    style.set_bg(parse_color(cell.bgcolor()));
    style.set_fg(parse_color(cell.fgcolor()));
    if cell.bold() {
        style.add_bold();
    }
    if cell.italic() {
        style.add_ital();
    }
    if cell.inverse() {
        style.add_reverse();
    }
    if cell.underline() {
        style.underline(None);
    }
    style
}

fn parse_color(base: Color) -> Option<crossterm::style::Color> {
    match base {
        Color::Default => None,
        Color::Idx(idx) => Some(crossterm::style::Color::AnsiValue(idx)),
        Color::Rgb(r, g, b) => Some(crossterm::style::Color::Rgb { r, g, b }),
    }
}

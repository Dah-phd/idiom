use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::ContentStyle,
};
use vt100::{Cell, Color, Parser, Screen};

use crate::render::backend::StyleExt;

pub struct TrackedParser {
    inner: Parser,
    updated: bool,
}

impl TrackedParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { inner: Parser::new(rows, cols, 2000), updated: false }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.updated = true;
        self.inner.process(bytes);
    }

    pub fn new_screen(&mut self) -> Option<Screen> {
        if !self.updated {
            return None;
        }
        self.updated = false;
        Some(self.inner.screen().clone())
    }

    pub fn screen(&mut self) -> Screen {
        self.updated = false;
        self.inner.screen().clone()
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

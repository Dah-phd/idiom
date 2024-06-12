use std::io::Write;

use super::{BackendProtocol, Style};

pub struct Backend {
    pub data: Vec<(Style, String)>,
    pub default_style: Style,
}

impl BackendProtocol for Backend {
    fn clear_all(&mut self) {
        self.data.push((Style::default(), String::from("<<clear all>>")));
    }
    fn clear_line(&mut self) {
        self.data.push((Style::default(), String::from("<<clear line>>")));
    }
    fn clear_to_eol(&mut self) {
        self.data.push((Style::default(), String::from("<<clear EOL>>")));
    }

    fn exit() -> std::io::Result<()> {
        Ok(())
    }

    fn get_style(&mut self) -> Style {
        self.default_style
    }

    fn go_to(&mut self, row: u16, col: u16) {
        self.data.push((Style::default(), format!("<<go to row: {row} col: {col}>>")))
    }

    fn hide_cursor(&mut self) {
        self.data.push((Style::default(), String::from("<<hide cursor>>")));
    }

    fn init() -> Self {
        Self { data: Vec::new(), default_style: Style::default() }
    }

    fn print<D: std::fmt::Display>(&mut self, text: D) {
        self.data.push((self.default_style, text.to_string()));
    }

    fn print_at<D: std::fmt::Display>(&mut self, row: u16, col: u16, text: D) {
        self.go_to(row, col);
        self.print(text)
    }
    fn print_styled<D: std::fmt::Display>(&mut self, text: D, style: Style) {
        self.data.push((style, text.to_string()))
    }

    fn print_styled_at<D: std::fmt::Display>(&mut self, row: u16, col: u16, text: D, style: Style) {
        self.go_to(row, col);
        self.print_styled(text, style);
    }

    fn render_cursor_at(&mut self, row: u16, col: u16) {
        self.data.push((self.default_style, format!("<<draw cursor row: {row} col: {col}>>")));
    }

    fn reset_style(&mut self) {
        self.default_style = Style::default();
    }

    fn restore_cursor(&mut self) {
        self.data.push((self.default_style, String::from("<<restored cursor>>")))
    }

    fn save_cursor(&mut self) {
        self.data.push((self.default_style, String::from("<<saved cursor>>")));
    }

    fn screen() -> std::io::Result<crate::render::layout::Rect> {
        Ok(crate::render::layout::Rect::new(0, 0, 120, 60))
    }

    fn set_bg(&mut self, color: Option<super::Color>) {
        self.default_style.set_bg(color);
        self.data.push((self.default_style, format!("<<set bg {:?}>>", color)));
    }

    fn set_fg(&mut self, color: Option<super::Color>) {
        self.default_style.set_fg(color);
        self.data.push((self.default_style, format!("<<set fg {:?}>>", color)));
    }

    fn set_style(&mut self, style: Style) {
        self.default_style = style;
        self.data.push((self.default_style, format!("<<style set to {:?}>>", self.default_style)))
    }

    fn show_cursor(&mut self) {
        self.data.push((self.default_style, String::from("<<show cursor>>")));
    }

    fn to_set_style(&mut self) {
        self.data.push((self.default_style, String::from("<<set style>>")));
    }

    fn update_style(&mut self, style: Style) {
        self.default_style.update(style);
        self.data.push((self.default_style, String::from("<<updated style>>")))
    }
}

impl Write for Backend {
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, mut _buf: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
}

impl Backend {
    pub fn unwrap(self) -> Vec<(Style, String)> {
        self.data
    }

    pub fn drain(&mut self) -> Vec<(Style, String)> {
        std::mem::take(&mut self.data)
    }
}

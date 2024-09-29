mod crossterm_backend;
use super::layout::Rect;
#[cfg(not(test))]
pub use crossterm_backend::Backend;
pub use crossterm_backend::{
    color::{self, pull_color, serialize_rgb},
    Color, Style,
};
use std::{
    fmt::Display,
    io::{Result, Write},
};

/// If stdout is returning errors the program should crash -> use expect
#[allow(dead_code)] // impl all utilities although not all are used
pub trait BackendProtocol: Write + Sized {
    fn init() -> Self;

    fn exit() -> std::io::Result<()>;

    /// get whole screen as rect
    fn screen() -> Result<Rect>;

    /// clears from cursor until the End Of Line
    fn clear_to_eol(&mut self);

    /// clears current cursor line
    fn clear_line(&mut self);

    fn clear_all(&mut self);

    /// stores the cursor
    fn save_cursor(&mut self);

    /// restores cursor position
    fn restore_cursor(&mut self);

    /// sets the style for the print/print at
    fn set_style(&mut self, style: Style);

    fn get_style(&mut self) -> Style;

    fn to_set_style(&mut self);

    /// update existing style if exists otherwise sets it to the new one
    /// mods will be taken from updating and will replace fg and bg if present
    fn update_style(&mut self, style: Style);

    /// adds foreground to the already set style
    fn set_fg(&mut self, color: Option<Color>);

    /// adds background to the already set style
    fn set_bg(&mut self, color: Option<Color>);

    /// restores the style of the writer to default
    fn reset_style(&mut self);

    /// sends the cursor to location
    fn go_to(&mut self, row: u16, col: u16);

    /// direct adding cursor at location - no buffer queing
    fn render_cursor_at(&mut self, row: u16, col: u16);

    /// direct showing cursor - no buffer queing
    fn show_cursor(&mut self);

    /// direct hiding cursor - no buffer queing
    fn hide_cursor(&mut self);

    fn print<D: Display>(&mut self, text: D);

    /// goes to location and prints text
    fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D);

    /// prints styled text without affecting the writer set style
    fn print_styled<D: Display>(&mut self, text: D, style: Style);

    /// goes to location and prints styled text without affecting the writer set style
    fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style);

    /// padding with empty space
    fn pad(&mut self, width: usize);
}

#[cfg(test)]
mod test;

#[cfg(test)]
pub use test::Backend;

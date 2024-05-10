mod crossterm_backend;
use std::{
    fmt::Display,
    io::{Result, Write},
};

pub use crossterm_backend::{color, Backend, Color, Style};

use crate::render::layout::Rect;

pub trait BackendProtocol: Write + Sized {
    fn init() -> Result<Self>;

    fn exit() -> Result<()>;

    /// get whole screen as rect
    fn screen() -> Result<Rect>;

    /// clears from cursor until the End Of Line
    fn clear_to_eol(&mut self) -> Result<()>;

    /// clears current cursor line
    fn clear_line(&mut self) -> Result<()>;

    fn clear_all(&mut self) -> Result<()>;

    /// stores the cursor and hides it
    fn save_cursor(&mut self) -> Result<()>;

    /// restores cursor position and shows cursor
    fn restore_cursor(&mut self) -> Result<()>;

    /// sets the style for the print/print at
    fn set_style(&mut self, style: Style) -> Result<()>;

    fn get_style(&mut self) -> Style;

    fn to_set_style(&mut self) -> Result<()>;

    /// update existing style if exists otherwise sets it to the new one
    /// mods will be taken from updating and will replace fg and bg if present
    fn update_style(&mut self, style: Style) -> Result<()>;

    /// adds foreground to the already set style
    fn set_fg(&mut self, color: Option<Color>) -> Result<()>;

    /// adds background to the already set style
    fn set_bg(&mut self, color: Option<Color>) -> Result<()>;

    /// restores the style of the writer to default
    fn reset_style(&mut self) -> Result<()>;

    /// sends the cursor to location
    fn go_to(&mut self, row: u16, col: u16) -> Result<()>;

    /// direct adding cursor at location - no buffer queing
    fn render_cursor_at(&mut self, row: u16, col: u16) -> Result<()>;

    /// direct showing cursor - no buffer queing
    fn show_cursor(&mut self) -> Result<()>;

    /// direct hiding cursor - no buffer queing
    fn hide_cursor(&mut self) -> Result<()>;

    fn print<D: Display>(&mut self, text: D) -> Result<()>;

    /// goes to location and prints text
    fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) -> Result<()>;

    /// prints styled text without affecting the writer set style
    fn print_styled<D: Display>(&mut self, text: D, style: Style) -> Result<()>;

    /// goes to location and prints styled text without affecting the writer set style
    fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style) -> Result<()>;
}

pub enum _Color {}

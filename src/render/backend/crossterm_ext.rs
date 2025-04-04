// pub mod color;
use super::style::StyleExt;
use crossterm::style::Color;
use crossterm::{
    cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show},
    execute, queue,
    style::{ContentStyle, Print, ResetColor, SetStyle},
    terminal::{size, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
#[allow(unused_imports)]
use std::{
    fmt::Display,
    io::{StderrLock, Stdout, Write},
};

const ERR_MSG: &str = "Rendering (Stdout) Err:";

use crate::render::layout::Rect;

use super::BackendProtocol;

/// Thin wrapper around rendering framework, allowing easy switching of backend
/// If stdout gets an error Backend will crash the program as rendering is to priority
/// Add cfg and new implementation of the wrapper to make the backend swichable
/// Main reason is to clear out the issue with PrintStyled on CrossTerm
pub struct Backend {
    writer: Stdout, // could be moved to locked state for performance but current frame generation is about 200 µs
    default_styled: Option<ContentStyle>,
}

impl Write for Backend {
    #[inline(always)]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(buf)
    }

    #[inline(always)]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.writer.write_fmt(fmt)
    }
}

impl BackendProtocol for Backend {
    #[inline]
    fn init() -> Self {
        init_terminal().expect(ERR_MSG);
        Self { writer: std::io::stdout(), default_styled: None }
    }

    #[inline]
    fn exit() -> std::io::Result<()> {
        graceful_exit()
    }

    /// get whole screen as rect
    #[inline]
    fn screen() -> std::io::Result<Rect> {
        size().map(Rect::from)
    }

    /// freeze screen allowing to build buffer
    #[inline]
    fn freeze(&mut self) {
        execute!(self, BeginSynchronizedUpdate).expect(ERR_MSG);
    }

    /// unfreeze allowing the buffer to render
    #[inline]
    fn unfreeze(&mut self) {
        execute!(self, EndSynchronizedUpdate).expect(ERR_MSG);
    }

    /// clears from cursor until the End Of Line
    #[inline]
    fn clear_to_eol(&mut self) {
        queue!(self, Clear(ClearType::UntilNewLine)).expect(ERR_MSG);
    }

    /// clears current cursor line
    #[inline]
    fn clear_line(&mut self) {
        queue!(self, Clear(ClearType::CurrentLine)).expect(ERR_MSG);
    }

    #[inline]
    fn clear_all(&mut self) {
        queue!(self, Clear(ClearType::All)).expect(ERR_MSG);
    }

    /// stores the cursor
    #[inline]
    fn save_cursor(&mut self) {
        execute!(self, SavePosition).expect(ERR_MSG);
    }

    /// restores cursor position
    #[inline]
    fn restore_cursor(&mut self) {
        queue!(self, RestorePosition).expect(ERR_MSG);
    }

    /// sets the style for the print/print at
    #[inline]
    fn set_style(&mut self, style: ContentStyle) {
        self.default_styled.replace(style);
        queue!(self, ResetColor, SetStyle(style)).expect(ERR_MSG);
    }

    #[inline]
    fn get_style(&mut self) -> ContentStyle {
        self.default_styled.unwrap_or_default()
    }

    #[inline]
    fn to_set_style(&mut self) {
        match self.default_styled {
            Some(style) => queue!(self, ResetColor, SetStyle(style)),
            None => queue!(self, ResetColor),
        }
        .expect(ERR_MSG);
    }

    /// update existing style if exists otherwise sets it to the new one
    /// mods will be taken from updating and will replace fg and bg if present
    #[inline]
    fn update_style(&mut self, style: ContentStyle) {
        if let Some(current) = self.default_styled.as_mut() {
            current.update(style);
        } else {
            self.default_styled.replace(style);
        };
        self.to_set_style();
    }

    /// adds foreground to the already set style
    #[inline]
    fn set_fg(&mut self, color: Option<Color>) {
        if let Some(current) = self.default_styled.as_mut() {
            current.set_fg(color);
        } else if let Some(color) = color {
            self.default_styled.replace(ContentStyle::fg(color));
        };
        self.to_set_style()
    }

    /// adds background to the already set style
    #[inline]
    fn set_bg(&mut self, color: Option<Color>) {
        if let Some(current) = self.default_styled.as_mut() {
            current.set_bg(color);
        } else if let Some(color) = color {
            let style = ContentStyle::bg(color);
            self.default_styled.replace(style);
        }
        self.to_set_style();
    }

    /// restores the style of the writer to default
    #[inline]
    fn reset_style(&mut self) {
        self.default_styled = None;
        queue!(self, ResetColor).expect(ERR_MSG);
    }

    /// sends the cursor to location
    #[inline]
    fn go_to(&mut self, row: u16, col: u16) {
        queue!(self, MoveTo(col, row)).expect(ERR_MSG);
    }

    /// direct adding cursor at location - no buffer queing
    #[inline]
    fn render_cursor_at(&mut self, row: u16, col: u16) {
        queue!(self, MoveTo(col, row), Show).expect(ERR_MSG);
    }

    /// direct showing cursor - no buffer queing
    #[inline]
    fn show_cursor() {
        queue!(std::io::stdout(), Show).expect(ERR_MSG);
    }

    /// direct hiding cursor - no buffer queing
    #[inline]
    fn hide_cursor() {
        queue!(std::io::stdout(), Hide).expect(ERR_MSG);
    }

    #[inline]
    fn print<D: Display>(&mut self, text: D) {
        queue!(self, Print(text)).expect(ERR_MSG);
    }

    /// goes to location and prints text
    #[inline]
    fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) {
        queue!(self, MoveTo(col, row), Print(text)).expect(ERR_MSG);
    }

    /// prints styled text without affecting the writer set style
    #[inline]
    fn print_styled<D: Display>(&mut self, text: D, style: ContentStyle) {
        match self.default_styled {
            Some(restore_style) => queue!(self, SetStyle(style), Print(text), ResetColor, SetStyle(restore_style),),
            None => queue!(self, SetStyle(style), Print(text), ResetColor,),
        }
        .expect(ERR_MSG);
    }

    /// goes to location and prints styled text without affecting the writer set style
    #[inline]
    fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: ContentStyle) {
        if let Some(restore_style) = self.default_styled {
            queue!(self, SetStyle(style), MoveTo(col, row), Print(text), ResetColor, SetStyle(restore_style),)
        } else {
            queue!(self, SetStyle(style), MoveTo(col, row), Print(text), ResetColor,)
        }
        .expect(ERR_MSG);
    }

    #[inline]
    fn pad(&mut self, width: usize) {
        queue!(self, Print(format!("{:width$}", ""))).expect(ERR_MSG);
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = Backend::exit();
    }
}

fn init_terminal() -> std::io::Result<()> {
    // Ensures panics are retported
    std::panic::set_hook(Box::new(|info| {
        let _ = graceful_exit();
        eprintln!("{info}");
    }));
    // Init terminal
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::terminal::DisableLineWrap,
        crossterm::style::ResetColor,
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste,
        crossterm::cursor::Hide,
    )
}

fn graceful_exit() -> std::io::Result<()> {
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::terminal::EnableLineWrap,
        crossterm::style::ResetColor,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::Show,
    )?;
    crossterm::terminal::disable_raw_mode()
}

use std::{
    fmt::Display,
    io::{Result, Stdout, Write},
};

use crossterm::{
    cursor::{MoveTo, RestorePosition, SavePosition},
    queue,
    style::{Color, ContentStyle, Print, ResetColor, SetStyle},
    terminal::{Clear, ClearType},
};

type Style = ContentStyle;

/// Thin wrapper around rendering framework, allowing easy switching of backend
/// Add cfg and new implementation of the wrapper to make the backend swichable
/// Main reason is to clear out the issue with PrintStyled on CrossTerm
/// TODO: add termios & wezterm
#[cfg(not(test))]
pub struct Backend {
    writer: Stdout,
    default_styled: Option<Style>,
}

#[cfg(not(test))]
impl Write for Backend {
    #[inline]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }

    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.writer.write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> Result<()> {
        self.writer.write_fmt(fmt)
    }
}

#[cfg(not(test))]
impl Backend {
    #[inline]
    pub fn init() -> Result<Self> {
        init_terminal().map(|_| Self { writer: std::io::stdout(), default_styled: None })
    }

    #[inline]
    pub fn exit() -> Result<()> {
        graceful_exit()
    }

    #[inline]
    pub fn clear_to_eol(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::UntilNewLine))
    }

    #[inline]
    pub fn clear_line(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::CurrentLine))
    }

    #[inline]
    pub fn clear_all(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::All))
    }

    #[inline]
    pub fn save_cursor(&mut self) -> std::io::Result<()> {
        queue!(self, SavePosition)
    }

    #[inline]
    pub fn restore_cursor(&mut self) -> std::io::Result<()> {
        queue!(self, RestorePosition)
    }

    #[inline]
    pub fn set_style(&mut self, style: Style) -> std::io::Result<()> {
        self.default_styled.replace(style);
        queue!(self, SetStyle(style))
    }

    #[inline]
    pub fn set_fg(&mut self, color: Color) -> std::io::Result<()> {
        let mut style = Style::new();
        style.foreground_color = Some(color);
        self.default_styled.replace(style);
        queue!(self, SetStyle(style))
    }

    #[inline]
    pub fn reset_style(&mut self) -> std::io::Result<()> {
        self.default_styled.take();
        queue!(self, ResetColor)
    }

    #[inline]
    pub fn go_to(&mut self, row: u16, col: u16) -> Result<()> {
        queue!(self, MoveTo(col, row))
    }

    #[inline]
    pub fn print<D: Display>(&mut self, text: D) -> Result<()> {
        queue!(self, Print(text))
    }

    #[inline]
    pub fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) -> Result<()> {
        queue!(self, MoveTo(col, row), Print(text))
    }

    #[inline]
    pub fn print_styled<D: Display>(&mut self, text: D, style: Style) -> Result<()> {
        if let Some(restore_style) = self.default_styled {
            queue!(self, SetStyle(style), Print(text), ResetColor, SetStyle(restore_style),)
        } else {
            queue!(self, SetStyle(style), Print(text), ResetColor,)
        }
    }

    #[inline]
    pub fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style) -> Result<()> {
        if let Some(restore_style) = self.default_styled {
            queue!(self, SetStyle(style), MoveTo(col, row), Print(text), ResetColor, SetStyle(restore_style),)
        } else {
            queue!(self, SetStyle(style), MoveTo(col, row), Print(text), ResetColor,)
        }
    }
}

fn init_terminal() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::event::EnableMouseCapture,
    )?;

    // loading panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        graceful_exit().unwrap();
        original_hook(panic);
    }));
    Ok(())
}

fn graceful_exit() -> Result<()> {
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::style::ResetColor,
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::Show,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

#[cfg(test)]
pub struct Backend();

#[cfg(test)]
impl Write for Backend {
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> Result<()> {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> Result<usize> {
        Ok((bufs.len()))
    }
}

#[cfg(test)]
impl Backend {
    pub fn init() -> Result<Self> {
        Ok(Self())
    }

    pub fn exit() -> Result<()> {
        Ok(())
    }

    pub fn clear_to_eol(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn clear_line(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn clear_all(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn save_cursor(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn restore_cursor(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn set_style(&mut self, style: Style) -> std::io::Result<()> {
        Ok(())
    }

    pub fn set_fg(&mut self, color: Color) -> std::io::Result<()> {
        Ok(())
    }

    pub fn reset_style(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    pub fn go_to(&mut self, row: u16, col: u16) -> Result<()> {
        Ok(())
    }

    pub fn print<D: Display>(&mut self, text: D) -> Result<()> {
        Ok(())
    }

    pub fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) -> Result<()> {
        Ok(())
    }

    pub fn print_styled<D: Display>(&mut self, text: D, style: Style) -> Result<()> {
        Ok(())
    }

    pub fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style) -> Result<()> {
        Ok(())
    }
}

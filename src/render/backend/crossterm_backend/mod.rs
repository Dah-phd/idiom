#[cfg(test)]
use test::DummyOut;

pub mod color;
mod style;
use crossterm::{
    cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show},
    execute, queue,
    style::{Color as CTColor, Print, ResetColor, SetStyle},
    terminal::{size, Clear, ClearType},
};
use std::{
    fmt::Display,
    io::{Result, Stdout, Write},
};

pub use style::Style;

use crate::render::layout::Rect;

use super::BackendProtocol;
pub type Color = CTColor;

/// Thin wrapper around rendering framework, allowing easy switching of backend
/// Add cfg and new implementation of the wrapper to make the backend swichable
/// Main reason is to clear out the issue with PrintStyled on CrossTerm
/// TODO: add termios & wezterm
pub struct Backend {
    #[cfg(not(test))]
    writer: Stdout,
    #[cfg(test)]
    writer: DummyOut,
    default_styled: Option<Style>,
}

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

impl BackendProtocol for Backend {
    #[inline]
    fn init() -> Result<Self> {
        #[cfg(test)]
        return Ok(Self { writer: DummyOut {}, default_styled: None });
        #[cfg(not(test))]
        init_terminal().map(|_| Self { writer: std::io::stdout(), default_styled: None })
    }

    #[inline]
    fn exit() -> Result<()> {
        #[cfg(test)]
        return Ok(());
        #[cfg(not(test))]
        graceful_exit()
    }

    /// get whole screen as rect
    #[inline]
    fn screen() -> Result<Rect> {
        size().map(|size| Rect::from(size))
    }

    /// clears from cursor until the End Of Line
    #[inline]
    fn clear_to_eol(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::UntilNewLine))
    }

    /// clears current cursor line
    #[inline]
    fn clear_line(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::CurrentLine))
    }

    #[inline]
    fn clear_all(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::All))
    }

    /// stores the cursor and hides it
    #[inline]
    fn save_cursor(&mut self) -> std::io::Result<()> {
        execute!(self, SavePosition, Hide)
    }

    /// restores cursor position and shows cursor
    #[inline]
    fn restore_cursor(&mut self) -> std::io::Result<()> {
        execute!(self, RestorePosition, Show)
    }

    /// sets the style for the print/print at
    #[inline]
    fn set_style(&mut self, style: Style) -> std::io::Result<()> {
        self.default_styled.replace(style);
        queue!(self, SetStyle(style.into()))
    }

    #[inline]
    fn get_style(&mut self) -> Style {
        self.default_styled.unwrap_or_default()
    }

    #[inline]
    fn to_set_style(&mut self) -> std::io::Result<()> {
        match self.default_styled {
            Some(style) => queue!(self, ResetColor, SetStyle(style.into())),
            None => queue!(self, ResetColor),
        }
    }

    /// update existing style if exists otherwise sets it to the new one
    /// mods will be taken from updating and will replace fg and bg if present
    #[inline]
    fn update_style(&mut self, style: Style) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.update(style);
        } else {
            self.default_styled.replace(style);
        };
        self.to_set_style()
    }

    /// adds foreground to the already set style
    #[inline]
    fn set_fg(&mut self, color: Option<Color>) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.set_fg(color);
            self.to_set_style()
        } else if let Some(color) = color {
            self.default_styled.replace(Style::fg(color));
            self.to_set_style()
        } else {
            Ok(())
        }
    }

    /// adds background to the already set style
    #[inline]
    fn set_bg(&mut self, color: Option<Color>) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.set_bg(color);
            self.to_set_style()
        } else if let Some(color) = color {
            let style = Style::bg(color);
            self.default_styled.replace(style);
            self.to_set_style()
        } else {
            Ok(())
        }
    }

    /// restores the style of the writer to default
    #[inline]
    fn reset_style(&mut self) -> std::io::Result<()> {
        self.default_styled = None;
        queue!(self, ResetColor)
    }

    /// sends the cursor to location
    #[inline]
    fn go_to(&mut self, row: u16, col: u16) -> Result<()> {
        queue!(self, MoveTo(col, row))
    }

    /// direct adding cursor at location - no buffer queing
    #[inline]
    fn render_cursor_at(&mut self, row: u16, col: u16) -> Result<()> {
        execute!(self, MoveTo(col, row), Show)
    }

    /// direct showing cursor - no buffer queing
    #[inline]
    fn show_cursor(&mut self) -> Result<()> {
        execute!(self, Show)
    }

    /// direct hiding cursor - no buffer queing
    #[inline]
    fn hide_cursor(&mut self) -> Result<()> {
        execute!(self, Hide)
    }

    #[inline]
    fn print<D: Display>(&mut self, text: D) -> Result<()> {
        queue!(self, Print(text))
    }

    /// goes to location and prints text
    #[inline]
    fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) -> Result<()> {
        queue!(self, MoveTo(col, row), Print(text))
    }

    /// prints styled text without affecting the writer set style
    #[inline]
    fn print_styled<D: Display>(&mut self, text: D, style: Style) -> Result<()> {
        if let Some(restore_style) = self.default_styled {
            queue!(self, SetStyle(style.into()), Print(text), ResetColor, SetStyle(restore_style.into()),)
        } else {
            queue!(self, SetStyle(style.into()), Print(text), ResetColor,)
        }
    }

    /// goes to location and prints styled text without affecting the writer set style
    #[inline]
    fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style) -> Result<()> {
        if let Some(restore_style) = self.default_styled {
            queue!(
                self,
                SetStyle(style.into()),
                MoveTo(col, row),
                Print(text),
                ResetColor,
                SetStyle(restore_style.into()),
            )
        } else {
            queue!(self, SetStyle(style.into()), MoveTo(col, row), Print(text), ResetColor,)
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
mod test {
    use std::io::{Result, Write};
    pub struct DummyOut {}

    impl Write for DummyOut {
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

        fn write_all(&mut self, _: &[u8]) -> Result<()> {
            Ok(())
        }

        fn write_fmt(&mut self, _: std::fmt::Arguments<'_>) -> Result<()> {
            Ok(())
        }

        fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> Result<usize> {
            Ok(bufs.iter().map(|b| b.len()).sum())
        }
    }
}

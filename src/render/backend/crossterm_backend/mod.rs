#[cfg(test)]
use test::DummyOut;

pub mod color;
mod style;
use crossterm::{
    cursor::{Hide, MoveTo, RestorePosition, SavePosition, Show},
    execute, queue,
    style::{Color as CTColor, Print, ResetColor, SetStyle},
    terminal::{Clear, ClearType},
};
use std::{
    fmt::Display,
    io::{Result, Stdout, Write},
};

pub use style::Style;
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

impl Backend {
    #[inline]
    pub fn init() -> Result<Self> {
        #[cfg(test)]
        return Ok(Self { writer: DummyOut {}, default_styled: None });
        #[cfg(not(test))]
        init_terminal().map(|_| Self { writer: std::io::stdout(), default_styled: None })
    }

    #[inline]
    pub fn exit() -> Result<()> {
        #[cfg(test)]
        return Ok(());
        #[cfg(not(test))]
        graceful_exit()
    }

    /// clears from cursor until the End Of Line
    #[inline]
    pub fn clear_to_eol(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::UntilNewLine))
    }

    /// clears current cursor line
    #[inline]
    pub fn clear_line(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::CurrentLine))
    }

    #[inline]
    pub fn clear_all(&mut self) -> std::io::Result<()> {
        queue!(self, Clear(ClearType::All))
    }

    /// stores the cursor and hides it
    #[inline]
    pub fn save_cursor(&mut self) -> std::io::Result<()> {
        execute!(self, SavePosition, Hide)
    }

    /// restores cursor position and shows cursor
    #[inline]
    pub fn restore_cursor(&mut self) -> std::io::Result<()> {
        execute!(self, RestorePosition, Show)
    }

    /// sets the style for the print/print at
    #[inline]
    pub fn set_style(&mut self, style: Style) -> std::io::Result<()> {
        self.default_styled.replace(style);
        queue!(self, SetStyle(style.into()))
    }

    #[inline]
    pub fn get_style(&mut self) -> Style {
        self.default_styled.unwrap_or_default()
    }

    #[inline]
    pub fn to_set_style(&mut self) -> std::io::Result<()> {
        match self.default_styled {
            Some(style) => queue!(self, ResetColor, SetStyle(style.into())),
            None => queue!(self, ResetColor),
        }
    }

    /// update existing style if exists otherwise sets it to the new one
    /// mods will be taken from updating and will replace fg and bg if present
    #[inline]
    pub fn update_style(&mut self, style: Style) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.update(style);
        } else {
            self.default_styled.replace(style);
        };
        self.to_set_style()
    }

    /// adds foreground to the already set style
    #[inline]
    pub fn add_fg(&mut self, color: Color) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.add_fg(color);
        } else {
            self.default_styled.replace(Style::fg(color));
        };
        self.to_set_style()
    }

    /// drops foreground to the already set style
    #[inline]
    pub fn drop_fg(&mut self) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.drop_fg();
        };
        self.to_set_style()
    }

    /// adds background to the already set style
    #[inline]
    pub fn add_bg(&mut self, color: Color) -> std::io::Result<()> {
        if let Some(current) = self.default_styled.as_mut() {
            current.add_bg(color);
        } else {
            let style = Style::bg(color);
            self.default_styled.replace(style);
        };
        self.to_set_style()
    }

    /// drops background to the already set style
    #[inline]
    pub fn drop_bg(&mut self) -> std::io::Result<()> {
        if let Some(style) = self.default_styled.as_mut() {
            style.drop_bg();
        };
        self.to_set_style()
    }

    /// restores the style of the writer to default
    #[inline]
    pub fn reset_style(&mut self) -> std::io::Result<()> {
        self.default_styled = None;
        queue!(self, ResetColor)
    }

    /// sends the cursor to location
    #[inline]
    pub fn go_to(&mut self, row: u16, col: u16) -> Result<()> {
        queue!(self, MoveTo(col, row))
    }

    #[inline]
    pub fn print<D: Display>(&mut self, text: D) -> Result<()> {
        queue!(self, Print(text))
    }

    /// goes to location and prints text
    #[inline]
    pub fn print_at<D: Display>(&mut self, row: u16, col: u16, text: D) -> Result<()> {
        queue!(self, MoveTo(col, row), Print(text))
    }

    /// prints styled text without affecting the writer set style
    #[inline]
    pub fn print_styled<D: Display>(&mut self, text: D, style: Style) -> Result<()> {
        if let Some(restore_style) = self.default_styled {
            queue!(self, SetStyle(style.into()), Print(text), ResetColor, SetStyle(restore_style.into()),)
        } else {
            queue!(self, SetStyle(style.into()), Print(text), ResetColor,)
        }
    }

    /// goes to location and prints styled text without affecting the writer set style
    #[inline]
    pub fn print_styled_at<D: Display>(&mut self, row: u16, col: u16, text: D, style: Style) -> Result<()> {
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

use super::Color;
use crossterm::style::Color as CTColor;

#[inline]
pub const fn reset() -> Color {
    CTColor::Reset
}

#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    CTColor::Rgb { r, g, b }
}

#[inline]
pub const fn ansi(val: u8) -> Color {
    CTColor::AnsiValue(val)
}

#[inline]
pub fn parse_ansi(code: &str) -> Option<Color> {
    CTColor::parse_ansi(code)
}

#[inline]
pub const fn red() -> Color {
    CTColor::Red
}

#[inline]
pub const fn dark_red() -> Color {
    CTColor::DarkRed
}

#[inline]
pub const fn yellow() -> Color {
    CTColor::Yellow
}

#[inline]
pub const fn dark_yellow() -> Color {
    CTColor::DarkYellow
}

#[inline]
pub const fn cyan() -> Color {
    CTColor::Cyan
}

#[inline]
pub const fn dark_cyan() -> Color {
    CTColor::DarkCyan
}

#[inline]
pub const fn magenta() -> Color {
    CTColor::Magenta
}

#[inline]
pub const fn dark_magenta() -> Color {
    CTColor::DarkMagenta
}

#[inline]
pub const fn grey() -> Color {
    CTColor::Grey
}

#[inline]
pub const fn dark_grey() -> Color {
    CTColor::DarkGrey
}

#[inline]
pub const fn black() -> Color {
    CTColor::Black
}

#[inline]
pub const fn white() -> Color {
    CTColor::White
}

#[inline]
pub const fn green() -> Color {
    CTColor::Green
}

#[inline]
pub const fn dark_green() -> Color {
    CTColor::DarkGreen
}

#[inline]
pub const fn blue() -> Color {
    CTColor::Blue
}

#[inline]
pub const fn dark_blue() -> Color {
    CTColor::DarkBlue
}

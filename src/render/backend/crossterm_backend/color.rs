use std::collections::HashMap;

use super::Color;
use crossterm::style::Color as CTColor;
use serde_json::{Map, Value};

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

#[allow(dead_code)]
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

pub fn serialize_rgb(r: u8, g: u8, b: u8) -> HashMap<&'static str, [u8; 3]> {
    let mut rgb = HashMap::new();
    rgb.insert("rgb", [r, g, b]);
    rgb
}

#[inline]
pub fn pull_color(map: &mut Map<String, Value>, key: &str) -> Option<Result<Color, String>> {
    map.remove(key).map(|obj| parse_color(obj))
}

pub fn parse_color(obj: Value) -> Result<Color, String> {
    match obj {
        Value::String(data) => from_str(&data).map_err(|e| e.to_string()),
        Value::Object(map) => {
            if let Some(data) = map.get("rgb").or(map.get("Rgb").or(map.get("RGB"))) {
                if let Value::Array(rgb_value) = data {
                    if rgb_value.len() == 3 {
                        let b = object_to_u8(rgb_value[2].clone()).ok_or("Failed to parse B in RGB color")?;
                        let g = object_to_u8(rgb_value[1].clone()).ok_or("Failed to parse G in RGB color")?;
                        let r = object_to_u8(rgb_value[0].clone()).ok_or("Failed to parse R in RGB color")?;
                        return Ok(rgb(r, g, b));
                    }
                }
            };
            Err(String::from("When representing Color as Object(Map) - should be {\"rgb\": [number, number, number]}!"))
        }
        _ => Err(String::from("Color definition should be String or Object!")),
    }
}

pub fn object_to_u8(obj: Value) -> Option<u8> {
    match obj {
        Value::Number(num) => Some(num.as_u64()? as u8),
        Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

fn from_str(s: &str) -> Result<Color, ParseColorError> {
    Ok(
        // There is a mix of different color names and formats in the wild.
        // This is an attempt to support as many as possible.
        match s
            .to_lowercase()
            .replace([' ', '-', '_'], "")
            .replace("bright", "light")
            .replace("grey", "gray")
            .replace("silver", "gray")
            .replace("lightblack", "darkgray")
            .replace("lightwhite", "white")
            .replace("lightgray", "white")
            .as_ref()
        {
            "reset" => reset(),
            "black" => black(),
            "red" => dark_red(),
            "lightred" => red(),
            "green" => dark_green(),
            "lightgreen" => green(),
            "yellow" => dark_yellow(),
            "lightyellow" => yellow(),
            "blue" => dark_blue(),
            "lightblue" => blue(),
            "magenta" => dark_magenta(),
            "lightmagenta" => magenta(),
            "cyan" => dark_cyan(),
            "lightcyan" => cyan(),
            "gray" => grey(),
            "darkgray" => dark_grey(),
            "white" => white(),
            _ => {
                if let Ok(index) = s.parse::<u8>() {
                    ansi(index)
                } else if let (Ok(r), Ok(g), Ok(b)) = {
                    if !s.starts_with('#') || s.len() != 7 {
                        return Err(ParseColorError);
                    }
                    (
                        u8::from_str_radix(&s[1..3], 16),
                        u8::from_str_radix(&s[3..5], 16),
                        u8::from_str_radix(&s[5..7], 16),
                    )
                } {
                    rgb(r, g, b)
                } else {
                    return Err(ParseColorError);
                }
            }
        },
    )
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ParseColorError;

impl std::fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse Colors")
    }
}

impl std::error::Error for ParseColorError {}

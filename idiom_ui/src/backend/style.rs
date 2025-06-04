use crossterm::style::{Attribute, Attributes, Color, ContentStyle};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[allow(dead_code)]
pub trait StyleExt: Sized {
    fn update(&mut self, rhs: Self);
    fn set_attr(&mut self, attr: Attribute);
    fn unset_attr(&mut self, attr: Attribute);
    fn with_fg(self, color: Color) -> Self;
    fn set_fg(&mut self, color: Option<Color>);
    fn fg(color: Color) -> Self;
    fn with_bg(self, color: Color) -> Self;
    fn set_bg(&mut self, color: Option<Color>);
    fn bg(color: Color) -> Self;
    fn drop_bg(&mut self);
    fn add_slowblink(&mut self);
    fn slowblink() -> Self;
    fn add_bold(&mut self);
    fn bold() -> Self;
    fn add_ital(&mut self);
    fn ital() -> Self;
    fn add_reverse(&mut self);
    fn reversed() -> Self;
    fn reset_mods(&mut self);
    fn undercurle(&mut self, color: Option<Color>);
    fn undercurled(color: Option<Color>) -> Self;
    fn underline(&mut self, color: Option<Color>);
    fn underlined(color: Option<Color>) -> Self;
}

impl StyleExt for ContentStyle {
    #[inline]
    fn update(&mut self, rhs: Self) {
        if let Some(c) = rhs.foreground_color {
            self.foreground_color.replace(c);
        }
        if let Some(c) = rhs.background_color {
            self.background_color.replace(c);
        }
        if let Some(c) = rhs.underline_color {
            self.underline_color.replace(c);
        }
        self.attributes = rhs.attributes;
    }

    fn set_attr(&mut self, attr: Attribute) {
        self.attributes.set(attr);
    }

    fn unset_attr(&mut self, attr: Attribute) {
        self.attributes.unset(attr);
    }

    #[inline]
    fn with_fg(mut self, color: Color) -> Self {
        self.foreground_color = Some(color);
        self
    }

    #[inline]
    fn set_fg(&mut self, color: Option<Color>) {
        self.foreground_color = color;
    }

    #[inline]
    fn fg(color: Color) -> Self {
        ContentStyle {
            foreground_color: Some(color),
            background_color: None,
            underline_color: None,
            attributes: Attributes::default(),
        }
    }

    #[inline]
    fn with_bg(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[inline]
    fn set_bg(&mut self, color: Option<Color>) {
        self.background_color = color;
    }

    #[inline]
    fn bg(color: Color) -> Self {
        ContentStyle {
            background_color: Some(color),
            foreground_color: None,
            underline_color: None,
            attributes: Attributes::default(),
        }
    }

    #[inline]
    fn drop_bg(&mut self) {
        self.background_color = None;
    }

    #[inline]
    fn add_slowblink(&mut self) {
        self.attributes.set(Attribute::SlowBlink);
    }

    #[inline]
    fn slowblink() -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::SlowBlink.into(),
        }
    }

    #[inline]
    fn add_bold(&mut self) {
        self.attributes.set(Attribute::Bold);
    }

    #[inline]
    fn bold() -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::Bold.into(),
        }
    }

    #[inline]
    fn add_ital(&mut self) {
        self.attributes.set(Attribute::Italic);
    }

    #[inline]
    fn ital() -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::Italic.into(),
        }
    }

    #[inline]
    fn add_reverse(&mut self) {
        self.attributes.set(Attribute::Reverse);
    }

    #[inline]
    fn reversed() -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::Reverse.into(),
        }
    }

    #[inline]
    fn reset_mods(&mut self) {
        self.attributes = Attributes::default();
        self.underline_color = None;
    }

    #[inline]
    fn undercurle(&mut self, color: Option<Color>) {
        self.attributes.set(Attribute::Undercurled);
        self.underline_color = color;
    }

    #[inline]
    fn undercurled(color: Option<Color>) -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: color,
            attributes: Attribute::Undercurled.into(),
        }
    }

    #[inline]
    fn underline(&mut self, color: Option<Color>) {
        self.attributes.set(Attribute::Underlined);
        self.underline_color = color;
    }

    #[inline]
    fn underlined(color: Option<Color>) -> Self {
        ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: color,
            attributes: Attribute::Underlined.into(),
        }
    }
}

#[cfg(not(test))]
#[inline]
pub fn background_rgb() -> Option<(u8, u8, u8)> {
    #[cfg(unix)]
    if let Some(result) = query_bg_color() {
        return Some(result);
    }
    env_rgb_color()
}

#[cfg(test)]
pub fn background_rgb() -> Option<(u8, u8, u8)> {
    None
}

#[allow(dead_code)] // test setup causes the function to be detected as unused
#[cfg(unix)]
fn query_bg_color() -> Option<(u8, u8, u8)> {
    let s = xterm_query::query_osc("\x1b]11;?\x07", 100_u16).ok()?;
    match s.strip_prefix("]11;rgb:") {
        Some(raw_color) if raw_color.len() >= 14 => Some((
            u8::from_str_radix(&raw_color[0..2], 16).ok()?,
            u8::from_str_radix(&raw_color[5..7], 16).ok()?,
            u8::from_str_radix(&raw_color[10..12], 16).ok()?,
        )),
        _ => None,
    }
}

#[allow(dead_code)] // test setup causes the function to be detected as unused
fn env_rgb_color() -> Option<(u8, u8, u8)> {
    let color_config = std::env::var("COLORFGBG").ok()?;
    let token: Vec<&str> = color_config.split(';').collect();
    let bg = match token.len() {
        2 => token[1],
        3 => token[2],
        _ => {
            return None;
        }
    };
    let code = bg.parse().ok()?;
    let coolor::Rgb { r, g, b } = coolor::AnsiColor { code }.to_rgb();
    Some((r, g, b))
}

pub fn serialize_rgb(r: u8, g: u8, b: u8) -> HashMap<&'static str, [u8; 3]> {
    let mut rgb = HashMap::new();
    rgb.insert("rgb", [r, g, b]);
    rgb
}

#[inline]
pub fn pull_color(map: &mut Map<String, Value>, key: &str) -> Option<Result<Color, String>> {
    map.remove(key).map(parse_color)
}

pub fn parse_color(obj: Value) -> Result<Color, String> {
    match obj {
        Value::String(data) => from_str(&data).map_err(|e| e.to_string()),
        Value::Object(map) => {
            if let Some(Value::Array(rgb_value)) = map.get("rgb").or(map.get("Rgb").or(map.get("RGB"))) {
                if rgb_value.len() == 3 {
                    let b = object_to_u8(rgb_value[2].clone()).ok_or("Failed to parse B in RGB color")?;
                    let g = object_to_u8(rgb_value[1].clone()).ok_or("Failed to parse G in RGB color")?;
                    let r = object_to_u8(rgb_value[0].clone()).ok_or("Failed to parse R in RGB color")?;
                    return Ok(Color::Rgb { r, g, b });
                }
            };
            Err(String::from("When representing Color as Object(Map) - should be {\"rgb\": [number, number, number]}!"))
        }
        _ => Err(String::from("Color definition should be String or Object!")),
    }
}

pub fn parse_raw_rgb(map: Value) -> Result<(u8, u8, u8), String> {
    if let Some(Value::Array(rgb_value)) = map.get("rgb").or(map.get("Rgb").or(map.get("RGB"))) {
        if rgb_value.len() == 3 {
            let b = object_to_u8(rgb_value[2].clone()).ok_or("Failed to parse B in RGB color")?;
            let g = object_to_u8(rgb_value[1].clone()).ok_or("Failed to parse G in RGB color")?;
            let r = object_to_u8(rgb_value[0].clone()).ok_or("Failed to parse R in RGB color")?;
            return Ok((r, g, b));
        }
    };
    Err(String::from("When representing Color as Object(Map) - should be {\"rgb\": [number, number, number]}!"))
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
            "reset" => Color::Reset,
            "black" => Color::Black,
            "red" => Color::DarkRed,
            "lightred" => Color::Red,
            "green" => Color::DarkGreen,
            "lightgreen" => Color::Green,
            "yellow" => Color::DarkYellow,
            "lightyellow" => Color::Yellow,
            "blue" => Color::DarkBlue,
            "lightblue" => Color::Blue,
            "magenta" => Color::DarkMagenta,
            "lightmagenta" => Color::Magenta,
            "cyan" => Color::DarkCyan,
            "lightcyan" => Color::Cyan,
            "gray" => Color::Grey,
            "darkgray" => Color::DarkGrey,
            "white" => Color::White,
            _ => {
                if let Ok(index) = s.parse::<u8>() {
                    Color::AnsiValue(index)
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
                    Color::Rgb { r, g, b }
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

#[cfg(test)]
mod tests {}

use std::collections::HashMap;

use crate::configs::{load_or_create_config, THEME_UI};
use crate::error::IdiomError;
use crate::render::backend::{color, Color, Style};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct UITheme {
    pub accent_background: Color,
    pub accent_style: Style,
}

impl<'de> serde::Deserialize<'de> for UITheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Object(mut map) => {
                let accent_background = pull_color(&mut map, "accent").map_err(serde::de::Error::custom)?;
                Ok(Self { accent_style: Style::bg(accent_background), accent_background })
            }
            _ => Err(IdiomError::io_err("theme_ui.json in not an Object!")).map_err(serde::de::Error::custom),
        }
    }
}

impl Serialize for UITheme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("UITheme", 1)?;
        s.serialize_field("accent", &serialize_rgb(25, 25, 24))?;
        s.end()
    }
}

impl Default for UITheme {
    fn default() -> Self {
        let accent_background = color::rgb(25, 25, 24);
        Self { accent_style: Style::bg(accent_background), accent_background }
    }
}

impl UITheme {
    pub fn new() -> Result<Self, serde_json::Error> {
        load_or_create_config(THEME_UI)
    }
}

pub fn serialize_rgb(r: u8, g: u8, b: u8) -> HashMap<&'static str, [u8; 3]> {
    let mut rgb = HashMap::new();
    rgb.insert("rgb", [r, g, b]);
    rgb
}

pub fn pull_color(map: &mut Map<String, Value>, key: &str) -> Result<Color, String> {
    match map.remove(key) {
        Some(obj) => parse_color(obj),
        None => Err(format!("Key not in object {key}")),
    }
}

pub fn parse_color(obj: Value) -> Result<Color, String> {
    match obj {
        Value::String(data) => from_str(&data).map_err(|e| e.to_string()),
        Value::Object(map) => {
            if let Some(data) = map.get("rgb").or(map.get("Rgb").or(map.get("RGB"))) {
                if let Value::Array(rgb) = data {
                    if rgb.len() == 3 {
                        let b = object_to_u8(rgb[2].clone()).ok_or("Failed to parse B in RGB color")?;
                        let g = object_to_u8(rgb[1].clone()).ok_or("Failed to parse G in RGB color")?;
                        let r = object_to_u8(rgb[0].clone()).ok_or("Failed to parse R in RGB color")?;
                        return Ok(color::rgb(r, g, b));
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
            "reset" => color::reset(),
            "black" => color::black(),
            "red" => color::dark_red(),
            "lightred" => color::red(),
            "green" => color::dark_green(),
            "lightgreen" => color::green(),
            "yellow" => color::dark_yellow(),
            "lightyellow" => color::yellow(),
            "blue" => color::dark_blue(),
            "lightblue" => color::blue(),
            "magenta" => color::dark_magenta(),
            "lightmagenta" => color::magenta(),
            "cyan" => color::dark_cyan(),
            "lightcyan" => color::cyan(),
            "gray" => color::grey(),
            "darkgray" => color::dark_grey(),
            "white" => color::white(),
            _ => {
                if let Ok(index) = s.parse::<u8>() {
                    color::ansi(index)
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
                    color::rgb(r, g, b)
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

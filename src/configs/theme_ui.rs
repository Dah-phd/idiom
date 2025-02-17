use super::{load_or_create_config, THEME_UI};
use crate::error::IdiomError;
use crate::render::backend::{background_rgb, parse_raw_rgb, serialize_rgb, StyleExt};
use crossterm::style::{Color, ContentStyle};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::Value;

const ACCENT_OFFSET: u8 = 24;
const ACCENT_KEY: &str = "accent_offset";

fn offset_color_part(base: u8, offset: u8) -> u8 {
    match base < 50 {
        true => base.saturating_add(offset),
        false => base.saturating_sub(offset),
    }
}

#[derive(Debug)]
pub struct UITheme {
    pub accent_background: Color,
    pub accent_style: ContentStyle,
}

impl<'de> serde::Deserialize<'de> for UITheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Object(mut map) => {
                let (r_offset, g_offset, b_offset) =
                    match map.remove(ACCENT_KEY).or(map.remove("accent")).map(parse_raw_rgb) {
                        Some(Ok(result)) => result,
                        Some(Err(msg)) => return Err(serde::de::Error::custom(msg)),
                        None => (ACCENT_OFFSET, ACCENT_OFFSET, ACCENT_OFFSET),
                    };
                let accent_background = match background_rgb() {
                    Some((r, g, b)) => Color::Rgb {
                        r: offset_color_part(r, r_offset),
                        g: offset_color_part(g, g_offset),
                        b: offset_color_part(b, b_offset),
                    },
                    // assume pitch black
                    None => Color::Rgb { r: ACCENT_OFFSET, g: ACCENT_OFFSET, b: ACCENT_OFFSET },
                };
                Ok(Self { accent_style: ContentStyle::bg(accent_background), accent_background })
            }
            _ => Err(serde::de::Error::custom(IdiomError::any("theme_ui.toml in not an Object!"))),
        }
    }
}

impl Serialize for UITheme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("UITheme", 1)?;
        s.serialize_field(ACCENT_KEY, &serialize_rgb(ACCENT_OFFSET, ACCENT_OFFSET, ACCENT_OFFSET))?;
        s.end()
    }
}

impl Default for UITheme {
    fn default() -> Self {
        let accent_background = match background_rgb() {
            Some((r, g, b)) => Color::Rgb {
                r: offset_color_part(r, ACCENT_OFFSET),
                g: offset_color_part(g, ACCENT_OFFSET),
                b: offset_color_part(b, ACCENT_OFFSET),
            },
            // assume pitch black
            None => Color::Rgb { r: ACCENT_OFFSET, g: ACCENT_OFFSET, b: ACCENT_OFFSET },
        };
        Self { accent_style: ContentStyle::bg(accent_background), accent_background }
    }
}

impl UITheme {
    pub fn new() -> Result<Self, toml::de::Error> {
        load_or_create_config(THEME_UI)
    }
}

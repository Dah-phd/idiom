use super::{load_or_create_config, THEME_UI};
use crate::error::IdiomError;
use crate::ext_tui::{background_rgb, parse_raw_rgb, pull_color, serialize_rgb, StyleExt};
use crossterm::style::{Color, ContentStyle, Stylize};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::{Map, Value};

const ACCENT_OFFSET: u8 = 40;
const ACCENT_SELECT_OFFSET: u8 = 80;
const ACCENT_KEY: &str = "accent";
const ACCENT_SELECT_KEY: &str = "accent_select";
const ACCENT_KEY_OFFSET: &str = "accent_offset";
const ACCENT_SELECT_KEY_OFFSET: &str = "accent_select_offset";

fn offset_color_part(base: u8, offset: u8) -> u8 {
    match base < 80 {
        true => base.saturating_add(offset),
        false => base.saturating_sub(offset),
    }
}

fn find_color_or_offset(
    map: &mut Map<String, Value>,
    color_key: &str,
    color_offset_key: &str,
    offset_default: u8,
) -> Result<Color, String> {
    if let Some(color) = pull_color(map, color_key) {
        return color;
    };
    let (r_offset, g_offset, b_offset) = match map.remove(color_offset_key).map(parse_raw_rgb) {
        Some(Ok(result)) => result,
        Some(Err(msg)) => return Err(msg),
        None => (offset_default, offset_default, offset_default),
    };
    match background_rgb() {
        Some((r, g, b)) => Ok(Color::Rgb {
            r: offset_color_part(r, r_offset),
            g: offset_color_part(g, g_offset),
            b: offset_color_part(b, b_offset),
        }),
        // assume pitch black
        None => Ok(Color::Rgb { r: r_offset, g: g_offset, b: b_offset }),
    }
}

#[derive(Debug)]
pub struct UITheme {
    accent: Color,
    accent_style_rev: ContentStyle,
    accent_style: ContentStyle,
    accent_fg: ContentStyle,
    accent_select: Color,
    accent_select_style: ContentStyle,
}

impl<'de> serde::Deserialize<'de> for UITheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Object(mut map) => {
                let accent = find_color_or_offset(&mut map, ACCENT_KEY, ACCENT_KEY_OFFSET, ACCENT_OFFSET)
                    .map_err(serde::de::Error::custom)?;
                let accent_select =
                    find_color_or_offset(&mut map, ACCENT_SELECT_KEY, ACCENT_SELECT_KEY_OFFSET, ACCENT_SELECT_OFFSET)
                        .map_err(serde::de::Error::custom)?;
                Ok(Self {
                    accent_style: ContentStyle::bg(accent),
                    accent_style_rev: ContentStyle::bg(accent).reverse(),
                    accent_fg: ContentStyle::fg(accent),
                    accent,
                    accent_select_style: ContentStyle::bg(accent_select),
                    accent_select,
                })
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
        let (accent, accent_select) = match background_rgb() {
            Some((r, g, b)) => (
                Color::Rgb {
                    r: offset_color_part(r, ACCENT_OFFSET),
                    g: offset_color_part(g, ACCENT_OFFSET),
                    b: offset_color_part(b, ACCENT_OFFSET),
                },
                Color::Rgb {
                    r: offset_color_part(r, ACCENT_SELECT_OFFSET),
                    g: offset_color_part(r, ACCENT_SELECT_OFFSET),
                    b: offset_color_part(r, ACCENT_SELECT_OFFSET),
                },
            ),
            // assume pitch black
            None => (
                Color::Rgb { r: ACCENT_OFFSET, g: ACCENT_OFFSET, b: ACCENT_OFFSET },
                Color::Rgb { r: ACCENT_SELECT_OFFSET, g: ACCENT_SELECT_OFFSET, b: ACCENT_SELECT_OFFSET },
            ),
        };
        Self {
            accent_style: ContentStyle::bg(accent),
            accent_style_rev: ContentStyle::bg(accent).reverse(),
            accent_fg: ContentStyle::fg(accent),
            accent,
            accent_select_style: ContentStyle::bg(accent_select),
            accent_select,
        }
    }
}

impl UITheme {
    pub fn new() -> Result<Self, toml::de::Error> {
        #[cfg(test)]
        return Ok(Self::default());
        #[allow(unreachable_code)]
        load_or_create_config(THEME_UI)
    }

    pub fn accent(&self) -> Color {
        self.accent
    }

    pub fn accent_select(&self) -> Color {
        self.accent_select
    }

    pub fn accent_fg(&self) -> ContentStyle {
        self.accent_fg
    }

    pub fn accent_style(&self) -> ContentStyle {
        self.accent_style
    }

    pub fn accent_select_style(&self) -> ContentStyle {
        self.accent_select_style
    }

    pub fn accent_style_reversed(&self) -> ContentStyle {
        self.accent_style_rev
    }
}

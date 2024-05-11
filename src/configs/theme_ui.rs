use crate::configs::{load_or_create_config, THEME_UI};
use crate::error::IdiomError;
use crate::render::backend::{color, pull_color, serialize_rgb, Color, Style};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::Value;

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

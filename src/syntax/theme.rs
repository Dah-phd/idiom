use crate::configs::{load_or_create_config, pull_color, THEME_FILE};
use ratatui::style::Color;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub imports: Color,
    pub key_words: Color,
    pub flow_control: Color,
    pub class_or_struct: Color,
    pub constant: Color,
    pub functions: Color,
    pub blank: Color,
    pub numeric: Color,
    pub default: Color,
    pub selected: Color,
    pub string: Color,
    pub string_escape: Color,
    pub comment: Color,
}

impl<'de> serde::Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Object(mut map) => Ok(Self {
                imports: pull_color(&mut map, "imports").map_err(serde::de::Error::custom)?,
                key_words: pull_color(&mut map, "keyWords").map_err(serde::de::Error::custom)?,
                flow_control: pull_color(&mut map, "flowControl").map_err(serde::de::Error::custom)?,
                class_or_struct: pull_color(&mut map, "classOrStruct").map_err(serde::de::Error::custom)?,
                constant: pull_color(&mut map, "constant").map_err(serde::de::Error::custom)?,
                blank: pull_color(&mut map, "blank").map_err(serde::de::Error::custom)?,
                comment: pull_color(&mut map, "comment").map_err(serde::de::Error::custom)?,
                default: pull_color(&mut map, "default").map_err(serde::de::Error::custom)?,
                functions: pull_color(&mut map, "functions").map_err(serde::de::Error::custom)?,
                numeric: pull_color(&mut map, "numeric").map_err(serde::de::Error::custom)?,
                selected: pull_color(&mut map, "selected").map_err(serde::de::Error::custom)?,
                string: pull_color(&mut map, "string").map_err(serde::de::Error::custom)?,
                string_escape: pull_color(&mut map, "stringEscape").map_err(serde::de::Error::custom)?,
            }),
            _ => Err(anyhow::anyhow!("theme.json in not an Object!")).map_err(serde::de::Error::custom),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            imports: Color::Rgb(112, 199, 176),
            key_words: Color::Rgb(79, 106, 214),
            numeric: Color::Rgb(153, 173, 142),
            flow_control: Color::LightMagenta,
            class_or_struct: Color::Rgb(112, 199, 176),
            constant: Color::Rgb(73, 162, 215),
            default: Color::Rgb(157, 221, 254),
            functions: Color::Rgb(218, 223, 170),
            blank: Color::Reset,
            selected: Color::Rgb(72, 72, 72),
            string: Color::Yellow,
            string_escape: Color::LightYellow,
            comment: Color::Rgb(82, 113, 67),
        }
    }
}

impl Theme {
    pub fn new() -> Result<Self, serde_json::Error> {
        load_or_create_config(THEME_FILE)
    }
}

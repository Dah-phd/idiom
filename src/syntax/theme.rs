use crate::configs::{load_or_create_config, pull_color, serialize_rgb, THEME_FILE};
use crate::error::IdiomError;
use crate::render::backend::{color, Color};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::Value;

#[derive(Debug, Clone)]
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

impl Serialize for Theme {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Theme", 13)?;
        s.serialize_field("imports", &serialize_rgb(112, 199, 176))?;
        s.serialize_field("keyWords", &serialize_rgb(79, 106, 214))?;
        s.serialize_field("flowControl", "lightmagenta")?;
        s.serialize_field("classOrStruct", &serialize_rgb(112, 199, 176))?;
        s.serialize_field("constant", &serialize_rgb(73, 162, 215))?;
        s.serialize_field("blank", "reset")?;
        s.serialize_field("comment", &serialize_rgb(82, 113, 67))?;
        s.serialize_field("default", &serialize_rgb(157, 221, 254))?;
        s.serialize_field("functions", &serialize_rgb(218, 223, 170))?;
        s.serialize_field("numeric", &serialize_rgb(153, 173, 142))?;
        s.serialize_field("selected", &serialize_rgb(72, 72, 72))?;
        s.serialize_field("string", "yellow")?;
        s.serialize_field("stringEscape", "lightyellow")?;
        s.end()
    }
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
            _ => Err(IdiomError::io_err("theme.json in not an Object!")).map_err(serde::de::Error::custom),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            imports: color::rgb(112, 199, 176),
            key_words: color::rgb(79, 106, 214),
            flow_control: color::magenta(),
            class_or_struct: color::rgb(112, 199, 176),
            constant: color::rgb(73, 162, 215),
            blank: color::reset(),
            comment: color::rgb(82, 113, 67),
            default: color::rgb(157, 221, 254),
            functions: color::rgb(218, 223, 170),
            numeric: color::rgb(153, 173, 142),
            selected: color::rgb(72, 72, 72),
            string: color::dark_yellow(),
            string_escape: color::yellow(),
        }
    }
}

impl Theme {
    pub fn new() -> Result<Self, serde_json::Error> {
        load_or_create_config(THEME_FILE)
    }
}

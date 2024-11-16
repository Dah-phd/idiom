use super::{load_or_create_config, THEME_FILE};
use crate::error::IdiomError;
use crate::render::backend::{color, pull_color, serialize_rgb, Color};
use serde::ser::{Serialize, SerializeStruct};
use serde_json::Value;

const IMPORTS: Color = color::rgb(112, 199, 176);
const KEY_WORDS: Color = color::rgb(79, 106, 214);
const FLOW_CONTROL: Color = color::magenta();
const CLASS_OR_STRUCT: Color = color::rgb(112, 199, 176);
const CONSTANT: Color = color::rgb(73, 162, 215);
const BLANK: Color = color::reset();
const COMMENT: Color = color::rgb(82, 113, 67);
const DEFAULT: Color = color::rgb(157, 221, 254);
const FUNCTIONS: Color = color::rgb(218, 223, 170);
const NUMERIC: Color = color::rgb(153, 173, 142);
const SELECTED: Color = color::rgb(72, 72, 72);
const STRING: Color = color::dark_yellow();
const STRING_ESCAPE: Color = color::yellow();

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
        s.serialize_field("key_words", &serialize_rgb(79, 106, 214))?;
        s.serialize_field("flow_control", "lightmagenta")?;
        s.serialize_field("class_or_struct", &serialize_rgb(112, 199, 176))?;
        s.serialize_field("constant", &serialize_rgb(73, 162, 215))?;
        s.serialize_field("blank", "reset")?;
        s.serialize_field("comment", &serialize_rgb(82, 113, 67))?;
        s.serialize_field("default", &serialize_rgb(157, 221, 254))?;
        s.serialize_field("functions", &serialize_rgb(218, 223, 170))?;
        s.serialize_field("numeric", &serialize_rgb(153, 173, 142))?;
        s.serialize_field("selected", &serialize_rgb(72, 72, 72))?;
        s.serialize_field("string", "yellow")?;
        s.serialize_field("string_escape", "lightyellow")?;
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
                imports: pull_color(&mut map, "imports").unwrap_or(Ok(IMPORTS)).map_err(serde::de::Error::custom)?,
                key_words: pull_color(&mut map, "key_words")
                    .unwrap_or(Ok(KEY_WORDS))
                    .map_err(serde::de::Error::custom)?,
                flow_control: pull_color(&mut map, "flow_control")
                    .unwrap_or(Ok(FLOW_CONTROL))
                    .map_err(serde::de::Error::custom)?,
                class_or_struct: pull_color(&mut map, "class_or_struct")
                    .unwrap_or(Ok(CLASS_OR_STRUCT))
                    .map_err(serde::de::Error::custom)?,
                constant: pull_color(&mut map, "constant").unwrap_or(Ok(CONSTANT)).map_err(serde::de::Error::custom)?,
                blank: pull_color(&mut map, "blank").unwrap_or(Ok(BLANK)).map_err(serde::de::Error::custom)?,
                comment: pull_color(&mut map, "comment").unwrap_or(Ok(COMMENT)).map_err(serde::de::Error::custom)?,
                default: pull_color(&mut map, "default").unwrap_or(Ok(DEFAULT)).map_err(serde::de::Error::custom)?,
                functions: pull_color(&mut map, "functions")
                    .unwrap_or(Ok(FUNCTIONS))
                    .map_err(serde::de::Error::custom)?,
                numeric: pull_color(&mut map, "numeric").unwrap_or(Ok(NUMERIC)).map_err(serde::de::Error::custom)?,
                selected: pull_color(&mut map, "selected").unwrap_or(Ok(SELECTED)).map_err(serde::de::Error::custom)?,
                string: pull_color(&mut map, "string").unwrap_or(Ok(STRING)).map_err(serde::de::Error::custom)?,
                string_escape: pull_color(&mut map, "string_escape")
                    .unwrap_or(Ok(STRING_ESCAPE))
                    .map_err(serde::de::Error::custom)?,
            }),
            _ => Err(serde::de::Error::custom(IdiomError::any("theme.json in not an Object!"))),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            imports: IMPORTS,
            key_words: KEY_WORDS,
            flow_control: FLOW_CONTROL,
            class_or_struct: CLASS_OR_STRUCT,
            constant: CONSTANT,
            blank: BLANK,
            comment: COMMENT,
            default: DEFAULT,
            functions: FUNCTIONS,
            numeric: NUMERIC,
            selected: SELECTED,
            string: STRING,
            string_escape: STRING_ESCAPE,
        }
    }
}

impl Theme {
    pub fn new() -> Result<Self, toml::de::Error> {
        load_or_create_config(THEME_FILE)
    }
}

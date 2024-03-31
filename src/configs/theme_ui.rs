use crate::configs::{load_or_create_config, THEME_UI};
use ratatui::style::Color;
use serde::Serialize;
use serde_json::{Map, Value};
use std::str::FromStr;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UITheme {
    pub footer_background: Color,
}

impl<'de> serde::Deserialize<'de> for UITheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Object(map) => {
                Ok(Self { footer_background: pull_color(&map, "footerBackground").map_err(serde::de::Error::custom)? })
            }
            _ => Err(anyhow::anyhow!("theme.json in not an Object!")).map_err(serde::de::Error::custom),
        }
    }
}

impl Default for UITheme {
    fn default() -> Self {
        Self { footer_background: Color::Rgb(25, 25, 24) }
    }
}

impl UITheme {
    pub fn new() -> Result<Self, serde_json::Error> {
        load_or_create_config(THEME_UI)
    }
}

pub fn pull_color(map: &Map<String, Value>, key: &str) -> Result<Color, String> {
    match map.get(key) {
        Some(obj) => parse_color(obj.clone()),
        None => Err(format!("Key not in object {key}")),
    }
}

pub fn parse_color(obj: Value) -> Result<Color, String> {
    match obj {
        Value::String(data) => Color::from_str(&data).map_err(|e| e.to_string()),
        Value::Object(map) => {
            if let Some(data) = map.get("rgb").or(map.get("Rgb").or(map.get("RGB"))) {
                if let Value::Array(rgb) = data {
                    if rgb.len() == 3 {
                        let b = object_to_u8(rgb[2].clone());
                        let g = object_to_u8(rgb[1].clone());
                        let r = object_to_u8(rgb[0].clone());
                        if r.is_some() && g.is_some() && b.is_some() {
                            return Ok(Color::Rgb(r.unwrap(), g.unwrap(), b.unwrap()));
                        };
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

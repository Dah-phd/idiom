use crate::configs::{load_or_create_config, THEME_FILE};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

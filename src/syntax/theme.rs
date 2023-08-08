use serde::{Deserialize, Serialize};
use tui::style::Color;

use crate::configs::load_or_create_config;

pub const DEFAULT_THEME_FILE: &str = "default_theme.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    pub key_words: Color,
    pub flow_control: Color,
    pub class_or_struct: Color,
    pub functions: Color,
    pub blank: Color,
    pub default: Color,
    pub selected: Color,
    pub string: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            key_words: Color::Rgb(79, 106, 214),
            flow_control: Color::LightMagenta,
            class_or_struct: Color::Rgb(112, 199, 176),
            default: Color::Rgb(108, 149, 214),
            functions: Color::Rgb(218, 223, 170),
            blank: Color::White,
            selected: Color::Rgb(72, 72, 72),
            string: Color::Yellow,
        }
    }
}

impl From<&String> for Theme {
    fn from(path: &String) -> Self {
        Self::from_path(path)
    }
}

impl Theme {
    pub fn from_path(path: &str) -> Self {
        load_or_create_config(path)
    }
}

use crate::configs::load_or_create_config;
use crate::configs::THEME_UI;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UITheme {
    pub footer_background: Color,
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

use crate::configs::load_or_create_config;
use crate::configs::THEME_UI;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UITheme {
    pub footer_background: Color,
}

impl Default for UITheme {
    fn default() -> Self {
        Self { footer_background: Color::Rgb(25, 25, 24) }
    }
}

impl UITheme {
    pub fn new() -> Self {
        load_or_create_config(THEME_UI)
    }
}

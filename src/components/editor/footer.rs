use ratatui::{
    style::{Modifier, Stylize},
    widgets::Paragraph,
};

pub struct Footer {}

impl Footer {
    pub fn widget(&mut self) -> Paragraph {
        Paragraph::new("Hello footer")
    }
}

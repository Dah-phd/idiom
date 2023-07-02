use crossterm::event::{KeyEvent, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::messages::PopupMessage;
pub mod editor_popups;

#[derive(Debug, Default, Clone)]
pub struct Popup {
    pub message: String,
    pub buttons: Vec<Button>,
    pub size: Option<(u16, u16)>,
}

impl Popup {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>) {
        let block = Block::default().title("Propmpt").borders(Borders::ALL);
        let (percent_x, percent_y) = self.size.unwrap_or((60, 20));
        let area = centered_rect(percent_x, percent_y, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    pub fn map(&mut self, key: &KeyEvent) -> PopupMessage {
        if let Some(button) = self.buttons.iter().find(|button| 
            matches!(&button.key, Some(key_code) if key_code.contains(&key.code))                    
        ) {
            return (button.command)();
        }
        PopupMessage::None
    }
}

#[derive(Clone)]
pub struct Button {
    pub command: fn() -> PopupMessage,
    pub name: String,
    pub key: Option<Vec<KeyCode>>
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").field(
            &self.name
        ).finish()
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

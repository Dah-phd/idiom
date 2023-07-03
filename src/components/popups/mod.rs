use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::messages::PopupMessage;
pub mod editor_popups;

#[derive(Debug, Default, Clone)]
pub struct Popup {
    pub message: String,
    pub buttons: Vec<Button>,
    pub size: Option<(u16, u16)>,
    pub state: usize,
}

impl Popup {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>) {
        let block = Block::default().title("Propmpt").borders(Borders::ALL);
        let (percent_x, percent_y) = self.size.unwrap_or((60, 20));
        let area = centered_rect(percent_x, percent_y, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .margin(2)
            .split(area);

        frame.render_widget(
            Paragraph::new(Span::from(self.message.to_owned())).alignment(Alignment::Center),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(Spans::from(self.spans_from_buttons())).alignment(Alignment::Center),
            chunks[1],
        );
    }

    pub fn map(&mut self, key: &KeyEvent) -> PopupMessage {
        if let Some(button) = self
            .buttons
            .iter()
            .find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            return (button.command)();
        }

        match key.code {
            KeyCode::Enter => (self.buttons[self.state].command)(),
            KeyCode::Left => {
                if self.state > 0 {
                    self.state -= 1;
                } else {
                    self.state = self.buttons.len() - 1;
                }
                PopupMessage::None
            }
            KeyCode::Right => {
                if self.state < self.buttons.len() - 1 {
                    self.state += 1;
                } else {
                    self.state = 0;
                }
                PopupMessage::None
            }
            KeyCode::Esc => PopupMessage::Done,
            _ => PopupMessage::None,
        }
    }

    fn spans_from_buttons(&self) -> Vec<Span<'_>> {
        self.buttons
            .iter()
            .enumerate()
            .map(|(idx, button)| {
                let padded_name = format!("  {}  ", button.name);
                if self.state == idx {
                    Span::styled(padded_name, Style::default().add_modifier(Modifier::REVERSED))
                } else {
                    Span::raw(padded_name)
                }
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct Button {
    pub command: fn() -> PopupMessage,
    pub name: String,
    pub key: Option<Vec<KeyCode>>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").field(&self.name).finish()
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

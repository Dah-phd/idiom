use std::io::Stdout;

use super::PopupInterface;
use crate::configs::PopupMessage;
use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct Popup {
    pub message: String,
    pub title: Option<String>,
    pub message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    pub buttons: Vec<Button>,
    pub size: Option<(u16, u16)>,
    pub state: usize,
}

impl PopupInterface for Popup {
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        let block = Block::default().title(self.title()).borders(Borders::ALL);
        let (percent_x, percent_y) = self.size.unwrap_or((60, 20));
        let area = centered_rect(percent_x, percent_y, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .margin(1)
            .split(area);

        frame.render_widget(self.p_from_message(), chunks[0]);

        frame.render_widget(self.spans_from_buttons(), chunks[1]);
    }

    fn map(&mut self, key: &KeyEvent) -> PopupMessage {
        if let Some(button) =
            self.buttons.iter().find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            return (button.command)(self);
        }

        match key.code {
            KeyCode::Char(ch) if self.message_as_buffer_builder.is_some() => {
                if let Some(buffer_builder) = self.message_as_buffer_builder {
                    if let Some(ch) = buffer_builder(ch) {
                        self.message.push(ch);
                    }
                }
                PopupMessage::None
            }
            KeyCode::Backspace if self.message_as_buffer_builder.is_some() => {
                self.message.pop();
                PopupMessage::None
            }
            KeyCode::Enter => (self.buttons[self.state].command)(self),
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
}

impl Popup {
    fn p_from_message(&self) -> Paragraph {
        if self.message_as_buffer_builder.is_none() {
            return Paragraph::new(Span::from(self.message.to_owned())).alignment(Alignment::Center);
        }
        Paragraph::new(Spans::from(vec![
            Span::raw(" >> "),
            Span::raw(self.message.to_owned()),
            Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]))
    }

    fn title(&self) -> String {
        if let Some(title) = &self.title {
            return format!("{title} ");
        }
        "Prompt".to_owned()
    }

    fn spans_from_buttons(&self) -> Paragraph<'_> {
        Paragraph::new(Spans::from(
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
                .collect::<Vec<_>>(),
        ))
        .alignment(Alignment::Center)
    }
}

pub struct PopupSelector<T> {
    pub options: Vec<T>,
    pub display: fn(&T) -> String,
    pub command: fn(&mut PopupSelector<T>) -> PopupMessage,
    pub state: usize,
    pub size: Option<(u16, u16)>,
}

impl<T> PopupInterface for PopupSelector<T> {
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>) {
        let block = Block::default().title("Select").borders(Borders::ALL);
        let (percent_x, percent_y) = self.size.unwrap_or((60, 20));
        let area = centered_rect(percent_x, percent_y, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let chunks = Layout::default().margin(1).split(area);
        let mut state = ListState::default();
        state.select(Some(self.state));
        let options = self.options.iter().map(|el| ListItem::new((self.display)(el))).collect::<Vec<_>>();
        frame.render_stateful_widget(List::new(options), chunks[0], &mut state);
    }

    fn map(&mut self, key: &KeyEvent) -> PopupMessage {
        match key.code {
            KeyCode::Enter => (self.command)(self),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                if self.state > 0 {
                    self.state -= 1;
                } else {
                    self.state = self.options.len() - 1;
                }
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                if self.state < self.options.len() - 1 {
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
}

#[derive(Clone)]
pub struct Button {
    pub command: fn(&mut Popup) -> PopupMessage,
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

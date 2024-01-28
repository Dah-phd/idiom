use crate::workspace::DocStats;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

const MSG_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Footer {
    clock: Instant,
    message: Option<Message>,
    message_que: Vec<Message>,
}

impl Default for Footer {
    fn default() -> Self {
        Self { clock: Instant::now(), message: None, message_que: Vec::new() }
    }
}

impl Footer {
    pub fn render_with_remainder(
        &mut self,
        frame: &mut Frame,
        screen: Rect,
        mode: Span<'static>,
        stats: Option<DocStats>,
    ) -> Rect {
        let layout = footer_render_area(screen);
        let footer_screen = layout[1];
        let editor_screen = layout[0];

        let message_p = self.get_message_widget().unwrap_or_default();
        let (stat_size, stat_p) = if let Some((len, sel, c)) = stats {
            let text = match sel {
                0 => format!("    Doc Len {len}, Ln {}, Col {}", c.line + 1, c.char + 1),
                _ => format!("    Doc Len {len}, Ln {}, Col {} ({sel} selected)", c.line + 1, c.char + 1),
            };
            (text.len(), Paragraph::new(Span::raw(text)))
        } else {
            (0, Paragraph::default())
        };
        let split = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(12),
                Constraint::Length(footer_screen.width.saturating_sub(12 + stat_size as u16)),
                Constraint::Length(stat_size as u16),
            ],
        )
        .split(footer_screen);
        frame.render_widget(Paragraph::new(mode).block(Block::new().borders(Borders::TOP)), split[0]);
        frame.render_widget(message_p.block(Block::new().borders(Borders::TOP)), split[1]);
        frame.render_widget(stat_p.block(Block::new().borders(Borders::TOP)), split[2]);
        editor_screen
    }

    pub fn message(&mut self, message: String) {
        if self.message.is_none() && self.message_que.is_empty() {
            self.message.replace(Message::msg(message));
        } else {
            self.message_que.push(Message::msg(message));
        }
    }

    pub fn error(&mut self, message: String) {
        self.push_ahead(Message::err(message));
    }

    pub fn success(&mut self, message: String) {
        self.push_ahead(Message::success(message));
    }

    fn push_ahead(&mut self, msg: Message) {
        self.message_que.retain(|m| m.is_err());
        self.message_que.push(msg);
        if matches!(&self.message, Some(maybe_err) if !maybe_err.is_err()) {
            self.message = None;
        }
    }

    fn get_message_widget(&mut self) -> Option<Paragraph<'static>> {
        if self.message.is_none() && self.message_que.is_empty() {
            return None;
        }
        self.que_pull_if_expaired();
        self.message.as_ref().map(|m| m.widget())
    }

    fn que_pull_if_expaired(&mut self) {
        if self.message.is_some() && self.clock.elapsed() <= MSG_DURATION {
            return;
        }
        match self.message_que.len() {
            0 => self.message = None,
            1..=3 => {
                self.message.replace(self.message_que.remove(0));
            }
            _ => {
                self.message_que = self.message_que.drain(..).rev().take(3).rev().collect();
            }
        }
        self.clock = Instant::now();
    }
}

#[derive(Debug)]
enum Message {
    Text(Paragraph<'static>),
    Success(Paragraph<'static>),
    Error(Paragraph<'static>),
}

impl Message {
    fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    fn widget(&self) -> Paragraph<'static> {
        match self {
            Self::Error(span) => span,
            Self::Text(span) => span,
            Self::Success(span) => span,
        }
        .clone()
    }

    fn msg(message: String) -> Self {
        Self::Text(
            Paragraph::new(Span::raw(message))
                .block(Block::default().borders(Borders::TOP).padding(Padding::horizontal(2))),
        )
    }

    fn success(message: String) -> Self {
        Self::Success(
            Paragraph::new(Span::styled(message, Style { fg: Some(Color::Blue), ..Default::default() }))
                .block(Block::default().borders(Borders::TOP).padding(Padding::horizontal(2))),
        )
    }

    fn err(message: String) -> Self {
        Self::Error(
            Paragraph::new(Span::styled(message, Style { fg: Some(Color::Red), ..Default::default() }))
                .block(Block::default().borders(Borders::TOP).padding(Padding::horizontal(2))),
        )
    }
}

pub fn footer_render_area(screen: Rect) -> Rc<[Rect]> {
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(screen.height.saturating_sub(2)),
            Constraint::Length(1),
        ],
    )
    .split(screen)
}

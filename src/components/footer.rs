use crate::{components::workspace::DocStats, configs::Mode};
use anyhow::Result;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

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
        mode: &Mode,
        stats: Option<DocStats>,
    ) -> Rect {
        let layout = Layout::default()
            .constraints([
                Constraint::Length(screen.height.checked_sub(2).unwrap_or_default()),
                Constraint::Length(1),
            ])
            .split(screen);
        let area = self.message_with_remainder(frame, layout[1]);
        frame.render_widget(self.widget_stats(mode, stats), area);
        layout[0]
    }

    pub fn logged_ok<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(val) => Some(val),
            Err(err) => {
                self.error(err.to_string());
                None
            }
        }
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

    fn message_with_remainder(&mut self, frame: &mut Frame, layout: Rect) -> Rect {
        self.get_message();
        if let Some(message) = self.message.as_ref() {
            let paragraph_areas = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout);
            frame.render_widget(message.widget(), paragraph_areas[0]);
            return paragraph_areas[1];
        }
        layout
    }

    fn widget_stats(&mut self, mode: &Mode, stats: Option<DocStats>) -> Paragraph {
        Paragraph::new(Line::from(
            stats
                .map(|(len, select, c)| {
                    vec![
                        Span::raw(match select {
                            0 => format!("    Doc Len {len}, Ln {}, Col {}", c.line, c.char),
                            _ => format!("    Doc Len {len}, Ln {}, Col {} ({select} selected)", c.line, c.char),
                        }),
                        Span::from(mode),
                    ]
                })
                .unwrap_or_default(),
        ))
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::TOP))
    }

    fn push_ahead(&mut self, msg: Message) {
        self.message_que.retain(|m| m.is_err());
        self.message_que.push(msg);
        if matches!(&self.message, Some(maybe_err) if !maybe_err.is_err()) {
            self.message = None;
        }
    }

    fn get_message(&mut self) {
        if self.message.is_none() && self.message_que.is_empty() {
            return;
        }
        self.que_pull_if_expaired();
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
    Plain(Paragraph<'static>),
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
            Self::Plain(span) => span,
            Self::Success(span) => span,
        }
        .clone()
    }

    fn msg(message: String) -> Self {
        Self::Plain(
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

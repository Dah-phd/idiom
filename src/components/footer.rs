use crate::{components::workspace::DocStats, configs::Mode};
use anyhow::Result;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
        let widget = self.widget_with_stats(mode, stats);
        let layout = Layout::default()
            .constraints([
                Constraint::Length(screen.height.checked_sub(2).unwrap_or_default()),
                Constraint::Length(1),
            ])
            .split(screen);
        frame.render_widget(widget, layout[1]);
        layout[0]
    }

    fn widget_with_stats(&mut self, mode: &Mode, stats: Option<DocStats>) -> Paragraph {
        let mut line = self.get_message();
        if let Some((doc_len, selected, cur)) = stats {
            line.push(Span::raw(match selected {
                0 => format!("    Doc Len {doc_len}, Ln {}, Col {}", cur.line, cur.char),
                _ => format!("    Doc Len {doc_len}, Ln {}, Col {} ({selected} selected)", cur.line, cur.char),
            }));
        }
        line.push(Span::from(mode));
        Paragraph::new(Line::from(line)).alignment(Alignment::Right).block(Block::default().borders(Borders::TOP))
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

    fn push_ahead(&mut self, msg: Message) {
        self.message_que.retain(|m| m.is_err());
        self.message_que.push(msg);
        if matches!(&self.message, Some(maybe_err) if !maybe_err.is_err()) {
            self.message = None;
        }
    }

    fn get_message(&mut self) -> Vec<Span<'static>> {
        if self.message.is_none() && self.message_que.is_empty() {
            return Vec::new();
        }
        self.que_pull_if_expaired();
        self.message.as_ref().map(|m| m.vec()).unwrap_or_default()
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
    Plain(Span<'static>),
    Success(Span<'static>),
    Error(Span<'static>),
}

impl Message {
    fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    fn vec(&self) -> Vec<Span<'static>> {
        vec![match self {
            Self::Error(span) => span,
            Self::Plain(span) => span,
            Self::Success(span) => span,
        }
        .clone()]
    }

    fn msg(message: String) -> Self {
        Self::Plain(Span::raw(message))
    }

    fn success(message: String) -> Self {
        Self::Success(Span::styled(message, Style { fg: Some(Color::Blue), ..Default::default() }))
    }

    fn err(message: String) -> Self {
        Self::Error(Span::styled(message, Style { fg: Some(Color::Red), ..Default::default() }))
    }
}

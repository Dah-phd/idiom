use crate::render::{
    backend::{color, Backend, Style},
    layout::Line,
};
use std::{
    io::Result,
    time::{Duration, Instant},
};

const MSG_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Messages {
    clock: Instant,
    message: Option<Message>,
    message_que: Vec<Message>,
    pub line: Line,
}

impl Messages {
    pub fn new() -> Self {
        Self { clock: Instant::now(), message: None, message_que: Vec::new(), line: Line::default() }
    }

    pub fn render(&mut self, mut accent_style: Style, backend: &mut Backend) -> Result<()> {
        if self.message.is_some() || !self.message_que.is_empty() {
            let line = self.line.clone();
            self.que_pull_if_expaired();
            match self.message.as_ref() {
                Some(Message::Error(text)) => {
                    accent_style.set_fg(Some(color::red()));
                    line.render_styled(text, accent_style, backend)
                }
                Some(Message::Success(text)) => {
                    accent_style.set_fg(Some(color::blue()));
                    line.render_styled(text, accent_style, backend)
                }
                Some(Message::Text(text)) => line.render_styled(text, accent_style, backend),
                None => return Ok(()),
            }?;
        }
        Ok(())
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
    Text(String),
    Success(String),
    Error(String),
}

impl Message {
    fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    fn msg(message: String) -> Self {
        Self::Text(message)
    }

    fn success(message: String) -> Self {
        Self::Success(message) //, Style { fg: Some(Color::Blue), ..Default::default() }))
    }

    fn err(message: String) -> Self {
        Self::Error(message) //, Style { fg: Some(Color::Red), ..Default::default() }))
    }
}

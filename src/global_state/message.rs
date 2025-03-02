use crate::render::{
    backend::{Backend, BackendProtocol, StyleExt},
    layout::Line,
};
use crossterm::style::{Color, ContentStyle};
use std::{
    collections::VecDeque,
    error::Error,
    time::{Duration, Instant},
};

const MSG_DURATION: Duration = Duration::from_secs(2);
const ERR_LOG_LIMIT: usize = 100;

#[derive(Debug)]
pub struct Messages {
    clock: Instant,
    active: bool,
    messages: Vec<Message>,
    last_message: Message,
    line: Line,
    error_log: VecDeque<String>,
}

impl Messages {
    pub fn new() -> Self {
        Self {
            clock: Instant::now() - MSG_DURATION,
            active: false,
            messages: Vec::new(),
            last_message: Message::empty(),
            line: Line::empty(),
            error_log: VecDeque::new(),
        }
    }

    pub fn render(&mut self, accent_style: ContentStyle, backend: &mut Backend) {
        if self.is_expaired() {
            match self.messages.pop() {
                Some(message) => {
                    self.last_message = message;
                    self.clock = Instant::now();
                    self.last_message.render(self.line.clone(), accent_style, backend);
                }
                None => {
                    self.active = false;
                    backend.set_style(accent_style);
                    self.line.clone().render_empty(backend);
                    backend.reset_style()
                }
            }
        } else {
            self.last_message.render(self.line.clone(), accent_style, backend);
        }
    }

    pub fn fast_render(&mut self, accent_style: ContentStyle, backend: &mut Backend) {
        if !self.active {
            return;
        }
        self.render(accent_style, backend);
    }

    pub fn get_logs(&self) -> impl Iterator<Item = &String> {
        self.error_log.iter()
    }

    pub fn set_line(&mut self, line: Line) {
        if line.width != self.line.width || line.col != self.line.col {
            self.active = true;
            self.line = line;
        }
    }

    pub fn message(&mut self, message: String) {
        self.messages.insert(0, Message::msg(message));
        self.active = true;
    }

    pub fn error(&mut self, error: String) {
        self.error_log.push_back(error.clone());
        if self.error_log.len() > ERR_LOG_LIMIT {
            self.error_log.pop_front();
        }
        if let Some(msg) = Message::err(error) {
            self.push_ahead(msg);
        }
    }

    pub fn success(&mut self, message: String) {
        self.push_ahead(Message::success(message));
    }

    #[inline]
    pub fn unwrap_or_default<T: Default, E: Error>(&mut self, result: std::result::Result<T, E>, prefix: &str) -> T {
        match result {
            Ok(value) => value,
            Err(err) => {
                if let Some(first_line) = err.to_string().lines().next() {
                    self.push_ahead(Message::Error(format!("{prefix} (run with defaults): {first_line}")));
                }
                T::default()
            }
        }
    }

    fn push_ahead(&mut self, message: Message) {
        self.messages.retain(|m| m.is_err());
        self.messages.insert(0, message);
        self.active = true;
    }

    #[inline]
    fn is_expaired(&self) -> bool {
        self.clock.elapsed() > MSG_DURATION
    }
}

#[derive(Debug)]
enum Message {
    Text(String),
    Success(String),
    Error(String),
}

impl Message {
    #[inline]
    fn render(&self, line: Line, mut accent_style: ContentStyle, backend: &mut Backend) {
        match self {
            Message::Error(text) => {
                accent_style.set_fg(Some(Color::Red));
                line.render_styled(text, accent_style, backend)
            }
            Message::Success(text) => {
                accent_style.set_fg(Some(Color::Blue));
                line.render_styled(text, accent_style, backend)
            }
            Message::Text(text) => line.render_styled(text, accent_style, backend),
        };
    }

    const fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    const fn empty() -> Self {
        Self::msg(String::new())
    }

    const fn msg(message: String) -> Self {
        Self::Text(message)
    }

    const fn success(message: String) -> Self {
        Self::Success(message)
    }

    fn err(error: String) -> Option<Self> {
        let first_line = error.lines().next()?.to_owned();
        Some(Self::Error(first_line))
    }
}

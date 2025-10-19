use crate::ext_tui::{CrossTerm, StyleExt};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{layout::Line, Backend, UTFSafe};
use std::{
    collections::VecDeque,
    error::Error,
    time::{Duration, Instant},
};

const MSG_DURATION: Duration = Duration::from_secs(2);
const ERR_LOG_LIMIT: usize = 100;
const LIMIT_MSG_LIST: usize = 3;

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

    pub fn render(&mut self, accent_style: ContentStyle, backend: &mut CrossTerm) {
        if self.is_expaired() {
            self.limit_message_que();
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

    #[inline]
    pub fn should_render(&self) -> bool {
        self.active
    }

    pub fn fast_render(&mut self, accent_style: ContentStyle, backend: &mut CrossTerm) {
        if !self.active {
            return;
        }
        self.render(accent_style, backend);
    }

    pub fn get_logs(&self) -> impl Iterator<Item = &String> {
        self.error_log.iter()
    }

    pub fn set_line(&mut self, line: Line) {
        if self.line == line {
            return;
        }
        self.active = true;
        self.line = line;
    }

    pub fn message(&mut self, message: String) {
        if let Some(msg) = Message::msg(message) {
            self.messages.insert(0, msg);
            self.active = true;
        }
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
        if self.error_log.len() > ERR_LOG_LIMIT {
            self.error_log.pop_front();
        }
        if let Some(msg) = Message::success(message) {
            self.push_ahead(msg);
        }
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

    fn limit_message_que(&mut self) {
        if self.messages.len() > LIMIT_MSG_LIST {
            self.messages.retain(|m| m.is_err());
            self.messages.truncate(LIMIT_MSG_LIST);
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
    fn render(&self, line: Line, mut accent_style: ContentStyle, backend: &mut CrossTerm) {
        let Line { width, row, col } = line;

        let (color, text) = match self {
            Message::Error(text) => (Some(Color::Red), text.as_str()),
            Message::Success(text) => (Some(Color::Blue), text.as_str()),
            Message::Text(text) => (None, text.as_str()),
        };

        let (pad_width, text) = text.truncate_width(width - 2);
        let reset_style = backend.get_style();
        accent_style.set_fg(color);
        backend.set_style(accent_style);
        backend.go_to(row, col);
        backend.pad(2);
        backend.print(text);
        if pad_width != 0 {
            backend.pad(pad_width);
        }
        backend.set_style(reset_style);
    }

    const fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    const fn empty() -> Self {
        Self::Text(String::new())
    }

    fn msg(message: String) -> Option<Self> {
        let first_line = message.lines().next()?.to_owned();
        Some(Self::Text(first_line))
    }

    fn success(message: String) -> Option<Self> {
        let first_line = message.lines().next()?.to_owned();
        Some(Self::Success(first_line))
    }

    fn err(error: String) -> Option<Self> {
        let first_line = error.lines().next()?.to_owned();
        Some(Self::Error(first_line))
    }
}

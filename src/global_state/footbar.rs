use crate::{
    editor::EditorStats,
    ext_tui::{CrossTerm, StyleExt},
};
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
pub struct FootBar {
    pub line: Line,
    stats: Option<DocumentStats>,
    message: Option<Message>,
    messages: Vec<Message>,
    error_log: VecDeque<String>,
    active: bool,
    clock: Instant,
}

impl FootBar {
    pub fn new() -> Self {
        Self {
            stats: Default::default(),
            message: Default::default(),
            messages: Default::default(),
            line: Default::default(),
            error_log: Default::default(),
            active: Default::default(),
            clock: Instant::now() - Duration::from_secs(4),
        }
    }

    pub fn fast_render(&mut self, stats: Option<EditorStats>, accent_style: ContentStyle, backend: &mut CrossTerm) {
        match (self.stats.as_ref(), stats.as_ref()) {
            (Some(doc_stats), Some(new_stats)) if &doc_stats.stats == new_stats => {}
            (None, None) => {}
            _ => {
                return self.render(stats, accent_style, backend);
            }
        }
        if !self.active {
            return;
        }
        if !self.is_expaired() {
            return;
        };
        self.next_message();
        self.force_rerender(accent_style, backend);
    }

    #[inline]
    pub fn render(&mut self, stats: Option<EditorStats>, accent_style: ContentStyle, backend: &mut CrossTerm) {
        self.stats = stats.map(DocumentStats::new);
        if self.is_expaired() {
            self.next_message();
        }
        self.force_rerender(accent_style, backend);
    }

    pub fn force_rerender(&mut self, accent_style: ContentStyle, backend: &mut CrossTerm) {
        backend.set_style(accent_style);
        backend.go_to(self.line.row, self.line.col);
        backend.clear_to_eol();
        let line = match self.stats.as_ref() {
            None => self.line.clone(),
            Some(stats) => stats.render_with_remainder(self.line.clone(), backend),
        };
        if let Some(msg) = self.message.as_ref() {
            msg.render(line, accent_style, backend);
        }
        backend.reset_style();
    }

    pub fn get_logs(&self) -> impl Iterator<Item = &String> {
        self.error_log.iter()
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

    fn next_message(&mut self) {
        self.limit_message_que();
        match self.messages.pop() {
            Some(message) => {
                self.message.replace(message);
                self.active = true;
                self.clock = Instant::now();
            }
            None => {
                self.message = None;
                self.active = false;
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
pub struct DocumentStats {
    text: String,
    stats: EditorStats,
}

impl DocumentStats {
    fn new(stats: EditorStats) -> Self {
        let text = match stats.select_len != 0 {
            true => format!(
                "  Doc Len {}, Ln {}, Col {} ({} selected) ",
                stats.len, stats.position.line, stats.position.char, stats.select_len,
            ),
            false => format!("  Doc Len {}, Ln {}, Col {} ", stats.len, stats.position.line, stats.position.char,),
        };
        Self { text, stats }
    }

    fn render_with_remainder(&self, mut line: Line, backend: &mut CrossTerm) -> Line {
        match line.width < self.text.len() {
            true => {
                line.width = 0;
                backend.print(&self.text[(self.text.len() - line.width)..]);
            }
            false => {
                line.width -= self.text.len();
                backend.go_to(line.row, line.col + line.width as u16);
                backend.print(&self.text);
            }
        }
        line
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

        let (.., text) = text.truncate_width(width - 2);
        accent_style.set_fg(color);
        backend.go_to(row, col);
        backend.pad(2);
        backend.print_styled(text, accent_style);
    }

    const fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
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

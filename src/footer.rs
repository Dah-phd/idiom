use crate::{configs::UITheme, global_state::GlobalState, workspace::DocStats};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, WidgetRef},
    Frame,
};
use std::time::{Duration, Instant};

const MSG_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Footer {
    clock: Instant,
    message: Option<Message>,
    message_que: Vec<Message>,
    color: Color,
}

impl Footer {
    pub fn new(gs: &mut GlobalState) -> Self {
        let theme = gs.unwrap_default_result(UITheme::new(), "theme_ui.josn: ");
        Self { clock: Instant::now(), message: None, message_que: Vec::new(), color: theme.footer_background }
    }

    pub fn render(&mut self, frame: &mut Frame, gs: &GlobalState, stats: Option<DocStats>) {
        frame.render_widget(Block::default().bg(self.color), gs.footer_area);

        let (stat_size, stat_p) = if let Some((len, sel, c)) = stats {
            let text = match sel {
                0 => format!("    Doc Len {len}, Ln {}, Col {}", c.line + 1, c.char + 1),
                _ => format!("    Doc Len {len}, Ln {}, Col {} ({sel} selected)", c.line + 1, c.char + 1),
            };
            (text.len(), Span::raw(text))
        } else {
            (0, Span::default())
        };

        let split = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(15),
                Constraint::Length(gs.footer_area.width.saturating_sub(15 + stat_size as u16)),
                Constraint::Length(stat_size as u16),
            ],
        )
        .split(gs.footer_area);

        frame.render_widget(gs.mode_span.clone(), split[0]);

        if self.message.is_some() || !self.message_que.is_empty() {
            self.que_pull_if_expaired();
            if let Some(msg) = &self.message {
                frame.render_widget_ref(msg, split[1]);
            }
        }

        frame.render_widget(stat_p, split[2]);
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

    pub fn reset_cfg(&mut self) -> Result<(), serde_json::Error> {
        let new_theme = UITheme::new()?;
        self.color = new_theme.footer_background;
        Ok(())
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
    Text(Span<'static>),
    Success(Span<'static>),
    Error(Span<'static>),
}

impl WidgetRef for &Message {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        match self {
            Message::Text(w) => w.render_ref(area, buf),
            Message::Error(w) => w.render_ref(area, buf),
            Message::Success(w) => w.render_ref(area, buf),
        }
    }
}

impl Message {
    fn is_err(&self) -> bool {
        matches!(self, Self::Error(..))
    }

    fn msg(message: String) -> Self {
        Self::Text(Span::raw(message))
    }

    fn success(message: String) -> Self {
        Self::Success(Span::styled(message, Style { fg: Some(Color::Blue), ..Default::default() }))
    }

    fn err(message: String) -> Self {
        Self::Error(Span::styled(message, Style { fg: Some(Color::Red), ..Default::default() }))
    }
}

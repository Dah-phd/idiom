use crate::{components::workspace::DocStats, configs::Mode};
use anyhow::Result;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

const MSG_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Footer {
    clock: Instant,
    message: String,
    message_que: Vec<String>,
}

impl Default for Footer {
    fn default() -> Self {
        Self { clock: Instant::now(), message: String::new(), message_que: Vec::new() }
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
        let mut line = vec![Span::raw(self.get_message())];
        if let Some((doc_len, selected, cur)) = stats {
            line.push(Span::raw(match selected {
                0 => format!("    Doc Len {doc_len}, Ln {}, Col {}", cur.line, cur.char),
                _ => format!("    Doc Len {doc_len}, Ln {}, Col {} ({selected} selected)", cur.line, cur.char),
            }));
        }
        line.push(Span::from(mode));
        Paragraph::new(Line::from(line)).alignment(Alignment::Right).block(Block::default().borders(Borders::TOP))
    }

    pub fn message(&mut self, message: String) {
        if self.message.is_empty() && self.message_que.is_empty() {
            self.message = message;
        } else {
            self.message_que.push(message);
        }
    }

    pub fn error(&mut self, error: String) {}

    pub fn logged_if_error<T>(&mut self, result: Result<T>) -> bool {
        if let Err(error) = result {
            self.overwrite(error.to_string());
        };
        false
    }

    pub fn overwrite(&mut self, message: String) {
        self.message = message;
        self.clock = Instant::now();
    }

    fn get_message(&mut self) -> &str {
        if self.message.is_empty() && self.message_que.is_empty() || self.clock.elapsed() <= MSG_DURATION {
            return &self.message;
        }
        self.message.clear();
        if !self.message_que.is_empty() {
            self.message = self.message_que.remove(0);
            self.clock = Instant::now();
        }
        &self.message
    }
}

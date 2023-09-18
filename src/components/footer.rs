use crate::components::editor::CursorPosition;
use ratatui::{
    backend::Backend,
    layout::Rect,
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
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, screen: Rect, stats: Option<(usize, &CursorPosition)>) {
        let widget = self.widget_with_stats(stats);
        frame.render_widget(widget, screen);
    }

    fn widget_with_stats(&mut self, stats: Option<(usize, &CursorPosition)>) -> Paragraph {
        let line = if let Some((selected, cursor)) = stats {
            if selected == 0 {
                format!("{}    Ln {}, Col {}", self.get_message(), cursor.line, cursor.char)
            } else {
                format!("{}    Ln {}, Col {} ({} selected)", self.get_message(), cursor.line, cursor.char, selected)
            }
        } else {
            String::from(self.get_message())
        };
        Paragraph::new(line).alignment(ratatui::prelude::Alignment::Right).block(Block::default().borders(Borders::TOP))
    }

    pub fn message(&mut self, message: String) {
        self.message_que.push(message);
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

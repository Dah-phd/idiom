use crate::components::editor::DocStats;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Rect},
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
    pub fn render(&mut self, frame: &mut Frame<impl Backend>, screen: Rect, stats: Option<DocStats>) {
        let widget = self.widget_with_stats(stats);
        frame.render_widget(widget, screen);
    }

    fn widget_with_stats(&mut self, stats: Option<DocStats>) -> Paragraph {
        let line = if let Some((doc_len, selected, cur)) = stats {
            let msg = self.get_message();
            match selected {
                0 => format!("{msg}    Doc Len {doc_len}, Ln {}, Col {}", cur.line, cur.char),
                _ => format!("{msg}    Doc Len {doc_len}, Ln {}, Col {} ({selected} selected)", cur.line, cur.char),
            }
        } else {
            String::from(self.get_message())
        };
        Paragraph::new(line).alignment(Alignment::Right).block(Block::default().borders(Borders::TOP))
    }

    pub fn message(&mut self, message: String) {
        if self.message.is_empty() && self.message_que.is_empty() {
            self.message = message;
        } else {
            self.message_que.push(message);
        }
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

mod rust_key_words;
mod theme;
pub use rust_key_words::RustSyntax;
use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

use self::theme::Theme;

pub const COLORS: [Color; 3] = [Color::Magenta, Color::Blue, Color::Yellow];


pub trait Linter<T: Default = Self> {
    fn get_token_buffer(&mut self) -> &mut String;

    fn handled_key_word(&mut self) -> Option<Span<'static>>;

    fn process_line(&mut self, content: &str, spans: &mut Vec<Span>);

    fn get_theme(&self) -> &Theme;

    fn linter<'a>(&mut self, idx: usize, content: &'a String, max_digits: usize) -> ListItem<'a> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, max_digits),
            Style::default().fg(Color::Gray),
        )];
        self.process_line(content, &mut spans);
        if !self.get_token_buffer().is_empty() {
            spans.push(self.drain_buf());
        }
        ListItem::new(Spans::from(spans))
    }

    fn white_char(&mut self, ch: char) -> Span<'static> {
        Span::styled(
            String::from(ch),
            Style {
                fg: Some(Color::White),
                ..Default::default()
            },
        )
    }

    fn drain_buf_colored(&mut self, color: Color) -> Span<'static> {
        if let Some(span) = self.handled_key_word() {
            return span;
        }
        Span::styled(
            self.get_token_buffer().drain(..).collect::<String>(),
            Style {
                fg: Some(color),
                ..Default::default()
            },
        )
    }

    fn drain_buf(&mut self) -> Span<'static> {
        if let Some(span) = self.handled_key_word() {
            return span;
        }
        Span::styled(
            self.get_token_buffer().drain(..).collect::<String>(),
            Style::default().fg(self.get_theme().default),
        )
    }
}
fn len_to_color(len: Option<usize>) -> Color {
    if let Some(len) = len {
        COLORS[len % COLORS.len()]
    } else {
        COLORS[COLORS.len() - 1]
    }
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = (idx+1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}

mod theme;
use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

use crate::messages::FileType;

use self::theme::Theme;

pub const COLORS: [Color; 3] = [Color::Magenta, Color::Blue, Color::Yellow];

#[derive(Debug)]
pub struct Lexer {
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    last_token: String,
    key_words: Vec<&'static str>,
    last_key_words: Vec<String>,
    theme: Theme,
}

impl Default for Lexer {
    fn default() -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut"],
            curly: vec![],
            brackets: vec![],
            square: vec![],
            last_token: String::default(),
            last_key_words: vec![],
            theme: Theme::default(),
        }
    }
}

impl Lexer {
    pub fn from_type(file_type: &FileType) -> Self {
        #[allow(clippy::match_single_binding)]
        match file_type {
            _ => Self::default(),
        }
    }

    pub fn max_line_digits_from(len: usize) {}

    fn process_line(&mut self, content: &str, spans: &mut Vec<Span>) {
        if content.starts_with("mod") {}
        if content.starts_with("use") {}
        let char_stream = content.chars();
        for ch in char_stream {
            match ch {
                ' ' => {
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                }
                '.' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch));
                }
                '<' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                '>' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                '(' => {
                    spans.push(self.drain_buf_colored(self.theme.function));
                    let color = len_to_color(Some(self.brackets.len()));
                    self.last_token.push(ch);
                    self.brackets.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                ')' => {
                    let color = if let Some(color) = self.brackets.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                '{' => {
                    spans.push(self.drain_buf());
                    let color = len_to_color(Some(self.curly.len()));
                    self.last_token.push(ch);
                    self.curly.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                '}' => {
                    let color = if let Some(color) = self.curly.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                '[' => {
                    spans.push(self.drain_buf());
                    let color = len_to_color(Some(self.square.len()));
                    self.last_token.push(ch);
                    self.square.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                ']' => {
                    let color = if let Some(color) = self.square.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                ':' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                _ => self.last_token.push(ch),
            }
        }
    }

    pub fn select(&mut self, from: (usize, usize), to: (usize, usize)) {}

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme
    }

    fn handled_key_word(&mut self) -> Option<Span<'static>> {
        if self.key_words.contains(&self.last_token.trim()) {
            self.last_key_words.push(self.last_token.to_owned());
            Some(Span::styled(
                self.last_token.drain(..).collect::<String>(),
                Style {
                    fg: Some(self.theme.kword),
                    ..Default::default()
                },
            ))
        } else {
            None
        }
    }

    pub fn syntax_spans<'a>(&mut self, idx: usize, content: &'a str, max_digits: usize) -> ListItem<'a> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, max_digits),
            Style::default().fg(Color::Gray),
        )];
        self.process_line(content, &mut spans);
        if !self.last_token.is_empty() {
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
            self.last_token.drain(..).collect::<String>(),
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
            self.last_token.drain(..).collect::<String>(),
            Style::default().fg(self.theme.default),
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
    let mut as_str = (idx + 1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}

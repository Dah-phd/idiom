use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

const WHITE_SPACE: char = ' ';
const COLORS: [Color; 3] = [Color::Magenta, Color::Blue, Color::Yellow];

pub struct Linter {
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    last_token: String,
    key_words: Vec<&'static str>,
    last_key_words: Vec<String>,
}

impl Default for Linter {
    fn default() -> Self {
        Self {
            last_key_words: vec![],
            curly: vec![],
            brackets: vec![],
            square: vec![],
            last_token: String::new(),
            key_words: vec!["fn", "pub", "struct", "enum"],
        }
    }
}

impl Linter {
    pub fn linter<'a>(&mut self, idx: usize, content: &'a String, max_digits: usize) -> ListItem<'a> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, max_digits),
            Style::default().fg(Color::Gray),
        )];
        self.process_line(content, &mut spans);
        ListItem::new(Spans::from(spans))
    }

    fn process_line(&mut self, content: &String, spans: &mut Vec<Span>) {
        let char_stream = content.chars();
        for ch in char_stream {
            match ch {
                WHITE_SPACE => {
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
                    spans.push(self.drain_buf_colored(Color::LightYellow));
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
        if !self.last_token.is_empty() {
            spans.push(self.drain_buf());
        }
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
            Style::default().fg(Color::LightBlue),
        )
    }

    fn handled_key_word(&mut self) -> Option<Span<'static>> {
        if self.key_words.contains(&self.last_token.trim()) {
            self.last_key_words.push(self.last_token.to_owned());
            Some(Span::styled(
                self.last_token.drain(..).collect::<String>(),
                Style {
                    fg: Some(Color::Blue),
                    ..Default::default()
                },
            ))
        } else {
            None
        }
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
    let mut as_str = idx.to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, WHITE_SPACE)
    }
    as_str.push(WHITE_SPACE);
    as_str
}
